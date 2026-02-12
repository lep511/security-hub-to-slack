mod errors;

use aws_config::BehaviorVersion;
use aws_sdk_account::types::AlternateContactType;
use aws_sdk_account::Client as AccountClient;
use aws_sdk_organizations::Client as OrganizationsClient;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_sts::Client as StsClient;
use chrono::Local;
use colored::Colorize;
use console::style;
use dialoguer::{Input, Select};
use errors::{
    error_is_access_denied, error_is_not_found, error_is_service_unavailable, error_is_throttling,
    AccountError, AppError, AppResult, BoxError, OrganizationsError, S3Error, StsError,
    ValidationError,
};
use serde_json::{json, Map, Value};
use std::fmt;
use std::time::Instant;

// ============================================================================
// Retry Configuration
// ============================================================================

#[derive(Clone)]
struct RetryConfig {
    max_attempts: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
        }
    }
}

async fn retry_with_backoff<T, F, Fut>(
    config: &RetryConfig,
    operation_name: &str,
    mut operation: F,
) -> Result<T, BoxError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, BoxError>>,
{
    let mut attempt = 0;
    loop {
        attempt += 1;
        match operation().await {
            Ok(result) => return Ok(result),
            Err(err) if attempt < config.max_attempts && error_is_throttling(&err) => {
                let delay = std::cmp::min(
                    config.base_delay_ms * 2u64.pow(attempt - 1),
                    config.max_delay_ms,
                );
                log::warn!(
                    "Throttled on {} (attempt {}/{}), retrying in {}ms",
                    operation_name,
                    attempt,
                    config.max_attempts,
                    delay
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }
            Err(err) if attempt < config.max_attempts && error_is_service_unavailable(&err) => {
                let delay = std::cmp::min(
                    config.base_delay_ms * 2u64.pow(attempt - 1),
                    config.max_delay_ms,
                );
                log::warn!(
                    "Service unavailable for {} (attempt {}/{}), retrying in {}ms",
                    operation_name,
                    attempt,
                    config.max_attempts,
                    delay
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            }
            Err(err) => return Err(err),
        }
    }
}

// ============================================================================
// AWS Operations with Proper Error Handling
// ============================================================================

async fn list_accounts_func(org_client: &OrganizationsClient) -> AppResult<Vec<String>> {
    let mut account_ids: Vec<String> = Vec::new();
    let mut next_token: Option<String> = None;
    let retry_config = RetryConfig::default();

    loop {
        let result = retry_with_backoff(&retry_config, "list_accounts", || {
            let req = if let Some(ref token) = next_token {
                org_client.list_accounts().next_token(token)
            } else {
                org_client.list_accounts()
            };
            async move { req.send().await.map_err(|e| Box::new(e) as BoxError) }
        })
        .await;

        match result {
            Ok(output) => {
                for account in output.accounts() {
                    if let Some(id) = account.id() {
                        account_ids.push(id.to_string());
                    }
                }
                match output.next_token() {
                    Some(token) => next_token = Some(token.to_string()),
                    None => break,
                }
            }
            Err(err) => {
                if error_is_access_denied(&err) {
                    return Err(OrganizationsError::AccessDenied.into());
                }
                if error_is_service_unavailable(&err) {
                    return Err(OrganizationsError::ServiceUnavailable.into());
                }
                return Err(OrganizationsError::ListAccounts {
                    message: err.to_string(),
                    source: Some(err),
                }
                .into());
            }
        }
    }

    log::info!("Found {} accounts in organization", account_ids.len());
    Ok(account_ids)
}

async fn get_account_id(sts_client: &StsClient) -> AppResult<String> {
    let retry_config = RetryConfig::default();

    let result = retry_with_backoff(&retry_config, "get_caller_identity", || async {
        sts_client
            .get_caller_identity()
            .send()
            .await
            .map_err(|e| Box::new(e) as BoxError)
    })
    .await
    .map_err(|err| StsError::GetCallerIdentity {
        message: err.to_string(),
        source: Some(err),
    })?;

    result
        .account()
        .map(|s| s.to_string())
        .ok_or_else(|| StsError::NoAccountId.into())
}

fn parse_contact_type(name: &str) -> AppResult<AlternateContactType> {
    match name.to_uppercase().as_str() {
        "BILLING" => Ok(AlternateContactType::Billing),
        "OPERATIONS" => Ok(AlternateContactType::Operations),
        "SECURITY" => Ok(AlternateContactType::Security),
        other => Err(AppError::UnknownContactType(other.to_string())),
    }
}

fn validate_accounts(accounts: &[String], org_accounts: &[String]) -> AppResult<()> {
    if accounts.is_empty() {
        return Err(ValidationError::NoAccountsProvided.into());
    }

    for id in accounts {
        if id.len() != 12 || !id.chars().all(|c| c.is_ascii_digit()) {
            return Err(ValidationError::InvalidAccountId {
                account_id: id.clone(),
            }
            .into());
        }

        if !org_accounts.contains(id) {
            return Err(ValidationError::AccountNotInOrganization {
                account_id: id.clone(),
            }
            .into());
        }
    }

    Ok(())
}

// ============================================================================
// Operation Result Types
// ============================================================================

#[derive(Debug)]
pub enum OperationOutcome {
    Success,
    PartialSuccess { errors: Vec<AppError> },
    Failure(AppError),
    Cancelled,
}

impl fmt::Display for OperationOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationOutcome::Success => write!(f, "Operation completed successfully"),
            OperationOutcome::PartialSuccess { errors } => {
                write!(f, "Operation completed with {} errors", errors.len())
            }
            OperationOutcome::Failure(err) => write!(f, "Operation failed: {}", err),
            OperationOutcome::Cancelled => write!(f, "Operation cancelled by user"),
        }
    }
}

// ============================================================================
// Core Business Logic Functions
// ============================================================================

async fn get_alternate_contact_for_account(
    account_client: &AccountClient,
    account_id: &str,
    current_account_id: &str,
    contact_type: &AlternateContactType,
    contact_type_name: &str,
) -> Result<Option<Value>, AccountError> {
    let retry_config = RetryConfig::default();

    let result = retry_with_backoff(&retry_config, "get_alternate_contact", || {
        let mut req = account_client
            .get_alternate_contact()
            .alternate_contact_type(contact_type.clone());

        if account_id != current_account_id {
            req = req.account_id(account_id);
        }

        async move { req.send().await.map_err(|e| Box::new(e) as BoxError) }
    })
    .await;

    match result {
        Ok(resp) => {
            if let Some(contact) = resp.alternate_contact() {
                let contact_json = json!({
                    "EmailAddress": contact.email_address().unwrap_or_default(),
                    "Name": contact.name().unwrap_or_default(),
                    "PhoneNumber": contact.phone_number().unwrap_or_default(),
                    "Title": contact.title().unwrap_or_default(),
                });
                Ok(Some(contact_json))
            } else {
                Ok(None)
            }
        }
        Err(err) => {
            if error_is_not_found(&err) {
                return Ok(None);
            }
            if error_is_access_denied(&err) {
                return Err(AccountError::AccessDenied {
                    account_id: account_id.to_string(),
                });
            }
            if error_is_throttling(&err) {
                return Err(AccountError::TooManyRequests);
            }
            Err(AccountError::GetAlternateContact {
                account_id: account_id.to_string(),
                contact_type: contact_type_name.to_string(),
                message: err.to_string(),
                source: Some(err),
            })
        }
    }
}

async fn list_func(
    accounts: &[String],
    current_account_id: &str,
    contact_types: &[String],
    account_client: &AccountClient,
    s3_client: &S3Client,
) -> OperationOutcome {
    let mut alternate_contacts: Map<String, Value> = Map::new();
    let mut errors: Vec<AppError> = Vec::new();

    for account_id in accounts {
        let mut type_map: Map<String, Value> = Map::new();

        for ct_name in contact_types {
            println!(
                "Getting {} alternate contact for {}...",
                ct_name.cyan(),
                account_id.yellow()
            );

            let ct = match parse_contact_type(ct_name) {
                Ok(ct) => ct,
                Err(e) => {
                    errors.push(e);
                    continue;
                }
            };

            match get_alternate_contact_for_account(
                account_client,
                account_id,
                current_account_id,
                &ct,
                ct_name,
            )
            .await
            {
                Ok(Some(contact_json)) => {
                    type_map.insert(ct_name.clone(), contact_json);
                }
                Ok(None) => {
                    type_map.insert(ct_name.clone(), Value::String("Null".into()));
                }
                Err(AccountError::AccessDenied { account_id }) => {
                    eprintln!(
                        "  {} Access denied for account {}",
                        "⚠".yellow(),
                        account_id
                    );
                    errors.push(
                        AccountError::AccessDenied {
                            account_id: account_id.clone(),
                        }
                        .into(),
                    );
                    type_map.insert(ct_name.clone(), Value::String("AccessDenied".into()));
                }
                Err(AccountError::TooManyRequests) => {
                    return OperationOutcome::Failure(AccountError::TooManyRequests.into());
                }
                Err(e) => {
                    log::error!("Error getting contact: {}", e);
                    errors.push(e.into());
                    type_map.insert(ct_name.clone(), Value::String("Error".into()));
                }
            }
        }

        alternate_contacts.insert(account_id.clone(), Value::Object(type_map));
    }

    let full_result = json!({ "AlternateContact": alternate_contacts });

    let export_choice: String = match Input::new()
        .with_prompt("\nDo you want to export the result to an S3 bucket? (y/n)")
        .validate_with(|input: &String| {
            let trimmed = input.trim().to_lowercase();
            if trimmed == "y" || trimmed == "yes" || trimmed == "n" || trimmed == "no" {
                Ok(())
            } else {
                Err(format!("Please enter 'y' for yes or 'n' for no.").red())
            }
        })
        .interact_text()
    {
        Ok(choice) => choice,
        Err(e) => {
            return OperationOutcome::Failure(AppError::UserInput(e.to_string()));
        }
    };

    match export_choice.trim().to_lowercase().as_str() {
        "y" | "yes" => {
            // List available buckets
            println!(
                "{}", 
                format!("\nListing available S3 buckets...").yellow()
            );
            let buckets = list_s3_buckets(s3_client).await;

            let bucket: String = if buckets.is_empty() {
                eprintln!("No S3 buckets found or unable to list buckets.");
                Input::new()
                    .with_prompt("Enter S3 bucket name manually")
                    .interact_text()
                    .unwrap()
            } else {
                // Add option to enter manually at the beginning
                let mut bucket_options = vec!["Enter manually".to_string()];
                bucket_options.extend(buckets.clone());
                
                let bucket_index = Select::new()
                    .with_prompt("Select an S3 bucket")
                    .items(&bucket_options)
                    .default(0)
                    .max_length(10)
                    .interact()
                    .unwrap();
                
                if bucket_index == 0 {
                    Input::new()
                        .with_prompt("S3 bucket name")
                        .interact_text()
                        .unwrap()
                } else {
                    buckets[bucket_index - 1].clone()
                }
            };


            // Navigate through folders
            let mut current_prefix = String::new();
            loop {
                println!(
                    "{}", 
                    format!("\nListing folders in bucket '{}'...", bucket).yellow()
                );
                let folders = list_s3_folders(s3_client, &bucket, &current_prefix).await;
                
                let mut folder_options = vec![
                    "Save here (root or current folder)".to_string(),
                    "Enter path manually".to_string(),
                ];
                
                // Add "Go back" option if not in root
                if !current_prefix.is_empty() {
                    folder_options.push(".. (Go back)".to_string());
                }
                
                // Add available folders
                for folder in &folders {
                    let display_name = folder
                        .trim_start_matches(&current_prefix)
                        .trim_end_matches('/')
                        .to_string();
                    folder_options.push(format!("📁 {}", display_name));
                }
                
                if folder_options.len() <= 3 && current_prefix.is_empty() {
                    // No folders found in root
                    println!("No subfolders found in bucket.");
                    break;
                }
                
                let folder_index = Select::new()
                    .with_prompt(&format!(
                        "Current path: s3://{}/{}",
                        bucket,
                        if current_prefix.is_empty() {
                            "".to_string()
                        } else {
                            current_prefix.clone()
                        }
                    ))
                    .items(&folder_options)
                    .default(0)
                    .max_length(15)
                    .interact()
                    .unwrap();
                
                if folder_index == 0 {
                    // Save here
                    break;
                } else if folder_index == 1 {
                    // Enter manually
                    let manual_path: String = Input::new()
                        .with_prompt("Enter folder path (e.g., folder1/folder2/)")
                        .interact_text()
                        .unwrap();
                    current_prefix = manual_path.trim().to_string();
                    if !current_prefix.is_empty() && !current_prefix.ends_with('/') {
                        current_prefix.push('/');
                    }
                    break;
                } else if folder_index == 2 && !current_prefix.is_empty() {
                    // Go back
                    if let Some(parent_pos) = current_prefix[..current_prefix.len() - 1].rfind('/') {
                        current_prefix = current_prefix[..parent_pos + 1].to_string();
                    } else {
                        current_prefix.clear();
                    }
                } else {
                    // Navigate into selected folder
                    let adjusted_index = if current_prefix.is_empty() {
                        folder_index - 2
                    } else {
                        folder_index - 3
                    };
                    
                    if adjusted_index < folders.len() {
                        current_prefix = folders[adjusted_index].clone();
                    }
                }
            }

            let key = format!(
                "{}alternate-contact-list_{}.json",
                current_prefix,
                Local::now().format("%d-%m-%Y_%H-%M-%S")
            );

            match upload_to_s3(s3_client, &bucket, &key, &full_result).await {
                Ok(_) => {
                    println!(
                        "  {} Successfully uploaded to s3://{}/{}",
                        "✓".green(),
                        bucket,
                        key
                    );
                    if errors.is_empty() {
                        OperationOutcome::Success
                    } else {
                        OperationOutcome::PartialSuccess { errors }
                    }
                }
                Err(e) => OperationOutcome::Failure(e),
            }
        }
        "n" | "no" => {
            println!("\n{}:\n", "Result".bold());
            match serde_json::to_string_pretty(&alternate_contacts) {
                Ok(pretty) => println!("{}", pretty),
                Err(e) => {
                    eprintln!("Failed to serialize result: {}", e);
                }
            }

            if errors.is_empty() {
                OperationOutcome::Success
            } else {
                OperationOutcome::PartialSuccess { errors }
            }
        }
        _ => {
            println!("\n{}", "Invalid input. Operation cancelled.".yellow());
            OperationOutcome::Cancelled
        }
    }
}

async fn upload_to_s3(
    s3_client: &S3Client,
    bucket: &str,
    key: &str,
    data: &Value,
) -> AppResult<()> {
    let body = serde_json::to_vec(data)
        .map_err(|e| AppError::UserInput(format!("Failed to serialize data: {}", e)))?;

    let retry_config = RetryConfig::default();

    retry_with_backoff(&retry_config, "put_object", || {
        let body_clone = body.clone();
        async move {
            s3_client
                .put_object()
                .bucket(bucket)
                .key(key)
                .body(ByteStream::from(body_clone))
                .send()
                .await
                .map_err(|e| Box::new(e) as BoxError)
        }
    })
    .await
    .map_err(|err| {
        let err_str = format!("{:?}", err);
        if err_str.contains("NoSuchBucket") {
            return S3Error::NoSuchBucket { bucket: bucket.to_string() }.into();
        }
        if err_str.contains("AccessDenied") {
            return S3Error::AccessDenied { bucket: bucket.to_string() }.into();
        }
        S3Error::PutObject {
            bucket: bucket.to_string(),
            key: key.to_string(),
            message: err.to_string(),
            source: Some(err),
        }
    })?;

    Ok(())
}

async fn update_func(
    accounts: &[String],
    current_account_id: &str,
    contact_types: &[String],
    account_client: &AccountClient,
) -> OperationOutcome {
    let email: String = match Input::new()
        .with_prompt("Type the email address (e.g. example@mail.com)")
        .validate_with({
            move |input: &String| -> Result<(), &str> {
                if input.contains('@') {
                    Ok(())
                } else {
                    Err("Invalid email format. Please include '@' symbol.")
                }
            }
        })
        .interact_text()
    {
        Ok(e) => e,
        Err(e) => return OperationOutcome::Failure(AppError::UserInput(e.to_string())),
    };

    let name: String = match Input::new()
        .with_prompt("Type the name (e.g. John Doe)")
        .interact_text()
    {
        Ok(n) => n,
        Err(e) => return OperationOutcome::Failure(AppError::UserInput(e.to_string())),
    };

    let phone: String = match Input::new()
        .with_prompt("Type the phone number (e.g. +5511900002222)")
        .validate_with(|input: &String| {
            let digits_only: String = input.chars().filter(|c| c.is_digit(10)).collect();
            if digits_only.len() >= 8 {
                Ok(())
            } else {
                Err("Phone number must contain at least 8 digits.")
            }
        })
        .interact_text()
    {
        Ok(p) => p,
        Err(e) => return OperationOutcome::Failure(AppError::UserInput(e.to_string())),
    };

    let title: String = match Input::new()
        .with_prompt("Type the title (e.g. Manager)")
        .interact_text()
    {
        Ok(t) => t,
        Err(e) => return OperationOutcome::Failure(AppError::UserInput(e.to_string())),
    };

    println!();

    let mut errors: Vec<AppError> = Vec::new();
    let mut success_count = 0;
    let retry_config = RetryConfig::default();

    for account_id in accounts {
        for ct_name in contact_types {
            println!(
                "Updating {} alternate contact for {}...",
                ct_name.cyan(),
                account_id.yellow()
            );

            let ct = match parse_contact_type(ct_name) {
                Ok(ct) => ct,
                Err(e) => {
                    errors.push(e);
                    continue;
                }
            };

            let result = retry_with_backoff(&retry_config, "put_alternate_contact", || {
                let mut req = account_client
                    .put_alternate_contact()
                    .alternate_contact_type(ct.clone())
                    .email_address(&email)
                    .name(&name)
                    .phone_number(&phone)
                    .title(&title);

                if account_id != current_account_id {
                    req = req.account_id(account_id.as_str());
                }

                async move { req.send().await.map_err(|e| Box::new(e) as BoxError) }
            })
            .await;

            match result {
                Ok(_) => {
                    println!("  {} Updated successfully", "✓".green());
                    success_count += 1;
                }
                Err(err) => {
                    let error = if error_is_access_denied(&err) {
                        AccountError::AccessDenied {
                            account_id: account_id.clone(),
                        }
                    } else if error_is_throttling(&err) {
                        AccountError::TooManyRequests
                    } else {
                        AccountError::PutAlternateContact {
                            account_id: account_id.clone(),
                            contact_type: ct_name.clone(),
                            message: err.to_string(),
                            source: Some(err),
                        }
                    };

                    eprintln!("  {} Failed: {}", "✗".red(), error);
                    log::error!("{}", error);

                    if matches!(error, AccountError::TooManyRequests) {
                        return OperationOutcome::Failure(error.into());
                    }

                    errors.push(error.into());
                }
            }
        }
    }

    println!(
        "\nUpdated {}/{} contacts",
        success_count,
        accounts.len() * contact_types.len()
    );

    if errors.is_empty() {
        OperationOutcome::Success
    } else if success_count > 0 {
        OperationOutcome::PartialSuccess { errors }
    } else {
        OperationOutcome::Failure(errors.remove(0))
    }
}

async fn delete_func(
    accounts: &[String],
    current_account_id: &str,
    contact_types: &[String],
    account_client: &AccountClient,
) -> OperationOutcome {
    let mut errors: Vec<AppError> = Vec::new();
    let mut success_count = 0;
    let retry_config = RetryConfig::default();

    for account_id in accounts {
        for ct_name in contact_types {
            println!(
                "Deleting {} alternate contact for {}...",
                ct_name.cyan(),
                account_id.yellow()
            );

            let ct = match parse_contact_type(ct_name) {
                Ok(ct) => ct,
                Err(e) => {
                    errors.push(e);
                    continue;
                }
            };

            let result = retry_with_backoff(&retry_config, "delete_alternate_contact", || {
                let mut req = account_client
                    .delete_alternate_contact()
                    .alternate_contact_type(ct.clone());

                if account_id != current_account_id {
                    req = req.account_id(account_id.as_str());
                }

                async move { req.send().await.map_err(|e| Box::new(e) as BoxError) }
            })
            .await;

            match result {
                Ok(_) => {
                    println!("  {} Deleted successfully", "✓".green());
                    success_count += 1;
                }
                Err(err) => {
                    if error_is_not_found(&err) {
                        println!("  {} Contact not found (already deleted)", "~".yellow());
                        success_count += 1;
                        continue;
                    }

                    let error = if error_is_access_denied(&err) {
                        AccountError::AccessDenied {
                            account_id: account_id.clone(),
                        }
                    } else if error_is_throttling(&err) {
                        AccountError::TooManyRequests
                    } else {
                        AccountError::DeleteAlternateContact {
                            account_id: account_id.clone(),
                            contact_type: ct_name.clone(),
                            message: err.to_string(),
                            source: Some(err),
                        }
                    };

                    eprintln!("  {} Failed: {}", "✗".red(), error);
                    log::error!("{}", error);

                    if matches!(error, AccountError::TooManyRequests) {
                        return OperationOutcome::Failure(error.into());
                    }

                    errors.push(error.into());
                }
            }
        }
    }

    println!(
        "\nDeleted {}/{} contacts",
        success_count,
        accounts.len() * contact_types.len()
    );

    if errors.is_empty() {
        OperationOutcome::Success
    } else if success_count > 0 {
        OperationOutcome::PartialSuccess { errors }
    } else {
        OperationOutcome::Failure(errors.remove(0))
    }
}

// ============================================================================
// User Input Helpers
// ============================================================================

fn get_user_selection<T: fmt::Display>(
    prompt: &str,
    items: &[T],
    default: usize,
) -> AppResult<usize> {
    Select::new()
        .with_prompt(prompt)
        .items(items)
        .default(default)
        .interact()
        .map_err(|e| AppError::UserInput(format!("Failed to read selection: {}", e)))
}

fn get_user_input(prompt: &str) -> AppResult<String> {
    Input::new()
        .with_prompt(prompt)
        .interact_text()
        .map_err(|e| AppError::UserInput(format!("Failed to read input: {}", e)))
}

// ============================================================================
// S3 Helpers
// ============================================================================

async fn list_s3_buckets(s3_client: &S3Client) -> Vec<String> {
    match s3_client.list_buckets().send().await {
        Ok(output) => {
            output
                .buckets()
                .iter()
                .filter_map(|bucket| bucket.name().map(|s| s.to_string()))
                .collect()
        }
        Err(e) => {
            eprintln!("Could not list S3 buckets: {}", e);
            Vec::new()
        }
    }
}

async fn list_s3_folders(
    s3_client: &S3Client,
    bucket: &str,
    prefix: &str,
) -> Vec<String> {   
    let mut req = s3_client
        .list_objects_v2()
        .bucket(bucket)
        .delimiter("/");
    
    if !prefix.is_empty() {
        req = req.prefix(prefix);
    }
    
    match req.send().await {
        Ok(output) => {
            let mut folders = Vec::new();
            
            // Get common prefixes (folders) - already returns a slice, not Option
            for cp in output.common_prefixes() {
                if let Some(folder_prefix) = cp.prefix() {
                    folders.push(folder_prefix.to_string());
                }
            }
            
            folders
        }
        Err(e) => {
            eprintln!("Could not list S3 folders: {}", e);
            Vec::new()
        }
    }
}

// ============================================================================
// Main Function
// ============================================================================

#[tokio::main]
async fn main() {
    env_logger::init();

    println!(
        "\n{}",
        style("AWS Organizations Alternate Contact Manager").bold()
    );
    println!(
        "{}\n",
        style(
            "Solution developed for batch management of alternate contacts. \
             For more information, visit: \
             https://github.com/aws-samples/aws-organizations-alternate-contact-manager"
        )
        .italic()
    );

    if let Err(e) = run_app().await {
        eprintln!("\n{} {}\n", "Error:".red().bold(), e);

        let mut source = std::error::Error::source(&e);
        while let Some(s) = source {
            log::debug!("Caused by: {}", s);
            source = s.source();
        }

        std::process::exit(1);
    }
}

async fn run_app() -> AppResult<()> {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let org_client = OrganizationsClient::new(&config);
    let sts_client = StsClient::new(&config);
    let account_client = AccountClient::new(&config);
    let s3_client = S3Client::new(&config);

    let actions = &["List", "Update", "Delete"];
    let action_idx = get_user_selection("Select action", actions, 0)?;
    let action = actions[action_idx];
    println!("Action: {}", action.green());

    let accounts: Vec<String> = if action == "Delete" {
        let raw = get_user_input("Account ID (delete action allowed for one account at a time)")?;
        raw.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        let raw = get_user_input(
            "Account IDs (enter a list of account ids separated by comma / all)",
        )?;

        if raw.trim().eq_ignore_ascii_case("all") {
            println!("Fetching all accounts from organization...");
            list_accounts_func(&org_client).await?
        } else {
            raw.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
    };

    println!("Validating account IDs...");
    let org_accounts = list_accounts_func(&org_client).await?;
    validate_accounts(&accounts, &org_accounts)?;
    println!("  {} All {} accounts validated", "✓".green(), accounts.len());

    let current_account_id = get_account_id(&sts_client).await?;
    log::debug!("Current account ID: {}", current_account_id);

    let contact_options = &["Billing", "Operations", "Security", "All"];
    let contact_idx = get_user_selection("Select alternate contact type", contact_options, 0)?;
    println!(
        "Alternate contact type: {}\n",
        contact_options[contact_idx].green()
    );

    let contact_types: Vec<String> = if contact_options[contact_idx] == "All" {
        vec!["Billing".into(), "Operations".into(), "Security".into()]
    } else {
        vec![contact_options[contact_idx].to_string()]
    };

    let start = Instant::now();

    let outcome = match action {
        "List" => {
            list_func(
                &accounts,
                &current_account_id,
                &contact_types,
                &account_client,
                &s3_client,
            )
            .await
        }
        "Update" => {
            update_func(
                &accounts,
                &current_account_id,
                &contact_types,
                &account_client,
            )
            .await
        }
        "Delete" => {
            delete_func(
                &accounts,
                &current_account_id,
                &contact_types,
                &account_client,
            )
            .await
        }
        _ => OperationOutcome::Failure(AppError::UserInput(format!(
            "Unknown action: {}",
            action
        ))),
    };

    let elapsed = start.elapsed();

    match outcome {
        OperationOutcome::Success => {
            println!(
                "\n{}\n",
                format!(
                    "✓ Completed successfully in {:.4} seconds!",
                    elapsed.as_secs_f64()
                )
                .green()
            );
            Ok(())
        }
        OperationOutcome::PartialSuccess { errors } => {
            println!(
                "\n{}",
                format!(
                    "⚠ Completed with {} errors in {:.4} seconds",
                    errors.len(),
                    elapsed.as_secs_f64()
                )
                .yellow()
            );
            println!("\nErrors encountered:");
            for (i, err) in errors.iter().enumerate() {
                eprintln!("  {}. {}", i + 1, err);
            }
            println!();
            Ok(())
        }
        OperationOutcome::Failure(err) => {
            eprintln!(
                "\n{}\n",
                format!("✗ Failed after {:.4} seconds", elapsed.as_secs_f64()).red()
            );
            Err(err)
        }
        OperationOutcome::Cancelled => {
            println!("\n{}\n", "Operation cancelled by user".yellow());
            Ok(())
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_contact_type() {
        assert!(matches!(
            parse_contact_type("billing"),
            Ok(AlternateContactType::Billing)
        ));
        assert!(matches!(
            parse_contact_type("OPERATIONS"),
            Ok(AlternateContactType::Operations)
        ));
        assert!(matches!(
            parse_contact_type("Security"),
            Ok(AlternateContactType::Security)
        ));
        assert!(matches!(
            parse_contact_type("invalid"),
            Err(AppError::UnknownContactType(_))
        ));
    }

    #[test]
    fn test_validate_accounts_valid() {
        let accounts = vec!["123456789012".to_string()];
        let org_accounts = vec!["123456789012".to_string(), "234567890123".to_string()];
        assert!(validate_accounts(&accounts, &org_accounts).is_ok());
    }

    #[test]
    fn test_validate_accounts_invalid_length() {
        let accounts = vec!["12345".to_string()];
        let org_accounts = vec!["123456789012".to_string()];
        let result = validate_accounts(&accounts, &org_accounts);
        assert!(matches!(
            result,
            Err(AppError::Validation(ValidationError::InvalidAccountId { .. }))
        ));
    }

    #[test]
    fn test_validate_accounts_not_in_org() {
        let accounts = vec!["999999999999".to_string()];
        let org_accounts = vec!["123456789012".to_string()];
        let result = validate_accounts(&accounts, &org_accounts);
        assert!(matches!(
            result,
            Err(AppError::Validation(
                ValidationError::AccountNotInOrganization { .. }
            ))
        ));
    }

    #[test]
    fn test_validate_accounts_empty() {
        let accounts: Vec<String> = vec![];
        let org_accounts = vec!["123456789012".to_string()];
        let result = validate_accounts(&accounts, &org_accounts);
        assert!(matches!(
            result,
            Err(AppError::Validation(ValidationError::NoAccountsProvided))
        ));
    }
}
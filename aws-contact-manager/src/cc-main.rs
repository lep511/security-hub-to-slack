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
use serde_json::{json, Map, Value};
use std::process;
use std::time::Instant;

async fn list_accounts_func(org_client: &OrganizationsClient) -> Vec<String> {
    let mut account_ids: Vec<String> = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let mut request = org_client.list_accounts();
        if let Some(ref token) = next_token {
            println!("tem next token");
            request = request.next_token(token);
        }

        match request.send().await {
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
            Err(e) => {
                eprintln!(
                    "\n {} {}",
                    "Could not list accounts... Error:".red(),
                    e
                );
                log::error!("{}", e);
                process::exit(1);
            }
        }
    }

    account_ids
}

async fn get_account_id(sts_client: &StsClient) -> String {
    sts_client
        .get_caller_identity()
        .send()
        .await
        .expect("Failed to get caller identity")
        .account()
        .expect("No account ID found")
        .to_string()
}

fn parse_contact_type(name: &str) -> AlternateContactType {
    match name.to_uppercase().as_str() {
        "BILLING" => AlternateContactType::Billing,
        "OPERATIONS" => AlternateContactType::Operations,
        "SECURITY" => AlternateContactType::Security,
        other => panic!("Unknown alternate contact type: {}", other),
    }
}

async fn list_func(
    accounts: &[String],
    current_account_id: &str,
    contact_types: &[String],
    account_client: &AccountClient,
    s3_client: &S3Client,
) -> bool {
    let mut alternate_contacts: Map<String, Value> = Map::new();

    for account_id in accounts {
        let mut type_map: Map<String, Value> = Map::new();

        for ct_name in contact_types {
            println!(
                "Getting {} alternate contact for {}...",
                ct_name.cyan(),
                account_id.yellow()
            );

            let ct = parse_contact_type(ct_name);

            let result = if account_id == current_account_id {
                account_client
                    .get_alternate_contact()
                    .alternate_contact_type(ct)
                    .send()
                    .await
            } else {
                account_client
                    .get_alternate_contact()
                    .account_id(account_id.as_str())
                    .alternate_contact_type(ct)
                    .send()
                    .await
            };

            match result {
                Ok(resp) => {
                    if let Some(contact) = resp.alternate_contact() {
                        let contact_json = json!({
                            "EmailAddress": contact.email_address().unwrap_or_default(),
                            "Name": contact.name().unwrap_or_default(),
                            "PhoneNumber": contact.phone_number().unwrap_or_default(),
                            "Title": contact.title().unwrap_or_default(),
                        });
                        type_map.insert(ct_name.clone(), contact_json);
                    } else {
                        type_map.insert(ct_name.clone(), Value::String("Null".into()));
                    }
                }
                Err(err) => {
                    let is_not_found = err
                        .as_service_error()
                        .map_or(false, |e| e.is_resource_not_found_exception());

                    if is_not_found {
                        type_map.insert(ct_name.clone(), Value::String("Null".into()));
                    } else {
                        eprintln!();
                        log::error!("{}", err);
                        return false;
                    }
                }
            }
        }

        alternate_contacts.insert(account_id.clone(), Value::Object(type_map));
    }

    let full_result = json!({ "AlternateContact": alternate_contacts });

    let export_choice: String = Input::new()
        .with_prompt("\nDo you want to export the result to an S3 bucket? (y/n)")
        .interact_text()
        .unwrap_or_default();

    match export_choice.trim() {
        "y" => {
            // List available buckets
            println!(
                "{}", 
                format!("\nListing available S3 buckets...").yellow()
            );
            let buckets = list_s3_buckets(s3_client).await;
            
            let bucket_name: String = if buckets.is_empty() {
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
                    format!("\nListing folders in bucket '{}'...", bucket_name).yellow()
                );
                let folders = list_s3_folders(s3_client, &bucket_name, &current_prefix).await;
                
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
                        bucket_name,
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

            let object_key = format!(
                "{}alternate-contact-list_{}.json",
                current_prefix,
                Local::now().format("%d-%m-%Y_%H-%M-%S")
            );

            println!(
                "\nSaving to: s3://{}/{}",
                bucket_name, object_key
            );

            let body_bytes = serde_json::to_vec_pretty(&full_result).unwrap();

            match s3_client
                .put_object()
                .bucket(&bucket_name)
                .key(&object_key)
                .body(ByteStream::from(body_bytes))
                .send()
                .await
            {
                Ok(_) => {
                    println!(
                        "{}", 
                        format!(
                            "✓ File uploaded successfully to s3://{}/{}", 
                            bucket_name, 
                            object_key
                        ).green()
                    );
                    true
                }
                Err(e) => {
                    eprintln!("\n{}", e);
                    false
                }
            }
        }
        "n" => {
            println!("\n{}:\n", "Return".bold());
            let pretty =
                serde_json::to_string_pretty(&alternate_contacts).unwrap_or_default();
            println!("{}", pretty);
            true
        }
        _ => {
            println!("\n{}", "Invalid input.".red());
            false
        }
    }
}

async fn update_func(
    accounts: &[String],
    current_account_id: &str,
    contact_types: &[String],
    account_client: &AccountClient,
) -> bool {
    let email: String = Input::new()
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
        .unwrap_or_default();

    let name: String = Input::new()
        .with_prompt("Type the name (e.g. John Doe)")
        .interact_text()
        .unwrap_or_default();

    let phone: String = Input::new()
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
        .unwrap_or_default();

    let title: String = Input::new()
        .with_prompt("Type the title (e.g. Technical Account Manager)")
        .interact_text()
        .unwrap_or_default();

    println!();

    for account_id in accounts {
        for ct_name in contact_types {
            println!(
                "Updating {} alternate contact for {}...",
                ct_name.cyan(),
                account_id.yellow()
            );

            let ct = parse_contact_type(ct_name);

            let mut req = account_client
                .put_alternate_contact()
                .alternate_contact_type(ct)
                .email_address(&email)
                .name(&name)
                .phone_number(&phone)
                .title(&title);

            if account_id != current_account_id {
                req = req.account_id(account_id.as_str());
            }

            if let Err(e) = req.send().await {
                eprintln!(
                    "\n {} {} {} {}",
                    "Could not update".red(),
                    format!("{} alternate contact for {}", ct_name, account_id).yellow(),
                    "... Error:".red(),
                    e
                );
                log::error!("{}", e);
                return false;
            }
        }
    }

    true
}

async fn delete_func(
    accounts: &[String],
    current_account_id: &str,
    contact_types: &[String],
    account_client: &AccountClient,
) -> bool {
    for account_id in accounts {
        for ct_name in contact_types {
            println!(
                "Deleting {} alternate contact for {}...",
                ct_name.cyan(),
                account_id.yellow()
            );

            let ct = parse_contact_type(ct_name);

            let mut req = account_client
                .delete_alternate_contact()
                .alternate_contact_type(ct);

            if account_id != current_account_id {
                req = req.account_id(account_id.as_str());
            }

            if let Err(e) = req.send().await {
                eprintln!(
                    "\n {} {} {} {}",
                    "Could not delete".red(),
                    format!("{} alternate contact for {}", ct_name, account_id).yellow(),
                    "... Error:".red(),
                    e
                );
                log::error!("{}", e);
                return false;
            }
        }
    }

    true
}

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

#[tokio::main]
async fn main() {
    env_logger::init();

    // Header
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

    // Initialize AWS clients
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let org_client = OrganizationsClient::new(&config);
    let sts_client = StsClient::new(&config);
    let account_client = AccountClient::new(&config);
    let s3_client = S3Client::new(&config);

    // Select action
    let actions = &["List", "Update", "Delete"];
    let action_idx = Select::new()
        .with_prompt("Select action")
        .items(actions)
        .default(0)
        .interact()
        .unwrap_or_else(|_| {
            eprintln!("Failed to read selection");
            process::exit(1);
        });

    let action = actions[action_idx];
    println!("Action: {}", action.green());

    // Gather account IDs
    let accounts: Vec<String> = if action == "Delete" {
        let raw: String = Input::new()
            .with_prompt("Account ID (delete action allowed for one account at a time)")
            .interact_text()
            .unwrap_or_default();
        raw.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        let raw: String = Input::new()
            .with_prompt("Account IDs (enter a list of account ids separated by comma / all)")
            .interact_text()
            .unwrap_or_default();

        if raw.trim() == "all" {
            list_accounts_func(&org_client).await
        } else {
            raw.split(',').map(|s| s.trim().to_string()).collect()
        }
    };

    // Validate account IDs
    let org_accounts = list_accounts_func(&org_client).await;
    for id in &accounts {
        if id.len() != 12 {
            eprintln!(
                "\n{} {} {}\n",
                "Account ID".red(),
                id,
                "is not a valid AWS Account ID.".red()
            );
            process::exit(1);
        }
        if !org_accounts.contains(id) {
            eprintln!(
                "\n{} {} {}\n",
                "Account ID".red(),
                id,
                "does not belong to your AWS Organization.".red()
            );
            process::exit(1);
        }
    }

    let current_account_id = get_account_id(&sts_client).await;

    // Select contact type
    let contact_options = &["Billing", "Operations", "Security", "All"];
    let contact_idx = Select::new()
        .with_prompt("Select alternate contact type")
        .items(contact_options)
        .default(0)
        .interact()
        .unwrap_or_else(|_| {
            eprintln!("Failed to read selection");
            process::exit(1);
        });

    println!(
        "Alternate contact type: {}\n",
        contact_options[contact_idx].green()
    );

    let contact_types: Vec<String> = if contact_options[contact_idx] == "All" {
        vec![
            "Billing".into(),
            "Operations".into(),
            "Security".into(),
        ]
    } else {
        vec![contact_options[contact_idx].to_string()]
    };

    // Execute chosen action
    let start = Instant::now();

    let success = match action {
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
        _ => false,
    };

    let elapsed = start.elapsed();

    if success {
        println!(
            "\n{}\n",
            format!(
                "Completed successfully in {:.4} seconds!",
                elapsed.as_secs_f64()
            )
            .green()
        );
    } else {
        eprintln!("\n{}\n", "ERROR: something went wrong.".red());
    }
}
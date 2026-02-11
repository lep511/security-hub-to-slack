use aws_sdk_account::types::AlternateContactType;
use aws_sdk_account::Client as AccountClient;
use aws_sdk_organizations::Client as OrganizationsClient;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_sts::Client as StsClient;
use chrono::Local;
use console::Style;
use dialoguer::{Input, Select};
use serde_json::{json, Map, Value};
use std::time::Instant;

async fn list_accounts_func(config: &aws_config::SdkConfig) -> Vec<String> {
    let client = OrganizationsClient::new(config);
    let mut account_ids = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let mut req = client.list_accounts();
        if let Some(ref token) = next_token {
            println!("tem next token");
            req = req.next_token(token);
        }

        match req.send().await {
            Ok(output) => {
                for account in output.accounts() {
                    if let Some(id) = account.id() {
                        account_ids.push(id.to_string());
                    }
                }
                next_token = output.next_token().map(|s| s.to_string());
                if next_token.is_none() {
                    break;
                }
            }
            Err(e) => {
                eprintln!("\n Could not list accounts... Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    account_ids
}

async fn get_account_id(config: &aws_config::SdkConfig) -> String {
    let client = StsClient::new(config);
    match client.get_caller_identity().send().await {
        Ok(output) => output.account().unwrap_or_default().to_string(),
        Err(e) => {
            eprintln!("Could not get caller identity: {}", e);
            std::process::exit(1);
        }
    }
}

fn parse_contact_type(s: &str) -> AlternateContactType {
    match s.to_uppercase().as_str() {
        "BILLING" => AlternateContactType::Billing,
        "OPERATIONS" => AlternateContactType::Operations,
        "SECURITY" => AlternateContactType::Security,
        other => panic!("Unknown alternate contact type: {}", other),
    }
}

async fn list_s3_buckets(config: &aws_config::SdkConfig) -> Vec<String> {
    let s3_client = S3Client::new(config);
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
    config: &aws_config::SdkConfig,
    bucket: &str,
    prefix: &str,
) -> Vec<String> {
    let s3_client = S3Client::new(config);
    
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

async fn list_func(
    config: &aws_config::SdkConfig,
    accounts: &[String],
    current_account_id: &str,
    contact_types: &[String],
) -> bool {
    let client = AccountClient::new(config);
    let mut result_map: Map<String, Value> = Map::new();

    for account_id in accounts {
        let mut type_map: Map<String, Value> = Map::new();

        for contact_type_name in contact_types {
            println!(
                "Getting {} alternate contact for {}...",
                contact_type_name, account_id
            );
            let contact_type = parse_contact_type(contact_type_name);

            let mut req = client
                .get_alternate_contact()
                .alternate_contact_type(contact_type);

            if account_id != current_account_id {
                req = req.account_id(account_id.as_str());
            }

            match req.send().await {
                Ok(output) => {
                    if let Some(contact) = output.alternate_contact() {
                        let mut contact_map: Map<String, Value> = Map::new();
                        if let Some(v) = contact.email_address() {
                            contact_map.insert("EmailAddress".to_string(), json!(v));
                        }
                        if let Some(v) = contact.name() {
                            contact_map.insert("Name".to_string(), json!(v));
                        }
                        if let Some(v) = contact.phone_number() {
                            contact_map.insert("PhoneNumber".to_string(), json!(v));
                        }
                        if let Some(v) = contact.title() {
                            contact_map.insert("Title".to_string(), json!(v));
                        }
                        type_map.insert(
                            contact_type_name.clone(),
                            Value::Object(contact_map),
                        );
                    } else {
                        type_map.insert(contact_type_name.clone(), json!("Null"));
                    }
                }
                Err(err) => {
                    let is_not_found = err
                        .as_service_error()
                        .map_or(false, |e| e.is_resource_not_found_exception());

                    if is_not_found {
                        type_map.insert(contact_type_name.clone(), json!("Null"));
                    } else {
                        eprintln!("\n{}", err);
                        return false;
                    }
                }
            }
        }

        result_map.insert(account_id.clone(), Value::Object(type_map));
    }

    let full_result = json!({ "AlternateContact": result_map });

    let export_choice: String = Input::new()
        .with_prompt("\nDo you want to export the result to an S3 bucket? (y/n)")
        .interact_text()
        .unwrap();

    if export_choice == "y" {
        // List available buckets
        println!("\nListing available S3 buckets...");
        let buckets = list_s3_buckets(config).await;
        
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
            println!("\nListing folders in bucket '{}'...", bucket_name);
            let folders = list_s3_folders(config, &bucket_name, &current_prefix).await;
            
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

        let s3_client = S3Client::new(config);
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
                    "✓ File uploaded successfully to s3://{}/{}",
                    bucket_name, object_key
                );
                true
            }
            Err(e) => {
                eprintln!("\n{}", e);
                false
            }
        }
    } else {
        println!("\nResult:\n{}", serde_json::to_string_pretty(&full_result).unwrap());
        true
    }
}

async fn update_func(
    config: &aws_config::SdkConfig,
    accounts: &[String],
    current_account_id: &str,
    contact_types: &[String],
) -> bool {
    let client = AccountClient::new(config);

    let email_address: String = Input::new()
        .with_prompt("Type the email address (see the README.md file for valid patterns)")
        .interact_text()
        .unwrap();
    let name: String = Input::new()
        .with_prompt("Type the name (see the README.md file for valid patterns)")
        .interact_text()
        .unwrap();
    let phone_number: String = Input::new()
        .with_prompt("Type the phone number (see the README.md file for valid patterns)")
        .interact_text()
        .unwrap();
    let title: String = Input::new()
        .with_prompt("Type the title (see the README.md file for valid patterns)")
        .interact_text()
        .unwrap();
    println!();

    for account_id in accounts {
        for contact_type_name in contact_types {
            println!(
                "Updating {} alternate contact for {}...",
                contact_type_name, account_id
            );
            let contact_type = parse_contact_type(contact_type_name);

            let mut req = client
                .put_alternate_contact()
                .alternate_contact_type(contact_type)
                .email_address(&email_address)
                .name(&name)
                .phone_number(&phone_number)
                .title(&title);

            if account_id != current_account_id {
                req = req.account_id(account_id.as_str());
            }

            if let Err(e) = req.send().await {
                eprintln!(
                    "\n Could not update {} alternate contact for {}... Error: {}",
                    contact_type_name, account_id, e
                );
                return false;
            }
        }
    }
    true
}

async fn delete_func(
    config: &aws_config::SdkConfig,
    accounts: &[String],
    current_account_id: &str,
    contact_types: &[String],
) -> bool {
    let client = AccountClient::new(config);

    for account_id in accounts {
        for contact_type_name in contact_types {
            println!(
                "Deleting {} alternate contact for {}...",
                contact_type_name, account_id
            );
            let contact_type = parse_contact_type(contact_type_name);

            let mut req = client
                .delete_alternate_contact()
                .alternate_contact_type(contact_type);

            if account_id != current_account_id {
                req = req.account_id(account_id.as_str());
            }

            if let Err(e) = req.send().await {
                eprintln!(
                    "\n Could not delete {} alternate contact for {}... Error: {}",
                    contact_type_name, account_id, e
                );
                return false;
            }
        }
    }
    true
}

#[tokio::main]
async fn main() {
    let bold = Style::new().bold();
    let italic = Style::new().italic();

    println!(
        "{}",
        bold.apply_to("\nAWS Organizations Alternate Contact Manager")
    );
    println!(
        "{}\n",
        italic.apply_to(
            "Solution developed for batch management of alternate contacts. \
             For more information, visit: \
             https://github.com/aws-samples/aws-organizations-alternate-contact-manager"
        )
    );

    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;

    // Action selection
    let action_options = &["List", "Update", "Delete"];
    let action_index = Select::new()
        .items(action_options)
        .default(0)
        .interact()
        .unwrap();
    let action = action_options[action_index];
    println!("Action: {}", action);

    // Account input
    let accounts: Vec<String> = if action == "Delete" {
        let input: String = Input::new()
            .with_prompt("Account ID (delete action allowed for one account at a time)")
            .interact_text()
            .unwrap();
        input.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        let input: String = Input::new()
            .with_prompt("Account IDs (enter a list of account ids separated by comma / all)")
            .interact_text()
            .unwrap();
        if input.trim() == "all" {
            list_accounts_func(&config).await
        } else {
            input.split(',').map(|s| s.trim().to_string()).collect()
        }
    };

    // Validate accounts
    let org_accounts = list_accounts_func(&config).await;
    for account_id in &accounts {
        if account_id.len() != 12 {
            eprintln!(
                "\nAccount ID {} is not a valid AWS Account ID.\n",
                account_id
            );
            std::process::exit(1);
        }
        if !org_accounts.contains(account_id) {
            eprintln!(
                "\nAccount ID {} does not belong to your AWS Organization.\n",
                account_id
            );
            std::process::exit(1);
        }
    }

    let current_account_id = get_account_id(&config).await;

    // Contact type selection
    let type_options = &["Billing", "Operations", "Security", "All"];
    let type_index = Select::new()
        .items(type_options)
        .default(0)
        .interact()
        .unwrap();
    println!("Alternate contact type: {}\n", type_options[type_index]);

    let contact_types: Vec<String> = if type_options[type_index] == "All" {
        vec![
            "Billing".to_string(),
            "Operations".to_string(),
            "Security".to_string(),
        ]
    } else {
        vec![type_options[type_index].to_string()]
    };

    let start = Instant::now();

    let success = match action {
        "List" => {
            list_func(&config, &accounts, &current_account_id, &contact_types).await
        }
        "Update" => {
            update_func(&config, &accounts, &current_account_id, &contact_types).await
        }
        "Delete" => {
            delete_func(&config, &accounts, &current_account_id, &contact_types).await
        }
        _ => false,
    };

    let elapsed = start.elapsed();

    if success {
        println!(
            "\nCompleted successfully in {:.1} seconds!\n",
            elapsed.as_secs_f64()
        );
    } else {
        println!("\nERROR: something went wrong.\n");
    }
}
use lambda_runtime::{tracing, Error, LambdaEvent};
use aws_lambda_events::event::eventbridge::EventBridgeEvent;
use aws_config::BehaviorVersion;
use aws_sdk_secretsmanager::Client as SMClient;
use crate::struct_event::{FindingSummary, Detail, Finding};
use crate::slack_client::post_slack_message;
use serde_json::Value;

pub(crate) async fn function_handler(event: LambdaEvent<EventBridgeEvent<Value>>) -> Result<(), Error> {
    let payload = event.payload;
    let config = aws_config::defaults(BehaviorVersion::latest())
        .load()
        .await;

    // Retrieve the token from AWS Secrets Manager
    let secrets_client = SMClient::new(&config);
    let secret_name = "slack-token";
    let token = match get_secret(&secrets_client, secret_name).await {
        Ok(token) => token,
        Err(e) => {
            let err_msg = format!("Failed to retrieve secret '{}': {}", secret_name, e);
            tracing::error!("{}", err_msg);
            return Err(err_msg.into());
        }
    };

    // Parse the detail field into our custom Detail struct
    let detail: Detail = serde_json::from_value(payload.detail.clone())
        .map_err(|e| format!("Failed to parse detail: {}", e))?;
   
    let findings = detail.findings.as_ref()
        .ok_or("Missing findings in detail")?;

    for finding in findings {
        let summary = FindingSummary::from_finding(finding);
        tracing::info!("Processing finding: {}", summary.title);

        if summary.severity == "High" || summary.severity == "Critical" {
             tracing::warn!("High severity finding detected: {}", summary.title);
             handle_high_severity_finding(finding).await?;
        }

        // Post the finding summary to Slack
        let channel = "#aws-security";
        match post_slack_message(&token, channel, summary).await {
            Ok(_) => (),
            Err(e) => tracing::error!("Failed to post finding to Slack: {}", e),
        }
    }

    Ok(())
}

pub async fn handle_high_severity_finding(_finding: &Finding) -> Result<(), Error> {
    tracing::warn!("High severity finding detected!");
    
    // TODO: Implement your high-severity handling logic:
    // - Send urgent notifications
    // - Trigger automated response
    // - Create high-priority tickets
    // - Alert security team
    
    Ok(())
}

async fn get_secret(
    client: &SMClient,
    secret_name: &str,
) -> Result<String, Error> {
    let response = client
        .get_secret_value()
        .secret_id(secret_name)
        .send()
        .await?;

    // Handle both string and JSON secrets
    let secret = if let Some(secret_string) = response.secret_string() {
        // If the secret is a JSON object with a "token" field
        if secret_string.starts_with('{') {
            let json: Value = serde_json::from_str(secret_string)?;
            json["token"]
                .as_str()
                .ok_or("Token field not found in secret")?
                .to_string()
        } else {
            // Plain string secret
            secret_string.to_string()
        }
    } else {
        return Err("Secret not found".into());
    };

    Ok(secret)
}
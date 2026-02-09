use reqwest::Client;
use serde_json::Value;
use std::error::Error;
use serde_json::json;

pub async fn post_slack_message(
    token: &str,
    channel: &str,
    text: &str
) -> Result<(), Box<dyn Error>> {

    // Build the blocks for the Slack message
    let mut blocks = vec![
        json!({
			"type": "section",
			"text": {
				"type": "plain_text",
				"text": format!("This is a plain text section block. You said: {}", text),
				"emoji": true
			}
		})
    ];

    let final_json = json!(blocks);

    match post_slack_message_with_blocks(token, channel, final_json).await {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to post results message to Slack: {}", e).into()),
    }
}


pub async fn post_slack_message_with_blocks(
    token: &str,
    channel: &str,
    all_blocks: Value,
) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let url = "https://slack.com/api/chat.postMessage";

    // Prepare JSON payload
    let payload = serde_json::json!({
        "channel": channel,
        "blocks": all_blocks
    });

    // Make the POST request with JSON
    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json; charset=utf-8")
        .json(&payload)
        .send()
        .await?;

    // Check response status
    if response.status().is_success() {
        let response_body: Value = response.json().await?;
        if response_body["ok"].as_bool().unwrap_or(false) {
            Ok(())
        } else {
            let error = response_body["error"]
                .as_str()
                .unwrap_or("Unknown error");
            Err(format!("Slack API error: {}", error).into())
        }
    } else {
        Err(format!("HTTP error: {}", response.status()).into())
    }
}
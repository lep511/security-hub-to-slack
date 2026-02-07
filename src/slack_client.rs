use reqwest::Client;
use crate::struct_event::FindingSummary;
use serde_json::Value;
use std::error::Error;
use serde_json::json;

pub async fn post_slack_message(
    token: &str,
    channel: &str,
    summary: FindingSummary,
) -> Result<(), Box<dyn Error>> {

    let image_icon_url = match summary.product_name.as_str() {
        "Inspector" => "https://raw.githubusercontent.com/lep511/security-hub-to-slack/refs/heads/main/image-icons/Arch_Amazon-Inspector_64.png",
        "Macie" => "https://raw.githubusercontent.com/lep511/security-hub-to-slack/refs/heads/main/image-icons/Arch_Amazon-Macie_64.png",
        "WAF" => "https://raw.githubusercontent.com/lep511/security-hub-to-slack/refs/heads/main/image-icons/Arch_AWS-WAF_64.png",
        "Shield" => "https://raw.githubusercontent.com/lep511/security-hub-to-slack/refs/heads/main/image-icons/Arch_AWS-Shield_64.png",
        "GuardDuty" => "https://raw.githubusercontent.com/lep511/security-hub-to-slack/refs/heads/main/image-icons/Arch_Amazon-Guard-Duty_64.png",
        "Detective" => "https://raw.githubusercontent.com/lep511/security-hub-to-slack/refs/heads/main/image-icons/Arch_Amazon-Detective_64.png",
        "Config" => "https://raw.githubusercontent.com/lep511/security-hub-to-slack/refs/heads/main/image-icons/Arch_AWS-Config_64.png",
        "IAM Access Analyzer" => "https://raw.githubusercontent.com/lep511/security-hub-to-slack/refs/heads/main/image-icons/Arch_AWS-Identity-and-Access-Management_64.png",
        _ => "https://raw.githubusercontent.com/lep511/security-hub-to-slack/refs/heads/main/image-icons/Arch_AWS-Security-Hub_64.png"
    };

    // Build the blocks for the Slack message
    let mut blocks = vec![
        json!({
			"type": "header",
			"text": {
				"type": "plain_text",
				"text": &summary.title,
				"emoji": true
			}
		}),
        json!({
			"type": "section",
			"text": {
				"type": "mrkdwn",
				"text": format!("_{}_", &summary.description)
			},
			"accessory": {
				"type": "image",
				"image_url": image_icon_url,
				"alt_text": "aws-service"
			}
		})
    ];

    blocks.push(json!(
		{
			"type": "rich_text",
			"elements": [
				{
					"type": "rich_text_section",
					"elements": [
						{
							"type": "text",
							"text": format!("• Product Name: {}", &summary.product_name),
                            "style": {
								"bold": true
							}
						},
                        {
                            "type": "text",
                            "text": format!("\n• Severity: {}", &summary.severity),
                            "style": {
								"bold": true
							}
                        },
						{
							"type": "text",
							"text": format!("\n• Account: {}", &summary.account)
						},
                        {
                            "type": "text",
                            "text": format!("  |  Region: {}", &summary.region)
                        },
                        {
                            "type": "text",
                            "text": format!("  |  Resource Id: {}", &summary.resource_id)
                        }
					]
				}
			]
		}
    ));

    if summary.remediation != "no_remediation" {
        blocks.push(json!(
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": "`Click the button to view the details of the remediation  ->`"
                },
                "accessory": {
                    "type": "button",
                    "text": {
                        "type": "plain_text",
                        "text": "Remediations",
                        "emoji": true
                    },
                    "value": "click_me_123",
                    "url": summary.remediation
                }
            }
        ));
    }
    
    blocks.push(json!(
        {
			"type": "divider"
		}
    ));

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
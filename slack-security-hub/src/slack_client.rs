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
        "Inspector" => "https://res.cloudinary.com/dgslmcpqb/image/upload/v1770473027/Arch_Amazon-Inspector_64_mwcrkr.png",
        "Macie" => "https://res.cloudinary.com/dgslmcpqb/image/upload/v1770473027/Arch_Amazon-Macie_64_fqdobr.png",
        "WAF" => "https://res.cloudinary.com/dgslmcpqb/image/upload/v1770473046/Arch_AWS-WAF_64_sy685i.png",
        "Shield" => "https://res.cloudinary.com/dgslmcpqb/image/upload/v1770473044/Arch_AWS-Shield_64_cgkrnf.png",
        "GuardDuty" => "https://res.cloudinary.com/dgslmcpqb/image/upload/v1770473027/Arch_Amazon-GuardDuty_64_olhgt8.png",
        "Detective" => "https://res.cloudinary.com/dgslmcpqb/image/upload/v1770473028/Arch_Amazon-Detective_64_c2ytyn.png",
        "Config" => "https://res.cloudinary.com/dgslmcpqb/image/upload/v1770473312/Arch_AWS-Config_64_qmcyvc.png",
        "IAM Access Analyzer" => "https://res.cloudinary.com/dgslmcpqb/image/upload/v1770473035/Arch_AWS-Identity-and-Access-Management_64_twn9yu.png",
        _ => "https://res.cloudinary.com/dgslmcpqb/image/upload/v1770473042/Arch_AWS-Security-Hub_64_r5hhru.png"
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
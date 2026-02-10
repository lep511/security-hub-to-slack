use lambda_http::{Body, Error, Request, Response, tracing};
use aws_config::BehaviorVersion;
use aws_sdk_secretsmanager::Client as SMClient;
use serde::Deserialize;
use serde_json::Value;
use crate::slack_client::post_slack_message;

#[derive(Deserialize, Debug)]
struct SlackChallenge {
    challenge: String,
}

#[derive(Deserialize, Debug)]
struct SlackEventCallback {
    event: SlackEvent,
}

#[derive(Deserialize, Debug)]
struct SlackEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    user: String,
    #[serde(default)]
    channel: String,
    #[serde(default)]
    blocks: Vec<Block>,
}

#[derive(Deserialize, Debug)]
struct Block {
    #[serde(default)]
    elements: Vec<BlockElement>,
}

#[derive(Deserialize, Debug)]
struct BlockElement {
    #[serde(default)]
    elements: Vec<RichTextElement>,
}

#[derive(Deserialize, Debug)]
struct RichTextElement {
    #[serde(rename = "type")]
    element_type: String,
    #[serde(default)]
    text: String,
}

fn extract_text_from_blocks(blocks: &[Block]) -> String {
    blocks
        .iter()
        .flat_map(|block| &block.elements)
        .flat_map(|element| &element.elements)
        .filter(|e| e.element_type == "text")
        .map(|e| e.text.trim())
        .collect::<Vec<_>>()
        .join(" ")
}

async fn event_app_mention_handler(event: &SlackEvent, token: &str) -> Result<(), Error> {
    let clean_text = extract_text_from_blocks(&event.blocks);
    
    tracing::info!("App mention from user: {}", event.user);
    tracing::info!("Text: {}", clean_text);
    tracing::info!("Channel: {}", event.channel);
    
    match post_slack_message(&token, &event.channel, &clean_text).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let err_msg = format!("Failed to post message to Slack: {}", e);
            tracing::error!("{}", err_msg);
            Err(err_msg.into())
        }
    }
}

pub(crate) async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let body_str = match event.body() {
        Body::Text(s) => s.as_str(),
        Body::Binary(b) => std::str::from_utf8(b)?,
        Body::Empty => "",
        _ => return Err("Unsupported body type".into()),
    };

    let payload: Value = serde_json::from_str(body_str)?;
    tracing::info!("Received event: {}", payload);
    let event_type = payload.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let config = aws_config::defaults(BehaviorVersion::latest())
        .load()
        .await;

    let secrets_client = SMClient::new(&config);
    let secret_name = "slack-token";

    match event_type {
        "url_verification" => {
            let challenge: SlackChallenge = serde_json::from_value(payload)?;
            tracing::info!("URL verification challenge");
            Ok(Response::builder()
                .status(200)
                .header("content-type", "text/plain")
                .body(challenge.challenge.into())?)
        }
        "event_callback" => {
            let event_callback: SlackEventCallback = serde_json::from_value(payload)?;
            let token = match get_secret(&secrets_client, secret_name).await {
                Ok(token) => token,
                Err(e) => {
                    let err_msg = format!("Failed to retrieve secret '{}': {}", secret_name, e);
                    tracing::error!("{}", err_msg);
                    return Err(err_msg.into());
                }
            };
            
            if event_callback.event.event_type == "app_mention" {
                let clean_text = extract_text_from_blocks(&event_callback.event.blocks);
                
                tracing::info!("App mention from user: {}", event_callback.event.user);
                tracing::info!("Text: {}", clean_text);
                tracing::info!("Channel: {}", event_callback.event.channel);
                
                event_app_mention_handler(&event_callback.event, &token).await?;
            }
            
            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body("{\"message\": \"Event handled\"}".into())?)
        }
        _ => Ok(Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body("{\"message\": \"Event not handled\"}".into())?),
    }
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
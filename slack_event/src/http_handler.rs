use lambda_http::{Body, Error, Request, Response};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct SlackChallenge {
    token: String,
    challenge: String,
    #[serde(rename = "type")]
    event_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackEventCallback {
    token: String,
    team_id: String,
    api_app_id: String,
    event: SlackEvent,
    #[serde(rename = "type")]
    event_type: String,
    event_id: String,
    event_time: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackEvent {
    #[serde(rename = "type")]
    event_type: String,
    user: String,
    text: String,
    channel: String,
    ts: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SlackMessage {
    channel: String,
    text: String,
}

pub(crate) async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    println!("Received request: {:?}", event);
    
    let body = event.body();
    let body_str = std::str::from_utf8(body.as_ref()).unwrap_or("");
    
    // first check for url_verification challenge
    if let Ok(slack_challenge) = serde_json::from_str::<SlackChallenge>(body_str) {
        if slack_challenge.event_type == "url_verification" {
            println!("Processing url_verification challenge");
            let resp = Response::builder()
                .status(200)
                .header("content-type", "text/plain")
                .body(slack_challenge.challenge.into())
                .map_err(Box::new)?;
            return Ok(resp);
        }
    }
    
    // app_mention event
    if let Ok(slack_event) = serde_json::from_str::<SlackEventCallback>(body_str) {
        if slack_event.event_type == "event_callback" {
            match slack_event.event.event_type.as_str() {
                "app_mention" => {
                    println!("Processing app_mention event");
                    println!("User: {}", slack_event.event.user);
                    println!("Text: {}", slack_event.event.text);
                    println!("Channel: {}", slack_event.event.channel);
                    
                    // Here you can process the mention and respond
                    // For now, just confirm receipt
                    let resp = Response::builder()
                        .status(200)
                        .header("content-type", "application/json")
                        .body(Body::from("{}"))
                        .map_err(Box::new)?;
                    return Ok(resp);
                }
                _ => {
                    println!("Unhandled event type: {}", slack_event.event.event_type);
                }
            }
        }
    }
    
    // Default response
    let resp = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .map_err(Box::new)?;
    Ok(resp)
}
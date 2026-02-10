use aws_sdk_sns::Client;
use aws_sdk_sns::Error as SnsError;

#[derive(Debug)]
enum SubscriptionStatus {
    Subscribed,
    PendingConfirmation,
    NotSubscribed,
}

async fn check_email_subscription(
    client: &Client,
    topic_arn: &str,
    email: &str
) -> Result<SubscriptionStatus, SnsError> {
    let mut next_token: Option<String> = None;
    
    loop {
        let mut request = client
            .list_subscriptions_by_topic()
            .topic_arn(topic_arn);
        
        if let Some(token) = next_token {
            request = request.next_token(token);
        }
        
        let response = request.send().await?;
        
        if let Some(subscriptions) = response.subscriptions() {
            for subscription in subscriptions {
                if let Some(endpoint) = subscription.endpoint() {
                    if endpoint == email && subscription.protocol() == Some("email") {
                        // Verificar el estado de la suscripción
                        if let Some(sub_arn) = subscription.subscription_arn() {
                            if sub_arn == "PendingConfirmation" {
                                return Ok(SubscriptionStatus::PendingConfirmation);
                            } else {
                                return Ok(SubscriptionStatus::Subscribed);
                            }
                        }
                    }
                }
            }
        }
        
        next_token = response.next_token().map(|s| s.to_string());
        if next_token.is_none() {
            break;
        }
    }
    
    Ok(SubscriptionStatus::NotSubscribed)
}

async fn subscribe_email(
    client: &Client,
    topic_arn: &str,
    email: &str
) -> Result<String, SnsError> {
    let response = client
        .subscribe()
        .topic_arn(topic_arn)
        .protocol("email")
        .endpoint(email)
        .return_subscription_arn(true)
        .send()
        .await?;
    
    let subscription_arn = response
        .subscription_arn()
        .unwrap_or("PendingConfirmation")
        .to_string();
    
    Ok(subscription_arn)
}

async fn ensure_email_subscribed(
    client: &Client,
    topic_arn: &str,
    email: &str
) -> Result<SubscriptionStatus, SnsError> {
    let status = check_email_subscription(client, topic_arn, email).await?;
    
    match status {
        SubscriptionStatus::Subscribed => {
            println!("✓ El email {} ya está suscrito y confirmado", email);
            Ok(SubscriptionStatus::Subscribed)
        }
        SubscriptionStatus::PendingConfirmation => {
            println!("⏳ El email {} está suscrito pero pendiente de confirmación", email);
            Ok(SubscriptionStatus::PendingConfirmation)
        }
        SubscriptionStatus::NotSubscribed => {
            println!("📧 Suscribiendo el email {} al topic...", email);
            let subscription_arn = subscribe_email(client, topic_arn, email).await?;
            
            if subscription_arn == "PendingConfirmation" {
                println!("✉️  Se ha enviado un correo de confirmación a {}", email);
                println!("   El usuario debe confirmar la suscripción desde su email");
                Ok(SubscriptionStatus::PendingConfirmation)
            } else {
                println!("✓ Email {} suscrito correctamente", email);
                Ok(SubscriptionStatus::Subscribed)
            }
        }
    }
}

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // Configurar el cliente de AWS
//     let config = aws_config::load_from_env().await;
//     let client = Client::new(&config);
    
//     let topic_arn = "arn:aws:sns:us-east-1:123456789012:MyTopic";
//     let email = "usuario@ejemplo.com";
    
//     // Verificar y suscribir si es necesario
//     match ensure_email_subscribed(&client, topic_arn, email).await {
//         Ok(status) => {
//             println!("\nEstado final: {:?}", status);
//         }
//         Err(e) => {
//             eprintln!("Error: {:?}", e);
//         }
//     }
    
//     Ok(())
// }
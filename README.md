# Security Hub to Slack

A Rust-based serverless application that integrates AWS Security Hub findings with Slack, enabling real-time security monitoring and notifications directly in your Slack workspace.

## 📋 Overview

This project provides automated security incident notifications from AWS Security Hub to Slack channels. It consists of three main components working together to deliver a seamless security monitoring experience:

- **Real-time Security Hub alerts** sent automatically to Slack
- **Interactive Slack bot** that responds to user queries
- **SNS email subscription management** for user notifications

## 🏗️ Architecture

```
AWS Security Hub → EventBridge → Lambda (slack-security-hub) → Slack Channel
                                                              ↓
User mentions Bot in Slack → Lambda (slack-event) → Process → Response
                                                              ↓
                                           SNS (sns-client) → Email notifications
```

## 📁 Project Structure

```
SECURITY-HUB-TO-SLACK/
├── slack-event/              # Slack bot event handler
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── slack-security-hub/       # Security Hub findings processor
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
└── Cargo.toml                # Workspace configuration
```

### Components

#### 1. `slack-event/`
Lambda function built with Cargo Lambda that responds to Slack events when the bot is mentioned. Handles interactive commands and user requests within Slack.

**Features:**
- Responds to bot mentions (@SecurityBot)
- Processes Slack slash commands
- Interactive message handling
- Real-time responses to user queries

#### 2. `slack-security-hub/`
Lambda function built with Cargo Lambda that listens to AWS Security Hub findings via EventBridge and sends formatted notifications to designated Slack channels.

**Features:**
- Automatic Security Hub finding notifications
- Severity-based message formatting
- Rich Slack message blocks with incident details
- Filtering by severity level
- Custom channel routing

## 🚀 Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (1.70 or later)
- [Cargo Lambda](https://www.cargo-lambda.info/guide/installation.html)
- AWS Account with appropriate permissions
- Slack workspace with admin access
- AWS CLI configured with credentials

### Installation

1. **Clone the repository**
```bash
git clone https://github.com/yourusername/security-hub-to-slack.git
cd security-hub-to-slack
```

2. **Install Cargo Lambda**
```bash
pip3 install cargo-lambda
```

3. **Build the workspace**
```bash
cargo build --release
```

4. **Build Lambda functions**
```bash
# Build all Lambda functions
cargo lambda build --release -p slack-event
cargo lambda build --release -p slack-security-hub
```

5. **Deploy Lambda functions**
```bash
# Build all Lambda functions
cargo lambda deploy -p slack-event
cargo lambda deploy -p slack-security-hub
```

## ⚙️ Configuration

### 1. Slack Setup

#### Create a Slack App

1. Go to [Slack API Dashboard](https://api.slack.com/apps)
2. Create a new app "From scratch"
3. Name it (e.g., "Security Hub Bot")
4. Select your workspace

#### Configure Bot Permissions

Add the following OAuth scopes under "OAuth & Permissions":
- `chat:write` - Send messages
- `chat:write.public` - Send messages to public channels
- `app_mentions:read` - Read mentions
- `commands` - Add slash commands

#### Enable Events

1. Go to "Event Subscriptions"
2. Enable events
3. Add bot user events:
   - `app_mention`
   - `message.channels`

4. Set Request URL to your `slack-event` Lambda function URL

#### Install App to Workspace

1. Go to "Install App"
2. Install to your workspace
3. Copy the "Bot User OAuth Token" (starts with `xoxb-`)

### 2. AWS Setup

#### Environment Variables

Set the following environment variables for your Lambda functions:

**slack-event:**
```bash
SLACK_BOT_TOKEN=xoxb-your-bot-token
SLACK_SIGNING_SECRET=your-signing-secret
SNS_TOPIC_ARN=arn:aws:sns:region:account-id:topic-name
```

**slack-security-hub:**
```bash
SLACK_WEBHOOK_URL=https://hooks.slack.com/services/YOUR/WEBHOOK/URL
SLACK_CHANNEL_ID=C01234567890
SEVERITY_FILTER=HIGH,CRITICAL  # Optional: filter by severity
```

**sns-client:**
```bash
AWS_REGION=us-east-1
SNS_TOPIC_ARN=arn:aws:sns:region:account-id:topic-name
```

#### IAM Permissions

Create an IAM role for your Lambda functions with these policies:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "sns:Publish",
        "sns:Subscribe",
        "sns:ListSubscriptionsByTopic"
      ],
      "Resource": "arn:aws:sns:*:*:*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "securityhub:GetFindings",
        "securityhub:BatchUpdateFindings"
      ],
      "Resource": "*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "logs:CreateLogGroup",
        "logs:CreateLogStream",
        "logs:PutLogEvents"
      ],
      "Resource": "*"
    }
  ]
}
```

### 3. EventBridge Rule

Create an EventBridge rule to trigger the `slack-security-hub` Lambda:

```json
{
  "source": ["aws.securityhub"],
  "detail-type": ["Security Hub Findings - Imported"],
  "detail": {
    "findings": {
      "Severity": {
        "Label": ["HIGH", "CRITICAL"]
      }
    }
  }
}
```

## 🚢 Deployment

### Store Slack Token in Secrets Manager

Create a secret in AWS Secrets Manager to store your Slack OAuth token:

```bash
aws secretsmanager create-secret \
    --name slack-token \
    --description "Slack OAuth token for Security Hub notifications" \
    --secret-string '{"token":"xoxb-your-token-here"}' \
    --region {REGION}
```

### Using Cargo Lambda

```bash
# Deploy slack-event function
cargo lambda build slack-event --release --arm64
cargo lambda deploy slack-event 

# Deploy slack-security-hub function
cargo lambda build slack-security-hub --release --arm64
cargo lambda deploy slack-security-hub

## 🔧 Development

### Local Development Setup

```bash
# Install development dependencies
cargo install cargo-watch
cargo install cargo-lambda

# Watch for changes and rebuild
cargo watch -x "build --release"
```

### Adding New Dependencies

Add workspace dependencies in the root `Cargo.toml`:

```toml
[workspace.dependencies]
new-dependency = "1.0"
```

Then use in individual crates:

```toml
[dependencies]
new-dependency.workspace = true
```

## 📊 Monitoring

### CloudWatch Logs

View Lambda logs:

```bash
aws logs tail /aws/lambda/slack-event --follow
aws logs tail /aws/lambda/slack-security-hub --follow
```

### Metrics

Key metrics to monitor:
- Lambda invocation count
- Lambda error rate
- Lambda duration
- SNS publish success rate
- Slack API response time

## 🛡️ Security Best Practices

1. **Secrets Management**: Store sensitive data in AWS Secrets Manager or Parameter Store
2. **Least Privilege**: Use minimal IAM permissions for Lambda functions
3. **Encryption**: Enable encryption at rest for all data
4. **Logging**: Enable CloudWatch Logs for audit trails
5. **Validation**: Verify Slack webhook signatures
6. **Rate Limiting**: Implement rate limiting for API calls

# Security Hub to Slack

🔔 Real-time AWS Security Hub findings notifications to Slack. This serverless solution automatically routes high-severity and critical security findings from AWS Security Hub to your Slack channels, helping your security team respond quickly to threats.

## Overview

This project implements an AWS Lambda function written in Rust that processes AWS Security Hub findings (OCSF v2 format) and sends formatted notifications to Slack channels. The Lambda is triggered by EventBridge rules that filter for high-severity and critical findings, ensuring your team is immediately notified of important security events.

## Features

- **Real-time Notifications**: Instant Slack alerts when critical or high-severity findings are detected
- **OCSF v2 Support**: Compatible with AWS Security Hub's OCSF (Open Cybersecurity Schema Framework) v2 format
- **Smart Filtering**: EventBridge integration filters findings by severity (High/Critical) and status (New)
- **Rich Slack Messages**: Formatted Slack messages with service icons, severity indicators, and remediation links
- **Multi-Service Support**: Handles findings from GuardDuty, Inspector, Macie, IAM Access Analyzer, Config, WAF, Shield, and Detective
- **Secure Secrets Management**: Uses AWS Secrets Manager to securely store Slack tokens
- **Production-Ready**: Built with Rust for performance, safety, and low memory footprint

## Architecture

The solution consists of the following components:

1. **AWS Security Hub**: Aggregates security findings from multiple AWS services
2. **Amazon EventBridge**: Filters findings based on severity and status
3. **AWS Lambda (Rust)**: Processes findings and formats Slack messages
4. **AWS Secrets Manager**: Stores Slack OAuth token securely
5. **Slack API**: Receives and displays formatted security alerts

```
AWS Security Hub → EventBridge Rule → Lambda Function → Slack Channel
                    (Filter)          (Process & Format)
```

## Prerequisites

Before deploying this solution, ensure you have:

- An AWS account with Security Hub enabled
- A Slack workspace with admin permissions
- Rust toolchain installed (1.70 or later)
- AWS CLI configured with appropriate credentials
- Cargo Lambda installed for building and deploying

To install Cargo Lambda:

```bash
pip install cargo-lambda
```

## Slack Setup

### Step 1: Create a Slack App

Navigate to the Slack API portal at https://api.slack.com/apps and create a new app. Choose "From scratch" and select your workspace.

### Step 2: Configure OAuth Permissions

In your Slack app settings, navigate to OAuth & Permissions and add the following Bot Token Scopes:

- `chat:write` - To post messages to channels
- `chat:write.public` - To post to public channels without joining

### Step 3: Install App to Workspace

After configuring permissions, install the app to your workspace. This will generate a Bot User OAuth Token that starts with `xoxb-`. Save this token as you'll need it for AWS Secrets Manager.

### Step 4: Invite Bot to Channel

Invite your bot to the channel where you want to receive notifications. For example, if using `#aws-security`, type `/invite @YourBotName` in that channel.

## AWS Setup

### Step 1: Store Slack Token in Secrets Manager

Create a secret in AWS Secrets Manager to store your Slack OAuth token:

```bash
aws secretsmanager create-secret \
    --name slack-token \
    --description "Slack OAuth token for Security Hub notifications" \
    --secret-string '{"token":"xoxb-your-token-here"}' \
    --region {REGION}
```

### Step 2: Build the Lambda Function

Build the Rust Lambda function using Cargo Lambda:

```bash
cargo lambda build --release --arm64
```

### Step 3: Deploy to AWS Lambda

```bash
cargo lambda deploy
```

Make sure your Lambda execution role has the following permissions:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "logs:CreateLogGroup",
        "logs:CreateLogStream",
        "logs:PutLogEvents"
      ],
      "Resource": "arn:aws:logs:*:*:*"
    },
    {
      "Effect": "Allow",
      "Action": [
        "secretsmanager:GetSecretValue"
      ],
      "Resource": "arn:aws:secretsmanager:{REGION}:{YOUR_ACCOUNT_ID}:secret:slack-token-*"
    }
  ]
}
```

### Step 4: Create EventBridge Rule

Create a file named `event-pattern.json` with the following content:

```json
{
  "source": ["aws.securityhub"],
  "detail-type": ["Findings Imported V2"],
  "detail": {
    "findings": {
      "severity": ["High", "Critical"],
      "status": ["New"]
    }
  }
}
```

Create an EventBridge rule to trigger the Lambda function when high-severity findings are imported:

```bash
aws events put-rule \
    --name security-hub-to-slack-rule \
    --event-pattern file://event-pattern.json \
    --state ENABLED \
    --region {REGION}
```

### Step 5: Add Lambda Permission and Target

Allow EventBridge to invoke the Lambda function:

```bash
aws lambda add-permission \
    --function-name security-hub-to-slack \
    --statement-id security-hub-eventbridge \
    --action lambda:InvokeFunction \
    --principal events.amazonaws.com \
    --source-arn arn:aws:events:{REGION}:{YOUR_ACCOUNT_ID}:rule/security-hub-to-slack-rule \
    --region {REGION}
```

Add the Lambda as a target for the EventBridge rule:

```bash
aws events put-targets \
    --rule security-hub-to-slack-rule \
    --targets "Id"="1","Arn"="arn:aws:lambda:{REGION}:{YOUR_ACCOUNT_ID}:function:security-hub-to-slack" \
    --region {REGION}
```

## Configuration

### Customizing the Slack Channel

By default, notifications are sent to `#aws-security`. To change this, modify the channel name in `event_handler.rs`:

```rust
let channel = "#aws-security"; // Change to your preferred channel
```

After making changes, rebuild and redeploy the Lambda function.

### Customizing Severity Filters

The EventBridge rule filters for High and Critical severity findings. To include Medium severity findings, update the event pattern:

```json
{
  "source": ["aws.securityhub"],
  "detail-type": ["Findings Imported V2"],
  "detail": {
    "findings": {
      "severity": ["High", "Critical", "Medium"],
      "status": ["New"]
    }
  }
}
```

## Project Structure

The project is organized into several Rust modules, each with a specific responsibility:

- **`main.rs`**: Entry point for the Lambda function, initializes the Lambda runtime
- **`event_handler.rs`**: Processes EventBridge events, retrieves secrets, and coordinates the notification flow
- **`struct_event.rs`**: Defines the data structures for OCSF v2 Security Hub findings
- **`slack_client.rs`**: Handles Slack API integration and message formatting
- **`utils.rs`**: Utility functions for processing findings (currently not actively used)

### Key Data Structures

The `Finding` struct represents a complete Security Hub finding with all its nested components including cloud metadata, evidence, resources, and remediation information. The `FindingSummary` struct extracts the most important information for display in Slack:

- Title of the finding
- AWS region and account ID
- AWS service that generated the finding
- Resource ID affected by the finding
- Severity level (High or Critical)
- Description of the security issue
- Link to remediation documentation

## Slack Message Format

The Lambda function sends rich, formatted messages to Slack that include:

1. **Header**: The finding title
2. **Description**: A brief explanation of the security issue with the AWS service icon
3. **Details Section**: 
   - Product name (e.g., GuardDuty, Inspector)
   - Severity level
   - AWS account ID
   - AWS region
   - Affected resource ID
4. **Remediation Button**: A clickable button linking to AWS documentation (when available)

## Troubleshooting

### Lambda Function Not Triggering

Check that your EventBridge rule is enabled and the event pattern matches the findings being generated. Verify that Security Hub is sending findings in OCSF v2 format.

### Slack Messages Not Appearing

Verify that the Slack OAuth token is correctly stored in Secrets Manager and that your Lambda execution role has permission to read it. Check that your bot has been invited to the target channel and has the necessary OAuth scopes.

### Permission Denied Errors

Ensure your Lambda execution role has the `secretsmanager:GetSecretValue` permission for the slack-token secret and that CloudWatch Logs permissions are properly configured.

### Missing Service Icons

The service icons are hosted on GitHub. If they're not displaying, verify the URLs in `slack_client.rs` are accessible. You can also host the icons in your own S3 bucket and update the URLs accordingly.

## Security Best Practices

When deploying this solution, follow these security guidelines:

- Use AWS Secrets Manager rotation for the Slack OAuth token
- Apply least-privilege IAM policies to the Lambda execution role
- Enable Lambda function URL only if necessary (not required for EventBridge triggers)
- Enable AWS CloudTrail logging for audit purposes
- Regularly review and update the EventBridge event pattern to avoid alert fatigue

## Cost Estimation

The cost of running this solution is minimal. For a typical AWS environment receiving 100 high-severity findings per day:

- **Lambda**: $0.00002 per invocation × 3,000 per month = $0.06
- **EventBridge**: No charge for rules
- **Secrets Manager**: $0.40 per secret per month
- **CloudWatch Logs**: Approximately $0.50 per month

**Total estimated monthly cost**: Less than $1.00

---

**Note**: This project processes sensitive security information. Ensure your Slack workspace and AWS account follow your organization's security policies and compliance requirements.
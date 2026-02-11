use anyhow::{Context, Result};
use aws_sdk_organizations::{types::PolicyType, Client};
use aws_config::BehaviorVersion;
use colored::*;

pub struct AwsScpManager {
    client: Client,
}

impl AwsScpManager {
    pub async fn new() -> Result<Self> {
        let config = aws_config::defaults(BehaviorVersion::latest())
            .load()
            .await;
        let client = Client::new(&config);
        Ok(Self { client })
    }

    /// Creates a new SCP in AWS Organizations
    pub async fn create_scp(
        &self,
        name: &str,
        description: &str,
        content: &str,
    ) -> Result<String> {
        println!("{}", "📝 Creating SCP in AWS Organizations...".cyan());

        let response = self
            .client
            .create_policy()
            .name(name)
            .r#type(PolicyType::ServiceControlPolicy)
            .description(description)
            .content(content)
            .send()
            .await
            .context("Error creating policy in AWS")?;

        let policy_id = response
            .policy()
            .and_then(|p| p.policy_summary())
            .and_then(|s| s.id())
            .context("Could not get Policy ID")?
            .to_string();

        println!("{}", "✅ SCP created successfully!".green().bold());
        println!("   Policy ID: {}", policy_id.yellow());

        if let Some(policy) = response.policy() {
            if let Some(summary) = policy.policy_summary() {
                if let Some(arn) = summary.arn() {
                    println!("   ARN: {}", arn.bright_black());
                }
            }
        }

        Ok(policy_id)
    }

    /// Lists all existing SCPs
    pub async fn list_scps(&self) -> Result<Vec<(String, String)>> {
        let response = self
            .client
            .list_policies()
            .filter(PolicyType::ServiceControlPolicy)
            .send()
            .await
            .context("Error listing policies")?;

        let mut policies = Vec::new();

        let policy_summaries = response.policies();
        for summary in policy_summaries {
            let id = summary.id().unwrap_or("N/A").to_string();
            let name = summary.name().unwrap_or("N/A").to_string();
            policies.push((id, name));
        }

        Ok(policies)
    }

    /// Attaches an SCP to a target (OU or Account)
    pub async fn attach_policy(&self, policy_id: &str, target_id: &str) -> Result<()> {
        println!(
            "{}",
            format!("📎 Attaching SCP {} to {}...", policy_id, target_id).cyan()
        );

        self.client
            .attach_policy()
            .policy_id(policy_id)
            .target_id(target_id)
            .send()
            .await
            .context("Error attaching policy")?;

        println!("{}", "✅ SCP attached successfully!".green().bold());

        Ok(())
    }

    /// Lists organization roots
    pub async fn list_roots(&self) -> Result<Vec<(String, String)>> {
        let response = self
            .client
            .list_roots()
            .send()
            .await
            .context("Error listing roots")?;

        let mut roots = Vec::new();

        let root_list = response.roots();
        for root in root_list {
            let id = root.id().unwrap_or("N/A").to_string();
            let name = root.name().unwrap_or("N/A").to_string();
            roots.push((id, name));
        }

        Ok(roots)
    }

    /// Lists OUs under a parent
    pub async fn list_ous(&self, parent_id: &str) -> Result<Vec<(String, String)>> {
        let response = self
            .client
            .list_organizational_units_for_parent()
            .parent_id(parent_id)
            .send()
            .await
            .context("Error listing OUs")?;

        let mut ous = Vec::new();

        let ou_list = response.organizational_units();
        for ou in ou_list {
            let id = ou.id().unwrap_or("N/A").to_string();
            let name = ou.name().unwrap_or("N/A").to_string();
            ous.push((id, name));
        }

        Ok(ous)
    }

    /// Lists accounts under a parent
    pub async fn list_accounts_for_parent(&self, parent_id: &str) -> Result<Vec<(String, String, String)>> {
        let response = self
            .client
            .list_accounts_for_parent()
            .parent_id(parent_id)
            .send()
            .await
            .context("Error listing accounts")?;

        let mut accounts = Vec::new();

        let account_list = response.accounts();
        for account in account_list {
            let id = account.id().unwrap_or("N/A").to_string();
            let name = account.name().unwrap_or("N/A").to_string();
            let email = account.email().unwrap_or("N/A").to_string();
            accounts.push((id, name, email));
        }

        Ok(accounts)
    }
}
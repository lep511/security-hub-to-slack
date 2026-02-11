use anyhow::{Context, Result};
use aws_sdk_organizations::{types::PolicyType, Client};
use colored::*;

pub struct AwsScpManager {
    client: Client,
}

impl AwsScpManager {
    pub async fn new() -> Result<Self> {
        let config = aws_config::load_from_env().await;
        let client = Client::new(&config);
        Ok(Self { client })
    }

    /// Crea una nueva SCP en AWS Organizations
    pub async fn create_scp(
        &self,
        name: &str,
        description: &str,
        content: &str,
    ) -> Result<String> {
        println!("{}", "📝 Creando SCP en AWS Organizations...".cyan());

        let response = self
            .client
            .create_policy()
            .name(name)
            .description(description)
            .r#type(PolicyType::ServiceControlPolicy)
            .content(content)
            .send()
            .await
            .context("Error al crear la política en AWS")?;

        let policy_id = response
            .policy()
            .and_then(|p| p.policy_summary())
            .and_then(|s| s.id())
            .context("No se pudo obtener el Policy ID")?
            .to_string();

        println!("{}", "✅ SCP creada exitosamente!".green().bold());
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

    /// Lista todas las SCPs existentes
    pub async fn list_scps(&self) -> Result<Vec<(String, String, String)>> {
        let response = self
            .client
            .list_policies()
            .filter(PolicyType::ServiceControlPolicy)
            .send()
            .await
            .context("Error al listar políticas")?;

        let mut policies = Vec::new();

        if let Some(policy_summaries) = response.policies() {
            for summary in policy_summaries {
                let id = summary.id().unwrap_or("N/A").to_string();
                let name = summary.name().unwrap_or("N/A").to_string();
                let description = summary.description().unwrap_or("").to_string();
                policies.push((id, name, description));
            }
        }

        Ok(policies)
    }

    /// Adjunta una SCP a un target (OU o Account)
    pub async fn attach_policy(&self, policy_id: &str, target_id: &str) -> Result<()> {
        println!(
            "{}",
            format!("📎 Adjuntando SCP {} a {}...", policy_id, target_id).cyan()
        );

        self.client
            .attach_policy()
            .policy_id(policy_id)
            .target_id(target_id)
            .send()
            .await
            .context("Error al adjuntar la política")?;

        println!("{}", "✅ SCP adjuntada exitosamente!".green().bold());

        Ok(())
    }

    /// Lista las raíces de la organización
    pub async fn list_roots(&self) -> Result<Vec<(String, String)>> {
        let response = self
            .client
            .list_roots()
            .send()
            .await
            .context("Error al listar roots")?;

        let mut roots = Vec::new();

        if let Some(root_list) = response.roots() {
            for root in root_list {
                let id = root.id().unwrap_or("N/A").to_string();
                let name = root.name().unwrap_or("N/A").to_string();
                roots.push((id, name));
            }
        }

        Ok(roots)
    }

    /// Lista las OUs bajo un parent
    pub async fn list_ous(&self, parent_id: &str) -> Result<Vec<(String, String)>> {
        let response = self
            .client
            .list_organizational_units_for_parent()
            .parent_id(parent_id)
            .send()
            .await
            .context("Error al listar OUs")?;

        let mut ous = Vec::new();

        if let Some(ou_list) = response.organizational_units() {
            for ou in ou_list {
                let id = ou.id().unwrap_or("N/A").to_string();
                let name = ou.name().unwrap_or("N/A").to_string();
                ous.push((id, name));
            }
        }

        Ok(ous)
    }

    /// Lista las cuentas bajo un parent
    pub async fn list_accounts_for_parent(&self, parent_id: &str) -> Result<Vec<(String, String, String)>> {
        let response = self
            .client
            .list_accounts_for_parent()
            .parent_id(parent_id)
            .send()
            .await
            .context("Error al listar cuentas")?;

        let mut accounts = Vec::new();

        if let Some(account_list) = response.accounts() {
            for account in account_list {
                let id = account.id().unwrap_or("N/A").to_string();
                let name = account.name().unwrap_or("N/A").to_string();
                let email = account.email().unwrap_or("N/A").to_string();
                accounts.push((id, name, email));
            }
        }

        Ok(accounts)
    }
}
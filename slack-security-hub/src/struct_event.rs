use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Detail {
    pub findings: Option<Vec<Finding>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Finding {
    pub activity_id: Option<i32>,
    pub activity_name: Option<String>,
    pub category_name: Option<String>,
    pub category_uid: Option<i32>,
    pub class_name: Option<String>,
    pub class_uid: Option<i32>,
    pub cloud: Option<Cloud>,
    pub count: Option<i32>,
    pub evidences: Option<Vec<Evidence>>,
    pub finding_info: Option<FindingInfo>,
    pub metadata: Option<Metadata>,
    pub osint: Option<Vec<Osint>>,
    pub remediation: Option<Remediation>,
    pub resources: Option<Vec<Resource>>,
    pub severity: Option<String>,
    pub severity_id: Option<i32>,
    pub status: Option<String>,
    pub status_id: Option<i32>,
    pub time: Option<i64>,
    pub time_dt: Option<String>,
    pub type_name: Option<String>,
    pub type_uid: Option<i32>,
    pub vendor_attributes: Option<VendorAttributes>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Cloud {
    pub account: Option<Account>,
    pub cloud_partition: Option<String>,
    pub provider: Option<String>,
    pub region: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    #[serde(rename = "type")]
    pub account_type: Option<String>,
    pub type_id: Option<i32>,
    pub uid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Evidence {
    pub connection_info: Option<ConnectionInfo>,
    pub data: Option<EvidenceData>,
    pub query: Option<Query>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub direction: Option<String>,
    pub direction_id: Option<i32>,
    pub protocol_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvidenceData {
    pub blocked: Option<bool>,
    pub domain: Option<String>,
    pub domain_with_suffix: Option<String>,
    pub protocol: Option<String>,
    pub vpc_owner_account_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Query {
    pub hostname: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FindingInfo {
    pub analytic: Option<Analytic>,
    pub created_time: Option<i64>,
    pub created_time_dt: Option<String>,
    pub desc: Option<String>,
    pub first_seen_time: Option<i64>,
    pub first_seen_time_dt: Option<String>,
    pub last_seen_time: Option<i64>,
    pub last_seen_time_dt: Option<String>,
    pub modified_time: Option<i64>,
    pub modified_time_dt: Option<String>,
    pub product: Option<Product>,
    pub title: Option<String>,
    pub types: Option<Vec<String>>,
    pub uid: Option<String>,
    pub uid_alt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Analytic {
    #[serde(rename = "type")]
    pub analytic_type: Option<String>,
    pub type_id: Option<i32>,
    pub uid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    pub uid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    pub extensions: Option<Vec<Extension>>,
    pub product: Option<MetadataProduct>,
    pub profiles: Option<Vec<String>>,
    pub uid: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Extension {
    pub name: Option<String>,
    pub uid: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataProduct {
    pub feature: Option<Feature>,
    pub name: Option<String>,
    pub uid: Option<String>,
    pub vendor_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Feature {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Osint {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub osint_type: Option<String>,
    pub type_id: Option<i32>,
    pub value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Remediation {
    pub desc: Option<String>,
    pub references: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Resource {
    pub cloud_partition: Option<String>,
    pub device: Option<Device>,
    pub owner: Option<Owner>,
    pub region: Option<String>,
    pub tags: Option<Vec<Tag>>,
    #[serde(rename = "type")]
    pub resource_type: Option<String>,
    pub uid: Option<String>,
    pub zone: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub image: Option<Image>,
    pub instance_profile: Option<InstanceProfile>,
    pub launch_time: Option<i64>,
    pub launch_time_dt: Option<String>,
    pub model: Option<String>,
    pub network_interfaces: Option<Vec<NetworkInterface>>,
    #[serde(rename = "type")]
    pub device_type: Option<String>,
    pub type_id: Option<i32>,
    pub uid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Image {
    pub uid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceProfile {
    pub uid: Option<String>,
    pub uid_alt: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub ip: Option<String>,
    pub security_groups: Option<Vec<SecurityGroup>>,
    pub uid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityGroup {
    pub name: Option<String>,
    pub uid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Owner {
    pub account: Option<Account>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tag {
    pub name: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VendorAttributes {
    pub severity: Option<String>,
    pub severity_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FindingSummary {
    pub title: String,
    pub region: String,
    pub account: String,
    pub product_name: String,
    pub product_aws: String,
    pub resource_id: String,
    pub severity: String,
    pub web_rule: String,
    pub button_text: String,
    pub description: String,
    pub remediation: String,
}

impl FindingSummary {
    pub fn from_finding(finding: &Finding) -> Self {
        // Extract title
        let title = finding.finding_info.as_ref()
            .and_then(|fi| fi.title.as_deref())
            .unwrap_or("No title")
            .to_string();

        // Extract region
        let region = finding.cloud.as_ref()
            .and_then(|c| c.region.as_deref())
            .unwrap_or("unknown-region")
            .to_string();

        // Extract account
        let account = finding.cloud.as_ref()
            .and_then(|c| c.account.as_ref())
            .and_then(|a| a.uid.as_deref())
            .unwrap_or("unknown-account")
            .to_string();

        // Extract product name
        let product_name = finding.metadata.as_ref()
            .and_then(|m| m.product.as_ref())
            .and_then(|p| p.name.as_deref())
            .unwrap_or("Unknown Product")
            .to_string();

        // Extract product ARN and get the last part (product_aws)
        let product_arn = finding.metadata.as_ref()
            .and_then(|m| m.product.as_ref())
            .and_then(|p| p.uid.as_deref())
            .unwrap_or("");
        
        let product_aws = product_arn.split('/').last().unwrap_or("unknown").to_string();

        // Extract resource_id from first resource
        let resource_id = finding.resources.as_ref()
            .and_then(|resources| resources.first())
            .and_then(|r| r.uid.as_deref())
            .unwrap_or("unknown-resource")
            .to_string();

        let description = finding.finding_info.as_ref()
            .and_then(|fi| fi.desc.as_deref())
            .unwrap_or("")
            .to_string();

        // Get the first remediation references if is available
        let remediation = finding.remediation.as_ref()
            .and_then(|r| r.references.as_ref())
            .and_then(|refs| refs.first())
            .unwrap_or(&"no_remediation".to_string())
            .to_string();   

        // Extract severity
        let severity = finding.severity.as_deref().unwrap_or("Unknown").to_string();

        // Build web rule URL
        let web_rule = format!("https://{}.console.aws.amazon.com/{}/", region, product_aws);
        
        // Button text is the product name
        let button_text = product_name.clone();

        Self {
            title,
            region,
            account,
            product_name,
            product_aws,
            resource_id,
            severity,
            web_rule,
            button_text,
            description,
            remediation,
        }
    }
}

// pub async fn process_finding(finding: &Finding) -> Result<(), Error> {
//     let summary = FindingSummary::from_finding(finding);
    
//     tracing::info!("Processing finding: {}", summary.title);
//     tracing::info!("  Severity: {}", summary.severity);
//     tracing::info!("  Region: {}", summary.region);
//     tracing::info!("  Account: {}", summary.account);
//     tracing::info!("  Product Name: {}", summary.product_name);
//     tracing::info!("  Product AWS: {}", summary.product_aws);
//     tracing::info!("  Resource ID: {}", summary.resource_id);
//     tracing::info!("  Web Rule: {}", summary.web_rule);
//     tracing::info!("  Button Text: {}", summary.button_text);
//     tracing::info!("  Description: {}", summary.description);
//     tracing::info!("  Remediation: {}", summary.remediation);
//     // Check if this is a high severity finding
//     if summary.severity == "High" || summary.severity == "Critical" {
//         handle_high_severity_finding(finding).await?;
//     }

//     // Extract and log affected resources
//     if let Some(resources) = &finding.resources {
//         for resource in resources {
//             log_resource_details(resource);
//         }
//     }

//     // Log evidence
//     if let Some(evidences) = &finding.evidences {
//         for evidence in evidences {
//             log_evidence_details(evidence);
//         }
//     }

//     Ok(())
// }
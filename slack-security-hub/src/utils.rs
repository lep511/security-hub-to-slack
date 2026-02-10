use lambda_runtime::{tracing, Error};
use crate::struct_event::{Detail, Finding, Resource, Evidence};

pub async fn process_security_hub_event(detail: &Detail) -> Result<(), Error> {
    let findings = detail.findings.as_ref()
        .ok_or("Missing findings in detail")?;

    for finding in findings {
        process_finding(finding).await?;
    }

    Ok(())
}

pub async fn process_finding(finding: &Finding) -> Result<(), Error> {
    // Extract key information
    let severity = finding.severity.as_deref().unwrap_or("Unknown");
    let status = finding.status.as_deref().unwrap_or("Unknown");
    
    let title = finding.finding_info.as_ref()
        .and_then(|fi| fi.title.as_deref())
        .unwrap_or("No title");

    tracing::info!("Processing finding: {} (Severity: {}, Status: {})", title, severity, status);

    // Check if this is a high severity finding
    if severity == "High" || severity == "Critical" {
        handle_high_severity_finding(finding).await?;
    }

    // Extract and log affected resources
    if let Some(resources) = &finding.resources {
        for resource in resources {
            log_resource_details(resource);
        }
    }

    // Log evidence
    if let Some(evidences) = &finding.evidences {
        for evidence in evidences {
            log_evidence_details(evidence);
        }
    }

    Ok(())
}

pub async fn handle_high_severity_finding(finding: &Finding) -> Result<(), Error> {
    tracing::warn!("High severity finding detected!");
    
    // TODO: Implement your high-severity handling logic:
    // - Send urgent notifications
    // - Trigger automated response
    // - Create high-priority tickets
    // - Alert security team
    
    Ok(())
}

pub fn log_resource_details(resource: &Resource) {
    if let Some(resource_type) = &resource.resource_type {
        tracing::info!("  Resource Type: {}", resource_type);
    }

    if let Some(uid) = &resource.uid {
        tracing::info!("  Resource ID: {}", uid);
    }

    if let Some(device) = &resource.device {
        if let Some(instance_id) = &device.uid {
            tracing::info!("  Instance: {}", instance_id);
        }
    }
}

pub fn log_evidence_details(evidence: &Evidence) {
    if let Some(data) = &evidence.data {
        if let Some(domain) = &data.domain {
            tracing::warn!("  Suspicious domain: {}", domain);
        }
        
        if let Some(protocol) = &data.protocol {
            tracing::info!("  Protocol: {}", protocol);
        }
    }
}
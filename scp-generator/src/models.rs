use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScpPolicy {
    #[serde(rename = "Version")]
    pub version: String,
    
    #[serde(rename = "Statement")]
    pub statement: Vec<Statement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    #[serde(rename = "Effect")]
    pub effect: String,
    
    #[serde(rename = "Action", skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionValue>,
    
    #[serde(rename = "NotAction", skip_serializing_if = "Option::is_none")]
    pub not_action: Option<ActionValue>,
    
    #[serde(rename = "Resource")]
    pub resource: ResourceValue,
    
    #[serde(rename = "Condition", skip_serializing_if = "Option::is_none")]
    pub condition: Option<HashMap<String, serde_json::Value>>,
    
    #[serde(rename = "Sid", skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ActionValue {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceValue {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct ScpTemplate {
    pub name: String,
    pub description: String,
    pub category: String,
    pub policy: ScpPolicy,
    pub file_path: String,
}

impl ScpTemplate {
    pub fn to_json_string(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(&self.policy)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_scp() {
        let json = r#"{
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Deny",
                    "Action": "s3:*",
                    "Resource": "*"
                }
            ]
        }"#;

        let policy: ScpPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(policy.version, "2012-10-17");
        assert_eq!(policy.statement.len(), 1);
        assert_eq!(policy.statement[0].effect, "Deny");
    }

    #[test]
    fn test_parse_action_array() {
        let json = r#"{
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Deny",
                    "Action": ["s3:*", "ec2:*"],
                    "Resource": "*"
                }
            ]
        }"#;

        let policy: ScpPolicy = serde_json::from_str(json).unwrap();
        
        if let Some(ActionValue::Multiple(actions)) = &policy.statement[0].action {
            assert_eq!(actions.len(), 2);
            assert!(actions.contains(&"s3:*".to_string()));
        } else {
            panic!("Action should be Multiple");
        }
    }

    #[test]
    fn test_template_to_json_string() {
        let policy = ScpPolicy {
            version: "2012-10-17".to_string(),
            statement: vec![Statement {
                effect: "Deny".to_string(),
                action: Some(ActionValue::Single("s3:*".to_string())),
                not_action: None,
                resource: ResourceValue::Single("*".to_string()),
                condition: None,
                sid: Some("TestSid".to_string()),
            }],
        };

        let template = ScpTemplate {
            name: "Test".to_string(),
            description: "Test template".to_string(),
            category: "test".to_string(),
            policy,
            file_path: "/tmp/test.json".to_string(),
        };

        let json_string = template.to_json_string().unwrap();
        assert!(json_string.contains("2012-10-17"));
        assert!(json_string.contains("Deny"));
    }
}
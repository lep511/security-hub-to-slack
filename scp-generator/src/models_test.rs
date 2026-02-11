#[cfg(test)]
mod tests {
    use crate::models::*;
    use serde_json;

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
    fn test_parse_with_condition() {
        let json = r#"{
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Effect": "Deny",
                    "Action": "*",
                    "Resource": "*",
                    "Condition": {
                        "StringNotEquals": {
                            "aws:RequestedRegion": ["us-east-1"]
                        }
                    }
                }
            ]
        }"#;

        let policy: ScpPolicy = serde_json::from_str(json).unwrap();
        assert!(policy.statement[0].condition.is_some());
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

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let original = ScpPolicy {
            version: "2012-10-17".to_string(),
            statement: vec![Statement {
                effect: "Allow".to_string(),
                action: Some(ActionValue::Multiple(vec![
                    "s3:GetObject".to_string(),
                    "s3:PutObject".to_string(),
                ])),
                not_action: None,
                resource: ResourceValue::Multiple(vec![
                    "arn:aws:s3:::bucket/*".to_string(),
                ]),
                condition: None,
                sid: None,
            }],
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ScpPolicy = serde_json::from_str(&json).unwrap();

        assert_eq!(original.version, deserialized.version);
        assert_eq!(original.statement.len(), deserialized.statement.len());
    }
}
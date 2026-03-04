//! Tests for FJ-1250: Saved plan files (plan --out + apply --plan-file).

use super::plan::*;
use crate::core::types::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_config() -> ForjarConfig {
        let config_yaml = r#"
version: "1.0"
name: test-plan-file
machines:
  m1:
    hostname: localhost
    addr: 127.0.0.1
resources:
  web-pkg:
    type: package
    machine: m1
    packages: [nginx]
  web-config:
    type: file
    machine: m1
    path: /etc/nginx/nginx.conf
    content: "server {}"
"#;
        crate::core::parser::parse_config(config_yaml).unwrap()
    }

    fn make_test_plan() -> ExecutionPlan {
        ExecutionPlan {
            name: "test-plan-file".to_string(),
            changes: vec![
                PlannedChange {
                    resource_id: "web-pkg".to_string(),
                    machine: "m1".to_string(),
                    resource_type: ResourceType::Package,
                    action: PlanAction::Create,
                    description: "web-pkg: install nginx".to_string(),
                },
                PlannedChange {
                    resource_id: "web-config".to_string(),
                    machine: "m1".to_string(),
                    resource_type: ResourceType::File,
                    action: PlanAction::Update,
                    description: "web-config: update (state changed)".to_string(),
                },
            ],
            execution_order: vec!["web-pkg".to_string(), "web-config".to_string()],
            to_create: 1,
            to_update: 1,
            to_destroy: 0,
            unchanged: 0,
        }
    }

    #[test]
    fn test_save_and_load_plan_file_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config_path = Path::new("forjar.yaml");
        let config = make_test_config();
        let plan = make_test_plan();

        // Save
        let save_result =
            super::super::plan::save_plan_file(&plan, &config, config_path, &plan_path);
        assert!(
            save_result.is_ok(),
            "save should succeed: {save_result:?}"
        );

        // Verify file exists and is valid JSON
        let content = std::fs::read_to_string(&plan_path).unwrap();
        let doc: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(doc["format"], "forjar-plan-v1");
        assert_eq!(doc["name"], "test-plan-file");
        assert_eq!(doc["to_create"], 1);
        assert_eq!(doc["to_update"], 1);
        assert_eq!(doc["changes"].as_array().unwrap().len(), 2);

        // Load
        let loaded = load_plan_file(&plan_path, &config).unwrap();
        assert_eq!(loaded.name, "test-plan-file");
        assert_eq!(loaded.to_create, 1);
        assert_eq!(loaded.to_update, 1);
        assert_eq!(loaded.to_destroy, 0);
        assert_eq!(loaded.changes.len(), 2);
        assert_eq!(loaded.changes[0].action, PlanAction::Create);
        assert_eq!(loaded.changes[1].action, PlanAction::Update);
        assert_eq!(loaded.changes[0].resource_type, ResourceType::Package);
        assert_eq!(loaded.changes[1].resource_type, ResourceType::File);
        assert_eq!(loaded.execution_order, vec!["web-pkg", "web-config"]);
    }

    #[test]
    fn test_load_plan_file_rejects_changed_config() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config_path = Path::new("forjar.yaml");
        let config = make_test_config();
        let plan = make_test_plan();

        // Save with original config
        super::super::plan::save_plan_file(&plan, &config, config_path, &plan_path).unwrap();

        // Modify config
        let mut modified_config = config;
        modified_config.name = "changed-name".to_string();

        // Load should fail with hash mismatch
        let result = load_plan_file(&plan_path, &modified_config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("config has changed"));
    }

    #[test]
    fn test_load_plan_file_rejects_invalid_format() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config = make_test_config();

        std::fs::write(&plan_path, r#"{"format": "unknown-v99"}"#).unwrap();
        let result = load_plan_file(&plan_path, &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported plan format"));
    }

    #[test]
    fn test_load_plan_file_rejects_missing_file() {
        let config = make_test_config();
        let result = load_plan_file(Path::new("/nonexistent/plan.json"), &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read plan file"));
    }

    #[test]
    fn test_load_plan_file_handles_all_action_types() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config = make_test_config();

        let plan_json = serde_json::json!({
            "format": "forjar-plan-v1",
            "config_hash": compute_config_hash(&config),
            "name": "test",
            "to_create": 1, "to_update": 1, "to_destroy": 1, "unchanged": 1,
            "execution_order": ["a", "b", "c", "d"],
            "changes": [
                {"resource_id": "a", "machine": "m1", "resource_type": "package", "action": "create", "description": "a: create"},
                {"resource_id": "b", "machine": "m1", "resource_type": "service", "action": "update", "description": "b: update"},
                {"resource_id": "c", "machine": "m1", "resource_type": "file", "action": "destroy", "description": "c: destroy"},
                {"resource_id": "d", "machine": "m1", "resource_type": "mount", "action": "no_op", "description": "d: no-op"},
            ],
        });
        std::fs::write(
            &plan_path,
            serde_json::to_string_pretty(&plan_json).unwrap(),
        )
        .unwrap();

        let loaded = load_plan_file(&plan_path, &config).unwrap();
        assert_eq!(loaded.changes[0].action, PlanAction::Create);
        assert_eq!(loaded.changes[1].action, PlanAction::Update);
        assert_eq!(loaded.changes[2].action, PlanAction::Destroy);
        assert_eq!(loaded.changes[3].action, PlanAction::NoOp);
        assert_eq!(loaded.changes[1].resource_type, ResourceType::Service);
        assert_eq!(loaded.changes[3].resource_type, ResourceType::Mount);
    }

    #[test]
    fn test_load_plan_file_handles_all_resource_types() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config = make_test_config();

        let types_to_test = [
            ("package", ResourceType::Package),
            ("file", ResourceType::File),
            ("service", ResourceType::Service),
            ("mount", ResourceType::Mount),
            ("user", ResourceType::User),
            ("docker", ResourceType::Docker),
            ("network", ResourceType::Network),
            ("cron", ResourceType::Cron),
            ("model", ResourceType::Model),
            ("gpu", ResourceType::Gpu),
        ];

        for (type_str, expected_type) in &types_to_test {
            let plan_json = serde_json::json!({
                "format": "forjar-plan-v1",
                "config_hash": compute_config_hash(&config),
                "name": "test",
                "to_create": 1, "to_update": 0, "to_destroy": 0, "unchanged": 0,
                "execution_order": ["r"],
                "changes": [
                    {"resource_id": "r", "machine": "m1", "resource_type": type_str, "action": "create", "description": "r: create"},
                ],
            });
            std::fs::write(
                &plan_path,
                serde_json::to_string_pretty(&plan_json).unwrap(),
            )
            .unwrap();
            let loaded = load_plan_file(&plan_path, &config).unwrap();
            assert_eq!(
                loaded.changes[0].resource_type, *expected_type,
                "type mismatch for {type_str}"
            );
        }
    }

    /// Helper to compute config hash for test plan files.
    fn compute_config_hash(config: &ForjarConfig) -> String {
        let yaml = serde_yaml_ng::to_string(config).unwrap();
        crate::tripwire::hasher::hash_string(&yaml)
    }
}

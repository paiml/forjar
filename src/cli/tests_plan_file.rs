//! Tests for FJ-1250 saved plan files, and for the seal that Refs #356/#358
//! added to them.

use super::plan_file::*;
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

    /// A saved plan plus the state dir it was sealed against.
    fn saved() -> (tempfile::TempDir, std::path::PathBuf, ForjarConfig) {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config = make_test_config();
        save_plan_file(
            &make_test_plan(),
            &config,
            Path::new("forjar.yaml"),
            dir.path(),
            &plan_path,
        )
        .unwrap();
        (dir, plan_path, config)
    }

    fn read_doc(path: &Path) -> serde_json::Value {
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn write_doc(path: &Path, doc: &serde_json::Value) {
        std::fs::write(path, serde_json::to_string_pretty(doc).unwrap()).unwrap();
    }

    #[test]
    fn test_save_and_load_plan_file_roundtrip() {
        let (dir, plan_path, config) = saved();

        let doc = read_doc(&plan_path);
        assert_eq!(doc["format"], FORMAT_V2);
        assert_eq!(doc["name"], "test-plan-file");
        assert_eq!(doc["to_create"], 1);
        assert_eq!(doc["to_update"], 1);
        assert_eq!(doc["changes"].as_array().unwrap().len(), 2);
        assert_eq!(doc["seal"]["version"], "forjar-plan-seal-v1");
        assert_eq!(doc["seal"]["config_hash"], doc["config_hash"]);
        assert_eq!(
            doc["seal"]["ttl_secs"], 0,
            "a plan file carries no wall-clock expiry"
        );
        for leg in ["config_hash", "state_hash", "diff_hash", "seal", "plan_id"] {
            assert!(
                doc["seal"][leg].as_str().is_some_and(|s| !s.is_empty()),
                "seal.{leg} must be populated"
            );
        }

        let loaded = load_plan_file(&plan_path, &config, dir.path()).unwrap();
        assert!(loaded.sealed);
        let plan = loaded.plan;
        assert_eq!(plan.name, "test-plan-file");
        assert_eq!(plan.to_create, 1);
        assert_eq!(plan.to_update, 1);
        assert_eq!(plan.to_destroy, 0);
        assert_eq!(plan.changes.len(), 2);
        assert_eq!(plan.changes[0].action, PlanAction::Create);
        assert_eq!(plan.changes[1].action, PlanAction::Update);
        assert_eq!(plan.changes[0].resource_type, ResourceType::Package);
        assert_eq!(plan.changes[1].resource_type, ResourceType::File);
        assert_eq!(plan.execution_order, vec!["web-pkg", "web-config"]);
    }

    #[test]
    fn test_load_plan_file_rejects_changed_config() {
        let (dir, plan_path, config) = saved();

        let mut modified_config = config;
        modified_config.name = "changed-name".to_string();

        let err = load_plan_file(&plan_path, &modified_config, dir.path()).unwrap_err();
        assert!(err.starts_with("PLAN_HASH_MISMATCH:"), "{err}");
        assert!(err.contains("config leg"), "{err}");
    }

    #[test]
    fn test_load_plan_file_rejects_edited_counters() {
        let (dir, plan_path, config) = saved();

        // The #358 defect, exactly: zero the counters and leave config_hash
        // byte-identical. Before the seal this made a requested apply print
        // "Plan has no changes to apply." and exit 0.
        let mut doc = read_doc(&plan_path);
        doc["to_create"] = serde_json::json!(0);
        doc["to_update"] = serde_json::json!(0);
        doc["to_destroy"] = serde_json::json!(0);
        write_doc(&plan_path, &doc);

        let err = load_plan_file(&plan_path, &config, dir.path()).unwrap_err();
        assert!(
            err.starts_with("PLAN_MALFORMED:") || err.starts_with("PLAN_HASH_MISMATCH:"),
            "{err}"
        );
    }

    #[test]
    fn test_load_plan_file_rejects_edited_change_list() {
        let (dir, plan_path, config) = saved();

        let mut doc = read_doc(&plan_path);
        doc["changes"][0]["resource_id"] = serde_json::json!("somebody-elses-resource");
        write_doc(&plan_path, &doc);

        let err = load_plan_file(&plan_path, &config, dir.path()).unwrap_err();
        assert!(err.starts_with("PLAN_HASH_MISMATCH:"), "{err}");
        assert!(err.contains("diff leg"), "{err}");
    }

    #[test]
    fn test_load_plan_file_rejects_a_lock_written_after_sealing() {
        let (dir, plan_path, config) = saved();

        let lock = crate::core::state::lock_file_path(dir.path(), "m1");
        std::fs::create_dir_all(lock.parent().unwrap()).unwrap();
        std::fs::write(&lock, "machine: m1\nresources: {}\n").unwrap();

        let err = load_plan_file(&plan_path, &config, dir.path()).unwrap_err();
        assert!(err.starts_with("PLAN_HASH_MISMATCH:"), "{err}");
        assert!(err.contains("state leg"), "{err}");
    }

    #[test]
    fn test_load_plan_file_rejects_moved_expiry() {
        let (dir, plan_path, config) = saved();

        let mut doc = read_doc(&plan_path);
        doc["seal"]["ttl_secs"] = serde_json::json!(86400);
        write_doc(&plan_path, &doc);

        let err = load_plan_file(&plan_path, &config, dir.path()).unwrap_err();
        assert!(err.starts_with("PLAN_HASH_MISMATCH:"), "{err}");
        assert!(err.contains("seal leg"), "{err}");
    }

    #[test]
    fn test_load_plan_file_rejects_seal_disagreeing_with_config_hash() {
        let (dir, plan_path, config) = saved();

        let mut doc = read_doc(&plan_path);
        doc["config_hash"] = serde_json::json!("blake3:deadbeef");
        write_doc(&plan_path, &doc);

        let err = load_plan_file(&plan_path, &config, dir.path()).unwrap_err();
        assert!(err.contains("disagrees with its own seal"), "{err}");
    }

    #[test]
    fn test_load_plan_file_rejects_v2_without_a_seal() {
        let (dir, plan_path, config) = saved();

        let mut doc = read_doc(&plan_path);
        doc.as_object_mut().unwrap().remove("seal");
        write_doc(&plan_path, &doc);

        let err = load_plan_file(&plan_path, &config, dir.path()).unwrap_err();
        assert!(err.contains("has no 'seal'"), "{err}");
    }

    #[test]
    fn test_load_plan_file_rejects_unreadable_seal() {
        let (dir, plan_path, config) = saved();

        let mut doc = read_doc(&plan_path);
        doc["seal"] = serde_json::json!({"version": "forjar-plan-seal-v1"});
        write_doc(&plan_path, &doc);

        let err = load_plan_file(&plan_path, &config, dir.path()).unwrap_err();
        assert!(err.contains("unreadable plan seal"), "{err}");
    }

    #[test]
    fn test_load_plan_file_rejects_invalid_format() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config = make_test_config();

        std::fs::write(&plan_path, r#"{"format": "unknown-v99"}"#).unwrap();
        let result = load_plan_file(&plan_path, &config, dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported plan format"));
    }

    #[test]
    fn test_load_plan_file_rejects_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = make_test_config();
        let result = load_plan_file(Path::new("/nonexistent/plan.json"), &config, dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("read plan file"));
    }

    /// Backward compatibility is asserted, not assumed: a hand-written v1
    /// document from an older binary still loads, and reports itself unsealed.
    #[test]
    fn test_load_plan_file_handles_all_action_types() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config = make_test_config();

        let plan_json = serde_json::json!({
            "format": FORMAT_V1,
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
        write_doc(&plan_path, &plan_json);

        let loaded = load_plan_file(&plan_path, &config, dir.path()).unwrap();
        assert!(!loaded.sealed, "a v1 document is not a sealed plan");
        let plan = loaded.plan;
        assert_eq!(plan.changes[0].action, PlanAction::Create);
        assert_eq!(plan.changes[1].action, PlanAction::Update);
        assert_eq!(plan.changes[2].action, PlanAction::Destroy);
        assert_eq!(plan.changes[3].action, PlanAction::NoOp);
        assert_eq!(plan.changes[1].resource_type, ResourceType::Service);
        assert_eq!(plan.changes[3].resource_type, ResourceType::Mount);
    }

    #[test]
    fn test_v1_still_reports_the_original_config_message() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config = make_test_config();
        let plan_json = serde_json::json!({
            "format": FORMAT_V1,
            "config_hash": "blake3:not-this-config",
            "name": "test",
            "to_create": 0, "to_update": 0, "to_destroy": 0, "unchanged": 0,
            "execution_order": [],
            "changes": [],
        });
        write_doc(&plan_path, &plan_json);
        let err = load_plan_file(&plan_path, &config, dir.path()).unwrap_err();
        assert!(err.contains("config has changed"), "{err}");
    }

    /// A v1 body whose counters contradict its own change list is refused even
    /// though there is no seal to check it against: the planner guarantees the
    /// counters partition the changes, so a document where they do not was
    /// edited.
    #[test]
    fn test_v1_counters_must_partition_the_change_list() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let config = make_test_config();
        let plan_json = serde_json::json!({
            "format": FORMAT_V1,
            "config_hash": compute_config_hash(&config),
            "name": "test",
            "to_create": 0, "to_update": 0, "to_destroy": 0, "unchanged": 0,
            "execution_order": ["a"],
            "changes": [
                {"resource_id": "a", "machine": "m1", "resource_type": "file", "action": "create", "description": "a: create"},
            ],
        });
        write_doc(&plan_path, &plan_json);
        let err = load_plan_file(&plan_path, &config, dir.path()).unwrap_err();
        assert!(err.starts_with("PLAN_MALFORMED:"), "{err}");
        assert!(err.contains("to_create"), "{err}");
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
                "format": FORMAT_V1,
                "config_hash": compute_config_hash(&config),
                "name": "test",
                "to_create": 1, "to_update": 0, "to_destroy": 0, "unchanged": 0,
                "execution_order": ["r"],
                "changes": [
                    {"resource_id": "r", "machine": "m1", "resource_type": type_str, "action": "create", "description": "r: create"},
                ],
            });
            write_doc(&plan_path, &plan_json);
            let loaded = load_plan_file(&plan_path, &config, dir.path()).unwrap();
            assert_eq!(
                loaded.plan.changes[0].resource_type, *expected_type,
                "type mismatch for {type_str}"
            );
        }
    }

    /// Helper to compute config hash for test plan files.
    /// GH-212: the ONE canonical hash. This helper used to re-implement the
    /// production expression (`serde_yaml_ng::to_string` + blake3), which is a
    /// second copy of exactly the thing that was nondeterministic — so the
    /// suite could not have caught the plan-file roundtrip failing in the wild.
    fn compute_config_hash(config: &ForjarConfig) -> String {
        crate::core::config_hash::config_hash(config).expect("hashable")
    }
}

//! Coverage tests for args structs and uncovered functions — part 4 (functional tests).

use super::apply_variants::*;
use super::commands::*;
use super::destroy::*;
use super::lint::*;
use super::observe::*;
use super::plan::*;
use super::validate_resources::*;
use crate::core::types;
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // 5. cmd_apply_dry_run_graph — tests via temp config files
    // ========================================================================

    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    fn minimal_config_yaml() -> &'static str {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n"
    }

    fn config_with_deps_yaml() -> &'static str {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on:\n      - a\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on:\n      - a\n      - b\n"
    }

    #[test]
    fn test_cov_cmd_apply_dry_run_graph_minimal() {
        let f = write_temp_config(minimal_config_yaml());
        let result = cmd_apply_dry_run_graph(f.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_cmd_apply_dry_run_graph_with_deps() {
        let f = write_temp_config(config_with_deps_yaml());
        let result = cmd_apply_dry_run_graph(f.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_cmd_apply_dry_run_graph_invalid_file() {
        let result = cmd_apply_dry_run_graph(std::path::Path::new("/nonexistent/file.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_cmd_apply_dry_run_graph_empty_resources() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n";
        let f = write_temp_config(yaml);
        let result = cmd_apply_dry_run_graph(f.path());
        assert!(result.is_ok());
    }

    // ========================================================================
    // 6. compute_rollback_changes — tests using YAML deserialization
    // ========================================================================

    fn parse_config(yaml: &str) -> types::ForjarConfig {
        serde_yaml_ng::from_str(yaml).unwrap()
    }

    #[test]
    fn test_cov_compute_rollback_no_changes() {
        let yaml = minimal_config_yaml();
        let previous = parse_config(yaml);
        let current = parse_config(yaml);
        let changes = compute_rollback_changes(&previous, &current, 1);
        assert!(changes.is_empty());
    }

    #[test]
    fn test_cov_compute_rollback_modified_resource() {
        let prev_yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n";
        let cur_yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: world\n";
        let previous = parse_config(prev_yaml);
        let current = parse_config(cur_yaml);
        let changes = compute_rollback_changes(&previous, &current, 1);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].contains("modified"));
    }

    #[test]
    fn test_cov_compute_rollback_added_resource() {
        let prev_yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n";
        let cur_yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n";
        let previous = parse_config(prev_yaml);
        let current = parse_config(cur_yaml);
        let changes = compute_rollback_changes(&previous, &current, 2);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].contains("re-added"));
        assert!(changes[0].contains("HEAD~2"));
    }

    #[test]
    fn test_cov_compute_rollback_removed_resource() {
        let prev_yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n";
        let cur_yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  new-res:\n    type: file\n    machine: m\n    path: /tmp/new\n    content: new\n";
        let previous = parse_config(prev_yaml);
        let current = parse_config(cur_yaml);
        let changes = compute_rollback_changes(&previous, &current, 1);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].contains("will remain"));
    }

    // ========================================================================
    // 7. lint_auto_fix — tests
    // ========================================================================

    #[test]
    fn test_cov_lint_auto_fix_sorts_resources() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  z-res:\n    type: file\n    machine: m\n    path: /tmp/z\n    content: z\n  a-res:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n";
        let f = write_temp_config(yaml);
        let fixes = lint_auto_fix(f.path()).unwrap();
        assert!(!fixes.is_empty());
        assert!(fixes[0].contains("sorted"));
    }

    #[test]
    fn test_cov_lint_auto_fix_already_sorted() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a-res:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n";
        let f = write_temp_config(yaml);
        let fixes = lint_auto_fix(f.path()).unwrap();
        // Even a single resource gets the sort applied
        assert!(fixes.len() <= 1);
    }

    #[test]
    fn test_cov_lint_auto_fix_invalid_file() {
        let result = lint_auto_fix(std::path::Path::new("/nonexistent/forjar.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_lint_auto_fix_invalid_yaml() {
        let f = write_temp_config("not: [valid: yaml: {{}}");
        let result = lint_auto_fix(f.path());
        assert!(result.is_err());
    }

    // ========================================================================
    // 8. print_plan_cost — tests
    // ========================================================================

    fn make_plan_change(
        resource_type: types::ResourceType,
        action: types::PlanAction,
    ) -> types::PlannedChange {
        types::PlannedChange {
            resource_id: "test".to_string(),
            machine: "m".to_string(),
            resource_type,
            action,
            description: "test change".to_string(),
        }
    }

    #[test]
    fn test_cov_print_plan_cost_empty() {
        let plan = types::ExecutionPlan {
            name: "test".to_string(),
            changes: vec![],
            execution_order: vec![],
            to_create: 0,
            to_update: 0,
            to_destroy: 0,
            unchanged: 0,
        };
        print_plan_cost(&plan);
    }

    #[test]
    fn test_cov_print_plan_cost_mixed_types() {
        let plan = types::ExecutionPlan {
            name: "test".to_string(),
            changes: vec![
                make_plan_change(types::ResourceType::Package, types::PlanAction::Create),
                make_plan_change(types::ResourceType::Service, types::PlanAction::Update),
                make_plan_change(types::ResourceType::File, types::PlanAction::NoOp),
            ],
            execution_order: vec!["a".to_string()],
            to_create: 1,
            to_update: 1,
            to_destroy: 0,
            unchanged: 1,
        };
        print_plan_cost(&plan);
    }

    #[test]
    fn test_cov_print_plan_cost_high_destroy_cost() {
        let plan = types::ExecutionPlan {
            name: "test".to_string(),
            changes: vec![
                make_plan_change(types::ResourceType::Docker, types::PlanAction::Destroy),
                make_plan_change(types::ResourceType::Mount, types::PlanAction::Destroy),
                make_plan_change(types::ResourceType::Model, types::PlanAction::Destroy),
            ],
            execution_order: vec!["a".to_string()],
            to_create: 0,
            to_update: 0,
            to_destroy: 3,
            unchanged: 0,
        };
        // destroy_cost = (5*2)+(4*2)+(5*2) = 28, triggers high warning
        print_plan_cost(&plan);
    }

    #[test]
    fn test_cov_print_plan_cost_all_resource_types() {
        let plan = types::ExecutionPlan {
            name: "test".to_string(),
            changes: vec![
                make_plan_change(types::ResourceType::Package, types::PlanAction::Create),
                make_plan_change(types::ResourceType::Service, types::PlanAction::Create),
                make_plan_change(types::ResourceType::Mount, types::PlanAction::Create),
                make_plan_change(types::ResourceType::Docker, types::PlanAction::Create),
                make_plan_change(types::ResourceType::User, types::PlanAction::Create),
                make_plan_change(types::ResourceType::Network, types::PlanAction::Create),
                make_plan_change(types::ResourceType::Gpu, types::PlanAction::Create),
                make_plan_change(types::ResourceType::Model, types::PlanAction::Create),
                make_plan_change(types::ResourceType::Cron, types::PlanAction::Create),
                make_plan_change(types::ResourceType::File, types::PlanAction::Create),
                make_plan_change(types::ResourceType::Recipe, types::PlanAction::Create),
                make_plan_change(types::ResourceType::Pepita, types::PlanAction::Create),
            ],
            execution_order: vec!["a".to_string()],
            to_create: 12,
            to_update: 0,
            to_destroy: 0,
            unchanged: 0,
        };
        print_plan_cost(&plan);
    }

    // ========================================================================
    // 9. print_resource_limits_text — tests
    // ========================================================================

    #[test]
    fn test_cov_print_resource_limits_text_no_violations() {
        let mut counts = std::collections::HashMap::new();
        counts.insert("machine-a".to_string(), 5);
        counts.insert("machine-b".to_string(), 10);
        let violations: Vec<(String, usize)> = vec![];
        print_resource_limits_text(&counts, &violations, 100);
    }

    #[test]
    fn test_cov_print_resource_limits_text_with_violations() {
        let mut counts = std::collections::HashMap::new();
        counts.insert("machine-a".to_string(), 150);
        counts.insert("machine-b".to_string(), 200);
        let violations = vec![
            ("machine-a".to_string(), 150),
            ("machine-b".to_string(), 200),
        ];
        print_resource_limits_text(&counts, &violations, 100);
    }

    #[test]
    fn test_cov_print_resource_limits_text_empty() {
        let counts = std::collections::HashMap::new();
        let violations: Vec<(String, usize)> = vec![];
        print_resource_limits_text(&counts, &violations, 100);
    }

    // ========================================================================
    // 10. handle_watch_change — tests
    // ========================================================================

    #[test]
    fn test_cov_handle_watch_change_valid_config() {
        let f = write_temp_config(minimal_config_yaml());
        let state_dir = tempfile::tempdir().unwrap();
        handle_watch_change(f.path(), state_dir.path(), false);
    }

    #[test]
    fn test_cov_handle_watch_change_invalid_config() {
        let f = write_temp_config("invalid: yaml: garbage: [[");
        let state_dir = tempfile::tempdir().unwrap();
        handle_watch_change(f.path(), state_dir.path(), false);
    }

    #[test]
    fn test_cov_handle_watch_change_with_deps() {
        let f = write_temp_config(config_with_deps_yaml());
        let state_dir = tempfile::tempdir().unwrap();
        handle_watch_change(f.path(), state_dir.path(), false);
    }

    #[test]
    fn test_cov_handle_watch_change_nonexistent_file() {
        let state_dir = tempfile::tempdir().unwrap();
        handle_watch_change(
            std::path::Path::new("/nonexistent/forjar.yaml"),
            state_dir.path(),
            false,
        );
    }

    // ========================================================================
    // Indirect coverage: cmd_lint with fix=true to exercise lint_auto_fix
    // ========================================================================

    #[test]
    fn test_cov_cmd_lint_with_fix_flag() {
        let yaml = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  z-res:\n    type: file\n    machine: m\n    path: /tmp/z\n    content: z\n  a-res:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n";
        let f = write_temp_config(yaml);
        let result = super::super::lint::cmd_lint(f.path(), false, false, true);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Indirect coverage: cmd_plan with cost=true for print_plan_cost
    // ========================================================================

    #[test]
    fn test_cov_cmd_plan_with_cost() {
        let f = write_temp_config(minimal_config_yaml());
        let state_dir = tempfile::tempdir().unwrap();
        let result = super::super::plan::cmd_plan(
            f.path(),
            state_dir.path(),
            None,
            None,
            None,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            true, // cost = true
            &[],
            None, // plan_out
        );
        assert!(result.is_ok());
    }

    // ========================================================================
    // Indirect coverage: cmd_validate_check_resource_limits (text & json)
    // ========================================================================

    #[test]
    fn test_cov_cmd_validate_check_resource_limits_text() {
        let f = write_temp_config(minimal_config_yaml());
        let result = super::super::validate_resources::cmd_validate_check_resource_limits(
            f.path(), false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_cmd_validate_check_resource_limits_json() {
        let f = write_temp_config(minimal_config_yaml());
        let result = super::super::validate_resources::cmd_validate_check_resource_limits(
            f.path(), true,
        );
        assert!(result.is_ok());
    }
}

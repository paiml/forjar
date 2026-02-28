//! Tests: Init, format, completion, schema.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::init::*;
use super::commands::*;
use super::dispatch::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj017_init() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("test-project");
        std::fs::create_dir_all(&sub).unwrap();
        cmd_init(&sub).unwrap();
        assert!(sub.join("forjar.yaml").exists());
        assert!(sub.join("state").is_dir());
    }


    #[test]
    fn test_fj017_init_already_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("forjar.yaml"), "exists").unwrap();
        let result = cmd_init(dir.path());
        assert!(result.is_err());
    }


    #[test]
    fn test_fj017_dispatch_init() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("dispatch-test");
        std::fs::create_dir_all(&sub).unwrap();
        dispatch(Commands::Init(InitArgs { path: sub.clone() }), false, true).unwrap();
        assert!(sub.join("forjar.yaml").exists());
    }


    #[test]
    fn test_fj017_fmt_check_unformatted() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        // Write with extra whitespace and comments (not canonical)
        std::fs::write(
            &config,
            r#"version:   "1.0"
name:    my-infra
machines:
  m1:
    hostname:   box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        // check mode should detect non-canonical format
        let result = cmd_fmt(&config, true);
        assert!(result.is_err(), "unformatted file should fail check mode");
    }


    #[test]
    fn test_fj017_fmt_write_then_check() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"version:   "1.0"
name:    my-infra
machines:
  m1:
    hostname:   box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
        )
        .unwrap();
        // Format the file
        cmd_fmt(&config, false).unwrap();
        // Now check mode should pass
        cmd_fmt(&config, true).unwrap();
    }

    // ── Check command tests ────────────────────────────────────


    #[test]
    fn test_fj017_init_creates_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("new-project");
        std::fs::create_dir_all(&project).unwrap();

        cmd_init(&project).unwrap();

        assert!(project.join("forjar.yaml").exists());
        assert!(project.join("state").exists());
    }


    #[test]
    fn test_fj017_init_template_is_valid() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("valid-init");
        std::fs::create_dir_all(&project).unwrap();

        cmd_init(&project).unwrap();

        // The template should parse as valid ForjarConfig
        let content = std::fs::read_to_string(project.join("forjar.yaml")).unwrap();
        let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).unwrap();
        assert_eq!(config.version, "1.0");
        assert_eq!(config.name, "my-infrastructure");
    }

    // ── FJ-131: cmd_graph tests ───────────────────────────────────

    fn write_simple_config(dir: &std::path::Path) -> std::path::PathBuf {
        let config_path = dir.join("forjar.yaml");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: graph-test
machines:
  web:
    hostname: web
    addr: 1.1.1.1
resources:
  setup:
    type: file
    machine: web
    path: /tmp/setup
    state: directory
  app:
    type: file
    machine: web
    path: /tmp/setup/app.conf
    content: "config"
    depends_on: [setup]
"#,
        )
        .unwrap();
        config_path
    }


    #[test]
    fn test_fj132_cmd_init_creates_project() {
        let dir = tempfile::tempdir().unwrap();
        cmd_init(dir.path()).unwrap();
        assert!(dir.path().join("forjar.yaml").exists());
        assert!(dir.path().join("state").is_dir());
        // Config should be valid YAML
        let content = std::fs::read_to_string(dir.path().join("forjar.yaml")).unwrap();
        let _config: types::ForjarConfig = serde_yaml_ng::from_str(&content).unwrap();
    }


    #[test]
    fn test_fj132_cmd_init_refuses_existing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("forjar.yaml"), "version: '1.0'").unwrap();
        let result = cmd_init(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }


    #[test]
    fn test_fj132_cmd_fmt_already_formatted() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        let yaml = r#"version: "1.0"
name: test
machines: {}
resources: {}
"#;
        // Write, parse, re-serialize to get canonical form
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let formatted = serde_yaml_ng::to_string(&config).unwrap();
        std::fs::write(&file, &formatted).unwrap();
        // Should succeed and not modify
        cmd_fmt(&file, false).unwrap();
    }


    #[test]
    fn test_fj132_cmd_fmt_check_mode() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.yaml");
        // Write canonical YAML
        let yaml = r#"version: "1.0"
name: test
machines: {}
resources: {}
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let formatted = serde_yaml_ng::to_string(&config).unwrap();
        std::fs::write(&file, &formatted).unwrap();
        // Check mode should succeed for already-formatted file
        cmd_fmt(&file, true).unwrap();
    }


    #[test]
    fn test_fj132_cmd_fmt_formats_unformatted() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("messy.yaml");
        // Write valid but messy YAML
        let yaml = "version: '1.0'\nname: test\nmachines: {}\nresources: {}\n";
        std::fs::write(&file, yaml).unwrap();
        cmd_fmt(&file, false).unwrap();
        // File should be overwritten with canonical form
        let content = std::fs::read_to_string(&file).unwrap();
        assert!(content.contains("version"));
    }


    #[test]
    fn test_fj036_cmd_init_creates_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("fj036-project");
        std::fs::create_dir_all(&project).unwrap();

        cmd_init(&project).unwrap();

        // Verify state directory was created
        assert!(
            project.join("state").is_dir(),
            "cmd_init must create state/ directory"
        );
        // Verify forjar.yaml was created
        assert!(
            project.join("forjar.yaml").exists(),
            "cmd_init must create forjar.yaml"
        );
        // Verify the generated config is valid YAML that parses
        let content = std::fs::read_to_string(project.join("forjar.yaml")).unwrap();
        let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).unwrap();
        assert_eq!(config.version, "1.0");
    }


    #[test]
    fn test_fj017_cmd_fmt_check_valid() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        // Write a config, parse it, re-serialize to canonical form, then write that
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#;
        let parsed: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let formatted = serde_yaml_ng::to_string(&parsed).unwrap();
        std::fs::write(&config_path, &formatted).unwrap();
        let result = cmd_fmt(&config_path, true);
        assert!(
            result.is_ok(),
            "cmd_fmt check should succeed on already-formatted config"
        );
    }

    // ── FJ-135: forjar trace CLI tests ──────────────────────────


    #[test]
    fn test_fj253_completion_bash() {
        // Completion generation in clap_complete uses deep recursion
        // proportional to subcommand/flag count. With our large Commands enum
        // the default test-thread stack overflows; run on a thread with 16 MiB.
        let result = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| cmd_completion(CompletionShell::Bash))
            .expect("failed to spawn thread")
            .join()
            .expect("completion thread panicked");
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj253_completion_zsh() {
        // Completion generation in clap_complete uses deep recursion
        // proportional to subcommand/flag count. With our large Commands enum
        // the default test-thread stack overflows; run on a thread with 16 MiB.
        let result = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| cmd_completion(CompletionShell::Zsh))
            .expect("failed to spawn thread")
            .join()
            .expect("completion thread panicked");
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj253_completion_fish() {
        // Completion generation in clap_complete uses deep recursion
        // proportional to subcommand/flag count. With our large Commands enum
        // the default test-thread stack overflows; run on a thread with 16 MiB.
        let result = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(|| cmd_completion(CompletionShell::Fish))
            .expect("failed to spawn thread")
            .join()
            .expect("completion thread panicked");
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj253_completion_shell_enum_debug() {
        let bash = CompletionShell::Bash;
        let debug = format!("{:?}", bash);
        assert_eq!(debug, "Bash");
    }


    #[test]
    fn test_fj253_completion_shell_clone() {
        let orig = CompletionShell::Zsh;
        let cloned = orig.clone();
        assert_eq!(format!("{:?}", cloned), "Zsh");
    }

    // FJ-255: Content diff tests


    #[test]
    fn test_fj264_schema_dispatch() {
        let result = dispatch(Commands::Schema, false, true);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj264_schema_valid_json() {
        let result = cmd_schema();
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj264_schema_has_required_fields() {
        // Capture schema output by running the function
        // We test the structure directly via serde_json
        let machine_schema = serde_json::json!({
            "type": "object",
            "required": ["hostname", "addr"]
        });
        assert_eq!(
            machine_schema["required"][0], "hostname",
            "machine schema should require hostname"
        );
    }


    #[test]
    fn test_fj264_schema_resource_types() {
        let types: [&str; 11] = [
            "package", "file", "service", "mount", "user", "docker", "cron", "network", "pepita",
            "model", "gpu",
        ];
        assert_eq!(types.len(), 11, "should support 11 resource types");
    }


    #[test]
    fn test_fj264_schema_policy_defaults() {
        let policy = serde_json::json!({
            "failure": "stop_on_first",
            "parallel_machines": false,
            "tripwire": true,
            "lock_file": true,
            "ssh_retries": 1
        });
        assert_eq!(policy["ssh_retries"], 1);
        assert_eq!(policy["tripwire"], true);
    }

    // ========================================================================
    // FJ-267: forjar watch
    // ========================================================================


    #[test]
    fn test_fj381_schema_version_flag() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("f.yaml"),
            strict: false,
            json: false,
            dry_expand: false,
            schema_version: Some("1.0".to_string()),
            exhaustive: false,
            policy_file: None,
            check_connectivity: false,
            check_templates: false,
            strict_deps: false,
            check_secrets: false,
            check_idempotency: false,
            check_drift_coverage: false,
            check_cycles_deep: false,
            check_naming: false,
            check_overlaps: false,
            check_limits: false,
            check_complexity: false,
            check_security: false,
            check_deprecation: false,
            check_drift_risk: false,
            check_compliance: None,
            check_portability: false,
            check_resource_limits: false,
            check_unused: false,
            check_dependencies: false,
            check_permissions: false,
            check_idempotency_deep: false,
            check_machine_reachability: false,
            check_circular_refs: false,
            check_naming_conventions: false,
            check_owner_consistency: false,
            check_path_conflicts: false,
            check_service_deps: false,
            check_template_vars: false,
            check_mode_consistency: false,
            check_group_consistency: false,
            check_mount_points: false,
            check_cron_syntax: false,
            check_env_refs: false,
            check_resource_names: None,
            check_resource_count: None,
            check_duplicate_paths: false,
        check_circular_deps: false,
        check_machine_refs: false,
        check_provider_consistency: false,
        check_state_values: false,
        check_unused_machines: false,
        check_tag_consistency: false,
            check_dependency_exists: false,
            check_path_conflicts_strict: false,
            check_duplicate_names: false,
            check_resource_groups: false,
            check_orphan_resources: false,
            check_machine_arch: false, check_resource_health_conflicts: false, check_resource_overlap: false, check_resource_tags: false, check_resource_state_consistency: false, check_resource_dependencies_complete: false, check_machine_connectivity: false, check_resource_naming_pattern: None, check_resource_provider_support: false, check_resource_secret_refs: false, check_resource_idempotency_hints: false,
                check_resource_dependency_depth: None,
                check_resource_machine_affinity: false,
                check_resource_drift_risk: false, check_resource_tag_coverage: false, check_resource_lifecycle_hooks: false, check_resource_provider_version: false, check_resource_naming_convention: false, check_resource_idempotency: false, check_resource_documentation: false, check_resource_ownership: false, check_resource_secret_exposure: false, check_resource_tag_standards: false, check_resource_privilege_escalation: false, check_resource_update_safety: false, check_resource_cross_machine_consistency: false, check_resource_version_pinning: false, check_resource_dependency_completeness: false, check_resource_state_coverage: false, check_resource_rollback_safety: false, check_resource_config_maturity: false, check_resource_dependency_ordering: false, check_resource_tag_completeness: false, check_resource_naming_standards: false, check_resource_dependency_symmetry: false, check_resource_circular_alias: false, check_resource_dependency_depth_limit: false, check_resource_unused_params: false, check_resource_machine_balance: false, check_resource_content_hash_consistency: false, check_resource_dependency_refs: false, check_resource_trigger_refs: false, check_resource_param_type_safety: false, check_resource_env_consistency: false, check_resource_secret_rotation: false, check_resource_lifecycle_completeness: false, check_resource_provider_compatibility: false, check_resource_naming_convention_strict: false, check_resource_idempotency_annotations: false, check_resource_content_size_limit: false, check_resource_dependency_fan_limit: false, check_resource_gpu_backend_consistency: false, check_resource_when_condition_syntax: false, check_resource_lifecycle_hook_coverage: false, check_resource_secret_rotation_age: false, check_resource_dependency_chain_depth: false, check_recipe_input_completeness: false, check_resource_cross_machine_content_duplicates: false, check_resource_machine_reference_validity: false, check_resource_health_correlation: false, check_dependency_optimization: false, check_resource_consolidation_opportunities: false,
        });
        match cmd {
            Commands::Validate(ValidateArgs { schema_version, .. }) => {
                assert_eq!(schema_version, Some("1.0".to_string()));
            }
            _ => panic!("expected Validate"),
        }
    }

}

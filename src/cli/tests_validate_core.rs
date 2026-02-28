//! Tests: Core validation command.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::validate_core::*;
use super::commands::*;
use super::dispatch::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj017_validate_valid() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
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
"#,
        )
        .unwrap();
        cmd_validate(&config, false, false, false).unwrap();
    }


    #[test]
    fn test_fj017_validate_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "2.0"
name: ""
machines: {}
resources: {}
"#,
        )
        .unwrap();
        let result = cmd_validate(&config, false, false, false);
        assert!(result.is_err());
    }


    #[test]
    fn test_fj017_dispatch_validate() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#,
        )
        .unwrap();
        dispatch(
            Commands::Validate(ValidateArgs {
                file: config.clone(),
                strict: false,
                json: false,
                dry_expand: false,
                schema_version: None,
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
                check_resource_drift_risk: false, check_resource_tag_coverage: false, check_resource_lifecycle_hooks: false, check_resource_provider_version: false, check_resource_naming_convention: false, check_resource_idempotency: false, check_resource_documentation: false, check_resource_ownership: false, check_resource_secret_exposure: false, check_resource_tag_standards: false, check_resource_privilege_escalation: false, check_resource_update_safety: false,
            }),
            false,
            true,
        )
        .unwrap();
    }


    #[test]
    fn test_fj132_cmd_validate_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: local
    provider: apt
    packages: [curl]
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_validate(&file, false, false, false).unwrap();
    }


    #[test]
    fn test_fj132_cmd_validate_invalid_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "2.0"
name: test
machines: {}
resources: {}
"#;
        std::fs::write(&file, yaml).unwrap();
        let result = cmd_validate(&file, false, false, false);
        assert!(result.is_err());
    }


    #[test]
    fn test_fj036_cmd_validate_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: valid-project
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
  db:
    hostname: db-01
    addr: 10.0.0.2
resources:
  web-pkg:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
  db-pkg:
    type: package
    machine: db
    provider: apt
    packages: [postgresql]
  app-config:
    type: file
    machine: web
    path: /etc/nginx/nginx.conf
    content: "server {}"
    depends_on: [web-pkg]
"#;
        std::fs::write(&file, yaml).unwrap();
        let result = cmd_validate(&file, false, false, false);
        assert!(
            result.is_ok(),
            "valid config should pass validation: {:?}",
            result.err()
        );
    }


    #[test]
    fn test_fj017_cmd_validate_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nonexistent.yaml");
        let result = cmd_validate(&missing, false, false, false);
        assert!(
            result.is_err(),
            "cmd_validate should fail for a nonexistent file"
        );
    }


    #[test]
    fn test_fj295_validate_json_valid() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/test.txt
    content: "hello"
"#,
        )
        .unwrap();
        let result = cmd_validate(&file, false, true, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj295_validate_json_flag_parse() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
            strict: true,
            json: true,
            dry_expand: false,
            schema_version: None,
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
                check_resource_drift_risk: false, check_resource_tag_coverage: false, check_resource_lifecycle_hooks: false, check_resource_provider_version: false, check_resource_naming_convention: false, check_resource_idempotency: false, check_resource_documentation: false, check_resource_ownership: false, check_resource_secret_exposure: false, check_resource_tag_standards: false, check_resource_privilege_escalation: false, check_resource_update_safety: false,
        });
        match cmd {
            Commands::Validate(ValidateArgs { json, strict, .. }) => {
                assert!(json);
                assert!(strict);
            }
            _ => panic!("expected Validate"),
        }
    }

    // ── FJ-296: history --json --since structured output ──


    #[test]
    fn test_fj330_validate_dry_expand_flag() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("f.yaml"),
            strict: false,
            json: false,
            dry_expand: true,
            schema_version: None,
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
                check_resource_drift_risk: false, check_resource_tag_coverage: false, check_resource_lifecycle_hooks: false, check_resource_provider_version: false, check_resource_naming_convention: false, check_resource_idempotency: false, check_resource_documentation: false, check_resource_ownership: false, check_resource_secret_exposure: false, check_resource_tag_standards: false, check_resource_privilege_escalation: false, check_resource_update_safety: false,
        });
        match cmd {
            Commands::Validate(ValidateArgs { dry_expand, .. }) => assert!(dry_expand),
            _ => panic!("expected Validate"),
        }
    }


    #[test]
    fn test_fj391_validate_exhaustive_flag() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
            strict: false,
            json: false,
            dry_expand: false,
            schema_version: None,
            exhaustive: true,
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
                check_resource_drift_risk: false, check_resource_tag_coverage: false, check_resource_lifecycle_hooks: false, check_resource_provider_version: false, check_resource_naming_convention: false, check_resource_idempotency: false, check_resource_documentation: false, check_resource_ownership: false, check_resource_secret_exposure: false, check_resource_tag_standards: false, check_resource_privilege_escalation: false, check_resource_update_safety: false,
        });
        match cmd {
            Commands::Validate(ValidateArgs { exhaustive, .. }) => assert!(exhaustive),
            _ => panic!("expected Validate"),
        }
    }


    #[test]
    fn test_fj401_validate_policy_file_flag() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
            strict: false,
            json: false,
            dry_expand: false,
            schema_version: None,
            exhaustive: false,
            policy_file: Some(PathBuf::from("policy.yaml")),
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
                check_resource_drift_risk: false, check_resource_tag_coverage: false, check_resource_lifecycle_hooks: false, check_resource_provider_version: false, check_resource_naming_convention: false, check_resource_idempotency: false, check_resource_documentation: false, check_resource_ownership: false, check_resource_secret_exposure: false, check_resource_tag_standards: false, check_resource_privilege_escalation: false, check_resource_update_safety: false,
        });
        match cmd {
            Commands::Validate(ValidateArgs { policy_file, .. }) => {
                assert_eq!(policy_file, Some(PathBuf::from("policy.yaml")));
            }
            _ => panic!("expected Validate"),
        }
    }

}

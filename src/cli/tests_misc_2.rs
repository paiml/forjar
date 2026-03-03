//! Tests: Misc.

#![allow(unused_imports)]
use super::commands::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::validate_core::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj281_group_field_parse() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  web-pkg:
    type: file
    machine: local
    path: /tmp/a.txt
    content: a
    resource_group: network
  db-pkg:
    type: file
    machine: local
    path: /tmp/b.txt
    content: b
    resource_group: database
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(
            config.resources["web-pkg"].resource_group,
            Some("network".to_string())
        );
        assert_eq!(
            config.resources["db-pkg"].resource_group,
            Some("database".to_string())
        );
    }

    #[test]
    fn test_fj281_group_default_none() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  pkg:
    type: file
    machine: local
    path: /tmp/a.txt
    content: a
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.resources["pkg"].resource_group, None);
    }

    #[test]
    fn test_fj282_strict_catches_relative_path() {
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
  cfg:
    type: file
    machine: local
    path: relative/path.txt
    content: "hello"
"#;
        std::fs::write(&file, yaml).unwrap();
        let result = cmd_validate(&file, true, false, false);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("strict validation failed"));
    }

    #[test]
    fn test_fj282_strict_flag_parse() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
            strict: true,
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
            check_machine_arch: false,
            check_resource_health_conflicts: false,
            check_resource_overlap: false,
            check_resource_tags: false,
            check_resource_state_consistency: false,
            check_resource_dependencies_complete: false,
            check_machine_connectivity: false,
            check_resource_naming_pattern: None,
            check_resource_provider_support: false,
            check_resource_secret_refs: false,
            check_resource_idempotency_hints: false,
            check_resource_dependency_depth: None,
            check_resource_machine_affinity: false,
            check_resource_drift_risk: false,
            check_resource_tag_coverage: false,
            check_resource_lifecycle_hooks: false,
            check_resource_provider_version: false,
            check_resource_naming_convention: false,
            check_resource_idempotency: false,
            check_resource_documentation: false,
            check_resource_ownership: false,
            check_resource_secret_exposure: false,
            check_resource_tag_standards: false,
            check_resource_privilege_escalation: false,
            check_resource_update_safety: false,
            check_resource_cross_machine_consistency: false,
            check_resource_version_pinning: false,
            check_resource_dependency_completeness: false,
            check_resource_state_coverage: false,
            check_resource_rollback_safety: false,
            check_resource_config_maturity: false,
            check_resource_dependency_ordering: false,
            check_resource_tag_completeness: false,
            check_resource_naming_standards: false,
            check_resource_dependency_symmetry: false,
            check_resource_circular_alias: false,
            check_resource_dependency_depth_limit: false,
            check_resource_unused_params: false,
            check_resource_machine_balance: false,
            check_resource_content_hash_consistency: false,
            check_resource_dependency_refs: false,
            check_resource_trigger_refs: false,
            check_resource_param_type_safety: false,
            check_resource_env_consistency: false,
            check_resource_secret_rotation: false,
            check_resource_lifecycle_completeness: false,
            check_resource_provider_compatibility: false,
            check_resource_naming_convention_strict: false,
            check_resource_idempotency_annotations: false,
            check_resource_content_size_limit: false,
            check_resource_dependency_fan_limit: false,
            check_resource_gpu_backend_consistency: false,
            check_resource_when_condition_syntax: false,
            check_resource_lifecycle_hook_coverage: false,
            check_resource_secret_rotation_age: false,
            check_resource_dependency_chain_depth: false,
            check_recipe_input_completeness: false,
            check_resource_cross_machine_content_duplicates: false,
            check_resource_machine_reference_validity: false,
            check_resource_health_correlation: false,
            check_dependency_optimization: false,
            check_resource_consolidation_opportunities: false,
            check_resource_compliance_tags: false,
            check_resource_rollback_coverage: false,
            check_resource_dependency_balance: false,
            check_resource_secret_scope: false,
            check_resource_deprecation_usage: false,
            check_resource_when_condition_coverage: false,
            check_resource_dependency_symmetry_deep: false,
            check_resource_tag_namespace: false,
            check_resource_machine_capacity: false,
            check_resource_dependency_fan_out_limit: false,
            check_resource_tag_required_keys: false,
            check_resource_content_drift_risk: false,
            check_resource_circular_dependency_depth: false,
            check_resource_orphan_detection_deep: false,
            check_resource_provider_diversity: false,
            check_resource_dependency_isolation: false,
            check_resource_tag_value_consistency: false,
            check_resource_machine_distribution_balance: false,
            check_resource_dependency_version_drift: false,
            check_resource_naming_length_limit: false,
            check_resource_type_coverage_per_machine: false,
            check_resource_dependency_depth_variance: false,
            check_resource_tag_key_naming: false,
            check_resource_content_length_limit: false,
            check_resource_dependency_completeness_audit: false,
            check_resource_machine_coverage_gap: false,
            check_resource_path_depth_limit: false,
            check_resource_dependency_ordering_consistency: false,
            check_resource_tag_value_format: false,
            check_resource_provider_version_pinning: false,
            check_recipe_purity: false,
            check_reproducibility_score: false,
        });
        match cmd {
            Commands::Validate(ValidateArgs { strict, .. }) => assert!(strict),
            _ => panic!("expected Validate"),
        }
    }

    #[test]
    fn test_fj282_strict_passes_clean_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        let yaml = r#"
version: "1.0"
name: test
description: "test project"
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
"#;
        std::fs::write(&file, yaml).unwrap();
        cmd_validate(&file, true, false, false).unwrap();
    }

    #[test]
    fn test_fj284_parse_duration_invalid() {
        assert!(parse_duration_secs("abc").is_err());
        assert!(parse_duration_secs("10x").is_err());
        assert!(parse_duration_secs("").is_err());
    }

    #[test]
    fn test_fj284_since_flag_parse() {
        let cmd = Commands::History(HistoryArgs {
            state_dir: PathBuf::from("state"),
            machine: None,
            limit: 10,
            json: false,
            since: Some("24h".to_string()),
            resource: None,
        });
        match cmd {
            Commands::History(HistoryArgs { since, .. }) => {
                assert_eq!(since, Some("24h".to_string()));
            }
            _ => panic!("expected History"),
        }
    }

    #[test]
    fn test_fj285_target_flag_parse() {
        let cmd = Commands::Plan(PlanArgs {
            file: PathBuf::from("forjar.yaml"),
            machine: None,
            resource: None,
            tag: None,
            group: None,
            state_dir: PathBuf::from("state"),
            json: false,
            output_dir: None,
            env_file: None,
            workspace: None,
            no_diff: false,
            target: Some("web-config".to_string()),
            cost: false,
            what_if: vec![],
            out: None,
            why: false,
        });
        match cmd {
            Commands::Plan(PlanArgs { target, .. }) => {
                assert_eq!(target, Some("web-config".to_string()));
            }
            _ => panic!("expected Plan"),
        }
    }
}

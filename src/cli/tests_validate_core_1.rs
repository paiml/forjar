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

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj411_validate_check_connectivity_flag() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
            strict: false,
            json: false,
            dry_expand: false,
            schema_version: None,
            exhaustive: false,
            policy_file: None,
            check_connectivity: true,
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
        });
        match cmd {
            Commands::Validate(ValidateArgs {
                check_connectivity, ..
            }) => assert!(check_connectivity),
            _ => panic!("expected Validate"),
        }
    }


    #[test]
    fn test_fj421_validate_check_templates_flag() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
            strict: false,
            json: false,
            dry_expand: false,
            schema_version: None,
            exhaustive: false,
            policy_file: None,
            check_connectivity: false,
            check_templates: true,
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
        });
        match cmd {
            Commands::Validate(ValidateArgs {
                check_templates, ..
            }) => assert!(check_templates),
            _ => panic!("expected Validate"),
        }
    }


    #[test]
    fn test_fj431_validate_strict_deps_flag() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
            strict: false,
            json: false,
            dry_expand: false,
            schema_version: None,
            exhaustive: false,
            policy_file: None,
            check_connectivity: false,
            check_templates: false,
            strict_deps: true,
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
        });
        match cmd {
            Commands::Validate(ValidateArgs { strict_deps, .. }) => assert!(strict_deps),
            _ => panic!("expected Validate"),
        }
    }


    #[test]
    fn test_fj441_validate_check_secrets_flag() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
            strict: false,
            json: false,
            dry_expand: false,
            schema_version: None,
            exhaustive: false,
            policy_file: None,
            check_connectivity: false,
            check_templates: false,
            strict_deps: false,
            check_secrets: true,
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
        });
        match cmd {
            Commands::Validate(ValidateArgs { check_secrets, .. }) => assert!(check_secrets),
            _ => panic!("expected Validate"),
        }
    }


    #[test]
    fn test_fj451_validate_check_idempotency_flag() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
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
            check_idempotency: true,
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
        });
        match cmd {
            Commands::Validate(ValidateArgs {
                check_idempotency, ..
            }) => assert!(check_idempotency),
            _ => panic!("expected Validate"),
        }
    }


    #[test]
    fn test_fj461_validate_check_drift_coverage_flag() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
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
            check_drift_coverage: true,
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
        });
        match cmd {
            Commands::Validate(ValidateArgs {
                check_drift_coverage,
                ..
            }) => assert!(check_drift_coverage),
            _ => panic!("expected Validate"),
        }
    }


    #[test]
    fn test_fj471_validate_check_cycles_deep_flag() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
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
            check_cycles_deep: true,
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
        });
        match cmd {
            Commands::Validate(ValidateArgs {
                check_cycles_deep, ..
            }) => assert!(check_cycles_deep),
            _ => panic!("expected Validate"),
        }
    }


    #[test]
    fn test_fj481_validate_check_naming_flag() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
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
            check_naming: true,
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
        });
        match cmd {
            Commands::Validate(ValidateArgs { check_naming, .. }) => assert!(check_naming),
            _ => panic!("expected Validate"),
        }
    }


    #[test]
    fn test_fj491_validate_check_overlaps_flag() {
        let cmd = Commands::Validate(ValidateArgs {
            file: PathBuf::from("forjar.yaml"),
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
            check_overlaps: true,
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
        });
        match cmd {
            Commands::Validate(ValidateArgs { check_overlaps, .. }) => assert!(check_overlaps),
            _ => panic!("expected Validate"),
        }
    }

}

//! Validate command dispatch — routes validate sub-flags to check handlers.

use std::path::Path;
use super::commands::*;
use super::validate_core::*;
use super::validate_policy::*;
use super::validate_structural::*;
use super::validate_paths::*;
use super::validate_quality::*;
use super::validate_compliance::*;
use super::validate_resources::*;
use super::validate_safety::*;
use super::validate_advanced::*;


/// Structural/resource validation checks.
fn try_validate_structural(
    file: &Path, json: bool,
    check_mount_points: bool, check_group_consistency: bool,
    check_mode_consistency: bool, check_template_vars: bool,
    check_service_deps: bool, check_path_conflicts: bool,
    check_owner_consistency: bool, check_naming_conventions: bool,
    check_circular_refs: bool, check_machine_reachability: bool,
) -> Option<Result<(), String>> {
    if check_mount_points { return Some(cmd_validate_check_mount_points(file, json)); }
    if check_group_consistency { return Some(cmd_validate_check_group_consistency(file, json)); }
    if check_mode_consistency { return Some(cmd_validate_check_mode_consistency(file, json)); }
    if check_template_vars { return Some(cmd_validate_check_template_vars(file, json)); }
    if check_service_deps { return Some(cmd_validate_check_service_deps(file, json)); }
    if check_path_conflicts { return Some(cmd_validate_check_path_conflicts(file, json)); }
    if check_owner_consistency { return Some(cmd_validate_check_owner_consistency(file, json)); }
    if check_naming_conventions { return Some(cmd_validate_check_naming_conventions(file, json)); }
    if check_circular_refs { return Some(cmd_validate_check_circular_refs(file, json)); }
    if check_machine_reachability { return Some(cmd_validate_check_machine_reachability(file, json)); }
    None
}

/// Quality/compliance validation checks.
fn try_validate_quality(
    file: &Path, json: bool,
    check_idempotency_deep: bool, check_permissions: bool,
    check_dependencies: bool, check_unused: bool,
    check_resource_limits: bool, check_portability: bool,
    check_compliance: Option<&str>, check_drift_risk: bool,
    check_deprecation: bool, check_security: bool,
    check_complexity: bool, check_limits: bool,
) -> Option<Result<(), String>> {
    if check_idempotency_deep { return Some(cmd_validate_check_idempotency_deep(file, json)); }
    if check_permissions { return Some(cmd_validate_check_permissions(file, json)); }
    if check_dependencies { return Some(cmd_validate_check_dependencies(file, json)); }
    if check_unused { return Some(cmd_validate_check_unused(file, json)); }
    if check_resource_limits { return Some(cmd_validate_check_resource_limits(file, json)); }
    if check_portability { return Some(cmd_validate_check_portability(file, json)); }
    if let Some(policy) = check_compliance { return Some(cmd_validate_check_compliance(file, policy, json)); }
    if check_drift_risk { return Some(cmd_validate_check_drift_risk(file, json)); }
    if check_deprecation { return Some(cmd_validate_check_deprecation(file, json)); }
    if check_security { return Some(cmd_validate_check_security(file, json)); }
    if check_complexity { return Some(cmd_validate_check_complexity(file, json)); }
    if check_limits { return Some(cmd_validate_check_limits(file, json)); }
    None
}

/// Core validation checks (overlaps through base validate).
fn try_validate_core(
    file: &Path, json: bool, strict: bool, dry_expand: bool,
    check_overlaps: bool, check_naming: bool,
    check_cycles_deep: bool, check_drift_coverage: bool,
    check_idempotency: bool, check_secrets: bool,
    strict_deps: bool, check_templates: bool,
    check_connectivity: bool, policy_file: Option<&Path>,
    exhaustive: bool,
) -> Option<Result<(), String>> {
    if check_overlaps { return Some(cmd_validate_check_overlaps(file, json)); }
    if check_naming { return Some(cmd_validate_check_naming(file, json)); }
    if check_cycles_deep { return Some(cmd_validate_check_cycles_deep(file, json)); }
    if check_drift_coverage { return Some(cmd_validate_check_drift_coverage(file, json)); }
    if check_idempotency { return Some(cmd_validate_check_idempotency(file, json)); }
    if check_secrets { return Some(cmd_validate_check_secrets(file, json)); }
    if strict_deps { return Some(cmd_validate_strict_deps(file, json)); }
    if check_templates { return Some(cmd_validate_check_templates(file, json)); }
    if check_connectivity { return Some(cmd_validate_connectivity(file, json)); }
    if let Some(pf) = policy_file { return Some(cmd_validate_policy_file(file, pf, json)); }
    if exhaustive { return Some(cmd_validate_exhaustive(file, json)); }
    None
}


/// Route validate command to specific check handlers.
pub(crate) fn dispatch_validate(args: ValidateArgs) -> Result<(), String> {
    let ValidateArgs {
        file, strict, json, dry_expand,
        schema_version: _schema_version, exhaustive, policy_file,
        check_connectivity, check_templates, strict_deps, check_secrets,
        check_idempotency, check_drift_coverage, check_cycles_deep,
        check_naming, check_overlaps, check_limits, check_complexity,
        check_security, check_deprecation, check_drift_risk, check_compliance,
        check_portability, check_resource_limits, check_unused,
        check_dependencies, check_permissions, check_idempotency_deep,
        check_machine_reachability, check_circular_refs,
        check_naming_conventions, check_owner_consistency,
        check_path_conflicts, check_service_deps, check_template_vars,
        check_mode_consistency, check_group_consistency, check_mount_points,
        check_cron_syntax,
        check_env_refs, check_resource_names,
        check_resource_count, check_duplicate_paths,
        check_circular_deps, check_machine_refs,
        check_provider_consistency, check_state_values,
        check_unused_machines, check_tag_consistency,
        check_dependency_exists, check_path_conflicts_strict,
        check_duplicate_names, check_resource_groups,
        check_orphan_resources, check_machine_arch,
        check_resource_health_conflicts, check_resource_overlap,
        check_resource_tags, check_resource_state_consistency,
    } = args;

    if check_cron_syntax {
        return cmd_validate_check_cron_syntax(&file, json);
    }
    if check_env_refs {
        return cmd_validate_check_env_refs(&file, json);
    }
    if let Some(ref pattern) = check_resource_names {
        return cmd_validate_check_resource_names(&file, json, pattern);
    }
    if let Some(limit) = check_resource_count {
        return cmd_validate_check_resource_count(&file, json, limit);
    }
    if check_duplicate_paths {
        return cmd_validate_check_duplicate_paths(&file, json);
    }
    if check_circular_deps {
        return cmd_validate_check_circular_deps(&file, json);
    }
    if check_machine_refs {
        return cmd_validate_check_machine_refs(&file, json);
    }
    if check_provider_consistency {
        return cmd_validate_check_provider_consistency(&file, json);
    }
    if check_state_values {
        return cmd_validate_check_state_values(&file, json);
    }
    if check_unused_machines {
        return cmd_validate_check_unused_machines(&file, json);
    }
    if check_tag_consistency {
        return cmd_validate_check_tag_consistency(&file, json);
    }
    if check_dependency_exists {
        return cmd_validate_check_dependency_exists(&file, json);
    }
    if check_path_conflicts_strict {
        return cmd_validate_check_path_conflicts_strict(&file, json);
    }
    if check_duplicate_names {
        return cmd_validate_check_duplicate_names(&file, json);
    }
    if check_resource_groups {
        return cmd_validate_check_resource_groups(&file, json);
    }
    if check_orphan_resources {
        return cmd_validate_check_orphan_resources(&file, json);
    }
    if check_machine_arch {
        return cmd_validate_check_machine_arch(&file, json);
    }
    if check_resource_health_conflicts {
        return cmd_validate_check_resource_health_conflicts(&file, json);
    }
    if check_resource_overlap {
        return cmd_validate_check_resource_overlap(&file, json);
    }
    if check_resource_tags {
        return cmd_validate_check_resource_tags(&file, json);
    }
    if check_resource_state_consistency {
        return cmd_validate_check_resource_state_consistency(&file, json);
    }
    if let Some(r) = try_validate_structural(&file, json, check_mount_points, check_group_consistency, check_mode_consistency, check_template_vars, check_service_deps, check_path_conflicts, check_owner_consistency, check_naming_conventions, check_circular_refs, check_machine_reachability) {
        return r;
    }
    if let Some(r) = try_validate_quality(&file, json, check_idempotency_deep, check_permissions, check_dependencies, check_unused, check_resource_limits, check_portability, check_compliance.as_deref(), check_drift_risk, check_deprecation, check_security, check_complexity, check_limits) {
        return r;
    }
    if let Some(r) = try_validate_core(&file, json, strict, dry_expand, check_overlaps, check_naming, check_cycles_deep, check_drift_coverage, check_idempotency, check_secrets, strict_deps, check_templates, check_connectivity, policy_file.as_deref(), exhaustive) {
        return r;
    }
    cmd_validate(&file, strict, json, dry_expand)
}

use super::validate_compliance::*;
use super::validate_core::*;
use super::validate_governance::*;
use super::validate_ordering::*;
use super::validate_ordering_ext::*;
use super::validate_ownership::*;
use super::validate_paths::*;
use super::validate_policy::*;
use super::validate_quality::*;
use super::validate_resources::*;
use super::validate_structural::*;
use super::validate_structural_constraints::*;
use std::path::Path;
#[allow(clippy::too_many_arguments)]
pub(super) fn try_validate_structural(
    file: &Path,
    json: bool,
    check_mount_points: bool,
    check_group_consistency: bool,
    check_mode_consistency: bool,
    check_template_vars: bool,
    check_service_deps: bool,
    check_path_conflicts: bool,
    check_owner_consistency: bool,
    check_naming_conventions: bool,
    check_circular_refs: bool,
    check_machine_reachability: bool,
) -> Option<Result<(), String>> {
    if check_mount_points {
        return Some(cmd_validate_check_mount_points(file, json));
    }
    if check_group_consistency {
        return Some(cmd_validate_check_group_consistency(file, json));
    }
    if check_mode_consistency {
        return Some(cmd_validate_check_mode_consistency(file, json));
    }
    if check_template_vars {
        return Some(cmd_validate_check_template_vars(file, json));
    }
    if check_service_deps {
        return Some(cmd_validate_check_service_deps(file, json));
    }
    if check_path_conflicts {
        return Some(cmd_validate_check_path_conflicts(file, json));
    }
    if check_owner_consistency {
        return Some(cmd_validate_check_owner_consistency(file, json));
    }
    if check_naming_conventions {
        return Some(cmd_validate_check_naming_conventions(file, json));
    }
    if check_circular_refs {
        return Some(cmd_validate_check_circular_refs(file, json));
    }
    if check_machine_reachability {
        return Some(cmd_validate_check_machine_reachability(file, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_validate_quality(
    file: &Path,
    json: bool,
    check_idempotency_deep: bool,
    check_permissions: bool,
    check_dependencies: bool,
    check_unused: bool,
    check_resource_limits: bool,
    check_portability: bool,
    check_compliance: Option<&str>,
    check_drift_risk: bool,
    check_deprecation: bool,
    check_security: bool,
    check_complexity: bool,
    check_limits: bool,
) -> Option<Result<(), String>> {
    if check_idempotency_deep {
        return Some(cmd_validate_check_idempotency_deep(file, json));
    }
    if check_permissions {
        return Some(cmd_validate_check_permissions(file, json));
    }
    if check_dependencies {
        return Some(cmd_validate_check_dependencies(file, json));
    }
    if check_unused {
        return Some(cmd_validate_check_unused(file, json));
    }
    if check_resource_limits {
        return Some(cmd_validate_check_resource_limits(file, json));
    }
    if check_portability {
        return Some(cmd_validate_check_portability(file, json));
    }
    if let Some(policy) = check_compliance {
        return Some(cmd_validate_check_compliance(file, policy, json));
    }
    if check_drift_risk {
        return Some(cmd_validate_check_drift_risk(file, json));
    }
    if check_deprecation {
        return Some(cmd_validate_check_deprecation(file, json));
    }
    if check_security {
        return Some(cmd_validate_check_security(file, json));
    }
    if check_complexity {
        return Some(cmd_validate_check_complexity(file, json));
    }
    if check_limits {
        return Some(cmd_validate_check_limits(file, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_validate_core(
    file: &Path,
    json: bool,
    _strict: bool,
    _dry_expand: bool,
    check_overlaps: bool,
    check_naming: bool,
    check_cycles_deep: bool,
    check_drift_coverage: bool,
    check_idempotency: bool,
    check_secrets: bool,
    strict_deps: bool,
    check_templates: bool,
    check_connectivity: bool,
    policy_file: Option<&Path>,
    exhaustive: bool,
) -> Option<Result<(), String>> {
    if check_overlaps {
        return Some(cmd_validate_check_overlaps(file, json));
    }
    if check_naming {
        return Some(cmd_validate_check_naming(file, json));
    }
    if check_cycles_deep {
        return Some(cmd_validate_check_cycles_deep(file, json));
    }
    if check_drift_coverage {
        return Some(cmd_validate_check_drift_coverage(file, json));
    }
    if check_idempotency {
        return Some(cmd_validate_check_idempotency(file, json));
    }
    if check_secrets {
        return Some(cmd_validate_check_secrets(file, json));
    }
    if strict_deps {
        return Some(cmd_validate_strict_deps(file, json));
    }
    if check_templates {
        return Some(cmd_validate_check_templates(file, json));
    }
    if check_connectivity {
        return Some(cmd_validate_connectivity(file, json));
    }
    if let Some(pf) = policy_file {
        return Some(cmd_validate_policy_file(file, pf, json));
    }
    if exhaustive {
        return Some(cmd_validate_exhaustive(file, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_validate_governance(
    file: &Path,
    json: bool,
    check_resource_naming_pattern: &Option<String>,
    check_resource_provider_support: bool,
    check_resource_secret_refs: bool,
    check_resource_idempotency_hints: bool,
    check_resource_dependency_depth: Option<usize>,
    check_resource_machine_affinity: bool,
    check_resource_drift_risk: bool,
    check_resource_tag_coverage: bool,
) -> Option<Result<(), String>> {
    if let Some(ref pattern) = check_resource_naming_pattern {
        return Some(cmd_validate_check_resource_naming_pattern(
            file, json, pattern,
        ));
    }
    if check_resource_provider_support {
        return Some(cmd_validate_check_resource_provider_support(file, json));
    }
    if check_resource_secret_refs {
        return Some(cmd_validate_check_resource_secret_refs(file, json));
    }
    if check_resource_idempotency_hints {
        return Some(cmd_validate_check_resource_idempotency_hints(file, json));
    }
    if let Some(depth) = check_resource_dependency_depth {
        return Some(cmd_validate_check_resource_dependency_depth(
            file, json, depth,
        ));
    }
    if check_resource_machine_affinity {
        return Some(cmd_validate_check_resource_machine_affinity(file, json));
    }
    if check_resource_drift_risk {
        return Some(cmd_validate_check_resource_drift_risk(file, json));
    }
    if check_resource_tag_coverage {
        return Some(cmd_validate_check_resource_tag_coverage(file, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_validate_governance_b(
    file: &Path,
    json: bool,
    check_resource_lifecycle_hooks: bool,
    check_resource_provider_version: bool,
    check_resource_naming_convention: bool,
    check_resource_idempotency: bool,
    check_resource_documentation: bool,
    check_resource_ownership: bool,
    check_resource_secret_exposure: bool,
    check_resource_tag_standards: bool,
    check_resource_privilege_escalation: bool,
    check_resource_update_safety: bool,
    check_resource_cross_machine_consistency: bool,
    check_resource_version_pinning: bool,
) -> Option<Result<(), String>> {
    if check_resource_lifecycle_hooks {
        return Some(cmd_validate_check_resource_lifecycle_hooks(file, json));
    }
    if check_resource_provider_version {
        return Some(cmd_validate_check_resource_provider_version(file, json));
    }
    if check_resource_naming_convention {
        return Some(cmd_validate_check_resource_naming_convention(file, json));
    }
    if check_resource_idempotency {
        return Some(cmd_validate_check_resource_idempotency(file, json));
    }
    if check_resource_documentation {
        return Some(cmd_validate_check_resource_documentation(file, json));
    }
    if check_resource_ownership {
        return Some(cmd_validate_check_resource_ownership(file, json));
    }
    if check_resource_secret_exposure {
        return Some(cmd_validate_check_resource_secret_exposure(file, json));
    }
    if check_resource_tag_standards {
        return Some(cmd_validate_check_resource_tag_standards(file, json));
    }
    if check_resource_privilege_escalation {
        return Some(cmd_validate_check_resource_privilege_escalation(file, json));
    }
    if check_resource_update_safety {
        return Some(cmd_validate_check_resource_update_safety(file, json));
    }
    if check_resource_cross_machine_consistency {
        return Some(cmd_validate_check_resource_cross_machine_consistency(
            file, json,
        ));
    }
    if check_resource_version_pinning {
        return Some(cmd_validate_check_resource_version_pinning(file, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_validate_governance_c(
    file: &Path,
    json: bool,
    check_resource_dependency_completeness: bool,
    check_resource_state_coverage: bool,
    check_resource_rollback_safety: bool,
    check_resource_config_maturity: bool,
    check_resource_dependency_ordering: bool,
    check_resource_tag_completeness: bool,
    check_resource_naming_standards: bool,
    check_resource_dependency_symmetry: bool,
    check_resource_circular_alias: bool,
    check_resource_dependency_depth_limit: bool,
    check_resource_unused_params: bool,
    check_resource_machine_balance: bool,
) -> Option<Result<(), String>> {
    if check_resource_dependency_completeness {
        return Some(cmd_validate_check_resource_dependency_completeness(
            file, json,
        ));
    }
    if check_resource_state_coverage {
        return Some(cmd_validate_check_resource_state_coverage(file, json));
    }
    if check_resource_rollback_safety {
        return Some(cmd_validate_check_resource_rollback_safety(file, json));
    }
    if check_resource_config_maturity {
        return Some(cmd_validate_check_resource_config_maturity(file, json));
    }
    if check_resource_dependency_ordering {
        return Some(cmd_validate_check_resource_dependency_ordering(file, json));
    }
    if check_resource_tag_completeness {
        return Some(cmd_validate_check_resource_tag_completeness(file, json));
    }
    if check_resource_naming_standards {
        return Some(cmd_validate_check_resource_naming_standards(file, json));
    }
    if check_resource_dependency_symmetry {
        return Some(cmd_validate_check_resource_dependency_symmetry(file, json));
    }
    if check_resource_circular_alias {
        return Some(cmd_validate_check_resource_circular_alias(file, json));
    }
    if check_resource_dependency_depth_limit {
        return Some(cmd_validate_check_resource_dependency_depth_limit(
            file, json,
        ));
    }
    if check_resource_unused_params {
        return Some(cmd_validate_check_resource_unused_params(file, json));
    }
    if check_resource_machine_balance {
        return Some(cmd_validate_check_resource_machine_balance(file, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_validate_governance_d(
    file: &Path,
    json: bool,
    check_resource_content_hash_consistency: bool,
    check_resource_dependency_refs: bool,
    check_resource_trigger_refs: bool,
    check_resource_param_type_safety: bool,
    check_resource_env_consistency: bool,
    check_resource_secret_rotation: bool,
    check_resource_lifecycle_completeness: bool,
    check_resource_provider_compatibility: bool,
    check_resource_naming_convention_strict: bool,
    check_resource_idempotency_annotations: bool,
    check_resource_content_size_limit: bool,
    check_resource_dependency_fan_limit: bool,
) -> Option<Result<(), String>> {
    if check_resource_content_hash_consistency {
        return Some(cmd_validate_check_resource_content_hash_consistency(
            file, json,
        ));
    }
    if check_resource_dependency_refs {
        return Some(cmd_validate_check_resource_dependency_refs(file, json));
    }
    if check_resource_trigger_refs {
        return Some(cmd_validate_check_resource_trigger_refs(file, json));
    }
    if check_resource_param_type_safety {
        return Some(cmd_validate_check_resource_param_type_safety(file, json));
    }
    if check_resource_env_consistency {
        return Some(cmd_validate_check_resource_env_consistency(file, json));
    }
    if check_resource_secret_rotation {
        return Some(cmd_validate_check_resource_secret_rotation(file, json));
    }
    if check_resource_lifecycle_completeness {
        return Some(cmd_validate_check_resource_lifecycle_completeness(
            file, json,
        ));
    }
    if check_resource_provider_compatibility {
        return Some(cmd_validate_check_resource_provider_compatibility(
            file, json,
        ));
    }
    if check_resource_naming_convention_strict {
        return Some(cmd_validate_check_resource_naming_convention_strict(
            file, json,
        ));
    }
    if check_resource_idempotency_annotations {
        return Some(cmd_validate_check_resource_idempotency_annotations(
            file, json,
        ));
    }
    if check_resource_content_size_limit {
        return Some(cmd_validate_check_resource_content_size_limit(file, json));
    }
    if check_resource_dependency_fan_limit {
        return Some(cmd_validate_check_resource_dependency_fan_limit(file, json));
    }
    None
}

pub(super) use super::dispatch_validate_b::*;
pub(super) use super::dispatch_validate_c::*;

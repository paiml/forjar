use super::commands::*;
use super::validate_advanced::*;
use super::validate_analytics::*;
use super::validate_audit::*;
use super::validate_compliance::*;
use super::validate_compliance_ext::*;
use super::validate_config_quality::*;
use super::validate_core::*;
use super::validate_governance::*;
use super::validate_governance_ext::*;
use super::validate_hygiene::*;
use super::validate_maturity::*;
use super::validate_ordering::*;
use super::validate_ordering_ext::*;
use super::validate_ownership::*;
use super::validate_paths::*;
use super::validate_policy::*;
use super::validate_quality::*;
use super::validate_resilience::*;
use super::validate_resources::*;
use super::validate_safety::*;
use super::validate_scoring::*;
use super::validate_security::*;
use super::validate_security_ext::*;
use super::validate_structural::*;
use super::validate_topology::*;
use super::validate_transport::*;
use std::path::Path;
#[allow(clippy::too_many_arguments)]
fn try_validate_structural(
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
fn try_validate_quality(
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
fn try_validate_core(
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
fn try_validate_governance(
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
fn try_validate_governance_b(
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
fn try_validate_governance_c(
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
fn try_validate_governance_d(
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
// try_validate_phase94 merged into try_validate_phases_94_96
#[allow(clippy::too_many_arguments)]
fn try_validate_advanced(
    file: &Path,
    json: bool,
    check_orphan_resources: bool,
    check_machine_arch: bool,
    check_resource_health_conflicts: bool,
    check_resource_overlap: bool,
    check_resource_tags: bool,
    check_resource_state_consistency: bool,
    check_resource_dependencies_complete: bool,
    check_machine_connectivity: bool,
) -> Option<Result<(), String>> {
    if check_orphan_resources {
        return Some(cmd_validate_check_orphan_resources(file, json));
    }
    if check_machine_arch {
        return Some(cmd_validate_check_machine_arch(file, json));
    }
    if check_resource_health_conflicts {
        return Some(cmd_validate_check_resource_health_conflicts(file, json));
    }
    if check_resource_overlap {
        return Some(cmd_validate_check_resource_overlap(file, json));
    }
    if check_resource_tags {
        return Some(cmd_validate_check_resource_tags(file, json));
    }
    if check_resource_state_consistency {
        return Some(cmd_validate_check_resource_state_consistency(file, json));
    }
    if check_resource_dependencies_complete {
        return Some(cmd_validate_check_resource_dependencies_complete(
            file, json,
        ));
    }
    if check_machine_connectivity {
        return Some(cmd_validate_check_machine_connectivity(file, json));
    }
    None
}
pub(crate) fn dispatch_validate(args: ValidateArgs) -> Result<(), String> {
    let ValidateArgs {
        file,
        strict,
        json,
        dry_expand,
        schema_version: _schema_version,
        exhaustive,
        policy_file,
        check_connectivity,
        check_templates,
        strict_deps,
        check_secrets,
        check_idempotency,
        check_drift_coverage,
        check_cycles_deep,
        check_naming,
        check_overlaps,
        check_limits,
        check_complexity,
        check_security,
        check_deprecation,
        check_drift_risk,
        check_compliance,
        check_portability,
        check_resource_limits,
        check_unused,
        check_dependencies,
        check_permissions,
        check_idempotency_deep,
        check_machine_reachability,
        check_circular_refs,
        check_naming_conventions,
        check_owner_consistency,
        check_path_conflicts,
        check_service_deps,
        check_template_vars,
        check_mode_consistency,
        check_group_consistency,
        check_mount_points,
        check_cron_syntax,
        check_env_refs,
        check_resource_names,
        check_resource_count,
        check_duplicate_paths,
        check_circular_deps,
        check_machine_refs,
        check_provider_consistency,
        check_state_values,
        check_unused_machines,
        check_tag_consistency,
        check_dependency_exists,
        check_path_conflicts_strict,
        check_duplicate_names,
        check_resource_groups,
        check_orphan_resources,
        check_machine_arch,
        check_resource_health_conflicts,
        check_resource_overlap,
        check_resource_tags,
        check_resource_state_consistency,
        check_resource_dependencies_complete,
        check_machine_connectivity,
        check_resource_naming_pattern,
        check_resource_provider_support,
        check_resource_secret_refs,
        check_resource_idempotency_hints,
        check_resource_dependency_depth,
        check_resource_machine_affinity,
        check_resource_drift_risk,
        check_resource_tag_coverage,
        check_resource_lifecycle_hooks,
        check_resource_provider_version,
        check_resource_naming_convention,
        check_resource_idempotency,
        check_resource_documentation,
        check_resource_ownership,
        check_resource_secret_exposure,
        check_resource_tag_standards,
        check_resource_privilege_escalation,
        check_resource_update_safety,
        check_resource_cross_machine_consistency,
        check_resource_version_pinning,
        check_resource_dependency_completeness,
        check_resource_state_coverage,
        check_resource_rollback_safety,
        check_resource_config_maturity,
        check_resource_dependency_ordering,
        check_resource_tag_completeness,
        check_resource_naming_standards,
        check_resource_dependency_symmetry,
        check_resource_circular_alias,
        check_resource_dependency_depth_limit,
        check_resource_unused_params,
        check_resource_machine_balance,
        check_resource_content_hash_consistency,
        check_resource_dependency_refs,
        check_resource_trigger_refs,
        check_resource_param_type_safety,
        check_resource_env_consistency,
        check_resource_secret_rotation,
        check_resource_lifecycle_completeness,
        check_resource_provider_compatibility,
        check_resource_naming_convention_strict,
        check_resource_idempotency_annotations,
        check_resource_content_size_limit,
        check_resource_dependency_fan_limit,
        check_resource_gpu_backend_consistency,
        check_resource_when_condition_syntax,
        check_resource_lifecycle_hook_coverage,
        check_resource_secret_rotation_age,
        check_resource_dependency_chain_depth,
        check_recipe_input_completeness,
        check_resource_cross_machine_content_duplicates,
        check_resource_machine_reference_validity,
        check_resource_health_correlation,
        check_dependency_optimization,
        check_resource_consolidation_opportunities,
        check_resource_compliance_tags,
        check_resource_rollback_coverage,
        check_resource_dependency_balance,
        check_resource_secret_scope,
        check_resource_deprecation_usage,
        check_resource_when_condition_coverage,
        check_resource_dependency_symmetry_deep,
        check_resource_tag_namespace,
        check_resource_machine_capacity,
        check_resource_dependency_fan_out_limit,
        check_resource_tag_required_keys,
        check_resource_content_drift_risk,
        check_resource_circular_dependency_depth,
        check_resource_orphan_detection_deep,
        check_resource_provider_diversity,
        check_resource_dependency_isolation,
        check_resource_tag_value_consistency,
        check_resource_machine_distribution_balance,
        check_resource_dependency_version_drift,
        check_resource_naming_length_limit,
        check_resource_type_coverage_per_machine,
        check_resource_dependency_depth_variance,
        check_resource_tag_key_naming,
        check_resource_content_length_limit,
        check_resource_dependency_completeness_audit,
        check_resource_machine_coverage_gap,
        check_resource_path_depth_limit,
        check_resource_dependency_ordering_consistency,
        check_resource_tag_value_format,
        check_resource_provider_version_pinning,
    } = args;
    if let Some(r) = try_validate_checks_early_a(
        &file,
        json,
        check_cron_syntax,
        check_env_refs,
        check_resource_names.as_deref(),
        check_resource_count,
        check_duplicate_paths,
        check_circular_deps,
        check_machine_refs,
        check_provider_consistency,
    ) {
        return r;
    }
    if let Some(r) = try_validate_checks_early_b(
        &file,
        json,
        check_state_values,
        check_unused_machines,
        check_tag_consistency,
        check_dependency_exists,
        check_path_conflicts_strict,
        check_duplicate_names,
        check_resource_groups,
    ) {
        return r;
    }
    if let Some(r) = try_validate_governance(
        &file,
        json,
        &check_resource_naming_pattern,
        check_resource_provider_support,
        check_resource_secret_refs,
        check_resource_idempotency_hints,
        check_resource_dependency_depth,
        check_resource_machine_affinity,
        check_resource_drift_risk,
        check_resource_tag_coverage,
    ) {
        return r;
    }
    if let Some(r) = try_validate_governance_c(
        &file,
        json,
        check_resource_dependency_completeness,
        check_resource_state_coverage,
        check_resource_rollback_safety,
        check_resource_config_maturity,
        check_resource_dependency_ordering,
        check_resource_tag_completeness,
        check_resource_naming_standards,
        check_resource_dependency_symmetry,
        check_resource_circular_alias,
        check_resource_dependency_depth_limit,
        check_resource_unused_params,
        check_resource_machine_balance,
    )
    .or_else(|| {
        try_validate_governance_d(
            &file,
            json,
            check_resource_content_hash_consistency,
            check_resource_dependency_refs,
            check_resource_trigger_refs,
            check_resource_param_type_safety,
            check_resource_env_consistency,
            check_resource_secret_rotation,
            check_resource_lifecycle_completeness,
            check_resource_provider_compatibility,
            check_resource_naming_convention_strict,
            check_resource_idempotency_annotations,
            check_resource_content_size_limit,
            check_resource_dependency_fan_limit,
        )
    }) {
        return r;
    }
    if let Some(r) = try_validate_phases_94_96(
        &file,
        json,
        check_resource_gpu_backend_consistency,
        check_resource_when_condition_syntax,
        check_resource_lifecycle_hook_coverage,
        check_resource_secret_rotation_age,
        check_resource_dependency_chain_depth,
        check_recipe_input_completeness,
        check_resource_cross_machine_content_duplicates,
        check_resource_machine_reference_validity,
    ) {
        return r;
    }
    if let Some(r) = try_validate_phases_97_100(
        &file,
        json,
        check_resource_health_correlation,
        check_dependency_optimization,
        check_resource_consolidation_opportunities,
        check_resource_compliance_tags,
        check_resource_rollback_coverage,
        check_resource_dependency_balance,
        check_resource_secret_scope,
        check_resource_deprecation_usage,
        check_resource_when_condition_coverage,
        check_resource_dependency_symmetry_deep,
        check_resource_tag_namespace,
        check_resource_machine_capacity,
    ) {
        return r;
    }
    if let Some(r) = try_validate_phases_101_103(
        &file,
        json,
        check_resource_dependency_fan_out_limit,
        check_resource_tag_required_keys,
        check_resource_content_drift_risk,
        check_resource_circular_dependency_depth,
        check_resource_orphan_detection_deep,
        check_resource_provider_diversity,
        check_resource_dependency_isolation,
        check_resource_tag_value_consistency,
        check_resource_machine_distribution_balance,
    )
    .or_else(|| {
        try_validate_phases_104_106(
            &file,
            json,
            check_resource_dependency_version_drift,
            check_resource_naming_length_limit,
            check_resource_type_coverage_per_machine,
            check_resource_dependency_depth_variance,
            check_resource_tag_key_naming,
            check_resource_content_length_limit,
            check_resource_dependency_completeness_audit,
            check_resource_machine_coverage_gap,
            check_resource_path_depth_limit,
        )
    })
    .or_else(|| {
        try_validate_phase107(
            &file,
            json,
            check_resource_dependency_ordering_consistency,
            check_resource_tag_value_format,
            check_resource_provider_version_pinning,
        )
    }) {
        return r;
    }
    if let Some(r) = try_validate_governance_b(
        &file,
        json,
        check_resource_lifecycle_hooks,
        check_resource_provider_version,
        check_resource_naming_convention,
        check_resource_idempotency,
        check_resource_documentation,
        check_resource_ownership,
        check_resource_secret_exposure,
        check_resource_tag_standards,
        check_resource_privilege_escalation,
        check_resource_update_safety,
        check_resource_cross_machine_consistency,
        check_resource_version_pinning,
    )
    .or_else(|| {
        try_validate_advanced(
            &file,
            json,
            check_orphan_resources,
            check_machine_arch,
            check_resource_health_conflicts,
            check_resource_overlap,
            check_resource_tags,
            check_resource_state_consistency,
            check_resource_dependencies_complete,
            check_machine_connectivity,
        )
    })
    .or_else(|| {
        try_validate_structural(
            &file,
            json,
            check_mount_points,
            check_group_consistency,
            check_mode_consistency,
            check_template_vars,
            check_service_deps,
            check_path_conflicts,
            check_owner_consistency,
            check_naming_conventions,
            check_circular_refs,
            check_machine_reachability,
        )
    }) {
        return r;
    }
    if let Some(r) = try_validate_quality(
        &file,
        json,
        check_idempotency_deep,
        check_permissions,
        check_dependencies,
        check_unused,
        check_resource_limits,
        check_portability,
        check_compliance.as_deref(),
        check_drift_risk,
        check_deprecation,
        check_security,
        check_complexity,
        check_limits,
    ) {
        return r;
    }
    if let Some(r) = try_validate_core(
        &file,
        json,
        strict,
        dry_expand,
        check_overlaps,
        check_naming,
        check_cycles_deep,
        check_drift_coverage,
        check_idempotency,
        check_secrets,
        strict_deps,
        check_templates,
        check_connectivity,
        policy_file.as_deref(),
        exhaustive,
    ) {
        return r;
    }
    cmd_validate(&file, strict, json, dry_expand)
}
#[allow(clippy::too_many_arguments)]
fn try_validate_phases_94_96(
    file: &Path,
    json: bool,
    x1: bool,
    x2: bool,
    a1: bool,
    a2: bool,
    a3: bool,
    b1: bool,
    b2: bool,
    b3: bool,
) -> Option<Result<(), String>> {
    if x1 {
        return Some(cmd_validate_check_resource_gpu_backend_consistency(
            file, json,
        ));
    }
    if x2 {
        return Some(cmd_validate_check_resource_when_condition_syntax(
            file, json,
        ));
    }
    if a1 {
        return Some(cmd_validate_check_resource_lifecycle_hook_coverage(
            file, json,
        ));
    }
    if a2 {
        return Some(cmd_validate_check_resource_secret_rotation_age(file, json));
    }
    if a3 {
        return Some(cmd_validate_check_resource_dependency_chain_depth(
            file, json,
        ));
    }
    if b1 {
        return Some(cmd_validate_check_recipe_input_completeness(file, json));
    }
    if b2 {
        return Some(cmd_validate_check_resource_cross_machine_content_duplicates(file, json));
    }
    if b3 {
        return Some(cmd_validate_check_resource_machine_reference_validity(
            file, json,
        ));
    }
    None
}
#[allow(clippy::too_many_arguments)]
fn try_validate_checks_early_a(
    file: &Path,
    json: bool,
    check_cron_syntax: bool,
    check_env_refs: bool,
    check_resource_names: Option<&str>,
    check_resource_count: Option<usize>,
    check_duplicate_paths: bool,
    check_circular_deps: bool,
    check_machine_refs: bool,
    check_provider_consistency: bool,
) -> Option<Result<(), String>> {
    if check_cron_syntax {
        return Some(cmd_validate_check_cron_syntax(file, json));
    }
    if check_env_refs {
        return Some(cmd_validate_check_env_refs(file, json));
    }
    if let Some(pattern) = check_resource_names {
        return Some(cmd_validate_check_resource_names(file, json, pattern));
    }
    if let Some(limit) = check_resource_count {
        return Some(cmd_validate_check_resource_count(file, json, limit));
    }
    if check_duplicate_paths {
        return Some(cmd_validate_check_duplicate_paths(file, json));
    }
    if check_circular_deps {
        return Some(cmd_validate_check_circular_deps(file, json));
    }
    if check_machine_refs {
        return Some(cmd_validate_check_machine_refs(file, json));
    }
    if check_provider_consistency {
        return Some(cmd_validate_check_provider_consistency(file, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
fn try_validate_checks_early_b(
    file: &Path,
    json: bool,
    check_state_values: bool,
    check_unused_machines: bool,
    check_tag_consistency: bool,
    check_dependency_exists: bool,
    check_path_conflicts_strict: bool,
    check_duplicate_names: bool,
    check_resource_groups: bool,
) -> Option<Result<(), String>> {
    if check_state_values {
        return Some(cmd_validate_check_state_values(file, json));
    }
    if check_unused_machines {
        return Some(cmd_validate_check_unused_machines(file, json));
    }
    if check_tag_consistency {
        return Some(cmd_validate_check_tag_consistency(file, json));
    }
    if check_dependency_exists {
        return Some(cmd_validate_check_dependency_exists(file, json));
    }
    if check_path_conflicts_strict {
        return Some(cmd_validate_check_path_conflicts_strict(file, json));
    }
    if check_duplicate_names {
        return Some(cmd_validate_check_duplicate_names(file, json));
    }
    if check_resource_groups {
        return Some(cmd_validate_check_resource_groups(file, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
fn try_validate_phases_97_100(
    file: &Path,
    json: bool,
    a1: bool,
    a2: bool,
    a3: bool,
    b1: bool,
    b2: bool,
    b3: bool,
    c1: bool,
    c2: bool,
    c3: bool,
    d1: bool,
    d2: bool,
    d3: bool,
) -> Option<Result<(), String>> {
    if a1 {
        return Some(cmd_validate_check_resource_health_correlation(file, json));
    }
    if a2 {
        return Some(cmd_validate_check_dependency_optimization(file, json));
    }
    if a3 {
        return Some(cmd_validate_check_resource_consolidation_opportunities(
            file, json,
        ));
    }
    if b1 {
        return Some(cmd_validate_check_resource_compliance_tags(file, json));
    }
    if b2 {
        return Some(cmd_validate_check_resource_rollback_coverage(file, json));
    }
    if b3 {
        return Some(cmd_validate_check_resource_dependency_balance(file, json));
    }
    if c1 {
        return Some(cmd_validate_check_resource_secret_scope(file, json));
    }
    if c2 {
        return Some(cmd_validate_check_resource_deprecation_usage(file, json));
    }
    if c3 {
        return Some(cmd_validate_check_resource_when_condition_coverage(
            file, json,
        ));
    }
    if d1 {
        return Some(cmd_validate_check_resource_dependency_symmetry_deep(
            file, json,
        ));
    }
    if d2 {
        return Some(cmd_validate_check_resource_tag_namespace(file, json));
    }
    if d3 {
        return Some(cmd_validate_check_resource_machine_capacity(file, json));
    }
    None
}
#[allow(clippy::too_many_arguments)]
fn try_validate_phases_101_103(
    file: &Path,
    json: bool,
    e1: bool,
    e2: bool,
    e3: bool,
    f1: bool,
    f2: bool,
    f3: bool,
    g1: bool,
    g2: bool,
    g3: bool,
) -> Option<Result<(), String>> {
    if e1 {
        return Some(cmd_validate_check_resource_dependency_fan_out_limit(
            file, json,
        ));
    }
    if e2 {
        return Some(cmd_validate_check_resource_tag_required_keys(file, json));
    }
    if e3 {
        return Some(cmd_validate_check_resource_content_drift_risk(file, json));
    }
    if f1 {
        return Some(cmd_validate_check_resource_circular_dependency_depth(
            file, json,
        ));
    }
    if f2 {
        return Some(cmd_validate_check_resource_orphan_detection_deep(
            file, json,
        ));
    }
    if f3 {
        return Some(cmd_validate_check_resource_provider_diversity(file, json));
    }
    if g1 {
        return Some(cmd_validate_check_resource_dependency_isolation(file, json));
    }
    if g2 {
        return Some(cmd_validate_check_resource_tag_value_consistency(
            file, json,
        ));
    }
    if g3 {
        return Some(cmd_validate_check_resource_machine_distribution_balance(
            file, json,
        ));
    }
    None
}
#[allow(clippy::too_many_arguments)]
fn try_validate_phases_104_106(
    file: &Path,
    json: bool,
    h1: bool,
    h2: bool,
    h3: bool,
    i1: bool,
    i2: bool,
    i3: bool,
    j1: bool,
    j2: bool,
    j3: bool,
) -> Option<Result<(), String>> {
    if h1 {
        return Some(cmd_validate_check_resource_dependency_version_drift(
            file, json,
        ));
    }
    if h2 {
        return Some(cmd_validate_check_resource_naming_length_limit(file, json));
    }
    if h3 {
        return Some(cmd_validate_check_resource_type_coverage_per_machine(
            file, json,
        ));
    }
    if i1 {
        return Some(cmd_validate_check_resource_dependency_depth_variance(
            file, json,
        ));
    }
    if i2 {
        return Some(cmd_validate_check_resource_tag_key_naming(file, json));
    }
    if i3 {
        return Some(cmd_validate_check_resource_content_length_limit(file, json));
    }
    if j1 {
        return Some(cmd_validate_check_resource_dependency_completeness_audit(
            file, json,
        ));
    }
    if j2 {
        return Some(cmd_validate_check_resource_machine_coverage_gap(file, json));
    }
    if j3 {
        return Some(cmd_validate_check_resource_path_depth_limit(file, json));
    }
    None
}
fn try_validate_phase107(
    file: &Path,
    json: bool,
    k1: bool,
    k2: bool,
    k3: bool,
) -> Option<Result<(), String>> {
    if k1 {
        return Some(cmd_validate_check_resource_dependency_ordering_consistency(
            file, json,
        ));
    }
    if k2 {
        return Some(cmd_validate_check_resource_tag_value_format(file, json));
    }
    if k3 {
        return Some(cmd_validate_check_resource_provider_version_pinning(
            file, json,
        ));
    }
    None
}

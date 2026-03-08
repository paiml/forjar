use super::commands::*;
use super::dispatch_validate::*;
use super::validate_advanced::*;
use super::validate_core::*;
use super::validate_deep::*;
use std::path::Path;

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
        deep,
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
        check_recipe_purity,
        check_reproducibility_score,
        deny_unknown_fields,
    } = args;

    // FJ-2500: Unknown fields are always errors during validate (P0 — silent data loss).
    // The --deny-unknown-fields flag is now the default behavior; kept for backward compat.
    let _ = deny_unknown_fields; // always true for validate
    {
        let content = std::fs::read_to_string(&file)
            .map_err(|e| format!("failed to read {}: {}", file.display(), e))?;
        let unknown_warnings = crate::core::parser::check_unknown_fields(&content);
        if !unknown_warnings.is_empty() {
            return Err(format!(
                "unknown field errors:\n{}",
                unknown_warnings
                    .iter()
                    .map(|e| format!("  - {e}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
    }

    // FJ-2503: --deep runs all deep checks in a single aggregated pass
    if deep {
        return cmd_validate_deep(&file, json);
    }

    if let Some(r) = try_validate_store(
        &file,
        json,
        check_recipe_purity,
        check_reproducibility_score,
    )
    .or_else(|| {
        try_validate_checks_early_a(
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
        )
    })
    .or_else(|| {
        try_validate_checks_early_b(
            &file,
            json,
            check_state_values,
            check_unused_machines,
            check_tag_consistency,
            check_dependency_exists,
            check_path_conflicts_strict,
            check_duplicate_names,
            check_resource_groups,
        )
    }) {
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

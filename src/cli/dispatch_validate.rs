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

/// One `validate --check-*` handler. Every flag-selected check has the same
/// shape: it re-reads the config file and reports its own findings, honouring
/// `--json`.
type ValidateCheck = fn(&Path, bool) -> Result<(), String>;

/// Run the first check whose flag is set, in table order, and return its
/// result; `None` when none of the flags in the table is set.
///
/// Exists because every `try_validate_*` dispatcher below is nothing but a
/// precedence table — first flag wins — previously spelled out as a dozen
/// identical `if flag { return Some(check(file, json)); }` lines each. Order is
/// significant and is the table's order; only the selected check is called.
fn first_enabled_check(
    file: &Path,
    json: bool,
    checks: &[(bool, ValidateCheck)],
) -> Option<Result<(), String>> {
    checks
        .iter()
        .find(|(enabled, _)| *enabled)
        .map(|(_, check)| check(file, json))
}

/// GH-91: `--strict` and `--dry-expand` are accepted by the `validate` CLI but
/// do nothing yet. Warn per flag so a caller is never silently ignored. Kept
/// separate from the dispatch table because these flags select no check.
fn warn_unimplemented_core_flags(strict: bool, dry_expand: bool) {
    if strict {
        eprintln!("Warning: --strict is not yet implemented for validate. Flag ignored.");
    }
    if dry_expand {
        eprintln!("Warning: --dry-expand is not yet implemented for validate. Flag ignored.");
    }
}

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
    first_enabled_check(
        file,
        json,
        &[
            (check_mount_points, cmd_validate_check_mount_points),
            (
                check_group_consistency,
                cmd_validate_check_group_consistency,
            ),
            (check_mode_consistency, cmd_validate_check_mode_consistency),
            (check_template_vars, cmd_validate_check_template_vars),
            (check_service_deps, cmd_validate_check_service_deps),
            (check_path_conflicts, cmd_validate_check_path_conflicts),
            (
                check_owner_consistency,
                cmd_validate_check_owner_consistency,
            ),
            (
                check_naming_conventions,
                cmd_validate_check_naming_conventions,
            ),
            (check_circular_refs, cmd_validate_check_circular_refs),
            (
                check_machine_reachability,
                cmd_validate_check_machine_reachability,
            ),
        ],
    )
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
    if let Some(result) = first_enabled_check(
        file,
        json,
        &[
            (check_idempotency_deep, cmd_validate_check_idempotency_deep),
            (check_permissions, cmd_validate_check_permissions),
            (check_dependencies, cmd_validate_check_dependencies),
            (check_unused, cmd_validate_check_unused),
            (check_resource_limits, cmd_validate_check_resource_limits),
            (check_portability, cmd_validate_check_portability),
        ],
    ) {
        return Some(result);
    }
    if let Some(policy) = check_compliance {
        return Some(cmd_validate_check_compliance(file, policy, json));
    }
    first_enabled_check(
        file,
        json,
        &[
            (check_drift_risk, cmd_validate_check_drift_risk),
            (check_deprecation, cmd_validate_check_deprecation),
            (check_security, cmd_validate_check_security),
            (check_complexity, cmd_validate_check_complexity),
            (check_limits, cmd_validate_check_limits),
        ],
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_validate_core(
    file: &Path,
    json: bool,
    strict: bool,
    dry_expand: bool,
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
    // GH-91: Warn on unimplemented validation flags
    warn_unimplemented_core_flags(strict, dry_expand);
    if let Some(result) = first_enabled_check(
        file,
        json,
        &[
            (check_overlaps, cmd_validate_check_overlaps),
            (check_naming, cmd_validate_check_naming),
            (check_cycles_deep, cmd_validate_check_cycles_deep),
            (check_drift_coverage, cmd_validate_check_drift_coverage),
            (check_idempotency, cmd_validate_check_idempotency),
            (check_secrets, cmd_validate_check_secrets),
            (strict_deps, cmd_validate_strict_deps),
            (check_templates, cmd_validate_check_templates),
            (check_connectivity, cmd_validate_connectivity),
        ],
    ) {
        return Some(result);
    }
    if let Some(pf) = policy_file {
        return Some(cmd_validate_policy_file(file, pf, json));
    }
    first_enabled_check(file, json, &[(exhaustive, cmd_validate_exhaustive)])
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
    if let Some(result) = first_enabled_check(
        file,
        json,
        &[
            (
                check_resource_provider_support,
                cmd_validate_check_resource_provider_support,
            ),
            (
                check_resource_secret_refs,
                cmd_validate_check_resource_secret_refs,
            ),
            (
                check_resource_idempotency_hints,
                cmd_validate_check_resource_idempotency_hints,
            ),
        ],
    ) {
        return Some(result);
    }
    if let Some(depth) = check_resource_dependency_depth {
        return Some(cmd_validate_check_resource_dependency_depth(
            file, json, depth,
        ));
    }
    first_enabled_check(
        file,
        json,
        &[
            (
                check_resource_machine_affinity,
                cmd_validate_check_resource_machine_affinity,
            ),
            (
                check_resource_drift_risk,
                cmd_validate_check_resource_drift_risk,
            ),
            (
                check_resource_tag_coverage,
                cmd_validate_check_resource_tag_coverage,
            ),
        ],
    )
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
    first_enabled_check(
        file,
        json,
        &[
            (
                check_resource_lifecycle_hooks,
                cmd_validate_check_resource_lifecycle_hooks,
            ),
            (
                check_resource_provider_version,
                cmd_validate_check_resource_provider_version,
            ),
            (
                check_resource_naming_convention,
                cmd_validate_check_resource_naming_convention,
            ),
            (
                check_resource_idempotency,
                cmd_validate_check_resource_idempotency,
            ),
            (
                check_resource_documentation,
                cmd_validate_check_resource_documentation,
            ),
            (
                check_resource_ownership,
                cmd_validate_check_resource_ownership,
            ),
            (
                check_resource_secret_exposure,
                cmd_validate_check_resource_secret_exposure,
            ),
            (
                check_resource_tag_standards,
                cmd_validate_check_resource_tag_standards,
            ),
            (
                check_resource_privilege_escalation,
                cmd_validate_check_resource_privilege_escalation,
            ),
            (
                check_resource_update_safety,
                cmd_validate_check_resource_update_safety,
            ),
            (
                check_resource_cross_machine_consistency,
                cmd_validate_check_resource_cross_machine_consistency,
            ),
            (
                check_resource_version_pinning,
                cmd_validate_check_resource_version_pinning,
            ),
        ],
    )
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
    first_enabled_check(
        file,
        json,
        &[
            (
                check_resource_dependency_completeness,
                cmd_validate_check_resource_dependency_completeness,
            ),
            (
                check_resource_state_coverage,
                cmd_validate_check_resource_state_coverage,
            ),
            (
                check_resource_rollback_safety,
                cmd_validate_check_resource_rollback_safety,
            ),
            (
                check_resource_config_maturity,
                cmd_validate_check_resource_config_maturity,
            ),
            (
                check_resource_dependency_ordering,
                cmd_validate_check_resource_dependency_ordering,
            ),
            (
                check_resource_tag_completeness,
                cmd_validate_check_resource_tag_completeness,
            ),
            (
                check_resource_naming_standards,
                cmd_validate_check_resource_naming_standards,
            ),
            (
                check_resource_dependency_symmetry,
                cmd_validate_check_resource_dependency_symmetry,
            ),
            (
                check_resource_circular_alias,
                cmd_validate_check_resource_circular_alias,
            ),
            (
                check_resource_dependency_depth_limit,
                cmd_validate_check_resource_dependency_depth_limit,
            ),
            (
                check_resource_unused_params,
                cmd_validate_check_resource_unused_params,
            ),
            (
                check_resource_machine_balance,
                cmd_validate_check_resource_machine_balance,
            ),
        ],
    )
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
    first_enabled_check(
        file,
        json,
        &[
            (
                check_resource_content_hash_consistency,
                cmd_validate_check_resource_content_hash_consistency,
            ),
            (
                check_resource_dependency_refs,
                cmd_validate_check_resource_dependency_refs,
            ),
            (
                check_resource_trigger_refs,
                cmd_validate_check_resource_trigger_refs,
            ),
            (
                check_resource_param_type_safety,
                cmd_validate_check_resource_param_type_safety,
            ),
            (
                check_resource_env_consistency,
                cmd_validate_check_resource_env_consistency,
            ),
            (
                check_resource_secret_rotation,
                cmd_validate_check_resource_secret_rotation,
            ),
            (
                check_resource_lifecycle_completeness,
                cmd_validate_check_resource_lifecycle_completeness,
            ),
            (
                check_resource_provider_compatibility,
                cmd_validate_check_resource_provider_compatibility,
            ),
            (
                check_resource_naming_convention_strict,
                cmd_validate_check_resource_naming_convention_strict,
            ),
            (
                check_resource_idempotency_annotations,
                cmd_validate_check_resource_idempotency_annotations,
            ),
            (
                check_resource_content_size_limit,
                cmd_validate_check_resource_content_size_limit,
            ),
            (
                check_resource_dependency_fan_limit,
                cmd_validate_check_resource_dependency_fan_limit,
            ),
        ],
    )
}

pub(super) use super::dispatch_validate_b::*;
pub(super) use super::dispatch_validate_c::*;

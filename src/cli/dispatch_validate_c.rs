use super::validate_analytics::*;
use super::validate_audit::*;
use super::validate_compliance_ext::*;
use super::validate_config_quality::*;
use super::validate_governance_ext::*;
use super::validate_hygiene::*;
use super::validate_maturity::*;
use super::validate_ordering_ext::*;
use super::validate_paths::*;
use super::validate_resilience::*;
use super::validate_safety::*;
use super::validate_scoring::*;
use super::validate_security::*;
use super::validate_security_ext::*;
use super::validate_store_purity::*;
use super::validate_topology::*;
use super::validate_transport::*;
use std::path::Path;

/// One `validate --check-*` check. Every flag-selected check has the same
/// shape: it re-reads the config file and reports its own findings, honouring
/// `--json`.
type ValidateCheck = fn(&Path, bool) -> Result<(), String>;

/// Run the first check whose flag is set, in table order, and return its
/// result; `None` when none of the flags in the table is set. Table order is
/// the precedence order, and only the selected check is called.
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

#[allow(clippy::too_many_arguments)]
pub(super) fn try_validate_phases_94_96(
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
pub(super) fn try_validate_checks_early_a(
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
pub(super) fn try_validate_checks_early_b(
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
pub(super) fn try_validate_phases_97_100(
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
    first_enabled_check(
        file,
        json,
        &[
            (a1, cmd_validate_check_resource_health_correlation),
            (a2, cmd_validate_check_dependency_optimization),
            (a3, cmd_validate_check_resource_consolidation_opportunities),
            (b1, cmd_validate_check_resource_compliance_tags),
            (b2, cmd_validate_check_resource_rollback_coverage),
            (b3, cmd_validate_check_resource_dependency_balance),
            (c1, cmd_validate_check_resource_secret_scope),
            (c2, cmd_validate_check_resource_deprecation_usage),
            (c3, cmd_validate_check_resource_when_condition_coverage),
            (d1, cmd_validate_check_resource_dependency_symmetry_deep),
            (d2, cmd_validate_check_resource_tag_namespace),
            (d3, cmd_validate_check_resource_machine_capacity),
        ],
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_validate_phases_101_103(
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
    first_enabled_check(
        file,
        json,
        &[
            (e1, cmd_validate_check_resource_dependency_fan_out_limit),
            (e2, cmd_validate_check_resource_tag_required_keys),
            (e3, cmd_validate_check_resource_content_drift_risk),
            (f1, cmd_validate_check_resource_circular_dependency_depth),
            (f2, cmd_validate_check_resource_orphan_detection_deep),
            (f3, cmd_validate_check_resource_provider_diversity),
            (g1, cmd_validate_check_resource_dependency_isolation),
            (g2, cmd_validate_check_resource_tag_value_consistency),
            (g3, cmd_validate_check_resource_machine_distribution_balance),
        ],
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn try_validate_phases_104_106(
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
    first_enabled_check(
        file,
        json,
        &[
            (h1, cmd_validate_check_resource_dependency_version_drift),
            (h2, cmd_validate_check_resource_naming_length_limit),
            (h3, cmd_validate_check_resource_type_coverage_per_machine),
            (i1, cmd_validate_check_resource_dependency_depth_variance),
            (i2, cmd_validate_check_resource_tag_key_naming),
            (i3, cmd_validate_check_resource_content_length_limit),
            (
                j1,
                cmd_validate_check_resource_dependency_completeness_audit,
            ),
            (j2, cmd_validate_check_resource_machine_coverage_gap),
            (j3, cmd_validate_check_resource_path_depth_limit),
        ],
    )
}
pub(super) fn try_validate_phase107(
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
pub(super) fn try_validate_store(
    file: &Path,
    json: bool,
    check_recipe_purity: bool,
    check_reproducibility_score: bool,
    min_purity: Option<&str>,
) -> Option<Result<(), String>> {
    if check_recipe_purity {
        return Some(cmd_validate_check_recipe_purity(file, json, min_purity));
    }
    if check_reproducibility_score {
        return Some(cmd_validate_check_reproducibility_score(file, json));
    }
    None
}

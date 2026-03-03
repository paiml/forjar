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
) -> Option<Result<(), String>> {
    if check_recipe_purity {
        return Some(cmd_validate_check_recipe_purity(file, json));
    }
    if check_reproducibility_score {
        return Some(cmd_validate_check_reproducibility_score(file, json));
    }
    None
}

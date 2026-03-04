//! Coverage tests for dispatch_validate.rs — exercises all validate dispatch routes.

#![allow(unused_imports)]
use super::dispatch_validate::*;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_cfg(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    const CFG: &str = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    state: present\n    tags:\n      - web\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: hi\n    state: present\n    depends_on:\n      - pkg\n    tags:\n      - web\n";

    // try_validate_structural — each flag
    #[test]
    fn test_structural_mount_points() {
        let f = write_cfg(CFG);
        assert!(try_validate_structural(f.path(), false, true, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_structural_group_consistency() {
        let f = write_cfg(CFG);
        assert!(try_validate_structural(f.path(), false, false, true, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_structural_mode_consistency() {
        let f = write_cfg(CFG);
        assert!(try_validate_structural(f.path(), false, false, false, true, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_structural_template_vars() {
        let f = write_cfg(CFG);
        assert!(try_validate_structural(f.path(), false, false, false, false, true, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_structural_service_deps() {
        let f = write_cfg(CFG);
        assert!(try_validate_structural(f.path(), false, false, false, false, false, true, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_structural_path_conflicts() {
        let f = write_cfg(CFG);
        assert!(try_validate_structural(f.path(), false, false, false, false, false, false, true, false, false, false, false).is_some());
    }
    #[test]
    fn test_structural_owner_consistency() {
        let f = write_cfg(CFG);
        assert!(try_validate_structural(f.path(), false, false, false, false, false, false, false, true, false, false, false).is_some());
    }
    #[test]
    fn test_structural_naming_conventions() {
        let f = write_cfg(CFG);
        assert!(try_validate_structural(f.path(), false, false, false, false, false, false, false, false, true, false, false).is_some());
    }
    #[test]
    fn test_structural_circular_refs() {
        let f = write_cfg(CFG);
        assert!(try_validate_structural(f.path(), false, false, false, false, false, false, false, false, false, true, false).is_some());
    }
    #[test]
    fn test_structural_machine_reachability() {
        let f = write_cfg(CFG);
        assert!(try_validate_structural(f.path(), false, false, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_structural_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_structural(f.path(), false, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_validate_quality — each flag
    #[test]
    fn test_quality_idempotency_deep() {
        let f = write_cfg(CFG);
        assert!(try_validate_quality(f.path(), false, true, false, false, false, false, false, None, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_quality_permissions() {
        let f = write_cfg(CFG);
        assert!(try_validate_quality(f.path(), false, false, true, false, false, false, false, None, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_quality_dependencies() {
        let f = write_cfg(CFG);
        assert!(try_validate_quality(f.path(), false, false, false, true, false, false, false, None, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_quality_unused() {
        let f = write_cfg(CFG);
        assert!(try_validate_quality(f.path(), false, false, false, false, true, false, false, None, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_quality_resource_limits() {
        let f = write_cfg(CFG);
        assert!(try_validate_quality(f.path(), false, false, false, false, false, true, false, None, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_quality_portability() {
        let f = write_cfg(CFG);
        assert!(try_validate_quality(f.path(), false, false, false, false, false, false, true, None, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_quality_compliance() {
        let f = write_cfg(CFG);
        assert!(try_validate_quality(f.path(), false, false, false, false, false, false, false, Some("soc2"), false, false, false, false, false).is_some());
    }
    #[test]
    fn test_quality_drift_risk() {
        let f = write_cfg(CFG);
        assert!(try_validate_quality(f.path(), false, false, false, false, false, false, false, None, true, false, false, false, false).is_some());
    }
    #[test]
    fn test_quality_deprecation() {
        let f = write_cfg(CFG);
        assert!(try_validate_quality(f.path(), false, false, false, false, false, false, false, None, false, true, false, false, false).is_some());
    }
    #[test]
    fn test_quality_security() {
        let f = write_cfg(CFG);
        assert!(try_validate_quality(f.path(), false, false, false, false, false, false, false, None, false, false, true, false, false).is_some());
    }
    #[test]
    fn test_quality_complexity() {
        let f = write_cfg(CFG);
        assert!(try_validate_quality(f.path(), false, false, false, false, false, false, false, None, false, false, false, true, false).is_some());
    }
    #[test]
    fn test_quality_limits() {
        let f = write_cfg(CFG);
        assert!(try_validate_quality(f.path(), false, false, false, false, false, false, false, None, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_quality_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_quality(f.path(), false, false, false, false, false, false, false, None, false, false, false, false, false).is_none());
    }

    // try_validate_core — 15 args: file, json, strict, dry_expand, check_overlaps..exhaustive + policy_file
    #[test]
    fn test_core_overlaps() {
        let f = write_cfg(CFG);
        assert!(try_validate_core(f.path(), false, false, false, true, false, false, false, false, false, false, false, false, None, false).is_some());
    }
    #[test]
    fn test_core_naming() {
        let f = write_cfg(CFG);
        assert!(try_validate_core(f.path(), false, false, false, false, true, false, false, false, false, false, false, false, None, false).is_some());
    }
    #[test]
    fn test_core_cycles_deep() {
        let f = write_cfg(CFG);
        assert!(try_validate_core(f.path(), false, false, false, false, false, true, false, false, false, false, false, false, None, false).is_some());
    }
    #[test]
    fn test_core_drift_coverage() {
        let f = write_cfg(CFG);
        assert!(try_validate_core(f.path(), false, false, false, false, false, false, true, false, false, false, false, false, None, false).is_some());
    }
    #[test]
    fn test_core_idempotency() {
        let f = write_cfg(CFG);
        assert!(try_validate_core(f.path(), false, false, false, false, false, false, false, true, false, false, false, false, None, false).is_some());
    }
    #[test]
    fn test_core_secrets() {
        let f = write_cfg(CFG);
        assert!(try_validate_core(f.path(), false, false, false, false, false, false, false, false, true, false, false, false, None, false).is_some());
    }
    #[test]
    fn test_core_strict_deps() {
        let f = write_cfg(CFG);
        assert!(try_validate_core(f.path(), false, false, false, false, false, false, false, false, false, true, false, false, None, false).is_some());
    }
    #[test]
    fn test_core_templates() {
        let f = write_cfg(CFG);
        assert!(try_validate_core(f.path(), false, false, false, false, false, false, false, false, false, false, true, false, None, false).is_some());
    }
    #[test]
    fn test_core_connectivity() {
        let f = write_cfg(CFG);
        assert!(try_validate_core(f.path(), false, false, false, false, false, false, false, false, false, false, false, true, None, false).is_some());
    }
    #[test]
    fn test_core_exhaustive() {
        let f = write_cfg(CFG);
        assert!(try_validate_core(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, None, true).is_some());
    }
    #[test]
    fn test_core_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_core(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, None, false).is_none());
    }

    // try_validate_governance — &Option<String>, bools, Option<usize>, bools
    #[test]
    fn test_governance_naming_pattern() {
        let f = write_cfg(CFG);
        let pat = Some("web".to_string());
        assert!(try_validate_governance(f.path(), false, &pat, false, false, false, None, false, false, false).is_some());
    }
    #[test]
    fn test_governance_provider_support() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance(f.path(), false, &None, true, false, false, None, false, false, false).is_some());
    }
    #[test]
    fn test_governance_secret_refs() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance(f.path(), false, &None, false, true, false, None, false, false, false).is_some());
    }
    #[test]
    fn test_governance_idempotency_hints() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance(f.path(), false, &None, false, false, true, None, false, false, false).is_some());
    }
    #[test]
    fn test_governance_dep_depth() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance(f.path(), false, &None, false, false, false, Some(5), false, false, false).is_some());
    }
    #[test]
    fn test_governance_machine_affinity() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance(f.path(), false, &None, false, false, false, None, true, false, false).is_some());
    }
    #[test]
    fn test_governance_drift_risk() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance(f.path(), false, &None, false, false, false, None, false, true, false).is_some());
    }
    #[test]
    fn test_governance_tag_coverage() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance(f.path(), false, &None, false, false, false, None, false, false, true).is_some());
    }
    #[test]
    fn test_governance_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance(f.path(), false, &None, false, false, false, None, false, false, false).is_none());
    }

    // try_validate_governance_b — 13 bools
    #[test]
    fn test_governance_b_lifecycle_hooks() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_b(f.path(), false, true, false, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_b_provider_version() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_b(f.path(), false, false, true, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_b_naming_convention() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_b(f.path(), false, false, false, true, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_b_idempotency() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_b(f.path(), false, false, false, false, true, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_b_documentation() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_b(f.path(), false, false, false, false, false, true, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_b_ownership() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_b(f.path(), false, false, false, false, false, false, true, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_b_secret_exposure() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_b(f.path(), false, false, false, false, false, false, false, true, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_b_tag_standards() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_b(f.path(), false, false, false, false, false, false, false, false, true, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_b_priv_escalation() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_b(f.path(), false, false, false, false, false, false, false, false, false, true, false, false, false).is_some());
    }
    #[test]
    fn test_governance_b_update_safety() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_b(f.path(), false, false, false, false, false, false, false, false, false, false, true, false, false).is_some());
    }
    #[test]
    fn test_governance_b_cross_machine() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_b(f.path(), false, false, false, false, false, false, false, false, false, false, false, true, false).is_some());
    }
    #[test]
    fn test_governance_b_version_pinning() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_b(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_governance_b_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_b(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_validate_governance_c — 12 bools
    #[test]
    fn test_governance_c_dep_completeness() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_c(f.path(), false, true, false, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_c_state_coverage() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_c(f.path(), false, false, true, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_c_rollback_safety() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_c(f.path(), false, false, false, true, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_c_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_c(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_validate_governance_d — 12 bools
    #[test]
    fn test_governance_d_content_hash() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_d(f.path(), false, true, false, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_d_dep_refs() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_d(f.path(), false, false, true, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_d_trigger_refs() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_d(f.path(), false, false, false, true, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_d_param_type() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_d(f.path(), false, false, false, false, true, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_d_env_consistency() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_d(f.path(), false, false, false, false, false, true, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_d_secret_rotation() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_d(f.path(), false, false, false, false, false, false, true, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_d_lifecycle() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_d(f.path(), false, false, false, false, false, false, false, true, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_d_provider_compat() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_d(f.path(), false, false, false, false, false, false, false, false, true, false, false, false, false).is_some());
    }
    #[test]
    fn test_governance_d_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_governance_d(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
    }
}

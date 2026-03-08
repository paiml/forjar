//! Coverage tests for validate_structural, validate_compliance, validate_quality, validate_policy.

use super::validate_compliance::*;
use super::validate_policy::*;
use super::validate_quality::*;
use super::validate_structural::*;
use super::validate_structural_constraints::*;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    fn basic_config() -> String {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n    owner: root\n    group: root\n    mode: \"0644\"\n  b:\n    type: service\n    machine: m\n    name: nginx\n    depends_on: [a]\n  c:\n    type: package\n    machine: m\n    provider: apt\n    packages: [curl]\n".to_string()
    }

    // ── validate_structural: cmd_validate_check_templates ──

    #[test]
    fn test_check_templates_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_templates(f.path(), false);
    }

    #[test]
    fn test_check_templates_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_templates(f.path(), true);
    }

    // ── validate_structural: cmd_validate_check_secrets ──

    #[test]
    fn test_check_secrets_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_secrets(f.path(), false);
    }

    #[test]
    fn test_check_secrets_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_secrets(f.path(), true);
    }

    // ── validate_structural: cmd_validate_check_cycles_deep ──

    #[test]
    fn test_check_cycles_deep_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_cycles_deep(f.path(), false);
    }

    #[test]
    fn test_check_cycles_deep_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_cycles_deep(f.path(), true);
    }

    // ── validate_structural: cmd_validate_check_naming ──

    #[test]
    fn test_check_naming_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_naming(f.path(), false);
    }

    #[test]
    fn test_check_naming_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_naming(f.path(), true);
    }

    // ── validate_structural: cmd_validate_check_overlaps ──

    #[test]
    fn test_check_overlaps_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_overlaps(f.path(), false);
    }

    #[test]
    fn test_check_overlaps_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_overlaps(f.path(), true);
    }

    // ── validate_structural: cmd_validate_check_limits ──

    #[test]
    fn test_check_limits_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_limits(f.path(), false);
    }

    #[test]
    fn test_check_limits_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_limits(f.path(), true);
    }

    // ── validate_structural: cmd_validate_check_circular_refs ──

    #[test]
    fn test_check_circular_refs_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_circular_refs(f.path(), false);
    }

    #[test]
    fn test_check_circular_refs_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_circular_refs(f.path(), true);
    }

    // ── validate_structural: cmd_validate_check_naming_conventions ──

    #[test]
    fn test_check_naming_conventions_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_naming_conventions(f.path(), false);
    }

    #[test]
    fn test_check_naming_conventions_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_naming_conventions(f.path(), true);
    }

    // ── validate_compliance: cmd_validate_check_drift_risk ──

    #[test]
    fn test_check_drift_risk_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_drift_risk(f.path(), false);
    }

    #[test]
    fn test_check_drift_risk_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_drift_risk(f.path(), true);
    }

    // ── validate_compliance: cmd_validate_check_compliance ──

    #[test]
    fn test_check_compliance_cis_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_compliance(f.path(), "CIS", false);
    }

    #[test]
    fn test_check_compliance_cis_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_compliance(f.path(), "CIS", true);
    }

    // ── validate_compliance: cmd_validate_check_portability ──

    #[test]
    fn test_check_portability_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_portability(f.path(), false);
    }

    #[test]
    fn test_check_portability_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_portability(f.path(), true);
    }

    // ── validate_compliance: cmd_validate_check_idempotency_deep ──

    #[test]
    fn test_check_idempotency_deep_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_idempotency_deep(f.path(), false);
    }

    #[test]
    fn test_check_idempotency_deep_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_idempotency_deep(f.path(), true);
    }

    // ── validate_quality: cmd_validate_check_idempotency ──

    #[test]
    fn test_check_idempotency_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_idempotency(f.path(), false);
    }

    #[test]
    fn test_check_idempotency_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_idempotency(f.path(), true);
    }

    // ── validate_quality: cmd_validate_check_drift_coverage ──

    #[test]
    fn test_check_drift_coverage_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_drift_coverage(f.path(), false);
    }

    #[test]
    fn test_check_drift_coverage_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_drift_coverage(f.path(), true);
    }

    // ── validate_quality: cmd_validate_check_complexity ──

    #[test]
    fn test_check_complexity_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_complexity(f.path(), false);
    }

    #[test]
    fn test_check_complexity_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_complexity(f.path(), true);
    }

    // ── validate_quality: cmd_validate_check_security ──

    #[test]
    fn test_check_security_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_security(f.path(), false);
    }

    #[test]
    fn test_check_security_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_security(f.path(), true);
    }

    // ── validate_quality: cmd_validate_check_deprecation ──

    #[test]
    fn test_check_deprecation_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_deprecation(f.path(), false);
    }

    #[test]
    fn test_check_deprecation_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_check_deprecation(f.path(), true);
    }

    // ── validate_policy: cmd_validate_policy_file ──

    #[test]
    fn test_policy_file_plain() {
        let f = write_temp_config(&basic_config());
        let mut pf = tempfile::NamedTempFile::new().unwrap();
        pf.write_all(b"rules:\n  - name: r1\n    check: no_root_owner\n")
            .unwrap();
        pf.flush().unwrap();
        let _ = cmd_validate_policy_file(f.path(), pf.path(), false);
    }

    #[test]
    fn test_policy_file_json() {
        let f = write_temp_config(&basic_config());
        let mut pf = tempfile::NamedTempFile::new().unwrap();
        pf.write_all(b"rules:\n  - name: r1\n    check: require_tags\n")
            .unwrap();
        pf.flush().unwrap();
        let _ = cmd_validate_policy_file(f.path(), pf.path(), true);
    }

    // ── validate_policy: cmd_validate_connectivity ──

    #[test]
    fn test_connectivity_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_connectivity(f.path(), false);
    }

    #[test]
    fn test_connectivity_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_connectivity(f.path(), true);
    }

    // ── validate_policy: cmd_validate_strict_deps ──

    #[test]
    fn test_strict_deps_plain() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_strict_deps(f.path(), false);
    }

    #[test]
    fn test_strict_deps_json() {
        let f = write_temp_config(&basic_config());
        let _ = cmd_validate_strict_deps(f.path(), true);
    }
}

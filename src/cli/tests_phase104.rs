//! Tests: Phase 104 — Operational Maturity & Dependency Governance (FJ-1093→FJ-1100).

use super::graph_governance::*;
use super::status_maturity::*;
use super::validate_maturity::*;
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

    fn write_yaml(dir: &std::path::Path, name: &str, content: &str) {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
    }

    const LOCK: &str = "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  svc:\n    type: service\n    status: drifted\n    hash: \"blake3:def\"\n";

    // ── FJ-1093: status --fleet-resource-maturity-index ──
    #[test]
    fn test_fj1093_maturity_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1093_maturity_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1093_maturity_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_maturity_index(d.path(), None, true).is_ok());
    }

    // ── FJ-1094: validate --check-resource-dependency-version-drift ──
    #[test]
    fn test_fj1094_version_drift_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_version_drift(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1094_version_drift_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_dependency_version_drift(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1094_version_drift_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_version_drift(f.path(), true).is_ok());
    }

    // ── FJ-1095: graph --resource-dependency-change-impact-radius ──
    #[test]
    fn test_fj1095_impact_radius_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_change_impact_radius(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1095_impact_radius_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_graph_resource_dependency_change_impact_radius(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1095_impact_radius_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_change_impact_radius(f.path(), true).is_ok());
    }

    // ── FJ-1096: status --machine-resource-convergence-stability-index ──
    #[test]
    fn test_fj1096_stability_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(
            cmd_status_machine_resource_convergence_stability_index(d.path(), None, false).is_ok()
        );
    }
    #[test]
    fn test_fj1096_stability_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(
            cmd_status_machine_resource_convergence_stability_index(d.path(), None, false).is_ok()
        );
    }
    #[test]
    fn test_fj1096_stability_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(
            cmd_status_machine_resource_convergence_stability_index(d.path(), None, true).is_ok()
        );
    }

    // ── FJ-1097: validate --check-resource-naming-length-limit ──
    #[test]
    fn test_fj1097_naming_length_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_naming_length_limit(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1097_naming_length_with_resources() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  very-long-resource-name-that-exceeds-reasonable-limits:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n");
        assert!(cmd_validate_check_resource_naming_length_limit(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1097_naming_length_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_naming_length_limit(f.path(), true).is_ok());
    }

    // ── FJ-1098: graph --resource-dependency-sibling-analysis ──
    #[test]
    fn test_fj1098_sibling_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_sibling_analysis(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1098_sibling_with_shared_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a]\n");
        assert!(cmd_graph_resource_dependency_sibling_analysis(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1098_sibling_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_sibling_analysis(f.path(), true).is_ok());
    }

    // ── FJ-1099: status --fleet-resource-drift-pattern-analysis ──
    #[test]
    fn test_fj1099_drift_pattern_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1099_drift_pattern_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1099_drift_pattern_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_pattern_analysis(d.path(), None, true).is_ok());
    }

    // ── FJ-1100: validate --check-resource-type-coverage-per-machine ──
    #[test]
    fn test_fj1100_type_coverage_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_type_coverage_per_machine(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1100_type_coverage_multi_machine() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\n  m2:\n    hostname: m2\n    addr: 127.0.0.2\nresources:\n  a:\n    type: file\n    machine: m1\n    path: /tmp/a\n    content: a\n  b:\n    type: service\n    machine: m1\n    service_name: b\n  c:\n    type: file\n    machine: m2\n    path: /tmp/c\n    content: c\n");
        assert!(cmd_validate_check_resource_type_coverage_per_machine(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1100_type_coverage_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_type_coverage_per_machine(f.path(), true).is_ok());
    }

    // ── File-not-found error paths ──
    #[test]
    fn test_fj1094_file_not_found() {
        assert!(cmd_validate_check_resource_dependency_version_drift(
            std::path::Path::new("/x"),
            false
        )
        .is_err());
    }
    #[test]
    fn test_fj1095_file_not_found() {
        assert!(cmd_graph_resource_dependency_change_impact_radius(
            std::path::Path::new("/x"),
            false
        )
        .is_err());
    }
    #[test]
    fn test_fj1097_file_not_found() {
        assert!(
            cmd_validate_check_resource_naming_length_limit(std::path::Path::new("/x"), false)
                .is_err()
        );
    }
    #[test]
    fn test_fj1098_file_not_found() {
        assert!(
            cmd_graph_resource_dependency_sibling_analysis(std::path::Path::new("/x"), false)
                .is_err()
        );
    }
    #[test]
    fn test_fj1100_file_not_found() {
        assert!(cmd_validate_check_resource_type_coverage_per_machine(
            std::path::Path::new("/x"),
            false
        )
        .is_err());
    }
}

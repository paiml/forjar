//! Tests: Phase 106 — Dependency Intelligence & Fleet Configuration (FJ-1109→FJ-1116).

use super::graph_weight::*;
use super::status_drift_intel2::*;
use super::validate_audit::*;
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

    // ── FJ-1109: status --fleet-resource-type-drift-correlation ──
    #[test]
    fn test_fj1109_drift_correlation_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_drift_correlation(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1109_drift_correlation_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_type_drift_correlation(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1109_drift_correlation_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_drift_correlation(d.path(), None, true).is_ok());
    }

    // ── FJ-1110: validate --check-resource-dependency-completeness-audit ──
    #[test]
    fn test_fj1110_completeness_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_completeness_audit(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1110_completeness_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_validate_check_resource_dependency_completeness_audit(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1110_completeness_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_completeness_audit(f.path(), true).is_ok());
    }

    // ── FJ-1111: graph --resource-dependency-weight-analysis ──
    #[test]
    fn test_fj1111_weight_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_weight_analysis(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1111_weight_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a]\n");
        assert!(cmd_graph_resource_dependency_weight_analysis(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1111_weight_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_weight_analysis(f.path(), true).is_ok());
    }

    // ── FJ-1112: status --machine-resource-apply-cadence-report ──
    #[test]
    fn test_fj1112_cadence_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_apply_cadence_report(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1112_cadence_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_machine_resource_apply_cadence_report(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1112_cadence_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_apply_cadence_report(d.path(), None, true).is_ok());
    }

    // ── FJ-1113: validate --check-resource-machine-coverage-gap ──
    #[test]
    fn test_fj1113_coverage_gap_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_machine_coverage_gap(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1113_coverage_gap_multi_machine() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\n  m2:\n    hostname: m2\n    addr: 127.0.0.2\nresources:\n  a:\n    type: file\n    machine: m1\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_validate_check_resource_machine_coverage_gap(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1113_coverage_gap_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_machine_coverage_gap(f.path(), true).is_ok());
    }

    // ── FJ-1114: graph --resource-dependency-topological-summary ──
    #[test]
    fn test_fj1114_topo_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_topological_summary(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1114_topo_with_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_topological_summary(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1114_topo_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_topological_summary(f.path(), true).is_ok());
    }

    // ── FJ-1115: status --fleet-resource-drift-recovery-trend ──
    #[test]
    fn test_fj1115_recovery_trend_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_recovery_trend(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1115_recovery_trend_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_drift_recovery_trend(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1115_recovery_trend_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_recovery_trend(d.path(), None, true).is_ok());
    }

    // ── FJ-1116: validate --check-resource-path-depth-limit ──
    #[test]
    fn test_fj1116_path_depth_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_path_depth_limit(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1116_path_depth_deep() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /a/b/c/d/e/f/g/h/i/j/k/l/m.txt\n    content: deep\n");
        assert!(cmd_validate_check_resource_path_depth_limit(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1116_path_depth_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_path_depth_limit(f.path(), true).is_ok());
    }

    // ── File-not-found error paths ──
    #[test]
    fn test_fj1110_file_not_found() {
        assert!(cmd_validate_check_resource_dependency_completeness_audit(
            std::path::Path::new("/x"),
            false
        )
        .is_err());
    }
    #[test]
    fn test_fj1111_file_not_found() {
        assert!(
            cmd_graph_resource_dependency_weight_analysis(std::path::Path::new("/x"), false)
                .is_err()
        );
    }
    #[test]
    fn test_fj1113_file_not_found() {
        assert!(cmd_validate_check_resource_machine_coverage_gap(
            std::path::Path::new("/x"),
            false
        )
        .is_err());
    }
    #[test]
    fn test_fj1114_file_not_found() {
        assert!(cmd_graph_resource_dependency_topological_summary(
            std::path::Path::new("/x"),
            false
        )
        .is_err());
    }
    #[test]
    fn test_fj1116_file_not_found() {
        assert!(
            cmd_validate_check_resource_path_depth_limit(std::path::Path::new("/x"), false)
                .is_err()
        );
    }
}

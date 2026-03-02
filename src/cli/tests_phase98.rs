//! Tests: Phase 98 — Compliance Automation & Drift Intelligence (FJ-1045→FJ-1052).

use super::graph_compliance::*;
use super::status_drift_intel::*;
use super::validate_compliance_ext::*;
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

    fn write_yaml(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
        p
    }

    // ── FJ-1045: status --fleet-drift-velocity-trend ──

    #[test]
    fn test_fj1045_drift_velocity_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_drift_velocity_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1045_drift_velocity_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: drifted\n    hash: \"blake3:abc\"\n");
        assert!(cmd_status_fleet_drift_velocity_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1045_drift_velocity_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_drift_velocity_trend(dir.path(), None, true).is_ok());
    }

    #[test]
    fn test_fj1045_drift_velocity_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: drifted\n    hash: \"blake3:abc\"\n");
        assert!(cmd_status_fleet_drift_velocity_trend(dir.path(), Some("web"), false).is_ok());
    }

    // ── FJ-1046: validate --check-resource-compliance-tags ──

    #[test]
    fn test_fj1046_compliance_tags_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_compliance_tags(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1046_compliance_tags_with_resources() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    tags: [pci-compliant]\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n");
        assert!(cmd_validate_check_resource_compliance_tags(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1046_compliance_tags_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_compliance_tags(f.path(), true).is_ok());
    }

    // ── FJ-1047: graph --resource-dependency-risk-score ──

    #[test]
    fn test_fj1047_risk_score_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_risk_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1047_risk_score_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: package\n    machine: m\n    packages: [curl]\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: service\n    machine: m\n    name: svc\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_risk_score(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1047_risk_score_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_risk_score(f.path(), true).is_ok());
    }

    // ── FJ-1048: status --machine-convergence-window ──

    #[test]
    fn test_fj1048_convergence_window_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_convergence_window(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1048_convergence_window_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: drifted\n    hash: \"blake3:def\"\n");
        assert!(cmd_status_machine_convergence_window(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1048_convergence_window_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_convergence_window(dir.path(), None, true).is_ok());
    }

    #[test]
    fn test_fj1048_convergence_window_filtered() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "db/state.lock.yaml", "schema: \"1.0\"\nmachine: db\nhostname: db\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  p:\n    type: package\n    status: converged\n    hash: \"blake3:abc\"\n");
        assert!(cmd_status_machine_convergence_window(dir.path(), Some("db"), false).is_ok());
    }

    // ── FJ-1049: validate --check-resource-rollback-coverage ──

    #[test]
    fn test_fj1049_rollback_coverage_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_rollback_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1049_rollback_coverage_with_resources() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: service\n    machine: m\n    name: svc\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_rollback_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1049_rollback_coverage_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_rollback_coverage(f.path(), true).is_ok());
    }

    // ── FJ-1050: graph --resource-dependency-layering ──

    #[test]
    fn test_fj1050_layering_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_layering(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1050_layering_mixed_types() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m\n    packages: [curl]\n  cfg:\n    type: file\n    machine: m\n    path: /tmp/cfg\n    content: c\n    depends_on: [pkg]\n  svc:\n    type: service\n    machine: m\n    name: svc\n    depends_on: [cfg]\n  cron:\n    type: cron\n    machine: m\n    schedule: \"0 * * * *\"\n    command: echo hi\n    depends_on: [svc]\n");
        assert!(cmd_graph_resource_dependency_layering(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1050_layering_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_layering(f.path(), true).is_ok());
    }

    // ── FJ-1051: status --fleet-resource-age-histogram ──

    #[test]
    fn test_fj1051_age_histogram_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_age_histogram(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1051_age_histogram_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n    applied_at: \"2026-02-27T00:00:00Z\"\n");
        assert!(cmd_status_fleet_resource_age_histogram(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1051_age_histogram_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_age_histogram(dir.path(), None, true).is_ok());
    }

    // ── FJ-1052: validate --check-resource-dependency-balance ──

    #[test]
    fn test_fj1052_dependency_balance_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_balance(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1052_dependency_balance_skewed() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  root:\n    type: package\n    machine: m\n    packages: [curl]\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    depends_on: [root]\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [root]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [root]\n  d:\n    type: file\n    machine: m\n    path: /tmp/d\n    content: d\n    depends_on: [root]\n  e:\n    type: file\n    machine: m\n    path: /tmp/e\n    content: e\n    depends_on: [root]\n");
        assert!(cmd_validate_check_resource_dependency_balance(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1052_dependency_balance_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_balance(f.path(), true).is_ok());
    }

    // ── File-not-found error paths ──

    #[test]
    fn test_fj1046_file_not_found() {
        assert!(cmd_validate_check_resource_compliance_tags(
            std::path::Path::new("/nonexistent"),
            false
        )
        .is_err());
    }

    #[test]
    fn test_fj1047_file_not_found() {
        assert!(cmd_graph_resource_dependency_risk_score(
            std::path::Path::new("/nonexistent"),
            false
        )
        .is_err());
    }

    #[test]
    fn test_fj1049_file_not_found() {
        assert!(cmd_validate_check_resource_rollback_coverage(
            std::path::Path::new("/nonexistent"),
            false
        )
        .is_err());
    }

    #[test]
    fn test_fj1050_file_not_found() {
        assert!(cmd_graph_resource_dependency_layering(
            std::path::Path::new("/nonexistent"),
            false
        )
        .is_err());
    }

    #[test]
    fn test_fj1052_file_not_found() {
        assert!(cmd_validate_check_resource_dependency_balance(
            std::path::Path::new("/nonexistent"),
            false
        )
        .is_err());
    }
}

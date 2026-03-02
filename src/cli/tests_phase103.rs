//! Tests: Phase 103 — Fleet Analytics & Configuration Quality (FJ-1085→FJ-1092).

use super::graph_analytics_ext::*;
use super::status_analytics::*;
use super::validate_config_quality::*;
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

    // ── FJ-1085: status --fleet-resource-error-rate-trend ──
    #[test]
    fn test_fj1085_error_rate_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_error_rate_trend(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1085_error_rate_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_error_rate_trend(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1085_error_rate_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_error_rate_trend(d.path(), None, true).is_ok());
    }

    // ── FJ-1086: validate --check-resource-dependency-isolation ──
    #[test]
    fn test_fj1086_isolation_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_isolation(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1086_isolation_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    tags: [\"env:prod\"]\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    tags: [\"env:staging\"]\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_dependency_isolation(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1086_isolation_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_isolation(f.path(), true).is_ok());
    }

    // ── FJ-1087: graph --resource-dependency-depth-histogram ──
    #[test]
    fn test_fj1087_depth_hist_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_depth_histogram(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1087_depth_hist_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_graph_resource_dependency_depth_histogram(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1087_depth_hist_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_depth_histogram(f.path(), true).is_ok());
    }

    // ── FJ-1088: status --machine-resource-drift-recovery-time ──
    #[test]
    fn test_fj1088_drift_recovery_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_recovery_time(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1088_drift_recovery_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_machine_resource_drift_recovery_time(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1088_drift_recovery_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_recovery_time(d.path(), None, true).is_ok());
    }

    // ── FJ-1089: validate --check-resource-tag-value-consistency ──
    #[test]
    fn test_fj1089_tag_consistency_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_tag_value_consistency(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1089_tag_consistency_mixed() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    tags: [prod]\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    tags: [staging]\n");
        assert!(cmd_validate_check_resource_tag_value_consistency(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1089_tag_consistency_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_tag_value_consistency(f.path(), true).is_ok());
    }

    // ── FJ-1090: graph --resource-dependency-redundancy-analysis ──
    #[test]
    fn test_fj1090_redundancy_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_redundancy_analysis(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1090_redundancy_with_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a, b]\n");
        assert!(cmd_graph_resource_dependency_redundancy_analysis(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1090_redundancy_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_redundancy_analysis(f.path(), true).is_ok());
    }

    // ── FJ-1091: status --fleet-resource-config-complexity-score ──
    #[test]
    fn test_fj1091_complexity_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_config_complexity_score(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1091_complexity_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_config_complexity_score(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1091_complexity_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_config_complexity_score(d.path(), None, true).is_ok());
    }

    // ── FJ-1092: validate --check-resource-machine-distribution-balance ──
    #[test]
    fn test_fj1092_balance_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_machine_distribution_balance(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1092_balance_single_machine() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_validate_check_resource_machine_distribution_balance(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1092_balance_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_machine_distribution_balance(f.path(), true).is_ok());
    }

    // ── File-not-found error paths ──
    #[test]
    fn test_fj1086_file_not_found() {
        assert!(cmd_validate_check_resource_dependency_isolation(
            std::path::Path::new("/x"),
            false
        )
        .is_err());
    }
    #[test]
    fn test_fj1087_file_not_found() {
        assert!(
            cmd_graph_resource_dependency_depth_histogram(std::path::Path::new("/x"), false)
                .is_err()
        );
    }
    #[test]
    fn test_fj1089_file_not_found() {
        assert!(cmd_validate_check_resource_tag_value_consistency(
            std::path::Path::new("/x"),
            false
        )
        .is_err());
    }
    #[test]
    fn test_fj1090_file_not_found() {
        assert!(cmd_graph_resource_dependency_redundancy_analysis(
            std::path::Path::new("/x"),
            false
        )
        .is_err());
    }
    #[test]
    fn test_fj1092_file_not_found() {
        assert!(cmd_validate_check_resource_machine_distribution_balance(
            std::path::Path::new("/x"),
            false
        )
        .is_err());
    }
}

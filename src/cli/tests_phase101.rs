//! Tests: Phase 101 — Fleet Insight & Dependency Quality (FJ-1069→FJ-1076).

use super::status_fleet_insight::*;
use super::validate_governance_ext::*;
use super::graph_quality::*;
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
        if let Some(parent) = p.parent() { std::fs::create_dir_all(parent).unwrap(); }
        std::fs::write(&p, content).unwrap();
    }

    const LOCK: &str = "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  svc:\n    type: service\n    status: drifted\n    hash: \"blake3:def\"\n";

    // ── FJ-1069: status --fleet-resource-staleness-report ──
    #[test]
    fn test_fj1069_staleness_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_staleness_report(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1069_staleness_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_staleness_report(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1069_staleness_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_staleness_report(d.path(), None, true).is_ok());
    }

    // ── FJ-1070: validate --check-resource-dependency-fan-out-limit ──
    #[test]
    fn test_fj1070_fan_out_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_fan_out_limit(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1070_fan_out_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_dependency_fan_out_limit(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1070_fan_out_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_fan_out_limit(f.path(), true).is_ok());
    }

    // ── FJ-1071: graph --resource-dependency-critical-path-highlight ──
    #[test]
    fn test_fj1071_critical_path_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_critical_path_highlight(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1071_critical_path_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_critical_path_highlight(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1071_critical_path_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_critical_path_highlight(f.path(), true).is_ok());
    }

    // ── FJ-1072: status --machine-resource-type-distribution ──
    #[test]
    fn test_fj1072_type_dist_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_type_distribution(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1072_type_dist_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_machine_resource_type_distribution(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1072_type_dist_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_type_distribution(d.path(), None, true).is_ok());
    }

    // ── FJ-1073: validate --check-resource-tag-required-keys ──
    #[test]
    fn test_fj1073_tag_keys_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_tag_required_keys(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1073_tag_keys_with_tags() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    tags: [\"env:prod\", \"team:platform\", \"tier:1\"]\n");
        assert!(cmd_validate_check_resource_tag_required_keys(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1073_tag_keys_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_tag_required_keys(f.path(), true).is_ok());
    }

    // ── FJ-1074: graph --resource-dependency-bottleneck-detection ──
    #[test]
    fn test_fj1074_bottleneck_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_bottleneck_detection(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1074_bottleneck_with_fan_in() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m\n    packages: [curl]\n  cfg:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [pkg]\n  svc:\n    type: file\n    machine: m\n    path: /tmp/s\n    content: s\n    depends_on: [pkg]\n");
        assert!(cmd_graph_resource_dependency_bottleneck_detection(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1074_bottleneck_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_bottleneck_detection(f.path(), true).is_ok());
    }

    // ── FJ-1075: status --fleet-machine-health-score ──
    #[test]
    fn test_fj1075_health_score_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_machine_health_score(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1075_health_score_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_machine_health_score(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1075_health_score_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_machine_health_score(d.path(), None, true).is_ok());
    }

    // ── FJ-1076: validate --check-resource-content-drift-risk ──
    #[test]
    fn test_fj1076_drift_risk_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_content_drift_risk(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1076_drift_risk_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: service\n    machine: m\n    service_name: svc\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_content_drift_risk(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1076_drift_risk_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_content_drift_risk(f.path(), true).is_ok());
    }

    // ── File-not-found error paths ──
    #[test]
    fn test_fj1070_file_not_found() { assert!(cmd_validate_check_resource_dependency_fan_out_limit(std::path::Path::new("/x"), false).is_err()); }
    #[test]
    fn test_fj1071_file_not_found() { assert!(cmd_graph_resource_dependency_critical_path_highlight(std::path::Path::new("/x"), false).is_err()); }
    #[test]
    fn test_fj1073_file_not_found() { assert!(cmd_validate_check_resource_tag_required_keys(std::path::Path::new("/x"), false).is_err()); }
    #[test]
    fn test_fj1074_file_not_found() { assert!(cmd_graph_resource_dependency_bottleneck_detection(std::path::Path::new("/x"), false).is_err()); }
    #[test]
    fn test_fj1076_file_not_found() { assert!(cmd_validate_check_resource_content_drift_risk(std::path::Path::new("/x"), false).is_err()); }
}

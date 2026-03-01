//! Tests: Phase 105 — Fleet Resilience & Configuration Hygiene (FJ-1101→FJ-1108).

use super::status_resilience::*;
use super::validate_hygiene::*;
use super::graph_resilience_ext::*;
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

    // ── FJ-1101: status --fleet-resource-apply-success-trend ──
    #[test]
    fn test_fj1101_apply_success_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_apply_success_trend(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1101_apply_success_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_apply_success_trend(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1101_apply_success_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_apply_success_trend(d.path(), None, true).is_ok());
    }

    // ── FJ-1102: validate --check-resource-dependency-depth-variance ──
    #[test]
    fn test_fj1102_depth_variance_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_depth_variance(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1102_depth_variance_with_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_validate_check_resource_dependency_depth_variance(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1102_depth_variance_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_depth_variance(f.path(), true).is_ok());
    }

    // ── FJ-1103: graph --resource-dependency-fan-in-hotspot ──
    #[test]
    fn test_fj1103_fan_in_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_fan_in_hotspot(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1103_fan_in_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a]\n");
        assert!(cmd_graph_resource_dependency_fan_in_hotspot(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1103_fan_in_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_fan_in_hotspot(f.path(), true).is_ok());
    }

    // ── FJ-1104: status --machine-resource-drift-age-distribution-report ──
    #[test]
    fn test_fj1104_drift_age_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_age_distribution(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1104_drift_age_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_machine_resource_drift_age_distribution(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1104_drift_age_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_age_distribution(d.path(), None, true).is_ok());
    }

    // ── FJ-1105: validate --check-resource-tag-key-naming ──
    #[test]
    fn test_fj1105_tag_naming_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_tag_key_naming(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1105_tag_naming_with_tags() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    tags: [\"env:prod\", \"team:backend\"]\n");
        assert!(cmd_validate_check_resource_tag_key_naming(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1105_tag_naming_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_tag_key_naming(f.path(), true).is_ok());
    }

    // ── FJ-1106: graph --resource-dependency-cross-machine-bridge ──
    #[test]
    fn test_fj1106_cross_machine_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_cross_machine_bridge(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1106_cross_machine_multi() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\n  m2:\n    hostname: m2\n    addr: 127.0.0.2\nresources:\n  a:\n    type: file\n    machine: m1\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m2\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_graph_resource_dependency_cross_machine_bridge(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1106_cross_machine_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_cross_machine_bridge(f.path(), true).is_ok());
    }

    // ── FJ-1107: status --fleet-resource-convergence-gap-analysis ──
    #[test]
    fn test_fj1107_gap_analysis_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_gap_analysis(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1107_gap_analysis_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_convergence_gap_analysis(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1107_gap_analysis_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_gap_analysis(d.path(), None, true).is_ok());
    }

    // ── FJ-1108: validate --check-resource-content-length-limit ──
    #[test]
    fn test_fj1108_content_length_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_content_length_limit(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1108_content_length_ok() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: short\n");
        assert!(cmd_validate_check_resource_content_length_limit(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1108_content_length_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_content_length_limit(f.path(), true).is_ok());
    }

    // ── File-not-found error paths ──
    #[test]
    fn test_fj1102_file_not_found() { assert!(cmd_validate_check_resource_dependency_depth_variance(std::path::Path::new("/x"), false).is_err()); }
    #[test]
    fn test_fj1103_file_not_found() { assert!(cmd_graph_resource_dependency_fan_in_hotspot(std::path::Path::new("/x"), false).is_err()); }
    #[test]
    fn test_fj1105_file_not_found() { assert!(cmd_validate_check_resource_tag_key_naming(std::path::Path::new("/x"), false).is_err()); }
    #[test]
    fn test_fj1106_file_not_found() { assert!(cmd_graph_resource_dependency_cross_machine_bridge(std::path::Path::new("/x"), false).is_err()); }
    #[test]
    fn test_fj1108_file_not_found() { assert!(cmd_validate_check_resource_content_length_limit(std::path::Path::new("/x"), false).is_err()); }
}

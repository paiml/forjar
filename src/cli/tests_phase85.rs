//! Tests: Phase 85 — Advanced Compliance & Dependency Intelligence (FJ-941→FJ-948).

use super::validate_ordering::*;
use super::graph_intelligence::*;
use super::graph_intelligence_ext::*;
use super::status_intelligence::*;
use super::status_intelligence_ext::*;
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
        if let Some(parent) = p.parent() { std::fs::create_dir_all(parent).unwrap(); }
        std::fs::write(&p, content).unwrap();
        p
    }

    // ── FJ-941: validate --check-resource-circular-alias ──

    #[test]
    fn test_fj941_circular_alias_none() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_circular_alias(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj941_circular_alias_detected() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    depends_on: [b]\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_circular_alias(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj941_circular_alias_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_validate_check_resource_circular_alias(f.path(), true).is_ok());
    }

    // ── FJ-942: status --machine-resource-drift-frequency ──

    #[test]
    fn test_fj942_drift_frequency_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_frequency(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj942_drift_frequency_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: drifted\n    hash: \"blake3:abc\"\n");
        assert!(cmd_status_machine_resource_drift_frequency(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj942_drift_frequency_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_frequency(dir.path(), None, true).is_ok());
    }

    // ── FJ-943: graph --resource-dependency-fan-out ──

    #[test]
    fn test_fj943_fan_out_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_fan_out(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj943_fan_out_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a]\n");
        assert!(cmd_graph_resource_dependency_fan_out(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj943_fan_out_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_fan_out(f.path(), true).is_ok());
    }

    // ── FJ-944: apply --notify-custom-dedup-window (tested via struct) ──

    // ── FJ-945: validate --check-resource-dependency-depth-limit ──

    #[test]
    fn test_fj945_depth_limit_ok() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_dependency_depth_limit(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj945_depth_limit_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_validate_check_resource_dependency_depth_limit(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj945_depth_limit_no_resources() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_depth_limit(f.path(), false).is_ok());
    }

    // ── FJ-946: status --fleet-resource-drift-frequency ──

    #[test]
    fn test_fj946_fleet_drift_frequency_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_frequency(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj946_fleet_drift_frequency_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_drift_frequency(dir.path(), None, true).is_ok());
    }

    #[test]
    fn test_fj946_fleet_drift_frequency_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "db/state.lock.yaml", "schema: \"1.0\"\nmachine: db\nhostname: db\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: drifted\n    hash: \"blake3:abc\"\n");
        assert!(cmd_status_fleet_resource_drift_frequency(dir.path(), None, false).is_ok());
    }

    // ── FJ-947: graph --resource-dependency-fan-in ──

    #[test]
    fn test_fj947_fan_in_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_fan_in(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj947_fan_in_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a]\n");
        assert!(cmd_graph_resource_dependency_fan_in(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj947_fan_in_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_fan_in(f.path(), true).is_ok());
    }

    // ── FJ-948: status --machine-resource-apply-duration-trend ──

    #[test]
    fn test_fj948_apply_duration_trend_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_apply_duration_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj948_apply_duration_trend_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    duration_seconds: 1.5\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: converged\n    duration_seconds: 2.5\n    hash: \"blake3:def\"\n");
        assert!(cmd_status_machine_resource_apply_duration_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj948_apply_duration_trend_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_apply_duration_trend(dir.path(), None, true).is_ok());
    }
}

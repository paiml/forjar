//! Tests: Phase 86 — Resource Lifecycle & Configuration Maturity (FJ-949→FJ-956).

#![allow(unused_imports)]
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

    // ── FJ-949: validate --check-resource-unused-params ──

    #[test]
    fn test_fj949_unused_params_none() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_unused_params(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj949_unused_params_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nparams:\n  data_dir: /mnt\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_unused_params(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj949_unused_params_all_used() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nparams:\n  data_dir: /mnt\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: \"path is {{data_dir}}\"\n");
        assert!(cmd_validate_check_resource_unused_params(f.path(), false).is_ok());
    }

    // ── FJ-950: status --machine-resource-convergence-streak ──

    #[test]
    fn test_fj950_convergence_streak_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_streak(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj950_convergence_streak_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: converged\n    hash: \"blake3:def\"\n");
        assert!(cmd_status_machine_resource_convergence_streak(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj950_convergence_streak_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_streak(dir.path(), None, true).is_ok());
    }

    // ── FJ-951: graph --resource-dependency-path-count ──

    #[test]
    fn test_fj951_path_count_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_path_count(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj951_path_count_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_graph_resource_dependency_path_count(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj951_path_count_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_path_count(f.path(), true).is_ok());
    }

    // ── FJ-953: validate --check-resource-machine-balance ──

    #[test]
    fn test_fj953_machine_balance_ok() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_validate_check_resource_machine_balance(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj953_machine_balance_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_machine_balance(f.path(), true).is_ok());
    }

    // ── FJ-954: status --fleet-resource-convergence-streak ──

    #[test]
    fn test_fj954_fleet_convergence_streak_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_streak(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj954_fleet_convergence_streak_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_streak(dir.path(), None, true).is_ok());
    }

    // ── FJ-955: graph --resource-dependency-articulation-points ──

    #[test]
    fn test_fj955_articulation_points_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_articulation_points(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj955_articulation_points_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_articulation_points(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj955_articulation_points_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_articulation_points(f.path(), true).is_ok());
    }

    // ── FJ-956: status --machine-resource-error-distribution ──

    #[test]
    fn test_fj956_error_distribution_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_error_distribution(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj956_error_distribution_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: failed\n    hash: \"blake3:abc\"\n  g:\n    type: file\n    status: drifted\n    hash: \"blake3:def\"\n");
        assert!(cmd_status_machine_resource_error_distribution(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj956_error_distribution_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_error_distribution(dir.path(), None, true).is_ok());
    }
}

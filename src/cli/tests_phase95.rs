//! Tests: Phase 95 — Operational Resilience & Runtime Diagnostics (FJ-1021→FJ-1028).

use super::graph_resilience::*;
use super::status_operational_ext::*;
use super::validate_resilience::*;
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

    // ── FJ-1021: status --fleet-apply-success-rate-trend ──

    #[test]
    fn test_fj1021_success_rate_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_apply_success_rate_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1021_success_rate_with_events() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/events.log", "{\"result\":\"ok\",\"timestamp\":\"2026-02-28T00:00:00Z\"}\n{\"result\":\"fail\",\"timestamp\":\"2026-02-28T00:01:00Z\"}\n{\"result\":\"ok\",\"timestamp\":\"2026-02-28T00:02:00Z\"}\n");
        assert!(cmd_status_fleet_apply_success_rate_trend(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1021_success_rate_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_apply_success_rate_trend(dir.path(), None, true).is_ok());
    }

    #[test]
    fn test_fj1021_success_rate_with_filter() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/events.log", "{\"result\":\"ok\"}\n");
        assert!(cmd_status_fleet_apply_success_rate_trend(dir.path(), Some("web"), false).is_ok());
    }

    // ── FJ-1022: validate --check-resource-lifecycle-hook-coverage ──

    #[test]
    fn test_fj1022_lifecycle_hooks_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_lifecycle_hook_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1022_lifecycle_hooks_service_no_hooks() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  svc:\n    type: service\n    machine: m\n    service_name: nginx\n");
        assert!(cmd_validate_check_resource_lifecycle_hook_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1022_lifecycle_hooks_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m\n    package_name: vim\n");
        assert!(cmd_validate_check_resource_lifecycle_hook_coverage(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1022_lifecycle_hooks_file_ok() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_validate_check_resource_lifecycle_hook_coverage(f.path(), false).is_ok());
    }

    // ── FJ-1023: graph --resource-dependency-parallel-groups ──

    #[test]
    fn test_fj1023_parallel_groups_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_parallel_groups(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1023_parallel_groups_no_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n");
        assert!(cmd_graph_resource_dependency_parallel_groups(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1023_parallel_groups_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_parallel_groups(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1023_parallel_groups_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_parallel_groups(f.path(), true).is_ok());
    }

    // ── FJ-1024: status --machine-resource-drift-flapping ──

    #[test]
    fn test_fj1024_drift_flapping_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_flapping(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1024_drift_flapping_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: drifted\n    hash: \"blake3:abc\"\n");
        assert!(cmd_status_machine_resource_drift_flapping(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1024_drift_flapping_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_flapping(dir.path(), None, true).is_ok());
    }

    // ── FJ-1025: validate --check-resource-secret-rotation-age ──

    #[test]
    fn test_fj1025_secret_rotation_no_secrets() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_secret_rotation_age(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1025_secret_rotation_with_enc() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: \"ENC[age,YWdlLWVuY3J5cHRpb24...]\"\n");
        assert!(cmd_validate_check_resource_secret_rotation_age(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1025_secret_rotation_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_secret_rotation_age(f.path(), true).is_ok());
    }

    // ── FJ-1026: graph --resource-dependency-execution-cost ──

    #[test]
    fn test_fj1026_execution_cost_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_execution_cost(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1026_execution_cost_single() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m\n    package_name: vim\n");
        assert!(cmd_graph_resource_dependency_execution_cost(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1026_execution_cost_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: package\n    machine: m\n    package_name: vim\n    depends_on: [a]\n  c:\n    type: service\n    machine: m\n    service_name: nginx\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_execution_cost(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1026_execution_cost_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_execution_cost(f.path(), true).is_ok());
    }

    // ── FJ-1027: status --fleet-resource-type-drift-heatmap ──

    #[test]
    fn test_fj1027_drift_heatmap_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_drift_heatmap(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1027_drift_heatmap_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: drifted\n    hash: \"blake3:abc\"\n  s:\n    type: service\n    status: drifted\n    hash: \"blake3:def\"\n");
        assert!(cmd_status_fleet_resource_type_drift_heatmap(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1027_drift_heatmap_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_drift_heatmap(dir.path(), None, true).is_ok());
    }

    // ── FJ-1028: validate --check-resource-dependency-chain-depth ──

    #[test]
    fn test_fj1028_dep_chain_depth_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_chain_depth(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1028_dep_chain_depth_within_limit() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_dependency_chain_depth(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1028_dep_chain_depth_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_chain_depth(f.path(), true).is_ok());
    }
}

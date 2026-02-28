//! Tests: Phase 96 — Transport Diagnostics & Recipe Governance (FJ-1029→FJ-1036).

use super::status_transport::*;
use super::validate_transport::*;
use super::graph_transport::*;
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

    // ── FJ-1029: status --machine-ssh-connection-health ──

    #[test]
    fn test_fj1029_ssh_health_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_ssh_connection_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1029_ssh_health_with_lock() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources: {}\n");
        assert!(cmd_status_machine_ssh_connection_health(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1029_ssh_health_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_ssh_connection_health(dir.path(), None, true).is_ok());
    }

    #[test]
    fn test_fj1029_ssh_health_with_filter() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar-ssh\n");
        assert!(cmd_status_machine_ssh_connection_health(dir.path(), Some("web"), false).is_ok());
    }

    // ── FJ-1030: validate --check-recipe-input-completeness ──

    #[test]
    fn test_fj1030_recipe_input_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_recipe_input_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1030_recipe_input_no_templates() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_recipe_input_completeness(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1030_recipe_input_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_recipe_input_completeness(f.path(), true).is_ok());
    }

    // ── FJ-1031: graph --resource-recipe-expansion-map ──

    #[test]
    fn test_fj1031_recipe_map_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_recipe_expansion_map(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1031_recipe_map_by_type() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: package\n    machine: m\n    package_name: vim\n");
        assert!(cmd_graph_resource_recipe_expansion_map(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1031_recipe_map_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_recipe_expansion_map(f.path(), true).is_ok());
    }

    // ── FJ-1032: status --lock-file-staleness-report ──

    #[test]
    fn test_fj1032_staleness_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_lock_file_staleness_report(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1032_staleness_with_lock() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n");
        assert!(cmd_status_lock_file_staleness_report(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1032_staleness_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_lock_file_staleness_report(dir.path(), None, true).is_ok());
    }

    #[test]
    fn test_fj1032_staleness_with_filter() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "db/state.lock.yaml", "schema: \"1.0\"\ngenerated_at: \"2026-02-28T00:00:00Z\"\n");
        assert!(cmd_status_lock_file_staleness_report(dir.path(), Some("db"), false).is_ok());
    }

    // ── FJ-1033: validate --check-resource-cross-machine-content-duplicates ──

    #[test]
    fn test_fj1033_content_dup_no_dups() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: hello\n");
        assert!(cmd_validate_check_resource_cross_machine_content_duplicates(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1033_content_dup_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_cross_machine_content_duplicates(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1033_content_dup_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_cross_machine_content_duplicates(f.path(), false).is_ok());
    }

    // ── FJ-1034: graph --resource-dependency-critical-chain-path ──

    #[test]
    fn test_fj1034_critical_chain_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_critical_chain_path(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1034_critical_chain_linear() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_dependency_critical_chain_path(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1034_critical_chain_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_dependency_critical_chain_path(f.path(), true).is_ok());
    }

    #[test]
    fn test_fj1034_critical_chain_no_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n");
        assert!(cmd_graph_resource_dependency_critical_chain_path(f.path(), false).is_ok());
    }

    // ── FJ-1035: status --fleet-transport-method-summary ──

    #[test]
    fn test_fj1035_transport_summary_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_transport_method_summary(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1035_transport_summary_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\naddr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_status_fleet_transport_method_summary(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1035_transport_summary_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_transport_method_summary(dir.path(), None, true).is_ok());
    }

    // ── FJ-1036: validate --check-resource-machine-reference-validity ──

    #[test]
    fn test_fj1036_machine_ref_valid() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_validate_check_resource_machine_reference_validity(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1036_machine_ref_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_machine_reference_validity(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1036_machine_ref_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_machine_reference_validity(f.path(), true).is_ok());
    }
}

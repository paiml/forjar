//! Tests: Phase 97 — State Analytics & Capacity Planning (FJ-1037→FJ-1044).

use super::status_analytics::*;
use super::validate_analytics::*;
use super::graph_analytics::*;
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

    // ── FJ-1037: status --fleet-state-churn-analysis ──

    #[test]
    fn test_fj1037_churn_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_state_churn_analysis(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1037_churn_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n");
        assert!(cmd_status_fleet_state_churn_analysis(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1037_churn_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_state_churn_analysis(dir.path(), None, true).is_ok());
    }

    // ── FJ-1038: validate --check-resource-health-correlation ──

    #[test]
    fn test_fj1038_health_corr_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_health_correlation(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1038_health_corr_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a]\n  d:\n    type: file\n    machine: m\n    path: /tmp/d\n    content: d\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_health_correlation(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1038_health_corr_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_health_correlation(f.path(), true).is_ok());
    }

    // ── FJ-1039: graph --resource-apply-order-simulation ──

    #[test]
    fn test_fj1039_apply_order_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_apply_order_simulation(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1039_apply_order_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n");
        assert!(cmd_graph_resource_apply_order_simulation(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1039_apply_order_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_apply_order_simulation(f.path(), true).is_ok());
    }

    // ── FJ-1040: status --config-maturity-score ──

    #[test]
    fn test_fj1040_maturity_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_config_maturity_score(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1040_maturity_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  s:\n    type: service\n    status: converged\n    hash: \"blake3:def\"\n");
        assert!(cmd_status_config_maturity_score(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1040_maturity_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_config_maturity_score(dir.path(), None, true).is_ok());
    }

    // ── FJ-1041: validate --check-dependency-optimization ──

    #[test]
    fn test_fj1041_dep_opt_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_dependency_optimization(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1041_dep_opt_redundant() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a, b]\n");
        assert!(cmd_validate_check_dependency_optimization(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1041_dep_opt_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_dependency_optimization(f.path(), true).is_ok());
    }

    // ── FJ-1042: graph --resource-provenance-summary ──

    #[test]
    fn test_fj1042_provenance_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_provenance_summary(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1042_provenance_mixed() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: package\n    machine: m\n    package_name: vim\n");
        assert!(cmd_graph_resource_provenance_summary(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1042_provenance_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_graph_resource_provenance_summary(f.path(), true).is_ok());
    }

    // ── FJ-1043: status --fleet-capacity-utilization ──

    #[test]
    fn test_fj1043_capacity_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_capacity_utilization(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1043_capacity_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n");
        assert!(cmd_status_fleet_capacity_utilization(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1043_capacity_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_capacity_utilization(dir.path(), None, true).is_ok());
    }

    // ── FJ-1044: validate --check-resource-consolidation-opportunities ──

    #[test]
    fn test_fj1044_consolidation_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_consolidation_opportunities(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1044_consolidation_no_dups() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: unique_content\n");
        assert!(cmd_validate_check_resource_consolidation_opportunities(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1044_consolidation_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_consolidation_opportunities(f.path(), true).is_ok());
    }
}

//! Tests: Phase 102 — Resource Intelligence & Topology Insight (FJ-1077→FJ-1084).

use super::graph_topology_ext::*;
use super::status_resource_intel::*;
use super::validate_topology::*;
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

    // ── FJ-1077: status --fleet-resource-dependency-lag ──
    #[test]
    fn test_fj1077_dep_lag_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_dependency_lag_report(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1077_dep_lag_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_dependency_lag_report(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1077_dep_lag_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_dependency_lag_report(d.path(), None, true).is_ok());
    }

    // ── FJ-1078: validate --check-resource-circular-dependency-depth ──
    #[test]
    fn test_fj1078_circular_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_circular_dependency_depth(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1078_circular_no_cycle() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_circular_dependency_depth(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1078_circular_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_circular_dependency_depth(f.path(), true).is_ok());
    }

    // ── FJ-1079: graph --resource-topology-cluster-analysis ──
    #[test]
    fn test_fj1079_cluster_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_topology_cluster_analysis(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1079_cluster_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n");
        assert!(cmd_graph_resource_topology_cluster_analysis(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1079_cluster_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_topology_cluster_analysis(f.path(), true).is_ok());
    }

    // ── FJ-1080: status --machine-resource-convergence-rate-trend ──
    #[test]
    fn test_fj1080_conv_rate_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_rate_trend(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1080_conv_rate_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_machine_resource_convergence_rate_trend(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1080_conv_rate_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_convergence_rate_trend(d.path(), None, true).is_ok());
    }

    // ── FJ-1081: validate --check-resource-orphan-detection-deep ──
    #[test]
    fn test_fj1081_orphan_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_orphan_detection_deep(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1081_orphan_with_chain() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_orphan_detection_deep(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1081_orphan_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_orphan_detection_deep(f.path(), true).is_ok());
    }

    // ── FJ-1082: graph --resource-dependency-island-detection ──
    #[test]
    fn test_fj1082_island_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_island_detection(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1082_island_with_isolated() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n");
        assert!(cmd_graph_resource_dependency_island_detection(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1082_island_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_island_detection(f.path(), true).is_ok());
    }

    // ── FJ-1083: status --fleet-resource-apply-lag ──
    #[test]
    fn test_fj1083_apply_lag_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_apply_lag(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1083_apply_lag_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_apply_lag(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1083_apply_lag_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_apply_lag(d.path(), None, true).is_ok());
    }

    // ── FJ-1084: validate --check-resource-provider-diversity ──
    #[test]
    fn test_fj1084_diversity_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_provider_diversity(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1084_diversity_mixed() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: package\n    machine: m\n    packages: [curl]\n");
        assert!(cmd_validate_check_resource_provider_diversity(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1084_diversity_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_provider_diversity(f.path(), true).is_ok());
    }

    // ── File-not-found error paths ──
    #[test]
    fn test_fj1078_file_not_found() {
        assert!(cmd_validate_check_resource_circular_dependency_depth(
            std::path::Path::new("/x"),
            false
        )
        .is_err());
    }
    #[test]
    fn test_fj1079_file_not_found() {
        assert!(
            cmd_graph_resource_topology_cluster_analysis(std::path::Path::new("/x"), false)
                .is_err()
        );
    }
    #[test]
    fn test_fj1081_file_not_found() {
        assert!(cmd_validate_check_resource_orphan_detection_deep(
            std::path::Path::new("/x"),
            false
        )
        .is_err());
    }
    #[test]
    fn test_fj1082_file_not_found() {
        assert!(
            cmd_graph_resource_dependency_island_detection(std::path::Path::new("/x"), false)
                .is_err()
        );
    }
    #[test]
    fn test_fj1084_file_not_found() {
        assert!(
            cmd_validate_check_resource_provider_diversity(std::path::Path::new("/x"), false)
                .is_err()
        );
    }
}

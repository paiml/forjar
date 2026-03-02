//! Tests: Phase 100 — Operational Intelligence & Graph Health (FJ-1061→FJ-1068).

use super::graph_health::*;
use super::status_operational_ext2::*;
use super::validate_security_ext::*;
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

    // ── FJ-1061: status --fleet-apply-cadence ──
    #[test]
    fn test_fj1061_cadence_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_apply_cadence(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1061_cadence_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_apply_cadence(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1061_cadence_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_apply_cadence(d.path(), None, true).is_ok());
    }

    // ── FJ-1062: validate --check-resource-dependency-symmetry-deep ──
    #[test]
    fn test_fj1062_symmetry_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_symmetry_deep(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1062_symmetry_no_cycle() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_dependency_symmetry_deep(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1062_symmetry_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_dependency_symmetry_deep(f.path(), true).is_ok());
    }

    // ── FJ-1063: graph --resource-dependency-health-overlay ──
    #[test]
    fn test_fj1063_health_overlay_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_health_overlay(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1063_health_overlay_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m\n    packages: [curl]\n    tags: [critical]\n  cfg:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [pkg]\n");
        assert!(cmd_graph_resource_dependency_health_overlay(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1063_health_overlay_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_health_overlay(f.path(), true).is_ok());
    }

    // ── FJ-1064: status --machine-resource-error-classification ──
    #[test]
    fn test_fj1064_error_class_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_error_classification(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1064_error_class_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_machine_resource_error_classification(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1064_error_class_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_error_classification(d.path(), None, true).is_ok());
    }

    // ── FJ-1065: validate --check-resource-tag-namespace ──
    #[test]
    fn test_fj1065_tag_namespace_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_tag_namespace(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1065_tag_namespace_mixed() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    tags: [\"env:prod\", unnamespaced]\n");
        assert!(cmd_validate_check_resource_tag_namespace(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1065_tag_namespace_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_tag_namespace(f.path(), true).is_ok());
    }

    // ── FJ-1066: graph --resource-dependency-width-analysis ──
    #[test]
    fn test_fj1066_width_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_width_analysis(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1066_width_with_levels() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [a, b]\n");
        assert!(cmd_graph_resource_dependency_width_analysis(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1066_width_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_width_analysis(f.path(), true).is_ok());
    }

    // ── FJ-1067: status --fleet-resource-convergence-summary ──
    #[test]
    fn test_fj1067_convergence_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_summary(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1067_convergence_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_convergence_summary(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1067_convergence_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_summary(d.path(), None, true).is_ok());
    }

    // ── FJ-1068: validate --check-resource-machine-capacity ──
    #[test]
    fn test_fj1068_capacity_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_machine_capacity(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1068_capacity_under() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n");
        assert!(cmd_validate_check_resource_machine_capacity(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1068_capacity_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_machine_capacity(f.path(), true).is_ok());
    }

    // ── File-not-found error paths ──
    #[test]
    fn test_fj1062_file_not_found() {
        assert!(cmd_validate_check_resource_dependency_symmetry_deep(
            std::path::Path::new("/x"),
            false
        )
        .is_err());
    }
    #[test]
    fn test_fj1063_file_not_found() {
        assert!(
            cmd_graph_resource_dependency_health_overlay(std::path::Path::new("/x"), false)
                .is_err()
        );
    }
    #[test]
    fn test_fj1065_file_not_found() {
        assert!(
            cmd_validate_check_resource_tag_namespace(std::path::Path::new("/x"), false).is_err()
        );
    }
    #[test]
    fn test_fj1066_file_not_found() {
        assert!(
            cmd_graph_resource_dependency_width_analysis(std::path::Path::new("/x"), false)
                .is_err()
        );
    }
    #[test]
    fn test_fj1068_file_not_found() {
        assert!(
            cmd_validate_check_resource_machine_capacity(std::path::Path::new("/x"), false)
                .is_err()
        );
    }
}

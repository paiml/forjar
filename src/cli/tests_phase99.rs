//! Tests: Phase 99 — Security Posture & Resource Lifecycle (FJ-1053→FJ-1060).

use super::status_security::*;
use super::validate_security::*;
use super::graph_lifecycle::*;
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

    // ── FJ-1053: status --fleet-security-posture-summary ──

    #[test]
    fn test_fj1053_security_posture_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_security_posture_summary(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1053_security_posture_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  svc:\n    type: service\n    status: converged\n    hash: \"blake3:abc\"\n");
        assert!(cmd_status_fleet_security_posture_summary(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1053_security_posture_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_security_posture_summary(dir.path(), None, true).is_ok());
    }

    // ── FJ-1054: validate --check-resource-secret-scope ──

    #[test]
    fn test_fj1054_secret_scope_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_secret_scope(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1054_secret_scope_with_resources() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: \"{{secret.db_pass}}\"\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: plain\n");
        assert!(cmd_validate_check_resource_secret_scope(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1054_secret_scope_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_secret_scope(f.path(), true).is_ok());
    }

    // ── FJ-1055: graph --resource-lifecycle-stage-map ──

    #[test]
    fn test_fj1055_lifecycle_stage_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_lifecycle_stage_map(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1055_lifecycle_stage_mixed() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    tags: [deprecated]\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    tags: [stable]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n");
        assert!(cmd_graph_resource_lifecycle_stage_map(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1055_lifecycle_stage_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_lifecycle_stage_map(f.path(), true).is_ok());
    }

    // ── FJ-1056: status --machine-resource-freshness-index ──

    #[test]
    fn test_fj1056_freshness_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_freshness_index(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1056_freshness_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n");
        assert!(cmd_status_machine_resource_freshness_index(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1056_freshness_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_freshness_index(dir.path(), None, true).is_ok());
    }

    // ── FJ-1057: validate --check-resource-deprecation-usage ──

    #[test]
    fn test_fj1057_deprecation_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_deprecation_usage(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1057_deprecation_with_dep_on_deprecated() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  old:\n    type: file\n    machine: m\n    path: /tmp/old\n    content: old\n    tags: [deprecated]\n  new:\n    type: file\n    machine: m\n    path: /tmp/new\n    content: new\n    depends_on: [old]\n");
        assert!(cmd_validate_check_resource_deprecation_usage(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1057_deprecation_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_deprecation_usage(f.path(), true).is_ok());
    }

    // ── FJ-1058: graph --resource-dependency-age-overlay ──

    #[test]
    fn test_fj1058_age_overlay_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_age_overlay(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1058_age_overlay_with_deps() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  pkg:\n    type: package\n    machine: m\n    packages: [curl]\n  cfg:\n    type: file\n    machine: m\n    path: /tmp/cfg\n    content: c\n    depends_on: [pkg]\n");
        assert!(cmd_graph_resource_dependency_age_overlay(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1058_age_overlay_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_graph_resource_dependency_age_overlay(f.path(), true).is_ok());
    }

    // ── FJ-1059: status --fleet-resource-type-coverage ──

    #[test]
    fn test_fj1059_type_coverage_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_coverage(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1059_type_coverage_with_data() {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  pkg:\n    type: package\n    status: converged\n    hash: \"blake3:abc\"\n  svc:\n    type: service\n    status: converged\n    hash: \"blake3:def\"\n");
        assert!(cmd_status_fleet_resource_type_coverage(dir.path(), None, false).is_ok());
    }

    #[test]
    fn test_fj1059_type_coverage_json() {
        let dir = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_type_coverage(dir.path(), None, true).is_ok());
    }

    // ── FJ-1060: validate --check-resource-when-condition-coverage ──

    #[test]
    fn test_fj1060_when_coverage_empty() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_when_condition_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1060_when_coverage_with_conditions() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    when: \"env == prod\"\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n");
        assert!(cmd_validate_check_resource_when_condition_coverage(f.path(), false).is_ok());
    }

    #[test]
    fn test_fj1060_when_coverage_json() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n");
        assert!(cmd_validate_check_resource_when_condition_coverage(f.path(), true).is_ok());
    }

    // ── File-not-found error paths ──

    #[test]
    fn test_fj1054_file_not_found() {
        assert!(cmd_validate_check_resource_secret_scope(std::path::Path::new("/nonexistent"), false).is_err());
    }

    #[test]
    fn test_fj1055_file_not_found() {
        assert!(cmd_graph_resource_lifecycle_stage_map(std::path::Path::new("/nonexistent"), false).is_err());
    }

    #[test]
    fn test_fj1057_file_not_found() {
        assert!(cmd_validate_check_resource_deprecation_usage(std::path::Path::new("/nonexistent"), false).is_err());
    }

    #[test]
    fn test_fj1058_file_not_found() {
        assert!(cmd_graph_resource_dependency_age_overlay(std::path::Path::new("/nonexistent"), false).is_err());
    }

    #[test]
    fn test_fj1060_file_not_found() {
        assert!(cmd_validate_check_resource_when_condition_coverage(std::path::Path::new("/nonexistent"), false).is_err());
    }
}

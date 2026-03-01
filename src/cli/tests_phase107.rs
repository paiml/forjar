//! Tests: Phase 107 — Resource Quality Scoring & Fleet Drift Analytics (FJ-1117→FJ-1124).

use super::status_quality::*;
use super::validate_scoring::*;
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
    const CFG: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources: {}\n";
    const CFG_DEPS: &str = "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n";

    // ── FJ-1117: status --fleet-resource-quality-score ──
    #[test]
    fn test_fj1117_quality_score_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_quality_score(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1117_quality_score_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_quality_score(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1117_quality_score_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_quality_score(d.path(), None, true).is_ok());
    }

    // ── FJ-1118: validate --check-resource-dependency-ordering-consistency ──
    #[test]
    fn test_fj1118_ordering_empty() {
        let f = write_temp_config(CFG);
        assert!(cmd_validate_check_resource_dependency_ordering_consistency(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1118_ordering_with_deps() {
        let f = write_temp_config(CFG_DEPS);
        assert!(cmd_validate_check_resource_dependency_ordering_consistency(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1118_ordering_json() {
        let f = write_temp_config(CFG);
        assert!(cmd_validate_check_resource_dependency_ordering_consistency(f.path(), true).is_ok());
    }

    // ── FJ-1119: graph --resource-dependency-critical-path ──
    #[test]
    fn test_fj1119_critical_path_empty() {
        let f = write_temp_config(CFG);
        assert!(cmd_graph_resource_dependency_critical_path(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1119_critical_path_chain() {
        let f = write_temp_config(CFG_DEPS);
        assert!(cmd_graph_resource_dependency_critical_path(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1119_critical_path_json() {
        let f = write_temp_config(CFG);
        assert!(cmd_graph_resource_dependency_critical_path(f.path(), true).is_ok());
    }

    // ── FJ-1120: status --machine-resource-drift-pattern-classification ──
    #[test]
    fn test_fj1120_drift_class_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_pattern_classification(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1120_drift_class_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_machine_resource_drift_pattern_classification(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1120_drift_class_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_machine_resource_drift_pattern_classification(d.path(), None, true).is_ok());
    }

    // ── FJ-1121: validate --check-resource-tag-value-format ──
    #[test]
    fn test_fj1121_tag_format_empty() {
        let f = write_temp_config(CFG);
        assert!(cmd_validate_check_resource_tag_value_format(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1121_tag_format_with_tags() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    tags: [prod, web, \"v1.0\"]\n");
        assert!(cmd_validate_check_resource_tag_value_format(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1121_tag_format_json() {
        let f = write_temp_config(CFG);
        assert!(cmd_validate_check_resource_tag_value_format(f.path(), true).is_ok());
    }

    // ── FJ-1122: graph --resource-dependency-cluster-analysis ──
    #[test]
    fn test_fj1122_cluster_empty() {
        let f = write_temp_config(CFG);
        assert!(cmd_graph_resource_dependency_cluster_analysis(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1122_cluster_chain() {
        let f = write_temp_config(CFG_DEPS);
        assert!(cmd_graph_resource_dependency_cluster_analysis(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1122_cluster_json() {
        let f = write_temp_config(CFG);
        assert!(cmd_graph_resource_dependency_cluster_analysis(f.path(), true).is_ok());
    }

    // ── FJ-1123: status --fleet-resource-convergence-window-analysis ──
    #[test]
    fn test_fj1123_convergence_window_empty() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_window_analysis(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1123_convergence_window_with_data() {
        let d = tempfile::tempdir().unwrap();
        write_yaml(d.path(), "web/state.lock.yaml", LOCK);
        assert!(cmd_status_fleet_resource_convergence_window_analysis(d.path(), None, false).is_ok());
    }
    #[test]
    fn test_fj1123_convergence_window_json() {
        let d = tempfile::tempdir().unwrap();
        assert!(cmd_status_fleet_resource_convergence_window_analysis(d.path(), None, true).is_ok());
    }

    // ── FJ-1124: validate --check-resource-provider-version-pinning ──
    #[test]
    fn test_fj1124_version_pinning_empty() {
        let f = write_temp_config(CFG);
        assert!(cmd_validate_check_resource_provider_version_pinning(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1124_version_pinning_with_data() {
        let f = write_temp_config("version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n    version: \"1.0\"\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n");
        assert!(cmd_validate_check_resource_provider_version_pinning(f.path(), false).is_ok());
    }
    #[test]
    fn test_fj1124_version_pinning_json() {
        let f = write_temp_config(CFG);
        assert!(cmd_validate_check_resource_provider_version_pinning(f.path(), true).is_ok());
    }

    // ── File-not-found error paths ──
    #[test]
    fn test_fj1118_file_not_found() { assert!(cmd_validate_check_resource_dependency_ordering_consistency(std::path::Path::new("/x"), false).is_err()); }
    #[test]
    fn test_fj1119_file_not_found() { assert!(cmd_graph_resource_dependency_critical_path(std::path::Path::new("/x"), false).is_err()); }
    #[test]
    fn test_fj1121_file_not_found() { assert!(cmd_validate_check_resource_tag_value_format(std::path::Path::new("/x"), false).is_err()); }
    #[test]
    fn test_fj1122_file_not_found() { assert!(cmd_graph_resource_dependency_cluster_analysis(std::path::Path::new("/x"), false).is_err()); }
    #[test]
    fn test_fj1124_file_not_found() { assert!(cmd_validate_check_resource_provider_version_pinning(std::path::Path::new("/x"), false).is_err()); }
}

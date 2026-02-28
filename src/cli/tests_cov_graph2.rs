//! Tests: Coverage for graph_core, graph_cross, graph_analysis.

use super::graph_analysis::*;
use super::graph_core::*;
use super::graph_cross::*;
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

    fn config_with_deps() -> String {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: file\n    machine: m\n    path: /tmp/c\n    content: c\n    depends_on: [b]\n".to_string()
    }

    fn config_no_deps() -> String {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  x:\n    type: file\n    machine: m\n    path: /tmp/x\n    content: x\n  y:\n    type: file\n    machine: m\n    path: /tmp/y\n    content: y\n".to_string()
    }

    fn config_diamond() -> String {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  root:\n    type: file\n    machine: m\n    path: /tmp/root\n    content: r\n  left:\n    type: file\n    machine: m\n    path: /tmp/left\n    content: l\n    depends_on: [root]\n  right:\n    type: file\n    machine: m\n    path: /tmp/right\n    content: r\n    depends_on: [root]\n  bottom:\n    type: file\n    machine: m\n    path: /tmp/bottom\n    content: b\n    depends_on: [left, right]\n".to_string()
    }

    fn config_single_resource() -> String {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  only:\n    type: file\n    machine: m\n    path: /tmp/only\n    content: x\n".to_string()
    }

    fn config_multi_machine() -> String {
        "version: \"1.0\"\nname: t\nmachines:\n  web:\n    hostname: web\n    addr: 10.0.0.1\n  db:\n    hostname: db\n    addr: 10.0.0.2\nresources:\n  app:\n    type: file\n    machine: web\n    path: /tmp/app\n    content: app\n  schema:\n    type: file\n    machine: db\n    path: /tmp/schema\n    content: schema\n  migrate:\n    type: file\n    machine: db\n    path: /tmp/migrate\n    content: mig\n    depends_on: [schema]\n  deploy:\n    type: file\n    machine: web\n    path: /tmp/deploy\n    content: dep\n    depends_on: [app, migrate]\n".to_string()
    }

    fn config_mixed_types() -> String {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  svc:\n    type: service\n    machine: m\n    name: mysvc\n  conf:\n    type: file\n    machine: m\n    path: /etc/myapp.conf\n    content: cfg\n    depends_on: [svc]\n  data:\n    type: file\n    machine: m\n    path: /tmp/data\n    content: d\n".to_string()
    }

    // ── cmd_graph_critical_path ──

    #[test]
    fn test_critical_path_with_chain() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_critical_path(f.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_critical_path_no_deps() {
        let f = write_temp_config(&config_no_deps());
        let result = cmd_graph_critical_path(f.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_critical_path_diamond() {
        let f = write_temp_config(&config_diamond());
        let result = cmd_graph_critical_path(f.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_critical_path_single_resource() {
        let f = write_temp_config(&config_single_resource());
        let result = cmd_graph_critical_path(f.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_critical_path_multi_machine() {
        let f = write_temp_config(&config_multi_machine());
        let result = cmd_graph_critical_path(f.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_critical_path_mixed_types() {
        let f = write_temp_config(&config_mixed_types());
        let result = cmd_graph_critical_path(f.path());
        assert!(result.is_ok());
    }

    // ── cmd_graph_affected ──

    #[test]
    fn test_affected_root_resource() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_affected(f.path(), "a");
        assert!(result.is_ok());
    }

    #[test]
    fn test_affected_middle_resource() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_affected(f.path(), "b");
        assert!(result.is_ok());
    }

    #[test]
    fn test_affected_leaf_resource() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_affected(f.path(), "c");
        assert!(result.is_ok());
    }

    #[test]
    fn test_affected_nonexistent_resource() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_affected(f.path(), "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_affected_no_deps() {
        let f = write_temp_config(&config_no_deps());
        let result = cmd_graph_affected(f.path(), "x");
        assert!(result.is_ok());
    }

    #[test]
    fn test_affected_diamond_root() {
        let f = write_temp_config(&config_diamond());
        let result = cmd_graph_affected(f.path(), "root");
        assert!(result.is_ok());
    }

    #[test]
    fn test_affected_diamond_branch() {
        let f = write_temp_config(&config_diamond());
        let result = cmd_graph_affected(f.path(), "left");
        assert!(result.is_ok());
    }

    #[test]
    fn test_affected_single_resource() {
        let f = write_temp_config(&config_single_resource());
        let result = cmd_graph_affected(f.path(), "only");
        assert!(result.is_ok());
    }

    // ── cmd_graph_reverse ──

    #[test]
    fn test_reverse_with_deps() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_reverse(f.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_reverse_no_deps() {
        let f = write_temp_config(&config_no_deps());
        let result = cmd_graph_reverse(f.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_reverse_diamond() {
        let f = write_temp_config(&config_diamond());
        let result = cmd_graph_reverse(f.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_reverse_single_resource() {
        let f = write_temp_config(&config_single_resource());
        let result = cmd_graph_reverse(f.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_reverse_multi_machine() {
        let f = write_temp_config(&config_multi_machine());
        let result = cmd_graph_reverse(f.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_reverse_mixed_types() {
        let f = write_temp_config(&config_mixed_types());
        let result = cmd_graph_reverse(f.path());
        assert!(result.is_ok());
    }

    // ── cmd_graph_change_impact ──

    #[test]
    fn test_change_impact_root_text() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_change_impact(f.path(), "a", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_impact_root_json() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_change_impact(f.path(), "a", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_impact_leaf_text() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_change_impact(f.path(), "c", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_impact_leaf_json() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_change_impact(f.path(), "c", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_impact_middle_text() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_change_impact(f.path(), "b", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_impact_middle_json() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_change_impact(f.path(), "b", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_impact_nonexistent() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_change_impact(f.path(), "nonexistent", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_change_impact_no_deps_text() {
        let f = write_temp_config(&config_no_deps());
        let result = cmd_graph_change_impact(f.path(), "x", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_impact_no_deps_json() {
        let f = write_temp_config(&config_no_deps());
        let result = cmd_graph_change_impact(f.path(), "x", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_impact_diamond_root_json() {
        let f = write_temp_config(&config_diamond());
        let result = cmd_graph_change_impact(f.path(), "root", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_impact_diamond_root_text() {
        let f = write_temp_config(&config_diamond());
        let result = cmd_graph_change_impact(f.path(), "root", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_impact_multi_machine_json() {
        let f = write_temp_config(&config_multi_machine());
        let result = cmd_graph_change_impact(f.path(), "schema", true);
        assert!(result.is_ok());
    }

    // ── compute_change_impact (exercised through cmd_graph_change_impact) ──
    // The above tests already exercise compute_change_impact transitively.
    // Additional edge cases below target the BFS traversal paths explicitly.

    #[test]
    fn test_change_impact_single_resource_text() {
        let f = write_temp_config(&config_single_resource());
        let result = cmd_graph_change_impact(f.path(), "only", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_impact_single_resource_json() {
        let f = write_temp_config(&config_single_resource());
        let result = cmd_graph_change_impact(f.path(), "only", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_impact_diamond_branch_json() {
        let f = write_temp_config(&config_diamond());
        let result = cmd_graph_change_impact(f.path(), "left", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_change_impact_diamond_bottom_text() {
        let f = write_temp_config(&config_diamond());
        let result = cmd_graph_change_impact(f.path(), "bottom", false);
        assert!(result.is_ok());
    }

    // ── cmd_graph_resource_types ──

    #[test]
    fn test_resource_types_text() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_resource_types(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_types_json() {
        let f = write_temp_config(&config_with_deps());
        let result = cmd_graph_resource_types(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_types_mixed_text() {
        let f = write_temp_config(&config_mixed_types());
        let result = cmd_graph_resource_types(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_types_mixed_json() {
        let f = write_temp_config(&config_mixed_types());
        let result = cmd_graph_resource_types(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_types_no_deps_text() {
        let f = write_temp_config(&config_no_deps());
        let result = cmd_graph_resource_types(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_types_no_deps_json() {
        let f = write_temp_config(&config_no_deps());
        let result = cmd_graph_resource_types(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_types_multi_machine_text() {
        let f = write_temp_config(&config_multi_machine());
        let result = cmd_graph_resource_types(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_types_multi_machine_json() {
        let f = write_temp_config(&config_multi_machine());
        let result = cmd_graph_resource_types(f.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_types_single_text() {
        let f = write_temp_config(&config_single_resource());
        let result = cmd_graph_resource_types(f.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_types_single_json() {
        let f = write_temp_config(&config_single_resource());
        let result = cmd_graph_resource_types(f.path(), true);
        assert!(result.is_ok());
    }
}

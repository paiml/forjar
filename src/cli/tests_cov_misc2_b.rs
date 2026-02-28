//! Tests: Coverage for plan and show (part 2).

use super::plan::*;
use super::show::*;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write_yaml(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
        p
    }

    fn minimal_config_yaml() -> &'static str {
        r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /tmp/test.txt
    content: "hello"
"#
    }

    fn two_machine_config_yaml() -> &'static str {
        r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web
    addr: 127.0.0.1
  db:
    hostname: db
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /tmp/test.txt
    content: "hello"
  db-cfg:
    type: file
    machine: db
    path: /tmp/db.txt
    content: "db"
"#
    }

    // ========================================================================
    // plan::cmd_plan_compact
    // ========================================================================

    #[test]
    fn test_plan_compact_basic_plain() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        assert!(cmd_plan_compact(&file, &state_dir, None, false).is_ok());
    }

    #[test]
    fn test_plan_compact_basic_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        assert!(cmd_plan_compact(&file, &state_dir, None, true).is_ok());
    }

    #[test]
    fn test_plan_compact_with_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "forjar.yaml", two_machine_config_yaml());
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        assert!(cmd_plan_compact(&file, &state_dir, Some("web"), false).is_ok());
    }

    #[test]
    fn test_plan_compact_with_machine_filter_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "forjar.yaml", two_machine_config_yaml());
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        assert!(cmd_plan_compact(&file, &state_dir, Some("db"), true).is_ok());
    }

    #[test]
    fn test_plan_compact_no_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state_dir = dir.path().join("nonexistent_state");
        assert!(cmd_plan_compact(&file, &state_dir, None, false).is_ok());
    }

    #[test]
    fn test_plan_compact_bad_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "forjar.yaml", "invalid: [[[");
        let state_dir = dir.path().join("state");
        assert!(cmd_plan_compact(&file, &state_dir, None, false).is_err());
    }

    #[test]
    fn test_plan_compact_empty_resources() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = "version: \"1.0\"\nname: empty\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources: {}\n";
        let file = write_yaml(dir.path(), "forjar.yaml", yaml);
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        assert!(cmd_plan_compact(&file, &state_dir, None, false).is_ok());
    }

    #[test]
    fn test_plan_compact_empty_resources_json() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = "version: \"1.0\"\nname: empty\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources: {}\n";
        let file = write_yaml(dir.path(), "forjar.yaml", yaml);
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        assert!(cmd_plan_compact(&file, &state_dir, None, true).is_ok());
    }

    // ========================================================================
    // show::cmd_compare
    // ========================================================================

    #[test]
    fn test_compare_identical_configs_plain() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_yaml(dir.path(), "a.yaml", minimal_config_yaml());
        let f2 = write_yaml(dir.path(), "b.yaml", minimal_config_yaml());
        assert!(cmd_compare(&f1, &f2, false).is_ok());
    }

    #[test]
    fn test_compare_identical_configs_json() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_yaml(dir.path(), "a.yaml", minimal_config_yaml());
        let f2 = write_yaml(dir.path(), "b.yaml", minimal_config_yaml());
        assert!(cmd_compare(&f1, &f2, true).is_ok());
    }

    #[test]
    fn test_compare_different_resources_plain() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_yaml(dir.path(), "a.yaml", minimal_config_yaml());
        let yaml2 = "version: \"1.0\"\nname: test\nmachines:\n  web:\n    hostname: web\n    addr: 127.0.0.1\nresources:\n  other:\n    type: file\n    machine: web\n    path: /tmp/other.txt\n    content: \"world\"\n";
        let f2 = write_yaml(dir.path(), "b.yaml", yaml2);
        assert!(cmd_compare(&f1, &f2, false).is_ok());
    }

    #[test]
    fn test_compare_different_resources_json() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_yaml(dir.path(), "a.yaml", minimal_config_yaml());
        let yaml2 = "version: \"1.0\"\nname: test\nmachines:\n  web:\n    hostname: web\n    addr: 127.0.0.1\nresources:\n  other:\n    type: file\n    machine: web\n    path: /tmp/other.txt\n    content: \"world\"\n";
        let f2 = write_yaml(dir.path(), "b.yaml", yaml2);
        assert!(cmd_compare(&f1, &f2, true).is_ok());
    }

    #[test]
    fn test_compare_modified_resource() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_yaml(dir.path(), "a.yaml", minimal_config_yaml());
        let yaml2 = "version: \"1.0\"\nname: test\nmachines:\n  web:\n    hostname: web\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: web\n    path: /tmp/test.txt\n    content: \"modified\"\n";
        let f2 = write_yaml(dir.path(), "b.yaml", yaml2);
        assert!(cmd_compare(&f1, &f2, false).is_ok());
    }

    #[test]
    fn test_compare_modified_resource_json() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_yaml(dir.path(), "a.yaml", minimal_config_yaml());
        let yaml2 = "version: \"1.0\"\nname: test\nmachines:\n  web:\n    hostname: web\n    addr: 127.0.0.1\nresources:\n  cfg:\n    type: file\n    machine: web\n    path: /tmp/test.txt\n    content: \"modified\"\n";
        let f2 = write_yaml(dir.path(), "b.yaml", yaml2);
        assert!(cmd_compare(&f1, &f2, true).is_ok());
    }

    #[test]
    fn test_compare_empty_resources() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = "version: \"1.0\"\nname: empty\nmachines:\n  m1:\n    hostname: m1\n    addr: 127.0.0.1\nresources: {}\n";
        let f1 = write_yaml(dir.path(), "a.yaml", yaml);
        let f2 = write_yaml(dir.path(), "b.yaml", yaml);
        assert!(cmd_compare(&f1, &f2, false).is_ok());
    }

    #[test]
    fn test_compare_bad_first_config() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_yaml(dir.path(), "a.yaml", "invalid: [[[");
        let f2 = write_yaml(dir.path(), "b.yaml", minimal_config_yaml());
        assert!(cmd_compare(&f1, &f2, false).is_err());
    }

    // ========================================================================
    // show::cmd_template
    // ========================================================================

    #[test]
    fn test_template_simple_expansion_plain() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = write_yaml(dir.path(), "recipe.yaml", "host: {{inputs.hostname}}\nport: {{inputs.port}}");
        let vars = vec!["hostname=myserver".to_string(), "port=8080".to_string()];
        assert!(cmd_template(&recipe, &vars, false).is_ok());
    }

    #[test]
    fn test_template_simple_expansion_json() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = write_yaml(dir.path(), "recipe.yaml", "host: {{inputs.hostname}}\nport: {{inputs.port}}");
        let vars = vec!["hostname=myserver".to_string(), "port=8080".to_string()];
        assert!(cmd_template(&recipe, &vars, true).is_ok());
    }

    #[test]
    fn test_template_no_vars() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = write_yaml(dir.path(), "recipe.yaml", "static: content\nno: templates\n");
        assert!(cmd_template(&recipe, &[], false).is_ok());
    }

    #[test]
    fn test_template_no_vars_json() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = write_yaml(dir.path(), "recipe.yaml", "static: content\nno: templates\n");
        assert!(cmd_template(&recipe, &[], true).is_ok());
    }

    #[test]
    fn test_template_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = write_yaml(dir.path(), "empty.yaml", "");
        assert!(cmd_template(&recipe, &[], false).is_ok());
    }

    #[test]
    fn test_template_nonexistent_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.yaml");
        let result = cmd_template(&missing, &[], false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot read"));
    }

    #[test]
    fn test_template_partial_var_match() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = write_yaml(dir.path(), "recipe.yaml", "host: {{inputs.hostname}}\nport: {{inputs.port}}");
        let vars = vec!["hostname=server1".to_string()];
        assert!(cmd_template(&recipe, &vars, false).is_ok());
    }

    #[test]
    fn test_template_var_without_equals() {
        let dir = tempfile::tempdir().unwrap();
        let recipe = write_yaml(dir.path(), "recipe.yaml", "content: here");
        let vars = vec!["no_equals_here".to_string()];
        assert!(cmd_template(&recipe, &vars, false).is_ok());
    }
}

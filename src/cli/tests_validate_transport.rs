//! Tests for validate_transport (Phase 96 — FJ-1030, FJ-1033, FJ-1036).

use super::validate_transport::*;
use crate::core::types;

#[cfg(test)]
mod tests {
    use super::*;

    // -- FJ-1030: extract_input_references --

    #[test]
    fn test_extract_input_references_basic() {
        let refs = extract_input_references("server={{inputs.hostname}}:{{inputs.port}}");
        assert_eq!(refs, vec!["hostname", "port"]);
    }

    #[test]
    fn test_extract_input_references_none() {
        let refs = extract_input_references("no template variables here");
        assert!(refs.is_empty());
    }

    #[test]
    fn test_extract_input_references_nested_braces() {
        let refs = extract_input_references("{{inputs.ok}}{{notinputs.skip}}");
        assert_eq!(refs, vec!["ok"]);
    }

    #[test]
    fn test_extract_input_references_underscore() {
        let refs = extract_input_references("{{inputs.my_var}}");
        assert_eq!(refs, vec!["my_var"]);
    }

    #[test]
    fn test_extract_input_references_empty_name() {
        let refs = extract_input_references("{{inputs.}}");
        assert!(refs.is_empty());
    }

    // -- FJ-1033: hash_content --

    #[test]
    fn test_hash_content_deterministic() {
        let h1 = hash_content("hello world");
        let h2 = hash_content("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_content_different() {
        let h1 = hash_content("hello");
        let h2 = hash_content("world");
        assert_ne!(h1, h2);
    }

    // -- FJ-1030: recipe input completeness with tempfile --

    #[test]
    fn test_recipe_input_completeness_ok() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web1
    addr: 10.0.0.1
resources:
  my-config:
    type: file
    machine: web
    path: /etc/app.conf
    content: "host={{inputs.hostname}}"
    inputs:
      hostname: web1.example.com
"#;
        let tmp = std::env::temp_dir().join("fj1030_ok.yaml");
        std::fs::write(&tmp, yaml).unwrap();
        let result = cmd_validate_check_recipe_input_completeness(&tmp, false);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_recipe_input_completeness_missing() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web1
    addr: 10.0.0.1
resources:
  my-config:
    type: file
    machine: web
    path: /etc/app.conf
    content: "host={{inputs.hostname}} port={{inputs.port}}"
    inputs:
      hostname: web1.example.com
"#;
        let tmp = std::env::temp_dir().join("fj1030_miss.yaml");
        std::fs::write(&tmp, yaml).unwrap();
        let result = cmd_validate_check_recipe_input_completeness(&tmp, false);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_recipe_input_completeness_json() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources:
  cfg:
    type: file
    path: /etc/a.conf
    content: "val={{inputs.x}}"
"#;
        let tmp = std::env::temp_dir().join("fj1030_json.yaml");
        std::fs::write(&tmp, yaml).unwrap();
        let result = cmd_validate_check_recipe_input_completeness(&tmp, true);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    // -- FJ-1033: content hash consistency with tempfile --

    #[test]
    fn test_content_hash_no_duplicates() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web1
    addr: 10.0.0.1
  db:
    hostname: db1
    addr: 10.0.0.2
resources:
  web-conf:
    type: file
    machine: web
    path: /etc/web.conf
    content: "web config"
  db-conf:
    type: file
    machine: db
    path: /etc/db.conf
    content: "db config"
"#;
        let tmp = std::env::temp_dir().join("fj1033_nodup.yaml");
        std::fs::write(&tmp, yaml).unwrap();
        let result = cmd_validate_check_resource_cross_machine_content_duplicates(&tmp, false);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_content_hash_with_duplicates() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web1
    addr: 10.0.0.1
  db:
    hostname: db1
    addr: 10.0.0.2
resources:
  web-conf:
    type: file
    machine: web
    path: /etc/app.conf
    content: "identical content"
  db-conf:
    type: file
    machine: db
    path: /etc/app.conf
    content: "identical content"
"#;
        let tmp = std::env::temp_dir().join("fj1033_dup.yaml");
        std::fs::write(&tmp, yaml).unwrap();
        let result = cmd_validate_check_resource_cross_machine_content_duplicates(&tmp, false);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_content_hash_json_output() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  a:
    hostname: a1
    addr: 10.0.0.1
  b:
    hostname: b1
    addr: 10.0.0.2
resources:
  r1:
    type: file
    machine: a
    path: /x
    content: "same"
  r2:
    type: file
    machine: b
    path: /y
    content: "same"
"#;
        let tmp = std::env::temp_dir().join("fj1033_json.yaml");
        std::fs::write(&tmp, yaml).unwrap();
        let result = cmd_validate_check_resource_cross_machine_content_duplicates(&tmp, true);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    // -- FJ-1036: machine affinity with tempfile --

    #[test]
    fn test_machine_affinity_all_valid() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web1
    addr: 10.0.0.1
resources:
  pkg:
    type: package
    machine: web
    packages: [nginx]
"#;
        let tmp = std::env::temp_dir().join("fj1036_ok.yaml");
        std::fs::write(&tmp, yaml).unwrap();
        let result = cmd_validate_check_resource_machine_reference_validity(&tmp, false);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_machine_affinity_localhost_always_valid() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources:
  pkg:
    type: package
    machine: localhost
    packages: [curl]
"#;
        let tmp = std::env::temp_dir().join("fj1036_local.yaml");
        std::fs::write(&tmp, yaml).unwrap();
        let result = cmd_validate_check_resource_machine_reference_validity(&tmp, false);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_machine_affinity_undefined_machine() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web1
    addr: 10.0.0.1
resources:
  pkg:
    type: package
    machine: ghost
    packages: [nginx]
"#;
        let tmp = std::env::temp_dir().join("fj1036_undef.yaml");
        std::fs::write(&tmp, yaml).unwrap();
        let result = cmd_validate_check_resource_machine_reference_validity(&tmp, false);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_machine_affinity_json_output() {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources:
  pkg:
    type: package
    machine: missing
    packages: [nginx]
"#;
        let tmp = std::env::temp_dir().join("fj1036_json.yaml");
        std::fs::write(&tmp, yaml).unwrap();
        let result = cmd_validate_check_resource_machine_reference_validity(&tmp, true);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_machine_affinity_multiple_targets() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web1
    addr: 10.0.0.1
resources:
  pkg:
    type: package
    machine:
      - web
      - db
    packages: [nginx]
"#;
        let tmp = std::env::temp_dir().join("fj1036_multi.yaml");
        std::fs::write(&tmp, yaml).unwrap();
        let result = cmd_validate_check_resource_machine_reference_validity(&tmp, false);
        assert!(result.is_ok());
        std::fs::remove_file(&tmp).ok();
    }

    // -- FJ-1030: find_recipe_input_completeness_gaps unit --

    #[test]
    fn test_find_gaps_no_content() {
        let config = make_config_no_content();
        let gaps = find_recipe_input_completeness_gaps(&config);
        assert!(gaps.is_empty());
    }

    #[test]
    fn test_find_content_hash_same_machine_no_warn() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web1
    addr: 10.0.0.1
resources:
  r1:
    type: file
    machine: web
    path: /a
    content: "dup"
  r2:
    type: file
    machine: web
    path: /b
    content: "dup"
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dupes = find_content_hash_duplicates(&config);
        assert!(dupes.is_empty(), "same machine duplicates should not warn");
    }

    fn make_config_no_content() -> types::ForjarConfig {
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources:
  pkg:
    type: package
    packages: [curl]
"#;
        serde_yaml_ng::from_str(yaml).unwrap()
    }
}

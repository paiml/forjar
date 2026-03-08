//! Tests: FJ-1411 data validation checks.

#![allow(unused_imports)]
use super::commands::*;
use super::data_validate::*;
use super::dispatch::*;
use super::helpers::*;
use super::test_fixtures::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
        let file = dir.join("forjar.yaml");
        std::fs::write(&file, yaml).unwrap();
        file
    }

    #[test]
    fn test_fj1411_validate_basic() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: validate\nmachines: {}\nresources: {}\n",
        );
        cmd_data_validate(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1411_validate_with_source() {
        let dir = tempfile::tempdir().unwrap();
        // Create source file
        std::fs::write(dir.path().join("data.csv"), "id,name\n1,alice\n").unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: validate-src
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  data-loader:
    type: file
    machine: m
    path: /tmp/data.csv
    source: data.csv
"#,
        );
        cmd_data_validate(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1411_validate_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: json\nmachines: {}\nresources: {}\n",
        );
        cmd_data_validate(&file, None, true).unwrap();
    }

    #[test]
    fn test_fj1411_validate_with_store() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: store\nmachines: {}\nresources: {}\n",
        );
        let store_dir = dir.path().join("store");
        std::fs::create_dir_all(&store_dir).unwrap();
        std::fs::write(store_dir.join("artifact.tar"), b"data").unwrap();
        cmd_data_validate(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1411_validate_missing_source() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: missing
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  data:
    type: file
    machine: m
    path: /tmp/data.csv
    source: nonexistent.csv
"#,
        );
        // Should fail because source file doesn't exist
        let result = cmd_data_validate(&file, None, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj1411_validate_resource_filter_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("src.txt"), "hello").unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: filter
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  data-a:
    type: file
    machine: m
    path: /tmp/a
    source: src.txt
  data-b:
    type: file
    machine: m
    path: /tmp/b
    source: nonexistent.csv
"#,
        );
        // Filtering to data-a only should pass (src.txt exists)
        cmd_data_validate(&file, Some("data-a"), false).unwrap();
    }

    #[test]
    fn test_fj1411_validate_resource_filter_miss() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: filter-miss
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  data-a:
    type: file
    machine: m
    path: /tmp/a
    source: nonexistent.csv
"#,
        );
        // Filter to non-existent resource → no checks → pass
        cmd_data_validate(&file, Some("data-z"), false).unwrap();
    }

    #[test]
    fn test_fj1411_validate_content_hash() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: content-hash
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  config-file:
    type: file
    machine: m
    path: /etc/app/config.yaml
    content: "port: 8080\nhost: 0.0.0.0\n"
"#,
        );
        cmd_data_validate(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1411_validate_output_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("app.bin"), b"binary").unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: artifacts
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  build:
    type: task
    machine: m
    command: "make build"
    output_artifacts:
      - app.bin
"#,
        );
        cmd_data_validate(&file, None, false).unwrap();
    }

    #[test]
    fn test_fj1411_validate_missing_artifact_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: missing-art
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  build:
    type: task
    machine: m
    command: "make"
    output_artifacts:
      - nonexistent-binary
"#,
        );
        let result = cmd_data_validate(&file, None, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj1411_validate_empty_source_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("empty.csv"), "").unwrap();
        let file = write_config(
            dir.path(),
            r#"
version: "1.0"
name: empty-src
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  data:
    type: file
    machine: m
    path: /tmp/data.csv
    source: empty.csv
"#,
        );
        // Empty file should fail non-empty check
        let result = cmd_data_validate(&file, None, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj1411_validate_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_config(
            dir.path(),
            "version: \"1.0\"\nname: dispatch\nmachines: {}\nresources: {}\n",
        );
        dispatch(
            Commands::DataValidate(DataValidateArgs {
                file,
                resource: None,
                json: false,
            }),
            0,
            true,
        )
        .unwrap();
    }
}

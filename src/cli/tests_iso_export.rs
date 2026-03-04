//! Tests: FJ-1422 ISO distribution export.

#![allow(unused_imports)]
use super::helpers::*;
use super::iso_export::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
        let p = dir.join("forjar.yaml");
        std::fs::write(&p, yaml).unwrap();
        p
    }

    #[test]
    fn test_iso_export_basic() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let output = dir.path().join("iso-out");
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test-iso
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: local
    provider: apt
    packages: [curl]
"#,
        );
        let result = cmd_iso_export(&p, &state, &output, false, false);
        assert!(result.is_ok());
        assert!(output.join("config/forjar.yaml").exists());
        assert!(output.join("manifest.json").exists());
    }

    #[test]
    fn test_iso_export_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let output = dir.path().join("iso-json");
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/test
    content: "hello"
"#,
        );
        let result = cmd_iso_export(&p, &state, &output, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_iso_export_with_state() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let machine_state = state.join("local");
        std::fs::create_dir_all(&machine_state).unwrap();
        std::fs::write(
            machine_state.join("state.lock.yaml"),
            "resources: {}\n",
        )
        .unwrap();
        let output = dir.path().join("iso-state");
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: local
    provider: apt
    packages: [curl]
"#,
        );
        let result = cmd_iso_export(&p, &state, &output, false, false);
        assert!(result.is_ok());
        assert!(output.join("state").exists());
    }

    #[test]
    fn test_iso_export_manifest_has_blake3() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let output = dir.path().join("iso-hash");
        let p = write_config(
            dir.path(),
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/test
    content: "test"
"#,
        );
        cmd_iso_export(&p, &state, &output, false, false).unwrap();
        let manifest = std::fs::read_to_string(output.join("manifest.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&manifest).unwrap();
        assert!(v["blake3_root"].as_str().unwrap().len() == 64);
    }

    #[test]
    fn test_iso_file_serde() {
        let f = IsoFile {
            path: "config/forjar.yaml".to_string(),
            size: 100,
            blake3: "a".repeat(64),
            category: "config".to_string(),
        };
        let json = serde_json::to_string(&f).unwrap();
        assert!(json.contains("\"category\":\"config\""));
    }
}

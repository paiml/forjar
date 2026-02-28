//! Tests: Coverage for secrets, doctor, plan, show.

use super::secrets::*;
use super::doctor::*;
use super::plan::*;
use super::show::*;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

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
    // secrets::cmd_secrets_rotate
    // ========================================================================

    #[test]
    fn test_secrets_rotate_no_re_encrypt_flag() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "config.yaml", "password: secret");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_secrets_rotate(
            &file,
            None,
            &["age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqpq5qsq".to_string()],
            false, // re_encrypt = false
            &state_dir,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--re-encrypt"));
    }

    #[test]
    fn test_secrets_rotate_no_markers() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "config.yaml", "password: plaintext_value");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_secrets_rotate(
            &file,
            None,
            &["age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqpq5qsq".to_string()],
            true,
            &state_dir,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_secrets_rotate_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "empty.yaml", "");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_secrets_rotate(
            &file,
            None,
            &["age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqpq5qsq".to_string()],
            true,
            &state_dir,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_secrets_rotate_nonexistent_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_secrets_rotate(
            &missing,
            None,
            &["age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqpq5qsq".to_string()],
            true,
            &state_dir,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot read"));
    }

    #[test]
    fn test_secrets_rotate_no_re_encrypt_with_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "cfg.yaml", "");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_secrets_rotate(
            &file,
            None,
            &[],
            false,
            &state_dir,
        );
        assert!(result.is_err());
    }

    // ========================================================================
    // secrets::cmd_secrets_rekey
    // ========================================================================

    #[test]
    fn test_secrets_rekey_no_markers() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "config.yaml", "password: plaintext_value");
        let result = cmd_secrets_rekey(
            &file,
            None,
            &["age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqpq5qsq".to_string()],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_secrets_rekey_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "empty.yaml", "");
        let result = cmd_secrets_rekey(
            &file,
            None,
            &["age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqpq5qsq".to_string()],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_secrets_rekey_nonexistent_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.yaml");
        let result = cmd_secrets_rekey(
            &missing,
            None,
            &["age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqpq5qsq".to_string()],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cannot read"));
    }

    #[test]
    fn test_secrets_rekey_multiline_no_markers() {
        let dir = tempfile::tempdir().unwrap();
        let content = "key1: value1\nkey2: value2\nkey3: value3\n";
        let file = write_yaml(dir.path(), "multi.yaml", content);
        let result = cmd_secrets_rekey(
            &file,
            None,
            &["age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqpq5qsq".to_string()],
        );
        assert!(result.is_ok());
    }

    // ========================================================================
    // doctor::cmd_doctor_network
    // ========================================================================

    #[test]
    fn test_doctor_network_local_machine_plain() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let result = cmd_doctor_network(Some(&file), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_doctor_network_local_machine_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let result = cmd_doctor_network(Some(&file), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_doctor_network_two_local_machines() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "forjar.yaml", two_machine_config_yaml());
        let result = cmd_doctor_network(Some(&file), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_doctor_network_two_local_machines_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "forjar.yaml", two_machine_config_yaml());
        let result = cmd_doctor_network(Some(&file), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_doctor_network_localhost_addr() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: localhost
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/t.txt
    content: "x"
"#;
        let file = write_yaml(dir.path(), "forjar.yaml", yaml);
        let result = cmd_doctor_network(Some(&file), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_doctor_network_bad_config() {
        let dir = tempfile::tempdir().unwrap();
        let file = write_yaml(dir.path(), "forjar.yaml", "invalid: [[[");
        let result = cmd_doctor_network(Some(&file), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_doctor_network_empty_machines() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#;
        let file = write_yaml(dir.path(), "forjar.yaml", yaml);
        let result = cmd_doctor_network(Some(&file), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_doctor_network_empty_machines_json() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = r#"
version: "1.0"
name: test
machines: {}
resources: {}
"#;
        let file = write_yaml(dir.path(), "forjar.yaml", yaml);
        let result = cmd_doctor_network(Some(&file), true);
        assert!(result.is_ok());
    }

}


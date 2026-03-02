//! Tests for `forjar pin` CLI commands.

#[cfg(test)]
mod tests {
    use crate::cli::store_pin::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_config(dir: &TempDir) -> std::path::PathBuf {
        let file = dir.path().join("forjar.yaml");
        fs::write(
            &file,
            r#"
version: "1.0"
name: test
resources:
  nginx:
    type: package
    machine: target
    provider: apt
    packages: [nginx]
    version: "1.24.0"
  curl:
    type: package
    machine: target
    provider: apt
    packages: [curl]
  data-dir:
    type: file
    machine: target
    path: /data
"#,
        )
        .unwrap();
        file
    }

    #[test]
    fn test_pin_creates_lockfile() {
        let dir = TempDir::new().unwrap();
        let file = write_config(&dir);
        let state_dir = dir.path().join("state");
        fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_pin(&file, &state_dir, false);
        assert!(result.is_ok(), "cmd_pin failed: {:?}", result);

        let lock_path = state_dir.join("forjar.inputs.lock.yaml");
        assert!(lock_path.exists(), "lock file not created");

        let content = fs::read_to_string(&lock_path).unwrap();
        assert!(content.contains("nginx"));
        assert!(content.contains("curl"));
        assert!(content.contains("data-dir"));
    }

    #[test]
    fn test_pin_json_output() {
        let dir = TempDir::new().unwrap();
        let file = write_config(&dir);
        let state_dir = dir.path().join("state");
        fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_pin(&file, &state_dir, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pin_deterministic_hashes() {
        let dir = TempDir::new().unwrap();
        let file = write_config(&dir);
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();

        cmd_pin(&file, &state, false).unwrap();
        let content1 = fs::read_to_string(state.join("forjar.inputs.lock.yaml")).unwrap();

        cmd_pin(&file, &state, false).unwrap();
        let content2 = fs::read_to_string(state.join("forjar.inputs.lock.yaml")).unwrap();

        assert_eq!(content1, content2, "pin should be deterministic");
    }

    #[test]
    fn test_pin_update_no_change() {
        let dir = TempDir::new().unwrap();
        let file = write_config(&dir);
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();

        cmd_pin(&file, &state, false).unwrap();
        let result = cmd_pin_update(&file, &state, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pin_update_specific() {
        let dir = TempDir::new().unwrap();
        let file = write_config(&dir);
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();

        cmd_pin(&file, &state, false).unwrap();
        let result = cmd_pin_update(&file, &state, Some("nginx"), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pin_check_fresh() {
        let dir = TempDir::new().unwrap();
        let file = write_config(&dir);
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();

        cmd_pin(&file, &state, false).unwrap();
        let result = cmd_pin_check(&file, &state, false);
        assert!(result.is_ok(), "fresh lock should pass: {:?}", result);
    }

    #[test]
    fn test_pin_check_missing_lockfile() {
        let dir = TempDir::new().unwrap();
        let file = write_config(&dir);
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();

        let result = cmd_pin_check(&file, &state, false);
        assert!(result.is_err(), "missing lock file should fail");
    }

    #[test]
    fn test_pin_check_json() {
        let dir = TempDir::new().unwrap();
        let file = write_config(&dir);
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();

        cmd_pin(&file, &state, false).unwrap();
        let result = cmd_pin_check(&file, &state, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pin_update_json() {
        let dir = TempDir::new().unwrap();
        let file = write_config(&dir);
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();

        cmd_pin(&file, &state, false).unwrap();
        let result = cmd_pin_update(&file, &state, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pin_invalid_config() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("bad.yaml");
        fs::write(&file, "not: valid: yaml: {{{}").unwrap();
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();

        let result = cmd_pin(&file, &state, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_pin_no_resources() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("empty.yaml");
        fs::write(&file, "version: '1.0'\nname: test\n").unwrap();
        let state = dir.path().join("state");
        fs::create_dir_all(&state).unwrap();

        let result = cmd_pin(&file, &state, false);
        assert!(result.is_err());
    }
}

//! Tests for `forjar convert --reproducible` CLI command.

#[cfg(test)]
mod tests {
    use crate::cli::store_convert::*;
    use std::fs;
    use tempfile::TempDir;

    fn config_mixed(dir: &TempDir) -> std::path::PathBuf {
        let file = dir.path().join("forjar.yaml");
        fs::write(
            &file,
            r#"
version: "1.0"
name: mixed-recipe
resources:
  pinned-pkg:
    type: package
    machine: target
    provider: apt
    packages: [nginx]
    version: "1.24.0"
    store: true
  unpinned-pkg:
    type: package
    machine: target
    provider: apt
    packages: [curl]
  file-resource:
    type: file
    machine: target
    path: /etc/app.conf
    content: "hello"
"#,
        )
        .unwrap();
        file
    }

    fn config_pure(dir: &TempDir) -> std::path::PathBuf {
        let file = dir.path().join("pure.yaml");
        fs::write(
            &file,
            r#"
version: "1.0"
name: pure-recipe
resources:
  pinned:
    type: package
    machine: target
    provider: apt
    packages: [nginx]
    version: "1.24.0"
    store: true
    sandbox:
      level: full
"#,
        )
        .unwrap();
        file
    }

    #[test]
    fn test_convert_mixed() {
        let dir = TempDir::new().unwrap();
        let file = config_mixed(&dir);

        let result = cmd_convert(&file, true, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_mixed_json() {
        let dir = TempDir::new().unwrap();
        let file = config_mixed(&dir);

        let result = cmd_convert(&file, true, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_pure() {
        let dir = TempDir::new().unwrap();
        let file = config_pure(&dir);

        let result = cmd_convert(&file, true, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_no_flag() {
        let dir = TempDir::new().unwrap();
        let file = config_mixed(&dir);

        let result = cmd_convert(&file, false, false, false);
        assert!(result.is_err(), "should require --reproducible flag");
    }

    #[test]
    fn test_convert_invalid_config() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("bad.yaml");
        fs::write(&file, "not valid yaml {{{}").unwrap();

        let result = cmd_convert(&file, true, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_no_resources() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("empty.yaml");
        fs::write(&file, "version: '1.0'\nname: test\n").unwrap();

        let result = cmd_convert(&file, true, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_missing_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("nope.yaml");

        let result = cmd_convert(&file, true, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_apply_creates_backup() {
        let dir = TempDir::new().unwrap();
        let file = config_mixed(&dir);

        let result = cmd_convert(&file, true, true, false);
        assert!(result.is_ok());

        let backup = dir.path().join("forjar.yaml.bak");
        assert!(backup.exists(), "apply should create backup");
    }

    #[test]
    fn test_convert_apply_json() {
        let dir = TempDir::new().unwrap();
        let file = config_mixed(&dir);

        let result = cmd_convert(&file, true, true, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_apply_pure_noop() {
        let dir = TempDir::new().unwrap();
        let file = config_pure(&dir);

        // Pure config has no changes to apply
        let result = cmd_convert(&file, true, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_convert_apply_missing_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("gone.yaml");

        let result = cmd_convert(&file, true, true, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_apply_no_reproducible() {
        let dir = TempDir::new().unwrap();
        let file = config_mixed(&dir);

        // --apply without --reproducible should fail
        let result = cmd_convert(&file, false, true, false);
        assert!(result.is_err());
    }
}

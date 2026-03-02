//! Tests for FJ-1306 / FJ-1329: --check-recipe-purity and --check-reproducibility-score.

#[cfg(test)]
mod tests {
    use crate::cli::validate_store_purity::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_config(dir: &TempDir, yaml: &str) -> std::path::PathBuf {
        let path = dir.path().join("forjar.yaml");
        fs::write(&path, yaml).unwrap();
        path
    }

    fn pure_config() -> &'static str {
        r#"resources:
  nginx:
    type: package
    provider: apt
    version: "1.24.0"
    store: true
    sandbox:
      isolation: namespace
"#
    }

    fn mixed_config() -> &'static str {
        r#"resources:
  nginx:
    type: package
    provider: apt
    version: "1.24.0"
    store: true
    sandbox:
      isolation: namespace
  redis:
    type: package
    provider: apt
  curl-installer:
    type: exec
    provider: shell
    content: "curl https://example.com | bash"
"#
    }

    fn constrained_config() -> &'static str {
        r#"resources:
  nginx:
    type: package
    provider: apt
"#
    }

    #[test]
    fn purity_pure_config() {
        let dir = TempDir::new().unwrap();
        let config = write_config(&dir, pure_config());
        let result = cmd_validate_check_recipe_purity(&config, false);
        assert!(result.is_ok());
    }

    #[test]
    fn purity_pure_config_json() {
        let dir = TempDir::new().unwrap();
        let config = write_config(&dir, pure_config());
        let result = cmd_validate_check_recipe_purity(&config, true);
        assert!(result.is_ok());
    }

    #[test]
    fn purity_mixed_config() {
        let dir = TempDir::new().unwrap();
        let config = write_config(&dir, mixed_config());
        let result = cmd_validate_check_recipe_purity(&config, false);
        assert!(result.is_ok());
    }

    #[test]
    fn purity_constrained_config() {
        let dir = TempDir::new().unwrap();
        let config = write_config(&dir, constrained_config());
        let result = cmd_validate_check_recipe_purity(&config, false);
        assert!(result.is_ok());
    }

    #[test]
    fn purity_missing_file() {
        let dir = TempDir::new().unwrap();
        let config = dir.path().join("missing.yaml");
        let result = cmd_validate_check_recipe_purity(&config, false);
        assert!(result.is_err());
    }

    #[test]
    fn purity_no_resources_section() {
        let dir = TempDir::new().unwrap();
        let config = write_config(&dir, "name: test\nmachines:\n  - hostname: web\n");
        let result = cmd_validate_check_recipe_purity(&config, false);
        assert!(result.is_err());
    }

    #[test]
    fn repro_score_pure_config() {
        let dir = TempDir::new().unwrap();
        let config = write_config(&dir, pure_config());
        let result = cmd_validate_check_reproducibility_score(&config, false);
        assert!(result.is_ok());
    }

    #[test]
    fn repro_score_pure_config_json() {
        let dir = TempDir::new().unwrap();
        let config = write_config(&dir, pure_config());
        let result = cmd_validate_check_reproducibility_score(&config, true);
        assert!(result.is_ok());
    }

    #[test]
    fn repro_score_mixed_config() {
        let dir = TempDir::new().unwrap();
        let config = write_config(&dir, mixed_config());
        let result = cmd_validate_check_reproducibility_score(&config, false);
        assert!(result.is_ok());
    }

    #[test]
    fn repro_score_with_lock_file() {
        let dir = TempDir::new().unwrap();
        let config = write_config(&dir, pure_config());
        // Write a lock file that contains the nginx pin
        fs::write(
            dir.path().join("forjar.inputs.lock.yaml"),
            "schema: \"1.0\"\npins:\n  nginx:\n    provider: apt\n    hash: blake3:abc\n",
        )
        .unwrap();
        let result = cmd_validate_check_reproducibility_score(&config, false);
        assert!(result.is_ok());
    }

    #[test]
    fn repro_score_missing_file() {
        let dir = TempDir::new().unwrap();
        let config = dir.path().join("missing.yaml");
        let result = cmd_validate_check_reproducibility_score(&config, false);
        assert!(result.is_err());
    }

    #[test]
    fn repro_score_constrained() {
        let dir = TempDir::new().unwrap();
        let config = write_config(&dir, constrained_config());
        let result = cmd_validate_check_reproducibility_score(&config, false);
        assert!(result.is_ok());
    }
}

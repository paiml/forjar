//! Tests: FJ-1395 SBOM generation.

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn test_fj1395_sbom_empty_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            "version: '1.0'\nname: test\nmachines:\n  local:\n    hostname: localhost
    addr: localhost\nresources: {}\n",
        )
        .unwrap();
        // Should succeed with 0 components
        let result = super::super::sbom::cmd_sbom(&config_path, &state_dir, false);
        assert!(result.is_ok(), "sbom failed: {:?}", result.err());
    }

    #[test]
    fn test_fj1395_sbom_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"version: '1.0'
name: test-sbom
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  base-pkgs:
    type: package
    machine: local
    provider: apt
    packages:
      - nginx
      - curl
"#,
        )
        .unwrap();
        let result = super::super::sbom::cmd_sbom(&config_path, &state_dir, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj1395_sbom_docker_image() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"version: '1.0'
name: docker-test
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  web:
    type: docker
    machine: local
    name: web-container
    image: nginx:1.25
"#,
        )
        .unwrap();
        let result = super::super::sbom::cmd_sbom(&config_path, &state_dir, false);
        assert!(result.is_ok(), "docker sbom: {:?}", result.err());
    }

    #[test]
    fn test_fj1395_sbom_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result =
            super::super::sbom::cmd_sbom(Path::new("/nonexistent.yaml"), &state_dir, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj1395_sbom_dispatch() {
        use crate::cli::commands::{Commands, SbomArgs};
        let cmd = Commands::Sbom(SbomArgs {
            file: std::path::PathBuf::from("forjar.yaml"),
            state_dir: std::path::PathBuf::from("state"),
            json: false,
        });
        match cmd {
            Commands::Sbom(SbomArgs { json, .. }) => assert!(!json),
            _ => panic!("expected Sbom"),
        }
    }
}

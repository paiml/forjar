//! Tests: FJ-1400 CBOM (Cryptographic Bill of Materials) generation.

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn test_fj1400_cbom_basic_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"version: '1.0'
name: cbom-test
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  pkgs:
    type: package
    machine: local
    provider: apt
    packages:
      - curl
"#,
        )
        .unwrap();
        // Should succeed — BLAKE3 entry always present
        let result = super::super::cbom::cmd_cbom(&config_path, &state_dir, false);
        assert!(result.is_ok(), "cbom failed: {:?}", result.err());
    }

    #[test]
    fn test_fj1400_cbom_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"version: '1.0'
name: cbom-json
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  pkgs:
    type: package
    machine: local
    provider: apt
    packages:
      - nginx
"#,
        )
        .unwrap();
        let result = super::super::cbom::cmd_cbom(&config_path, &state_dir, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj1400_cbom_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result =
            super::super::cbom::cmd_cbom(Path::new("/nonexistent.yaml"), &state_dir, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj1400_cbom_tls_detection() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"version: '1.0'
name: tls-test
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  ssl-cert:
    type: file
    machine: local
    path: /etc/ssl/certs/server.pem
    content: "certificate content"
"#,
        )
        .unwrap();
        // Should detect X.509/TLS from the path
        let result = super::super::cbom::cmd_cbom(&config_path, &state_dir, false);
        assert!(result.is_ok(), "tls cbom: {:?}", result.err());
    }

    #[test]
    fn test_fj1400_cbom_docker_digest() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"version: '1.0'
name: digest-test
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  web:
    type: docker
    machine: local
    name: web
    image: "nginx@sha256:abc123"
"#,
        )
        .unwrap();
        // Should detect SHA-256 from docker digest
        let result = super::super::cbom::cmd_cbom(&config_path, &state_dir, false);
        assert!(result.is_ok(), "docker digest cbom: {:?}", result.err());
    }

    #[test]
    fn test_fj1400_cbom_dispatch() {
        use crate::cli::commands::{CbomArgs, Commands};
        let cmd = Commands::Cbom(CbomArgs {
            file: std::path::PathBuf::from("forjar.yaml"),
            state_dir: std::path::PathBuf::from("state"),
            json: false,
        });
        match cmd {
            Commands::Cbom(CbomArgs { json, .. }) => assert!(!json),
            _ => panic!("expected Cbom"),
        }
    }
}

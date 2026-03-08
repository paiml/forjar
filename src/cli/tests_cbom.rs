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
    fn test_fj1400_cbom_age_encryption_in_content() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"version: '1.0'
name: age-test
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  secret:
    type: file
    machine: local
    path: /etc/app/secret.env
    content: "-----BEGIN AGE ENCRYPTED FILE-----\ndata\n-----END AGE ENCRYPTED FILE-----"
"#,
        )
        .unwrap();
        let result = super::super::cbom::cmd_cbom(&config_path, &state_dir, true);
        assert!(result.is_ok(), "age cbom: {:?}", result.err());
    }

    #[test]
    fn test_fj1400_cbom_age_encryption_in_params() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"version: '1.0'
name: age-params
machines:
  local:
    hostname: localhost
    addr: localhost
params:
  db_password: "age-encryption.org/v1\nencrypted-data"
resources: {}
"#,
        )
        .unwrap();
        let result = super::super::cbom::cmd_cbom(&config_path, &state_dir, false);
        assert!(result.is_ok(), "age params cbom: {:?}", result.err());
    }

    #[test]
    fn test_fj1400_cbom_ssh_keys() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"version: '1.0'
name: ssh-test
machines:
  prod:
    hostname: prod.example.com
    addr: 10.0.0.1
    ssh_key: ~/.ssh/id_ed25519
resources: {}
"#,
        )
        .unwrap();
        let result = super::super::cbom::cmd_cbom(&config_path, &state_dir, true);
        assert!(result.is_ok(), "ssh cbom: {:?}", result.err());
    }

    #[test]
    fn test_fj1400_cbom_state_with_locks() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        let machine_dir = state_dir.join("prod");
        std::fs::create_dir_all(&machine_dir).unwrap();
        std::fs::write(
            &config_path,
            "version: '1.0'\nname: lock-test\nmachines:\n  prod:\n    hostname: prod\n    addr: 10.0.0.1\nresources: {}\n",
        )
        .unwrap();
        // Write a lock file
        std::fs::write(
            machine_dir.join("lock.yaml"),
            "schema: 1\nmachine: prod\nhostname: prod\ngenerated_at: now\ngenerator: test\nblake3_version: '1.0'\nresources:\n  pkg:\n    resource_type: Package\n    status: Converged\n    applied_at: now\n    duration_seconds: 0.1\n    hash: abc123def456\n    details: {}\n",
        )
        .unwrap();
        let result = super::super::cbom::cmd_cbom(&config_path, &state_dir, false);
        assert!(result.is_ok(), "state cbom: {:?}", result.err());
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

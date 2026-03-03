//! Tests: FJ-1401 convergence proof.

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn test_fj1401_prove_basic() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"version: '1.0'
name: prove-test
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  pkgs:
    type: package
    provider: apt
    machine: local
    packages:
      - curl
"#,
        )
        .unwrap();
        let result = super::super::prove::cmd_prove(&config_path, &state_dir, None, false);
        assert!(result.is_ok(), "prove failed: {:?}", result.err());
    }

    #[test]
    fn test_fj1401_prove_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"version: '1.0'
name: prove-json
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  web:
    type: service
    machine: local
    name: nginx
"#,
        )
        .unwrap();
        let result = super::super::prove::cmd_prove(&config_path, &state_dir, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj1401_prove_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result =
            super::super::prove::cmd_prove(Path::new("/nonexistent.yaml"), &state_dir, None, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj1401_prove_multi_resource() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"version: '1.0'
name: prove-multi
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  pkgs:
    type: package
    provider: apt
    machine: local
    packages:
      - curl
      - wget
  web:
    type: service
    machine: local
    name: nginx
    depends_on:
      - pkgs
  config:
    type: file
    machine: local
    path: /etc/nginx/nginx.conf
    content: "server {}"
    depends_on:
      - web
"#,
        )
        .unwrap();
        let result = super::super::prove::cmd_prove(&config_path, &state_dir, None, false);
        assert!(result.is_ok(), "multi prove: {:?}", result.err());
    }

    #[test]
    fn test_fj1401_prove_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"version: '1.0'
name: prove-filter
machines:
  web:
    hostname: web-01
    addr: web-01
  db:
    hostname: db-01
    addr: db-01
resources:
  web-pkgs:
    type: package
    provider: apt
    machine: web
    packages:
      - nginx
  db-pkgs:
    type: package
    provider: apt
    machine: db
    packages:
      - postgresql
"#,
        )
        .unwrap();
        let result =
            super::super::prove::cmd_prove(&config_path, &state_dir, Some("web"), false);
        assert!(result.is_ok(), "filtered prove: {:?}", result.err());
    }

    #[test]
    fn test_fj1401_prove_empty_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            "version: '1.0'\nname: empty\nmachines:\n  local:\n    hostname: localhost\n    addr: localhost\nresources: {}\n",
        )
        .unwrap();
        let result = super::super::prove::cmd_prove(&config_path, &state_dir, None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj1401_prove_dispatch() {
        use crate::cli::commands::{Commands, ProveArgs};
        let cmd = Commands::Prove(ProveArgs {
            file: std::path::PathBuf::from("forjar.yaml"),
            state_dir: std::path::PathBuf::from("state"),
            machine: None,
            json: false,
        });
        match cmd {
            Commands::Prove(ProveArgs { json, .. }) => assert!(!json),
            _ => panic!("expected Prove"),
        }
    }
}

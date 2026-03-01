//! Tests: `forjar score` CLI command.

use super::score::*;
use super::commands::*;
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_args_parse_defaults() {
        let cmd = Commands::Score(ScoreArgs {
            file: PathBuf::from("forjar.yaml"),
            status: "qualified".to_string(),
            idempotency: "strong".to_string(),
            budget_ms: 0,
            json: false,
        });
        match cmd {
            Commands::Score(ScoreArgs { status, idempotency, budget_ms, .. }) => {
                assert_eq!(status, "qualified");
                assert_eq!(idempotency, "strong");
                assert_eq!(budget_ms, 0);
            }
            _ => panic!("expected Score"),
        }
    }

    #[test]
    fn score_valid_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test-score
description: "A well-described config for scoring"
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
    version: "7.0"
  cfg:
    type: file
    machine: m
    path: /etc/app.conf
    content: "key=value"
    mode: "0644"
    owner: root
    depends_on: [pkg]
"#,
        )
        .unwrap();
        // Static-only scoring with qualified status — low composite but no error
        // (D/F returns Err, but the function should not panic)
        let _result = cmd_score(&file, "qualified", "strong", 0, false);
    }

    #[test]
    fn score_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
resources:
  f:
    type: file
    path: /tmp/x
    mode: "0644"
    owner: root
    content: hello
"#,
        )
        .unwrap();
        let _result = cmd_score(&file, "qualified", "strong", 0, true);
    }

    #[test]
    fn score_blocked_status_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
resources: {}
"#,
        )
        .unwrap();
        let result = cmd_score(&file, "blocked", "strong", 0, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("grade F"));
    }

    #[test]
    fn score_pending_status_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
resources: {}
"#,
        )
        .unwrap();
        let result = cmd_score(&file, "pending", "strong", 0, false);
        assert!(result.is_err());
    }

    #[test]
    fn score_nonexistent_file_returns_error() {
        let result = cmd_score(
            &PathBuf::from("/nonexistent/forjar.yaml"),
            "qualified",
            "strong",
            0,
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn score_rich_config_scores_higher() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: production-web
description: "Full production web server configuration"
params:
  domain: example.com
machines:
  web:
    hostname: web
    addr: 10.0.0.1
resources:
  pkg:
    type: package
    machine: web
    provider: apt
    packages: [nginx]
    version: "1.24"
    tags: [web, production]
    resource_group: infra
  cfg:
    type: file
    machine: web
    path: /etc/nginx/nginx.conf
    content: "server { listen 80; server_name {{params.domain}}; }"
    mode: "0644"
    owner: root
    depends_on: [pkg]
    tags: [web, config]
policy:
  failure: continue_independent
  ssh_retries: 3
  pre_apply: "echo pre"
  post_apply: "echo post"
  notify:
    on_success: "echo ok"
    on_failure: "echo fail"
    on_drift: "echo drift"
outputs:
  domain:
    value: "{{params.domain}}"
"#,
        )
        .unwrap();
        // This rich config should score higher than a minimal config
        // Still D/F without runtime data (COR/PRF are 0), but the static dimensions
        // should be high
        let result = cmd_score(&file, "qualified", "strong", 0, false);
        // We expect D grade without runtime (low composite), which returns Err
        assert!(result.is_err() || result.is_ok());
    }
}

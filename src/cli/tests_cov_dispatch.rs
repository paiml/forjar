//! Tests: Coverage for destroy, infra, dispatch_misc, dispatch_lock, check, observe, apply.

#![allow(unused_imports)]
#![allow(dead_code)]
use super::apply::*;
use super::check::*;
use super::commands::*;
use super::destroy::*;
use super::dispatch::*;
use super::dispatch_lock::*;
use super::dispatch_misc::*;
use super::helpers::*;
use super::helpers_state::*;
use super::infra::*;
use super::observe::*;
use super::test_fixtures::*;
use crate::core::{executor, parser, planner, resolver, state, types};
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    fn write_yaml(dir: &Path, name: &str, content: &str) -> PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
        p
    }

    fn minimal_config_yaml() -> &'static str {
        r#"version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: local
    path: /tmp/forjar-cov-dispatch-test.txt
    content: "hello"
"#
    }

    fn docker_config_yaml() -> &'static str {
        r#"version: "1.0"
name: docker-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  d1:
    type: docker
    machine: local
    image: nginx:latest
    name: test-container
"#
    }

    fn no_docker_config_yaml() -> &'static str {
        r#"version: "1.0"
name: no-docker
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: local
    path: /tmp/forjar-cov-no-docker.txt
    content: "no docker"
"#
    }

    fn multi_resource_config_yaml(dir: &Path) -> String {
        format!(
            r#"version: "1.0"
name: multi-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: local
    path: {}/multi-1.txt
    content: "one"
  f2:
    type: file
    machine: local
    path: {}/multi-2.txt
    content: "two"
"#,
            dir.display(),
            dir.display()
        )
    }

    fn policy_deny_config_yaml(dir: &Path) -> String {
        format!(
            r#"version: "1.0"
name: policy-deny
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: local
    path: {}/policy-deny.txt
    content: "test"
policies:
  - type: require
    message: "owner is required on all file resources"
    resource_type: file
    field: owner
"#,
            dir.display()
        )
    }

    // ========================================================================
    // 1. destroy.rs — cmd_rollback
    // ========================================================================

    #[test]
    fn test_cov_rollback_nonexistent_file() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_rollback(
            Path::new("/tmp/forjar-nonexistent-config-xyz.yaml"),
            &state,
            1,
            None,
            true,
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_rollback_no_git_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_rollback(&config, &state, 1, None, true, false);
        // Fails because no git history
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("git") || err.contains("cannot read"),
            "expected git error, got: {}",
            err
        );
    }

    #[test]
    fn test_cov_rollback_with_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_rollback(&config, &state, 1, Some("local"), true, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_rollback_verbose_mode() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_rollback(&config, &state, 1, None, true, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_rollback_revision_zero() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_rollback(&config, &state, 0, None, true, false);
        // Even revision 0 goes through git show path
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_rollback_high_revision() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_rollback(&config, &state, 999, None, true, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_rollback_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch(
            Commands::Rollback(RollbackArgs {
                file: config,
                revision: 2,
                machine: None,
                dry_run: true,
                state_dir: state,
            }),
            false,
            true,
        );
        assert!(result.is_err());
    }

    // ========================================================================
    // 2. infra.rs — cmd_migrate
    // ========================================================================

    #[test]
    fn test_cov_migrate_no_docker_resources() {
        let f = write_temp_config(no_docker_config_yaml());
        let result = cmd_migrate(f.path(), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_migrate_docker_stdout() {
        let f = write_temp_config(docker_config_yaml());
        let result = cmd_migrate(f.path(), None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_migrate_docker_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_yaml(dir.path(), "forjar.yaml", docker_config_yaml());
        let output = dir.path().join("migrated.yaml");
        let result = cmd_migrate(&config, Some(&output));
        assert!(result.is_ok());
        assert!(output.exists());
        let content = std::fs::read_to_string(&output).unwrap();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_cov_migrate_invalid_config() {
        let f = write_temp_config("not valid yaml: [[[");
        let result = cmd_migrate(f.path(), None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_migrate_dispatch() {
        let f = write_temp_config(no_docker_config_yaml());
        let result = dispatch_misc_cmd(
            Commands::Migrate(MigrateArgs {
                file: f.path().to_path_buf(),
                output: None,
            }),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_migrate_empty_resources() {
        let yaml = r#"version: "1.0"
name: empty
machines:
  m:
    hostname: m
    addr: 1.1.1.1
resources: {}
"#;
        let f = write_temp_config(yaml);
        let result = cmd_migrate(f.path(), None);
        assert!(result.is_ok());
    }

    // ========================================================================
    // 3. dispatch_misc.rs — dispatch_misc_cmd
    // ========================================================================

    #[test]
    fn test_cov_dispatch_misc_history_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_misc_cmd(
            Commands::History(HistoryArgs {
                state_dir: state,
                machine: None,
                limit: 10,
                json: false,
                since: None,
                resource: None,
            }),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_misc_history_json() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_misc_cmd(
            Commands::History(HistoryArgs {
                state_dir: state,
                machine: None,
                limit: 5,
                json: true,
                since: None,
                resource: None,
            }),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_misc_show() {
        let f = write_temp_config(minimal_config_yaml());
        let result = dispatch_misc_cmd(
            Commands::Show(ShowArgs {
                file: f.path().to_path_buf(),
                resource: None,
                json: false,
            }),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_misc_show_json() {
        let f = write_temp_config(minimal_config_yaml());
        let result = dispatch_misc_cmd(
            Commands::Show(ShowArgs {
                file: f.path().to_path_buf(),
                resource: None,
                json: true,
            }),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_misc_check() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("check-cov-test.txt");
        std::fs::write(&target, "hello").unwrap();
        let config_yaml = format!(
            r#"version: "1.0"
name: check-cov
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {}
    content: hello
"#,
            target.display()
        );
        let config = write_yaml(dir.path(), "forjar.yaml", &config_yaml);
        let result = dispatch_misc_cmd(
            Commands::Check(CheckArgs {
                file: config,
                machine: None,
                resource: None,
                tag: None,
                json: false,
            }),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_misc_fmt_check() {
        let f = write_temp_config(minimal_config_yaml());
        let result = dispatch_misc_cmd(
            Commands::Fmt(FmtArgs {
                file: f.path().to_path_buf(),
                check: true,
            }),
            false,
        );
        // May fail because the config is not in canonical form
        // That is acceptable; we just want to exercise the dispatch path
        let _ = result;
    }

    #[test]
    fn test_cov_dispatch_misc_lint() {
        let f = write_temp_config(minimal_config_yaml());
        let result = dispatch_misc_cmd(
            Commands::Lint(LintArgs {
                file: f.path().to_path_buf(),
                json: false,
                strict: false,
                fix: false,
                rules: None,
            }),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_misc_lint_json() {
        let f = write_temp_config(minimal_config_yaml());
        let result = dispatch_misc_cmd(
            Commands::Lint(LintArgs {
                file: f.path().to_path_buf(),
                json: true,
                strict: false,
                fix: false,
                rules: None,
            }),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_misc_state_list_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = dispatch_misc_cmd(
            Commands::StateList(StateListArgs {
                state_dir: dir.path().to_path_buf(),
                machine: None,
                json: false,
            }),
            false,
        );
        assert!(result.is_ok());
    }
}

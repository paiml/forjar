//! Tests: Coverage for apply edge cases, dispatch_lock extras (part 4).

#![allow(unused_imports)]
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

    #[test]
    fn test_cov_apply_with_confirm_destructive_no_yes() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("apply-cd.txt");
        let config_yaml = format!(
            r#"version: "1.0"
name: apply-cd
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: local
    path: {}
    content: "confirm"
"#,
            target.display()
        );
        let config = write_yaml(dir.path(), "forjar.yaml", &config_yaml);
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        // confirm_destructive=true, yes=false, dry_run=false
        // With no existing state, there are no destroy actions, so it should proceed
        // but then prompt for confirmation (which will fail on stdin)
        let result = cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            false,
            None,
            false,
            false,
            0,
            false, // yes=false — will fail on stdin prompt
            false,
            None,
            false,
            None,
            None,
            None,
            true, // confirm_destructive
            None,
            false,
            None, // telemetry_endpoint
            false, // refresh
            None, // force_tag
        );
        // This will either fail on "aborted by user" or stdin error
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_apply_with_subset_filter() {
        let dir = tempfile::tempdir().unwrap();
        let t1 = dir.path().join("subset-1.txt");
        let t2 = dir.path().join("subset-2.txt");
        let config_yaml = format!(
            r#"version: "1.0"
name: subset-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  web-config:
    type: file
    machine: local
    path: {}
    content: "web"
  db-config:
    type: file
    machine: local
    path: {}
    content: "db"
"#,
            t1.display(),
            t2.display()
        );
        let config = write_yaml(dir.path(), "forjar.yaml", &config_yaml);
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            false,
            None,
            false,
            false,
            0,
            true, // yes
            false,
            None,
            false,
            None,
            None,
            Some("web*"), // subset — only web-config
            false,
            None,
            false,
            None, // telemetry_endpoint
            false, // refresh
            None, // force_tag
        );
        assert!(result.is_ok());
        assert!(t1.exists());
        assert!(!t2.exists());
        let _ = std::fs::remove_file(&t1);
    }

    #[test]
    fn test_cov_apply_with_exclude_filter() {
        let dir = tempfile::tempdir().unwrap();
        let t1 = dir.path().join("excl-1.txt");
        let t2 = dir.path().join("excl-2.txt");
        let config_yaml = format!(
            r#"version: "1.0"
name: exclude-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  web-config:
    type: file
    machine: local
    path: {}
    content: "web"
  db-config:
    type: file
    machine: local
    path: {}
    content: "db"
"#,
            t1.display(),
            t2.display()
        );
        let config = write_yaml(dir.path(), "forjar.yaml", &config_yaml);
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            false,
            None,
            false,
            false,
            0,
            true, // yes
            false,
            None,
            false,
            None,
            None,
            None,
            false,
            Some("db*"), // exclude — remove db-config
            false,
            None, // telemetry_endpoint
            false, // refresh
            None, // force_tag
        );
        assert!(result.is_ok());
        assert!(t1.exists());
        assert!(!t2.exists());
        let _ = std::fs::remove_file(&t1);
    }

    #[test]
    fn test_cov_apply_subset_no_match() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_yaml(dir.path(), "forjar.yaml", minimal_config_yaml());
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            false,
            None,
            false,
            false,
            0,
            true,
            false,
            None,
            false,
            None,
            None,
            Some("nonexistent-pattern*"), // subset that matches nothing
            false,
            None,
            false,
            None, // telemetry_endpoint
            false, // refresh
            None, // force_tag
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("no resources match"), "got: {err}");
    }

    #[test]
    fn test_cov_apply_dry_run_json() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("dry-json.txt");
        let config_yaml = format!(
            r#"version: "1.0"
name: dry-json
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: local
    path: {}
    content: "dry json"
"#,
            target.display()
        );
        let config = write_yaml(dir.path(), "forjar.yaml", &config_yaml);
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None,
            false,
            true, // dry_run
            false,
            &[],
            false,
            None,
            true, // json
            false,
            None,
            None,
            false,
            false,
            None,
            false,
            false,
            0,
            true,
            false,
            None,
            false,
            None,
            None,
            None,
            false,
            None,
            false,
            None, // telemetry_endpoint
            false, // refresh
            None, // force_tag
        );
        assert!(result.is_ok());
    }

    // ========================================================================
    // Additional edge case tests
    // ========================================================================

    #[test]
    fn test_cov_dispatch_misc_state_mv_same() {
        let dir = tempfile::tempdir().unwrap();
        let result = dispatch_misc_cmd(
            Commands::StateMv(StateMvArgs {
                old_id: "same".to_string(),
                new_id: "same".to_string(),
                state_dir: dir.path().to_path_buf(),
                machine: None,
            }),
            false,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_dispatch_misc_state_rm_missing() {
        let dir = tempfile::tempdir().unwrap();
        let result = dispatch_misc_cmd(
            Commands::StateRm(StateRmArgs {
                resource_id: "missing".to_string(),
                state_dir: dir.path().to_path_buf(),
                machine: None,
                force: false,
            }),
            false,
        );
        assert!(result.is_err());
    }

}

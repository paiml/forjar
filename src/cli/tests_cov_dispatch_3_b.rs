//! Tests: Coverage for check, observe, apply (part 3).

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
    // 7. apply.rs — apply_pre_validate (line 169, 55% cov)
    // ========================================================================

    #[test]
    fn test_cov_apply_pre_validate_no_policies() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("apply-pv.txt");
        let config_yaml = format!(
            r#"version: "1.0"
name: apply-pv
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: local
    path: {}
    content: "test"
"#,
            target.display()
        );
        let config = write_yaml(dir.path(), "forjar.yaml", &config_yaml);
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        // Apply with yes=true, dry_run=false — no policies means it proceeds
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
            None,
            false,
        );
        assert!(result.is_ok());
        // Clean up
        let _ = std::fs::remove_file(&target);
    }

    #[test]
    fn test_cov_apply_pre_validate_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("apply-dry.txt");
        let config_yaml = format!(
            r#"version: "1.0"
name: apply-dry
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: local
    path: {}
    content: "dry run test"
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
            None,
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_apply_pre_validate_with_policy_deny() {
        let dir = tempfile::tempdir().unwrap();
        let config_yaml = policy_deny_config_yaml(dir.path());
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
            None,
            false,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("policy") || err.contains("denied") || err.contains("block"),
            "expected policy error, got: {err}"
        );
    }
}

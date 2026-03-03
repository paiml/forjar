//! Tests: Apply command.

#![allow(unused_imports)]
use super::apply::*;
use super::commands::*;
use super::dispatch::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj017_apply_with_results_summary() {
        // Tests the full apply path with real local execution, covering the
        // results iteration and summary output lines
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();

        let target = dir.path().join("apply-summary.txt");
        std::fs::write(
            &config,
            format!(
                r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  summary-file:
    type: file
    machine: local
    path: {}
    content: "summary test"
"#,
                target.display()
            ),
        )
        .unwrap();

        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None, // no group filter
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
            false,
            None,  // no env_file
            None,  // no workspace
            false, // no report
            false, // no force_unlock
            None,  // no output mode
            false, // no progress
            false, // no timing
            0,     // no retry
            true,  // yes (skip prompt)
            false,
            None,
            false,
            None,
            None,
            None,  // subset
            false, // confirm_destructive
            None,  // exclude
            false, // sequential
        )
        .unwrap();
        assert!(target.exists());

        // Second apply — should be unchanged (NoOp)
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None, // no group filter
            false,
            false,
            false,
            &[],
            false,
            None, // no timeout
            false,
            false,
            None,  // no env_file
            None,  // no workspace
            false, // no report
            false, // no force_unlock
            None,  // no output mode
            false, // no progress
            false, // no timing
            0,     // no retry
            true,  // yes (skip prompt)
            false,
            None,
            false,
            None,
            None,
            None,  // subset
            false, // confirm_destructive
            None,  // exclude
            false, // sequential
        )
        .unwrap();
    }

    #[test]
    fn test_fj017_apply_with_param_override() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: param-test
params:
  env: dev
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  conf:
    type: file
    machine: local
    path: /tmp/forjar-param-test.txt
    content: "env={{params.env}}"
"#,
        )
        .unwrap();
        // Apply with param override in dry-run
        cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None, // no group filter
            false,
            true, // dry-run
            false,
            &["env=prod".to_string()],
            false,
            None,
            false,
            false,
            None,  // no env_file
            None,  // no workspace
            false, // no report
            false, // no force_unlock
            None,  // no output mode
            false, // no progress
            false, // no timing
            0,     // no retry
            true,  // yes (skip prompt)
            false,
            None,
            false,
            None,
            None,
            None,  // subset
            false, // confirm_destructive
            None,  // exclude
            false, // sequential
        )
        .unwrap();
    }

    #[test]
    fn test_fj205_apply_json_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: /tmp/forjar-fj205-test.txt
    content: "x"
"#,
        )
        .unwrap();
        // Dry-run with json=true should succeed (dry run exits before JSON output)
        let result = cmd_apply(
            &config,
            &state,
            None,
            None,
            None,
            None, // no group filter
            false,
            true,
            false,
            &[],
            false,
            None, // no timeout
            true,
            false,
            None,  // no env_file
            None,  // no workspace
            false, // no report
            false, // no force_unlock
            None,  // no output mode
            false, // no progress
            false, // no timing
            0,     // no retry
            true,  // yes (skip prompt)
            false,
            None,
            false,
            None,
            None,
            None,  // subset
            false, // confirm_destructive
            None,  // exclude
            false, // sequential
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj205_apply_result_serialize() {
        // Verify ApplyResult serializes correctly (duration as f64 seconds)
        use crate::core::types::ApplyResult;
        let result = ApplyResult {
            machine: "web".to_string(),
            resources_converged: 3,
            resources_unchanged: 1,
            resources_failed: 0,
            total_duration: std::time::Duration::from_millis(1500),
            resource_reports: Vec::new(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"machine\":\"web\""));
        assert!(json.contains("\"resources_converged\":3"));
        assert!(json.contains("\"total_duration\":1.5"));
    }
}

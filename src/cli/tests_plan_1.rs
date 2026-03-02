//! Tests: Plan command.

#![allow(unused_imports)]
use super::commands::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::plan::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj255_plan_with_diff() {
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
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m1
    path: /tmp/test.txt
    content: |
      line1
      line2
      line3
"#,
        )
        .unwrap();
        // no_diff=false → show diff
        cmd_plan(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            None,
            None,
            None,
            false,
            None,
            false, // no cost,
            &[],   // what_if
            None,  // plan_out
        )
        .unwrap();
    }

    #[test]
    fn test_fj255_plan_with_no_diff_flag() {
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
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m1
    path: /tmp/test.txt
    content: "hello"
"#,
        )
        .unwrap();
        // no_diff=true → suppress diff
        cmd_plan(
            &config,
            &state,
            None,
            None,
            None,
            false,
            false,
            None,
            None,
            None,
            true,
            None,
            false, // no cost,
            &[],   // what_if
            None,  // plan_out
        )
        .unwrap();
    }

    // ── FJ-256: forjar lock tests ────────────────────────────────

    #[test]
    fn test_fj312_plan_cost_flag_parse() {
        let cmd = Commands::Plan(PlanArgs {
            file: PathBuf::from("forjar.yaml"),
            machine: None,
            resource: None,
            tag: None,
            group: None,
            state_dir: PathBuf::from("state"),
            json: false,
            output_dir: None,
            env_file: None,
            workspace: None,
            no_diff: false,
            target: None,
            cost: true,
            what_if: vec![],
            out: None,
        });
        match cmd {
            Commands::Plan(PlanArgs { cost, .. }) => assert!(cost),
            _ => panic!("expected Plan"),
        }
    }

    // ── FJ-313: apply --max-parallel ──

    #[test]
    fn test_fj333_plan_what_if_flag() {
        let cmd = Commands::Plan(PlanArgs {
            file: PathBuf::from("f.yaml"),
            machine: None,
            resource: None,
            tag: None,
            group: None,
            state_dir: PathBuf::from("state"),
            json: false,
            output_dir: None,
            env_file: None,
            workspace: None,
            no_diff: false,
            target: None,
            cost: false,
            what_if: vec!["port=8080".to_string()],
            out: None,
        });
        match cmd {
            Commands::Plan(PlanArgs { what_if, .. }) => {
                assert_eq!(what_if.len(), 1);
                assert_eq!(what_if[0], "port=8080");
            }
            _ => panic!("expected Plan"),
        }
    }

    #[test]
    fn test_fj344_plan_compact_parse() {
        let cmd = Commands::PlanCompact(PlanCompactArgs {
            file: PathBuf::from("forjar.yaml"),
            state_dir: PathBuf::from("state"),
            machine: None,
            json: false,
        });
        match cmd {
            Commands::PlanCompact(PlanCompactArgs { file, .. }) => {
                assert_eq!(file, PathBuf::from("forjar.yaml"));
            }
            _ => panic!("expected PlanCompact"),
        }
    }
}

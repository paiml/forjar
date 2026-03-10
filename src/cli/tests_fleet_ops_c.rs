//! Tests: Fleet operations.

#![allow(unused_imports)]
use super::commands::*;
use super::fleet_ops::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj327_retry_failed_no_failures() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: test-retry
machines:
  local:
    hostname: local
    addr: 127.0.0.1
    user: test
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/fj327/test.txt
    content: hello
"#,
        )
        .unwrap();
        // No event logs → no failures to retry
        let result = cmd_retry_failed(&config_path, &state_dir, &[], None);
        assert!(result.is_ok());
    }
}

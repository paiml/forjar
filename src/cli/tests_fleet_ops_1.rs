//! Tests: Fleet operations.

use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::fleet_ops::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj325_canary_invalid_machine() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: test-canary
machines:
  web-1:
    hostname: web1
    addr: 127.0.0.1
    user: test
resources:
  cfg:
    type: file
    machine: web-1
    path: /tmp/fj325/test.txt
    content: hello
"#,
        )
        .unwrap();
        // Invalid canary machine should error
        let result = cmd_canary(&config_path, &state_dir, "nonexistent", false, &[], None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found in config"));
    }

    // ── Phase 19 tests ──

}

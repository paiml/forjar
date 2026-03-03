//! Tests: Misc.

#![allow(unused_imports)]
use super::commands::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::status_core::*;
use super::validate_core::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj311_strict_missing_description() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("forjar.yaml");
        std::fs::write(
            &file,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/test.txt
    content: "hello"
"#,
        )
        .unwrap();
        let result = cmd_validate(&file, true, false, false);
        assert!(result.is_err()); // missing description triggers strict error
    }
}

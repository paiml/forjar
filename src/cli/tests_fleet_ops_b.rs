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
    fn test_fj324_rolling_flag_parse() {
        let cmd = Commands::Rolling(RollingArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("state"),
            batch_size: 3,
            params: vec![],
            timeout: None,
        });
        match cmd {
            Commands::Rolling(RollingArgs { batch_size, .. }) => {
                assert_eq!(batch_size, 3);
            }
            _ => panic!("expected Rolling"),
        }
    }

    #[test]
    fn test_fj325_canary_flag_parse() {
        let cmd = Commands::Canary(CanaryArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("state"),
            machine: "web-1".to_string(),
            auto_proceed: true,
            params: vec![],
            timeout: None,
        });
        match cmd {
            Commands::Canary(CanaryArgs {
                machine,
                auto_proceed,
                ..
            }) => {
                assert_eq!(machine, "web-1");
                assert!(auto_proceed);
            }
            _ => panic!("expected Canary"),
        }
    }
}

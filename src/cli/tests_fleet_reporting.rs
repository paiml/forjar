//! Tests: Fleet reporting.

#![allow(unused_imports)]
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::fleet_reporting::*;
use super::commands::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj341_audit_command_parse() {
        let cmd = Commands::Audit(AuditArgs {
            state_dir: PathBuf::from("state"),
            machine: Some("gpu-box".to_string()),
            limit: 50,
            json: false,
        });
        match cmd {
            Commands::Audit(AuditArgs { machine, limit, .. }) => {
                assert_eq!(machine, Some("gpu-box".to_string()));
                assert_eq!(limit, 50);
            }
            _ => panic!("expected Audit"),
        }
    }


    #[test]
    fn test_fj341_audit_empty_state_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let state_dir = tmp.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let result = cmd_audit(&state_dir, None, 20, false);
        assert!(result.is_ok());
    }


    #[test]
    fn test_fj351_compliance_parse() {
        let cmd = Commands::Compliance(ComplianceArgs {
            file: PathBuf::from("forjar.yaml"),
            json: false,
        });
        match cmd {
            Commands::Compliance(ComplianceArgs { file, .. }) => {
                assert_eq!(file, PathBuf::from("forjar.yaml"));
            }
            _ => panic!("expected Compliance"),
        }
    }


    #[test]
    fn test_fj352_export_csv_format() {
        let cmd = Commands::Export(ExportArgs {
            state_dir: PathBuf::from("state"),
            format: "csv".to_string(),
            machine: None,
            output: None,
        });
        match cmd {
            Commands::Export(ExportArgs { format, .. }) => assert_eq!(format, "csv"),
            _ => panic!("expected Export"),
        }
    }


    #[test]
    fn test_fj361_suggest_parse() {
        let cmd = Commands::Suggest(SuggestArgs {
            file: PathBuf::from("forjar.yaml"),
            json: false,
        });
        match cmd {
            Commands::Suggest(SuggestArgs { file, .. }) => assert_eq!(file, PathBuf::from("forjar.yaml")),
            _ => panic!("expected Suggest"),
        }
    }

}

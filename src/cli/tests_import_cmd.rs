//! Tests: Import infrastructure.

#![allow(unused_imports)]
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::import_cmd::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj065_import_localhost() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        // Import just packages from localhost (most likely to succeed in test env)
        cmd_import(
            "localhost",
            "root",
            Some("test-machine"),
            &output,
            &["packages".to_string()],
            false,
        )
        .unwrap();

        // Output file should exist and be valid YAML
        assert!(output.exists());
        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("version: \"1.0\""));
        assert!(content.contains("test-machine"));
        assert!(content.contains("addr: localhost"));
    }


    #[test]
    fn test_fj065_import_generates_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            Some("local"),
            &output,
            &["packages".to_string()],
            false,
        )
        .unwrap();

        // The generated YAML should parse as a valid forjar config
        let content = std::fs::read_to_string(&output).unwrap();
        // Parse the YAML (strip comments that aren't YAML-compatible)
        let config: types::ForjarConfig = serde_yaml_ng::from_str(&content).unwrap();
        assert_eq!(config.version, "1.0");
        assert!(config.machines.contains_key("local"));
    }


    #[test]
    fn test_fj065_import_services_scan() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        // Import services from localhost
        cmd_import(
            "localhost",
            "root",
            Some("svc-box"),
            &output,
            &["services".to_string()],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("version: \"1.0\""));
        assert!(content.contains("svc-box"));
    }


    #[test]
    fn test_fj065_import_users_scan() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            Some("user-box"),
            &output,
            &["users".to_string()],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("version: \"1.0\""));
        assert!(content.contains("user-box"));
    }


    #[test]
    fn test_fj065_import_files_scan() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            Some("file-box"),
            &output,
            &["files".to_string()],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("version: \"1.0\""));
        assert!(content.contains("file-box"));
    }


    #[test]
    fn test_fj065_import_cron_scan() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            Some("cron-box"),
            &output,
            &["cron".to_string()],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("version: \"1.0\""));
        assert!(content.contains("cron-box"));
    }


    #[test]
    fn test_fj065_import_multi_scan() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            Some("multi-box"),
            &output,
            &[
                "packages".to_string(),
                "services".to_string(),
                "users".to_string(),
            ],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("version: \"1.0\""));
        assert!(content.contains("multi-box"));
    }


    #[test]
    fn test_fj065_import_verbose() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            Some("verbose-box"),
            &output,
            &["packages".to_string()],
            true, // verbose
        )
        .unwrap();

        assert!(output.exists());
    }


    #[test]
    fn test_fj065_import_default_name_localhost() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        cmd_import(
            "localhost",
            "root",
            None, // name derived from addr
            &output,
            &["packages".to_string()],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("localhost"));
    }


    #[test]
    fn test_fj065_import_default_name_ip() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("imported.yaml");

        // Use 127.0.0.1 — name should default to "localhost"
        cmd_import(
            "127.0.0.1",
            "root",
            None,
            &output,
            &["packages".to_string()],
            false,
        )
        .unwrap();

        let content = std::fs::read_to_string(&output).unwrap();
        assert!(content.contains("localhost"));
    }

    // ── Show command tests ─────────────────────────────────────

}

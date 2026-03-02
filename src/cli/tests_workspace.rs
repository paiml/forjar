//! Tests: Workspace management.

#![allow(unused_imports)]
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::workspace::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj210_resolve_state_dir_no_workspace() {
        let base = Path::new("state");
        let resolved = resolve_state_dir(base, None);
        // Without active workspace or flag, uses base as-is
        // (current_workspace() may or may not return something depending on env)
        // We just check it doesn't panic
        assert!(resolved.to_str().unwrap().starts_with("state"));
    }

    #[test]
    fn test_fj210_resolve_state_dir_with_flag() {
        let base = Path::new("state");
        let resolved = resolve_state_dir(base, Some("production"));
        assert_eq!(resolved, Path::new("state/production"));
    }

    #[test]
    fn test_fj210_inject_workspace_param() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
"#;
        let mut config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        inject_workspace_param(&mut config, Some("staging"));
        assert_eq!(
            config.params.get("workspace").unwrap(),
            &serde_yaml_ng::Value::String("staging".to_string())
        );
    }

    #[test]
    fn test_fj210_inject_workspace_param_default() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
"#;
        let mut config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        // No workspace flag and no .forjar/workspace file → "default"
        inject_workspace_param(&mut config, None);
        // Should have a workspace param
        assert!(config.params.contains_key("workspace"));
    }

    #[test]
    fn test_fj210_workspace_new_and_select() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let state_base = root.join("state");
        std::fs::create_dir_all(&state_base).unwrap();

        workspace_new_in(root, &state_base, "staging").unwrap();
        assert!(state_base.join("staging").exists());
        assert!(root.join(".forjar/workspace").exists());

        let ws = std::fs::read_to_string(root.join(".forjar/workspace")).unwrap();
        assert_eq!(ws.trim(), "staging");

        workspace_new_in(root, &state_base, "production").unwrap();
        let ws = std::fs::read_to_string(root.join(".forjar/workspace")).unwrap();
        assert_eq!(ws.trim(), "production");

        workspace_select_in(root, &state_base, "staging").unwrap();
        let ws = std::fs::read_to_string(root.join(".forjar/workspace")).unwrap();
        assert_eq!(ws.trim(), "staging");
    }

    #[test]
    fn test_fj210_workspace_list() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let state_base = root.join("state");
        std::fs::create_dir_all(&state_base).unwrap();

        workspace_new_in(root, &state_base, "dev").unwrap();
        workspace_new_in(root, &state_base, "prod").unwrap();
        workspace_list_in(root, &state_base).unwrap();
    }

    #[test]
    fn test_fj210_workspace_delete() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let state_base = root.join("state");
        std::fs::create_dir_all(&state_base).unwrap();

        workspace_new_in(root, &state_base, "temp").unwrap();
        assert!(state_base.join("temp").exists());

        // Without --yes, just prints warning
        workspace_delete_in(root, &state_base, "temp", false).unwrap();
        assert!(state_base.join("temp").exists());

        // With --yes, deletes
        workspace_delete_in(root, &state_base, "temp", true).unwrap();
        assert!(!state_base.join("temp").exists());
    }

    #[test]
    fn test_fj210_workspace_new_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let state_base = root.join("state");
        std::fs::create_dir_all(&state_base).unwrap();

        workspace_new_in(root, &state_base, "dup").unwrap();
        let result = workspace_new_in(root, &state_base, "dup");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_fj210_workspace_select_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let state_base = root.join("state");
        std::fs::create_dir_all(&state_base).unwrap();

        let result = workspace_select_in(root, &state_base, "nope");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_fj210_workspace_delete_clears_active() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let state_base = root.join("state");
        std::fs::create_dir_all(&state_base).unwrap();

        workspace_new_in(root, &state_base, "active-ws").unwrap();
        let ws = std::fs::read_to_string(root.join(".forjar/workspace")).unwrap();
        assert_eq!(ws.trim(), "active-ws");

        workspace_delete_in(root, &state_base, "active-ws", true).unwrap();
        assert!(!root.join(".forjar/workspace").exists());
    }

    #[test]
    fn test_fj210_workspace_current() {
        // Just verify it doesn't panic
        cmd_workspace_current().unwrap();
    }

    #[test]
    fn test_fj210_current_workspace_in() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // No workspace file → None
        assert!(current_workspace_in(root).is_none());

        // Create workspace file
        let state_base = root.join("state");
        std::fs::create_dir_all(&state_base).unwrap();
        workspace_new_in(root, &state_base, "test-ws").unwrap();

        assert_eq!(current_workspace_in(root).as_deref(), Some("test-ws"));
    }
}

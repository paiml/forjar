//! Tests: Snapshot management.

#![allow(unused_imports)]
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::snapshot::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj260_snapshot_save_and_list() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        // Create some state data
        let machine_dir = state_dir.join("m1");
        std::fs::create_dir_all(&machine_dir).unwrap();
        std::fs::write(machine_dir.join("state.lock.yaml"), "schema: '1.0'").unwrap();

        // Save snapshot
        cmd_snapshot_save("before-update", &state_dir).unwrap();

        // Verify snapshot dir exists
        let snap = state_dir.join("snapshots").join("before-update");
        assert!(snap.exists());
        assert!(snap.join(".snapshot.yaml").exists());
        assert!(snap.join("m1").join("state.lock.yaml").exists());

        // List snapshots
        cmd_snapshot_list(&state_dir, false).unwrap();
        cmd_snapshot_list(&state_dir, true).unwrap();
    }

    #[test]
    fn test_fj260_snapshot_save_duplicate_fails() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        cmd_snapshot_save("v1", &state_dir).unwrap();
        let result = cmd_snapshot_save("v1", &state_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_fj260_snapshot_restore() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        let machine_dir = state_dir.join("m1");
        std::fs::create_dir_all(&machine_dir).unwrap();
        std::fs::write(machine_dir.join("state.lock.yaml"), "version: 1").unwrap();

        // Save snapshot
        cmd_snapshot_save("checkpoint", &state_dir).unwrap();

        // Modify state
        std::fs::write(machine_dir.join("state.lock.yaml"), "version: 2").unwrap();
        assert_eq!(
            std::fs::read_to_string(machine_dir.join("state.lock.yaml")).unwrap(),
            "version: 2"
        );

        // Restore (without --yes should fail)
        let result = cmd_snapshot_restore("checkpoint", &state_dir, false);
        assert!(result.is_err());

        // Restore with --yes
        cmd_snapshot_restore("checkpoint", &state_dir, true).unwrap();
        assert_eq!(
            std::fs::read_to_string(machine_dir.join("state.lock.yaml")).unwrap(),
            "version: 1"
        );
    }

    #[test]
    fn test_fj260_snapshot_delete() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        cmd_snapshot_save("temp", &state_dir).unwrap();
        assert!(state_dir.join("snapshots").join("temp").exists());

        cmd_snapshot_delete("temp", &state_dir).unwrap();
        assert!(!state_dir.join("snapshots").join("temp").exists());
    }

    #[test]
    fn test_fj260_snapshot_delete_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_snapshot_delete("ghost", &state_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_fj260_snapshot_restore_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_snapshot_restore("ghost", &state_dir, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_fj260_snapshot_list_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        // No snapshots dir yet
        cmd_snapshot_list(&state_dir, false).unwrap();
        cmd_snapshot_list(&state_dir, true).unwrap();
    }

    #[test]
    fn test_fj260_snapshot_preserves_multiple_machines() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        for m in &["web", "db", "cache"] {
            let md = state_dir.join(m);
            std::fs::create_dir_all(&md).unwrap();
            std::fs::write(md.join("state.lock.yaml"), format!("machine: {m}")).unwrap();
        }

        cmd_snapshot_save("multi", &state_dir).unwrap();

        let snap = state_dir.join("snapshots").join("multi");
        for m in &["web", "db", "cache"] {
            assert!(snap.join(m).join("state.lock.yaml").exists());
        }
    }

    #[test]
    fn test_fj260_snapshot_save_no_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("nonexistent");
        let result = cmd_snapshot_save("test", &state_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_fj260_copy_dir_recursive() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        let dst = dir.path().join("dst");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.txt"), "aaa").unwrap();
        std::fs::write(src.join("sub").join("b.txt"), "bbb").unwrap();
        std::fs::write(src.join("skip.me"), "skip").unwrap();

        std::fs::create_dir_all(&dst).unwrap();
        copy_dir_recursive(&src, &dst, "skip.me").unwrap();

        assert!(dst.join("a.txt").exists());
        assert!(dst.join("sub").join("b.txt").exists());
        assert!(!dst.join("skip.me").exists());
    }

    // ── FJ-262: Apply report with per-resource timing ──
}

//! Tests: Shared CLI helpers: color, parsing, state utilities.

#![allow(unused_imports)]
use super::helpers::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj132_discover_machines_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let machines = discover_machines(dir.path());
        assert!(machines.is_empty());
    }

    #[test]
    fn test_fj132_discover_machines_with_locks() {
        let dir = tempfile::tempdir().unwrap();
        // Machine with state.lock.yaml — should be discovered
        let web_dir = dir.path().join("web");
        std::fs::create_dir_all(&web_dir).unwrap();
        std::fs::write(web_dir.join("state.lock.yaml"), "schema: '1.0'").unwrap();
        // Machine without lock — should NOT be discovered
        let nolock_dir = dir.path().join("orphan");
        std::fs::create_dir_all(&nolock_dir).unwrap();
        // Plain file — should NOT be discovered
        std::fs::write(dir.path().join("readme.txt"), "ignore").unwrap();
        let machines = discover_machines(dir.path());
        assert_eq!(machines, vec!["web"]);
    }

    #[test]
    fn test_fj132_discover_machines_sorted() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["zeta", "alpha", "mid"] {
            let m_dir = dir.path().join(name);
            std::fs::create_dir_all(&m_dir).unwrap();
            std::fs::write(m_dir.join("state.lock.yaml"), "schema: '1.0'").unwrap();
        }
        let machines = discover_machines(dir.path());
        assert_eq!(machines, vec!["alpha", "mid", "zeta"]);
    }

    #[test]
    fn test_fj132_discover_machines_nonexistent_dir() {
        let machines = discover_machines(std::path::Path::new("/nonexistent/path/state"));
        assert!(machines.is_empty(), "nonexistent dir should return empty");
    }

    #[test]
    fn test_fj036_discover_container_machines() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path();

        // Create a container-transport machine directory with a state.lock.yaml
        let container_dir = state.join("docker-box");
        std::fs::create_dir_all(&container_dir).unwrap();
        std::fs::write(container_dir.join("state.lock.yaml"), "schema: '1.0'").unwrap();

        // Create another machine directory (non-container, but discover_machines
        // only checks for state.lock.yaml presence, not transport type)
        let ssh_dir = state.join("ssh-box");
        std::fs::create_dir_all(&ssh_dir).unwrap();
        std::fs::write(ssh_dir.join("state.lock.yaml"), "schema: '1.0'").unwrap();

        let machines = discover_machines(state);
        assert_eq!(machines.len(), 2);
        assert!(
            machines.contains(&"docker-box".to_string()),
            "container transport machine should be discovered"
        );
        assert!(
            machines.contains(&"ssh-box".to_string()),
            "ssh transport machine should also be discovered"
        );
        // discover_machines returns sorted results
        assert_eq!(machines[0], "docker-box");
        assert_eq!(machines[1], "ssh-box");
    }

    #[test]
    fn test_fj263_green_with_color() {
        NO_COLOR.store(false, Ordering::Relaxed);
        let s = green("ok");
        assert!(s.contains("\x1b[32m"));
        assert!(s.contains("ok"));
        assert!(s.contains("\x1b[0m"));
    }

    #[test]
    fn test_fj263_green_no_color() {
        NO_COLOR.store(true, Ordering::Relaxed);
        let s = green("ok");
        assert_eq!(s, "ok");
        assert!(!s.contains("\x1b["));
    }

    #[test]
    fn test_fj263_red_with_color() {
        NO_COLOR.store(false, Ordering::Relaxed);
        let s = red("fail");
        assert!(s.contains("\x1b[31m"));
        assert!(s.contains("fail"));
    }

    #[test]
    fn test_fj263_yellow_with_color() {
        NO_COLOR.store(false, Ordering::Relaxed);
        let s = yellow("warn");
        assert!(s.contains("\x1b[33m"));
        assert!(s.contains("warn"));
    }

    #[test]
    fn test_fj263_dim_with_color() {
        NO_COLOR.store(false, Ordering::Relaxed);
        let s = dim("muted");
        assert!(s.contains("\x1b[2m"));
        assert!(s.contains("muted"));
    }

    #[test]
    fn test_fj263_bold_with_color() {
        NO_COLOR.store(false, Ordering::Relaxed);
        let s = bold("header");
        assert!(s.contains("\x1b[1m"));
        assert!(s.contains("header"));
    }

    #[test]
    fn test_fj263_color_enabled_tracks_flag() {
        NO_COLOR.store(false, Ordering::Relaxed);
        assert!(color_enabled());
        NO_COLOR.store(true, Ordering::Relaxed);
        assert!(!color_enabled());
        NO_COLOR.store(false, Ordering::Relaxed);
    }

    // ── FJ-264: forjar schema ──
}

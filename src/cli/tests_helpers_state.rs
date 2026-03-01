//! Tests: State loading and machine discovery helpers.

#![allow(unused_imports)]
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::helpers_state::*;

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_fj017_load_machine_locks_missing_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = serde_yaml_ng::from_str::<types::ForjarConfig>(
            r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources: {}
"#,
        )
        .unwrap();
        // State dir doesn't exist → returns empty map
        let missing = dir.path().join("nonexistent");
        let locks = load_machine_locks(&config, &missing, None).unwrap();
        assert!(locks.is_empty());
    }


    #[test]
    fn test_fj267_load_all_locks() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test:
    type: file
    machine: local
    path: /tmp/fj267-lock.txt
    content: "test"
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let locks = load_all_locks(dir.path(), &config);
        // No locks exist yet
        assert!(locks.is_empty());
    }


    #[test]
    fn test_fj267_load_all_locks_with_existing() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = r#"
version: "1.0"
name: test
machines:
  srv:
    hostname: srv
    addr: 192.168.1.1
resources:
  test:
    type: file
    machine: srv
    path: /tmp/test.txt
    content: "test"
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        // Create a lock for "srv"
        let lock = state::new_lock("srv", "srv");
        state::save_lock(dir.path(), &lock).unwrap();

        let locks = load_all_locks(dir.path(), &config);
        assert_eq!(locks.len(), 1);
        assert!(locks.contains_key("srv"));
    }

    // ========================================================================
    // FJ-270: Structured event output
    // ========================================================================


    #[test]
    fn test_fj285_collect_transitive_deps() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  a:
    type: file
    machine: local
    path: /tmp/a.txt
    content: "a"
    depends_on: [b]
  b:
    type: file
    machine: local
    path: /tmp/b.txt
    content: "b"
    depends_on: [c]
  c:
    type: file
    machine: local
    path: /tmp/c.txt
    content: "c"
  d:
    type: file
    machine: local
    path: /tmp/d.txt
    content: "d"
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let deps = collect_transitive_deps(&config, "a").unwrap();
        assert!(deps.contains("a"));
        assert!(deps.contains("b"));
        assert!(deps.contains("c"));
        assert!(!deps.contains("d"));
        assert_eq!(deps.len(), 3);
    }

    // ── FJ-286: Apply confirmation prompt ──────────────────────────


    #[test]
    fn test_fj331_simple_glob_match() {
        assert!(simple_glob_match("web-*", "web-server"));
        assert!(simple_glob_match("*-pkg", "base-pkg"));
        assert!(simple_glob_match("*config*", "app-config-main"));
        assert!(!simple_glob_match("web-*", "db-server"));
        assert!(simple_glob_match("exact", "exact"));
        assert!(!simple_glob_match("exact", "other"));
        assert!(simple_glob_match("*", "anything"));
    }

}

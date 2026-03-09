//! Additional coverage tests for helpers_state functions.

use super::helpers_state::*;
use crate::core::{state, types};

// ── list_state_machines ────────────────────────────────────────────

#[test]
fn list_state_machines_basic() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("web")).unwrap();
    std::fs::create_dir(dir.path().join("db")).unwrap();
    std::fs::create_dir(dir.path().join("cache")).unwrap();
    let machines = list_state_machines(dir.path()).unwrap();
    assert_eq!(machines, vec!["cache", "db", "web"]); // sorted
}

#[test]
fn list_state_machines_skips_hidden() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("web")).unwrap();
    std::fs::create_dir(dir.path().join(".generation.yaml")).unwrap();
    std::fs::create_dir(dir.path().join(".internal")).unwrap();
    let machines = list_state_machines(dir.path()).unwrap();
    assert_eq!(machines, vec!["web"]);
}

#[test]
fn list_state_machines_skips_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("web")).unwrap();
    std::fs::write(dir.path().join("not-a-dir.txt"), "data").unwrap();
    let machines = list_state_machines(dir.path()).unwrap();
    assert_eq!(machines, vec!["web"]);
}

#[test]
fn list_state_machines_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let machines = list_state_machines(dir.path()).unwrap();
    assert!(machines.is_empty());
}

#[test]
fn list_state_machines_nonexistent_dir() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nonexistent");
    let result = list_state_machines(&missing);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cannot read state dir"));
}

// ── load_generation_locks ──────────────────────────────────────────

fn make_lock(machine: &str) -> types::StateLock {
    types::StateLock {
        schema: "1.0".to_string(),
        machine: machine.to_string(),
        hostname: "host".to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 1.0.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources: indexmap::IndexMap::new(),
    }
}

#[test]
fn load_generation_locks_basic() {
    let dir = tempfile::tempdir().unwrap();
    let m1 = dir.path().join("m1");
    std::fs::create_dir_all(&m1).unwrap();
    let lock = make_lock("m1");
    let yaml = serde_yaml_ng::to_string(&lock).unwrap();
    std::fs::write(m1.join("state.lock.yaml"), &yaml).unwrap();

    let locks = load_generation_locks(dir.path(), None);
    assert_eq!(locks.len(), 1);
    assert!(locks.contains_key("m1"));
}

#[test]
fn load_generation_locks_with_filter() {
    let dir = tempfile::tempdir().unwrap();
    for name in &["m1", "m2", "m3"] {
        let mdir = dir.path().join(name);
        std::fs::create_dir_all(&mdir).unwrap();
        let lock = make_lock(name);
        let yaml = serde_yaml_ng::to_string(&lock).unwrap();
        std::fs::write(mdir.join("state.lock.yaml"), &yaml).unwrap();
    }

    let locks = load_generation_locks(dir.path(), Some("m2"));
    assert_eq!(locks.len(), 1);
    assert!(locks.contains_key("m2"));
}

#[test]
fn load_generation_locks_skips_hidden() {
    let dir = tempfile::tempdir().unwrap();
    let hidden = dir.path().join(".meta");
    std::fs::create_dir_all(&hidden).unwrap();
    std::fs::write(hidden.join("state.lock.yaml"), "schema: '1'").unwrap();

    let locks = load_generation_locks(dir.path(), None);
    assert!(locks.is_empty());
}

#[test]
fn load_generation_locks_skips_missing_lock() {
    let dir = tempfile::tempdir().unwrap();
    let m1 = dir.path().join("m1");
    std::fs::create_dir_all(&m1).unwrap();
    // No state.lock.yaml file
    let locks = load_generation_locks(dir.path(), None);
    assert!(locks.is_empty());
}

#[test]
fn load_generation_locks_skips_invalid_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let m1 = dir.path().join("m1");
    std::fs::create_dir_all(&m1).unwrap();
    std::fs::write(m1.join("state.lock.yaml"), "{{broken yaml").unwrap();

    let locks = load_generation_locks(dir.path(), None);
    assert!(locks.is_empty());
}

#[test]
fn load_generation_locks_nonexistent_dir() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("nonexistent");
    let locks = load_generation_locks(&missing, None);
    assert!(locks.is_empty());
}

#[test]
fn load_generation_locks_filter_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let m1 = dir.path().join("m1");
    std::fs::create_dir_all(&m1).unwrap();
    let lock = make_lock("m1");
    let yaml = serde_yaml_ng::to_string(&lock).unwrap();
    std::fs::write(m1.join("state.lock.yaml"), &yaml).unwrap();

    let locks = load_generation_locks(dir.path(), Some("m99"));
    assert!(locks.is_empty());
}

// ── collect_transitive_deps edge cases ─────────────────────────────

#[test]
fn collect_transitive_deps_not_found() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
"#;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let result = collect_transitive_deps(&config, "nonexistent");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[test]
fn collect_transitive_deps_single_no_deps() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  standalone:
    type: file
    machine: m1
    path: /tmp/a.txt
    content: "a"
"#;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let deps = collect_transitive_deps(&config, "standalone").unwrap();
    assert_eq!(deps.len(), 1);
    assert!(deps.contains("standalone"));
}

#[test]
fn collect_transitive_deps_diamond() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  top:
    type: file
    machine: m1
    path: /tmp/top.txt
    content: "t"
    depends_on: [left, right]
  left:
    type: file
    machine: m1
    path: /tmp/left.txt
    content: "l"
    depends_on: [bottom]
  right:
    type: file
    machine: m1
    path: /tmp/right.txt
    content: "r"
    depends_on: [bottom]
  bottom:
    type: file
    machine: m1
    path: /tmp/bottom.txt
    content: "b"
"#;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let deps = collect_transitive_deps(&config, "top").unwrap();
    assert_eq!(deps.len(), 4);
    assert!(deps.contains("top"));
    assert!(deps.contains("left"));
    assert!(deps.contains("right"));
    assert!(deps.contains("bottom"));
}

// ── load_machine_locks with filter ─────────────────────────────────

#[test]
fn load_machine_locks_with_filter_match() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web
    addr: 10.0.0.1
  db:
    hostname: db
    addr: 10.0.0.2
resources: {}
"#;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    // Create locks for both machines
    let lock_web = state::new_lock("web", "web");
    state::save_lock(dir.path(), &lock_web).unwrap();
    let lock_db = state::new_lock("db", "db");
    state::save_lock(dir.path(), &lock_db).unwrap();

    // Filter to just "web"
    let locks = load_machine_locks(&config, dir.path(), Some("web")).unwrap();
    assert_eq!(locks.len(), 1);
    assert!(locks.contains_key("web"));
}

#[test]
fn load_machine_locks_with_filter_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = r#"
version: "1.0"
name: test
machines:
  web:
    hostname: web
    addr: 10.0.0.1
resources: {}
"#;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let lock = state::new_lock("web", "web");
    state::save_lock(dir.path(), &lock).unwrap();

    // Filter to nonexistent machine
    let locks = load_machine_locks(&config, dir.path(), Some("db")).unwrap();
    assert!(locks.is_empty());
}

// ── load_all_locks with localhost target ────────────────────────────

#[test]
fn load_all_locks_localhost_target() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
resources:
  local-file:
    type: file
    machine: localhost
    path: /tmp/local.txt
    content: "x"
"#;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    // No locks exist — but the localhost path should be traversed
    let locks = load_all_locks(dir.path(), &config);
    assert!(locks.is_empty());
}

#[test]
fn load_all_locks_local_target_with_lock() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 10.0.0.1
resources:
  local-file:
    type: file
    machine: local
    path: /tmp/local.txt
    content: "x"
"#;
    let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    // Create a lock for "local"
    let lock = state::new_lock("local", "localhost");
    state::save_lock(dir.path(), &lock).unwrap();

    let locks = load_all_locks(dir.path(), &config);
    assert!(locks.contains_key("local"));
}

// ── simple_glob_match additional cases ─────────────────────────────

#[test]
fn simple_glob_match_star_only() {
    assert!(simple_glob_match("*", ""));
    assert!(simple_glob_match("*", "anything-at-all"));
}

#[test]
fn simple_glob_match_exact_empty() {
    assert!(simple_glob_match("", ""));
    assert!(!simple_glob_match("", "notempty"));
}

#[test]
fn simple_glob_match_contains_pattern() {
    assert!(simple_glob_match("*test*", "my-test-file"));
    assert!(simple_glob_match("*test*", "test"));
    assert!(!simple_glob_match("*test*", "no-match"));
}

// ── pre_apply_generation ───────────────────────────────────────────

#[test]
fn pre_apply_generation_no_state() {
    let dir = tempfile::tempdir().unwrap();
    let result = pre_apply_generation(dir.path());
    assert!(result.is_none());
}

// ── maybe_rollback_generation ──────────────────────────────────────

#[test]
fn maybe_rollback_generation_disabled() {
    let dir = tempfile::tempdir().unwrap();
    // Should return immediately without doing anything
    maybe_rollback_generation(false, dir.path(), Some(0), false);
}

#[test]
fn maybe_rollback_generation_no_pre_gen() {
    let dir = tempfile::tempdir().unwrap();
    // Should return immediately when pre_apply_gen is None
    maybe_rollback_generation(true, dir.path(), None, false);
}

#[test]
fn maybe_rollback_generation_missing_gen_dir() {
    let dir = tempfile::tempdir().unwrap();
    // Should print warning but not panic
    maybe_rollback_generation(true, dir.path(), Some(99), false);
}

#[test]
fn maybe_rollback_generation_verbose() {
    let dir = tempfile::tempdir().unwrap();
    // Missing generation dir — triggers error path, not verbose success
    maybe_rollback_generation(true, dir.path(), Some(0), true);
}

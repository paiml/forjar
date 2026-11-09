//! FJ-LOCK-001 C2 (#922) falsifiers: the state lock is held from the
//! pre-validate gates through the post-apply actions, and released on every
//! exit — `?`, success and panic alike.

use super::apply::cmd_apply;
use crate::core::executor;
use std::path::Path;

fn write_config(dir: &Path, lock: &Path) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    let target = dir.join("target.txt");
    let (pre, post) = (dir.join("pre.ok"), dir.join("post.ok"));
    let yaml = format!(
        r#"
version: "1.0"
name: fj-lock-c2
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: local
    path: {target}
    content: "x"
policy:
  pre_apply: "test -s {lock} && touch {pre}"
  post_apply: "test -s {lock} && touch {post}"
"#,
        target = target.display(),
        lock = lock.display(),
        pre = pre.display(),
        post = post.display(),
    );
    std::fs::write(&file, yaml).unwrap();
    file
}

fn run_apply(file: &Path, state_dir: &Path) -> Result<(), String> {
    cmd_apply(
        file, state_dir, None, None, None, None, false, false, false, &[], false, None, false,
        false, None, None, false, false, None, false, false, 0, true, false, None, false, None,
        None, None, false, None, false, None, false, None, &[],
    )
}

/// The pre_apply hook runs inside `apply_pre_validate` and the post_apply hook
/// inside `apply_post_actions`; each succeeds only if the lock file is there.
/// Before the fix the lock was taken in the executor, after the drift gate had
/// already written state, and dropped before the generation was recorded.
#[test]
fn falsify_fj_lock_c2_lock_held_from_pre_validate_to_post_actions() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let lock = state_dir.join(".forjar.lock");
    let file = write_config(dir.path(), &lock);
    run_apply(&file, &state_dir).unwrap();
    assert!(dir.path().join("pre.ok").exists(), "pre_apply ran unlocked");
    assert!(dir.path().join("post.ok").exists(), "post_apply ran unlocked");
    assert!(!lock.exists(), "lock must be released when apply returns");
}

/// A `?` after the acquire (a dependency cycle fails `build_execution_order`)
/// used to return with the lock still on disk, wedging every later apply.
#[test]
fn falsify_fj_lock_c2_error_after_acquire_releases_lock() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = format!(
        "version: \"1.0\"\nname: c2\nmachines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: local\n    path: {}\n    content: x\n    depends_on: [a]\n",
        dir.path().join("a.txt").display()
    );
    let config: crate::core::types::ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    let cfg = executor::ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: false,
        machine_filter: None,
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
        refresh: false,
        force_tag: None,
    };
    assert!(executor::apply(&cfg).is_err(), "a self-cycle must fail");
    assert!(
        !dir.path().join(".forjar.lock").exists(),
        "lock leaked on an error after acquire"
    );
}

/// A panic after the acquire unwinds through the guard and frees the lock;
/// a nested take on the same thread joins the hold instead of self-refusing.
#[test]
fn falsify_fj_lock_c2_panic_after_acquire_releases_lock() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join(".forjar.lock");
    let caught = std::panic::catch_unwind(|| {
        let _outer = crate::core::state::hold_process_lock(dir.path(), false).unwrap();
        let inner = crate::core::state::hold_process_lock(dir.path(), true).unwrap();
        drop(inner);
        assert!(lock.exists(), "an inner (re-entrant) guard must not release");
        panic!("injected failure after acquire");
    });
    assert!(caught.is_err());
    assert!(!lock.exists(), "lock leaked across a panic");
}

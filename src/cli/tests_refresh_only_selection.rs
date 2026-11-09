//! Tests: `apply --refresh-only` honours `-r` (Refs #470). A scoped refresh
//! must leave every lock entry outside the selection untouched.

use super::apply_selection::{resolve_selection, Selectors};
use super::apply_variants::{cmd_refresh_only, cmd_refresh_only_with};
use super::helpers::parse_and_validate;
use super::helpers_state::load_machine_locks;
use std::path::Path;

fn two_resource_cfg(dir: &Path) -> std::path::PathBuf {
    let yaml = format!(
        "version: \"1.0\"\nname: sel\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: {a}\n    content: one\n  b:\n    type: file\n    machine: m\n    path: {b}\n    content: two\n",
        a = dir.join("a.txt").display(),
        b = dir.join("b.txt").display(),
    );
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

fn observed(cfg: &Path, sd: &Path, id: &str) -> Option<String> {
    let config = parse_and_validate(cfg).unwrap();
    let locks = load_machine_locks(&config, sd, None).unwrap();
    locks["m"]
        .resources
        .get(id)
        .and_then(|r| r.observed_state().map(String::from))
}

#[test]
fn scoped_refresh_leaves_unselected_entry_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = two_resource_cfg(dir.path());
    let sd = dir.path().join("state");
    std::fs::create_dir_all(&sd).unwrap();
    std::fs::write(dir.path().join("a.txt"), "one").unwrap();
    std::fs::write(dir.path().join("b.txt"), "two").unwrap();
    let config = parse_and_validate(&cfg).unwrap();
    crate::core::executor::apply(&crate::core::executor::ApplyConfig {
        config: &config,
        state_dir: &sd,
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
    })
    .expect("apply");
    cmd_refresh_only(&cfg, &sd, None, false, None, None, None, None).unwrap();
    let b_before = observed(&cfg, &sd, "b");
    let a_before = observed(&cfg, &sd, "a");
    // Drift both out-of-band; only `a` is selected.
    std::fs::write(dir.path().join("a.txt"), "drifted-a").unwrap();
    std::fs::write(dir.path().join("b.txt"), "drifted-b").unwrap();
    let sel = Selectors {
        resource: Some("a"),
        ..Selectors::default()
    };
    cmd_refresh_only_with(&cfg, &sd, None, false, None, None, None, None, |c| {
        resolve_selection(c, &sel, false).map(|_| ())
    })
    .unwrap();
    assert_eq!(
        observed(&cfg, &sd, "b"),
        b_before,
        "unselected b must not be refreshed"
    );
    assert_ne!(
        observed(&cfg, &sd, "a"),
        a_before,
        "selected a must be refreshed"
    );
}

#[test]
fn unknown_selector_is_refused_before_the_lock_is_touched() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = two_resource_cfg(dir.path());
    let sd = dir.path().join("state");
    std::fs::create_dir_all(&sd).unwrap();
    let sel = Selectors {
        resource: Some("ghost"),
        ..Selectors::default()
    };
    let r = cmd_refresh_only_with(&cfg, &sd, None, false, None, None, None, None, |c| {
        resolve_selection(c, &sel, false).map(|_| ())
    });
    assert!(r.is_err(), "unknown -r must be refused: {r:?}");
}

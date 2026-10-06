//! Refs #590: `--refresh` must consult the completion_check of a resource whose
//! fields contain a template, exactly as it does for one that has none.
//!
//! MEASURED on 1.31.0 (forjar#590) and on main at ce0a7cc2 by this file: one
//! `type: task` with `completion_check: "[ -d / ]"` whose command exits 1, with
//! `--refresh` against a fresh state dir —
//!
//! | command                         | result                          |
//! |---------------------------------|---------------------------------|
//! | no template                     | `1 unchanged` — check consulted |
//! | contains one `{{params.x}}`     | `1 FAILED` — command ran anyway |
//!
//! THE MECHANISM. `--refresh` ran the check, saw it pass, and recorded a
//! converged lock entry whose hash was `hash_desired_state` over the RAW
//! declaration. The planner hashes the TEMPLATE-RESOLVED declaration. With no
//! template the two are the same bytes; with one they differ, so the planner
//! read the fresh entry as `~ update (state changed)` and ran the command — and
//! a guard's command exits 1 by design. Every guard on the fleet names a path
//! through `{{params.*}}`, so none of them could converge under `--refresh`.
//!
//! These tests drive the real binary against a state dir in the test's own
//! tempdir. The command writes a marker, so "did the command run" is observed,
//! not inferred from a summary line.

use std::fs;
use std::path::{Path, PathBuf};

/// One guard: the command reports the violation (and leaves a marker proving it
/// ran); the completion_check is the assertion, and it holds on any host.
/// `templated` routes the marker path through `{{params.marker}}`; otherwise
/// the same path is written literally. Nothing else differs.
fn config_yaml(marker: &Path, templated: bool) -> String {
    let target = if templated {
        "{{params.marker}}".to_string()
    } else {
        marker.display().to_string()
    };
    format!(
        r#"version: "1.0"
name: refresh-590
machines:
  localhost:
    hostname: localhost
    addr: localhost
params:
  marker: "{marker}"
resources:
  guard:
    type: task
    machine: localhost
    command: "touch {target}; echo 'guard violated' >&2; exit 1"
    completion_check: "[ -d / ]"
"#,
        marker = marker.display(),
    )
}

fn apply(cfg: &Path, state: &Path, extra: &[&str]) -> (String, bool) {
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_forjar"));
    c.arg("apply")
        .arg("-f")
        .arg(cfg)
        .arg("--state-dir")
        .arg(state)
        .arg("--yes")
        .args(extra);
    let out = c.output().expect("forjar must run");
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

struct Fixture {
    _dir: tempfile::TempDir,
    cfg: PathBuf,
    state: PathBuf,
    marker: PathBuf,
}

fn fixture(templated: bool) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let marker = dir.path().join("command-ran");
    let cfg = dir.path().join("forjar.yaml");
    fs::write(&cfg, config_yaml(&marker, templated)).unwrap();
    fs::create_dir_all(&state).unwrap();
    Fixture {
        _dir: dir,
        cfg,
        state,
        marker,
    }
}

/// Close the latch the way the fleet closed it: a plain apply runs the command,
/// which exits 1, and the lock records the guard as FAILED.
fn latch(f: &Fixture) {
    let (out, ok) = apply(&f.cfg, &f.state, &[]);
    assert!(
        !ok,
        "a plain apply must run the guard's command and fail:\n{out}"
    );
    assert!(f.marker.exists(), "the command must have run:\n{out}");
    fs::remove_file(&f.marker).unwrap();
}

/// `--refresh` (with `extra`) must be green and must not run the command. When
/// `persisted`, the entry it wrote must also hold: a plain apply is a no-op.
fn assert_refresh_consults_the_check(f: &Fixture, extra: &[&str], case: &str, persisted: bool) {
    let mut args = vec!["--refresh"];
    args.extend_from_slice(extra);
    let (out, ok) = apply(&f.cfg, &f.state, &args);
    assert!(
        !f.marker.exists(),
        "{case}: --refresh ran the command of a guard whose completion_check \
         `[ -d / ]` holds on every host:\n{out}"
    );
    assert!(
        ok,
        "{case}: the check passes, so the apply must be green:\n{out}"
    );
    if !persisted {
        // A SEEDED entry (no prior lock entry) is planner-only and never
        // written; see `persist_unlatched`. Only an unlatched one reaches disk.
        return;
    }

    // The unlatched entry is written, and must be the one the planner would
    // write: if its hash is over a different declaration than the planner's,
    // the next apply re-plans an update and the command runs anyway.
    let (out, ok) = apply(&f.cfg, &f.state, &[]);
    assert!(
        !f.marker.exists() && ok,
        "{case}: the entry --refresh recorded did not hold — a plain apply right \
         after it re-ran the command:\n{out}"
    );
}

#[test]
fn a_templated_guard_with_no_lock_entry_is_checked_not_run() {
    let f = fixture(true);
    assert_refresh_consults_the_check(&f, &[], "templated, fresh state dir", false);
}

#[test]
fn a_templated_guard_the_lock_records_as_failed_is_checked_not_run() {
    let f = fixture(true);
    latch(&f);
    assert_refresh_consults_the_check(&f, &[], "templated, lock entry failed", true);
}

/// yoga's exact shape: `apply --refresh -r <guard>` on a latched guard.
#[test]
fn a_templated_guard_scoped_by_name_is_checked_not_run() {
    let f = fixture(true);
    latch(&f);
    assert_refresh_consults_the_check(&f, &["-r", "guard"], "templated, -r guard", true);
}

/// THE CONTROL. Identical but for the literal path. It passed before the fix and
/// must pass after it; if it went red the fixture, not the template, is broken.
#[test]
fn the_same_guard_without_a_template_is_checked_not_run() {
    let f = fixture(false);
    assert_refresh_consults_the_check(&f, &[], "literal, fresh state dir", false);
    let g = fixture(false);
    latch(&g);
    assert_refresh_consults_the_check(&g, &[], "literal, lock entry failed", true);
}

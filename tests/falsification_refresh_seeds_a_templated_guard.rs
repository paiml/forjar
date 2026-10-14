//! `--refresh` must not re-run the command of a TEMPLATED resource whose check
//! already passes.
//!
//! forjar#590, measured on 1.31.0. Two manifests that differ ONLY by one
//! `{{params.*}}`, with a byte-identical `completion_check` satisfied before
//! either run, applied with `--refresh` against a fresh state dir:
//!
//! | variant           | reported      | marker on disk |
//! |-------------------|---------------|----------------|
//! | no template       | `1 unchanged` | absent         |
//! | one `{{params.*}}`| `1 converged` | **created**    |
//!
//! The lock written by the templated run held a `guard.update.log` — the
//! planner chose UPDATE, not create. So seeding DID happen and the check DID
//! pass; what diverged was the hash. `refresh_seed::converged_entry` stamped
//! the seeded entry with `hash_desired_state` over the UNRESOLVED resource,
//! while the planner (and every executor write) hashes the RESOLVED one. For
//! a resource with no template the two are equal, which is why the plain
//! variant passed and every templated guard on the fleet did not.
//!
//! These tests drive the real binary end to end against a state dir inside the
//! test's own tempdir, and prove the command did or did not run by a MARKER on
//! disk rather than by a status word.

use std::fs;
use std::path::Path;

fn forjar_bin() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

/// One guard. `cmd_dir` / `check_dir` are spliced in either literally or
/// through `{{params.dir}}`, so the only difference between variants is the
/// presence of a template.
fn config_yaml(dir: &Path, command: &str, check: &str) -> String {
    format!(
        r#"version: "1.0"
name: fj590
params:
  dir: "{dir}"
machines:
  localhost:
    hostname: localhost
    addr: localhost
resources:
  guard:
    type: task
    machine: localhost
    command: "{command}"
    completion_check: "{check}"
"#,
        dir = dir.display(),
    )
}

fn apply(cfg: &Path, state: &Path, extra: &[&str]) -> (String, bool) {
    let mut c = std::process::Command::new(forjar_bin());
    c.arg("apply")
        .arg("-f")
        .arg(cfg)
        .arg("--state-dir")
        .arg(state)
        .arg("--yes");
    for e in extra {
        c.arg(e);
    }
    let out = c.output().expect("forjar must run");
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

struct Fixture {
    _dir: tempfile::TempDir,
    cfg: std::path::PathBuf,
    state: std::path::PathBuf,
    marker: std::path::PathBuf,
    satisfied: std::path::PathBuf,
}

/// `templated_command` / `templated_check` choose which field names its path
/// through `{{params.dir}}`. The check is `test -f <satisfied>`, so the test
/// decides whether the host is in its declared state.
fn fixture(templated_command: bool, templated_check: bool) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().to_path_buf();
    let state = base.join("state");
    let marker = base.join("marker");
    let satisfied = base.join("satisfied");
    let at = |templated: bool| {
        if templated {
            "{{params.dir}}".to_string()
        } else {
            base.display().to_string()
        }
    };
    let command = format!("touch {}/marker", at(templated_command));
    let check = format!("test -f {}/satisfied", at(templated_check));
    let cfg = base.join("forjar.yaml");
    fs::write(&cfg, config_yaml(&base, &command, &check)).unwrap();
    fs::create_dir_all(&state).unwrap();
    Fixture {
        _dir: dir,
        cfg,
        state,
        marker,
        satisfied,
    }
}

fn assert_refresh_skips(f: &Fixture, what: &str) {
    fs::write(&f.satisfied, "").unwrap();
    let (out, ok) = apply(&f.cfg, &f.state, &["--refresh"]);
    assert!(ok, "{what}: --refresh must be green:\n{out}");
    assert!(
        !f.marker.exists(),
        "{what}: --refresh ran the command of a resource whose completion_check \
         already passes (forjar#590).\noutput:\n{out}"
    );
    assert!(
        out.contains("1 unchanged"),
        "{what}: a satisfied guard must report unchanged:\n{out}"
    );
}

/// The control the issue measured as GREEN on 1.31.0. If this goes red the
/// fixture is wrong, not the fix.
#[test]
fn refresh_skips_a_satisfied_guard_with_no_template() {
    assert_refresh_skips(&fixture(false, false), "no template");
}

/// THE REGRESSION: one `{{params.*}}` in `command`.
#[test]
fn refresh_skips_a_satisfied_guard_with_a_templated_command() {
    assert_refresh_skips(&fixture(true, false), "templated command");
}

/// The issue's field-placement matrix: a template in the CHECK reproduced
/// identically, so it is the presence of a template anywhere that mattered.
#[test]
fn refresh_skips_a_satisfied_guard_with_a_templated_check() {
    assert_refresh_skips(&fixture(false, true), "templated check");
}

/// Both fields templated at once.
#[test]
fn refresh_skips_a_satisfied_guard_templated_in_both_fields() {
    assert_refresh_skips(&fixture(true, true), "templated both");
}

/// The unlatch half (PMAT-214) writes through the same entry constructor, so a
/// templated guard the lock records as FAILED must also come back green.
#[test]
fn refresh_unlatches_a_failed_templated_guard() {
    let f = fixture(true, true);
    // Make the command fail, so the first apply latches the entry as FAILED.
    let failing = fs::read_to_string(&f.cfg).unwrap().replace(
        "touch {{params.dir}}/marker",
        "touch {{params.dir}}/marker; exit 1",
    );
    fs::write(&f.cfg, failing).unwrap();
    let (out, ok) = apply(&f.cfg, &f.state, &[]);
    assert!(!ok, "the unsatisfied guard must fail first:\n{out}");
    fs::remove_file(&f.marker).unwrap();

    assert_refresh_skips(&f, "latched templated guard");
}

/// THE NEGATIVE: the fix must not launder an unsatisfied templated guard into
/// converged. The check fails, so the command runs.
#[test]
fn refresh_still_runs_a_templated_guard_whose_check_fails() {
    let f = fixture(true, true);
    let (out, _) = apply(&f.cfg, &f.state, &["--refresh"]);
    assert!(
        f.marker.exists(),
        "the check fails on the host, so --refresh must run the command:\n{out}"
    );
}

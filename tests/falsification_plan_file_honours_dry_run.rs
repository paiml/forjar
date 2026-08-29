//! Refs #358 — `apply --plan-file` must honour `--dry-run` and `-m`.
//!
//! `cmd_apply_from_plan` took neither argument. `dispatch_apply_b.rs` had
//! `args.dry_run` and `args.machine` in hand — the sibling branch two lines
//! above already passed `args.machine.as_deref()` — and dropped both on the
//! floor, and `execute_scoped_plan` hard-coded `dry_run: false,
//! machine_filter: None` under a doc comment auditing "Every selector below
//! stays `None` on purpose".
//!
//! MEASURED against the built binary before the fix, on a plan with one
//! pending create:
//!
//! ```text
//! $ forjar apply -f forjar.yaml --plan-file p.json --dry-run
//! Plan applied: 1 converged, 0 unchanged, 0 failed
//! $ echo $?; test -f alpha.txt && echo CREATED
//! 0
//! CREATED
//! ```
//!
//! A two-phase plan/review/apply feature whose `--dry-run` converges the
//! machine instead of previewing it is the worst available default: the flag
//! exists precisely so an operator can ask "what would this sealed plan do?"
//! without doing it.
//!
//! # `-m` intersects the reviewed scope, and an empty intersection is an error
//!
//! `--plan-file` executes the REVIEWED delta, so `-m` can only ever narrow it —
//! widening would be the #358 defect again, an unreviewed resource converged
//! from a plan that never named it. Narrowing to a machine the plan does touch
//! is a legitimate staged rollout and is honoured.
//!
//! Narrowing to a machine the plan does NOT touch leaves nothing, and that is
//! refused rather than reported as a successful apply of zero resources. The
//! silent version is how an operator asks for one machine, gets `exit 0`, and
//! believes it was converged; the loud version costs them one re-run and a
//! message naming the machines the plan actually covers.

use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

struct Project {
    cfg: PathBuf,
    state: PathBuf,
    alpha: PathBuf,
    bravo: PathBuf,
}

/// Two resources on two (locally-executed) machines, so `-m` has something to
/// include and something to exclude.
fn project(dir: &Path) -> Project {
    let alpha = dir.join("alpha.txt");
    let bravo = dir.join("bravo.txt");
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\n\
             name: plan-dry-run\n\
             machines:\n\
             \x20 web:\n\
             \x20   hostname: localhost\n\
             \x20   addr: 127.0.0.1\n\
             \x20 db:\n\
             \x20   hostname: localhost\n\
             \x20   addr: 127.0.0.1\n\
             resources:\n\
             \x20 alpha:\n\
             \x20   type: file\n\
             \x20   machine: web\n\
             \x20   path: {}\n\
             \x20   content: \"alpha\"\n\
             \x20 bravo:\n\
             \x20   type: file\n\
             \x20   machine: db\n\
             \x20   path: {}\n\
             \x20   content: \"bravo\"\n",
            alpha.display(),
            bravo.display()
        ),
    )
    .expect("write config");
    Project {
        cfg,
        state: dir.join("state"),
        alpha,
        bravo,
    }
}

fn combined(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn plan_out(p: &Project, out: &Path) -> std::process::Output {
    forjar()
        .args(["plan", "-f"])
        .arg(&p.cfg)
        .arg("--state-dir")
        .arg(&p.state)
        .arg("--out")
        .arg(out)
        .output()
        .expect("run plan")
}

fn apply_plan(p: &Project, plan: &Path, extra: &[&str]) -> std::process::Output {
    let mut cmd = forjar();
    cmd.args(["apply", "-f"])
        .arg(&p.cfg)
        .arg("--state-dir")
        .arg(&p.state)
        .arg("--plan-file")
        .arg(plan)
        .arg("--yes");
    cmd.args(extra);
    cmd.output().expect("run apply")
}

/// RED-1: the defect. `--dry-run` over a plan holding two real creates must
/// create neither file.
#[test]
fn dry_run_over_a_plan_with_real_creates_creates_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&p, &plan_path).status.success());

    let out = apply_plan(&p, &plan_path, &["--dry-run"]);
    let text = combined(&out);
    assert!(out.status.success(), "dry run must succeed: {text}");

    assert!(
        !p.alpha.exists(),
        "--dry-run converged 'alpha' for real: {text}"
    );
    assert!(
        !p.bravo.exists(),
        "--dry-run converged 'bravo' for real: {text}"
    );
    assert!(
        !p.state.join("web").exists() && !p.state.join("db").exists(),
        "--dry-run must not write state either: {text}"
    );
    assert!(
        !text.contains("Plan applied:"),
        "a preview must not report an apply: {text}"
    );
}

/// RED-1b: the preview has to be a preview OF THE PLAN, not a bare "0
/// converged". An operator running `--dry-run` on a sealed plan is asking what
/// the reviewed delta contains.
#[test]
fn dry_run_names_the_reviewed_changes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&p, &plan_path).status.success());

    let text = combined(&apply_plan(&p, &plan_path, &["--dry-run"]));
    assert!(text.contains("alpha"), "preview must name 'alpha': {text}");
    assert!(text.contains("bravo"), "preview must name 'bravo': {text}");
    assert!(
        text.to_lowercase().contains("dry run") || text.to_lowercase().contains("would"),
        "the preview must say it did nothing: {text}"
    );
}

/// RED-2: `-m` naming a machine outside the reviewed plan converged the whole
/// plan anyway. It must now be refused, and the refusal must name what the
/// plan does cover.
#[test]
fn a_machine_outside_the_reviewed_plan_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&p, &plan_path).status.success());

    // The whole plan covers web and db; a third machine name covers neither.
    let out = apply_plan(&p, &plan_path, &["-m", "nonexistent-machine"]);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "-m over a machine the plan does not name must not exit 0: {text}"
    );
    assert!(
        text.contains("nonexistent-machine"),
        "the refusal must quote the machine asked for: {text}"
    );
    assert!(
        text.contains("web") && text.contains("db"),
        "the refusal must name the machines the plan covers: {text}"
    );
    assert!(
        !p.alpha.exists() && !p.bravo.exists(),
        "-m over a machine outside the plan converged the plan anyway: {text}"
    );
}

/// RED-2b: the intersection is real. `-m web` on a plan covering web and db
/// converges web's resource and leaves db's alone — `-m` narrows, and the
/// narrowing is honoured rather than ignored.
#[test]
fn a_machine_inside_the_reviewed_plan_narrows_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&p, &plan_path).status.success());

    let out = apply_plan(&p, &plan_path, &["-m", "web"]);
    let text = combined(&out);
    assert!(out.status.success(), "apply: {text}");
    assert_eq!(
        std::fs::read_to_string(&p.alpha).expect("alpha must be converged"),
        "alpha"
    );
    assert!(
        !p.bravo.exists(),
        "-m web must not converge db's resource: {text}"
    );
    assert!(
        !p.state.join("db").exists(),
        "-m web must not even reach machine 'db': {text}"
    );
}

/// GREEN GUARD: honouring `--dry-run` must not turn `--plan-file` into "never
/// apply". Without the flag, the same plan still converges.
#[test]
fn without_dry_run_the_same_plan_still_converges() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&p, &plan_path).status.success());

    let out = apply_plan(&p, &plan_path, &[]);
    let text = combined(&out);
    assert!(out.status.success(), "{text}");
    assert_eq!(std::fs::read_to_string(&p.alpha).expect("alpha"), "alpha");
    assert_eq!(std::fs::read_to_string(&p.bravo).expect("bravo"), "bravo");
}

/// GREEN GUARD: a `--dry-run` preview leaves the plan appliable. The preview
/// must not consume or invalidate what it previewed — in particular it must not
/// move the state the seal is bound to.
#[test]
fn a_previewed_plan_is_still_appliable_afterwards() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&p, &plan_path).status.success());

    assert!(apply_plan(&p, &plan_path, &["--dry-run"]).status.success());

    let out = apply_plan(&p, &plan_path, &[]);
    let text = combined(&out);
    assert!(
        out.status.success(),
        "a previewed plan must still apply: {text}"
    );
    assert_eq!(std::fs::read_to_string(&p.alpha).expect("alpha"), "alpha");
    assert_eq!(std::fs::read_to_string(&p.bravo).expect("bravo"), "bravo");
}

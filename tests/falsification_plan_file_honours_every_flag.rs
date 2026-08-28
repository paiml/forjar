//! Refs #358 — every apply flag on `--plan-file` is honoured or refused, never
//! dropped.
//!
//! # The claim this file exists to falsify
//!
//! `execute_scoped_plan` built an `ApplyConfig` from a literal in which exactly
//! one field, `machine_filter`, came from the invocation. The other fifteen were
//! hard-coded. Measured against the built binary, with two creates pending and
//! one honest sealed plan:
//!
//! ```text
//!   $ forjar apply -f forjar.yaml --plan-file p.json --yes -r alpha
//!   Plan applied: 2 converged, 0 unchanged, 0 failed
//!   >>> bravo CONVERGED despite -r alpha
//! ```
//!
//! and the same for `-t`, `-g`, `--progress`, `--force`, `--timeout`,
//! `--retry`, `--parallel`, `--max-parallel`, `--resource-timeout`,
//! `--rollback-on-failure`, `--trace`, `--refresh`, `--force-unlock` and
//! `--force-tag`. An operator who believed `--rollback-on-failure` was armed on
//! a plan apply was wrong, silently.
//!
//! What made it worse than a plain gap: the doc comment written to FIX an
//! earlier false-comment defect supplied a rationale for it — that the selectors
//! "were already applied when the plan body was written". True of how the plan
//! was PRODUCED; no reason at all to ignore a flag the operator is passing NOW.
//!
//! # The three kinds of flag, and why they are treated differently
//!
//! * **Selectors** (`-m`, `-r`, `-t`, `-g`) intersect the reviewed scope. A
//!   selector can only narrow a reviewed delta, never widen it — the executor
//!   already intersects all four with the scope — and an EMPTY intersection is
//!   an error, because converging nothing at exit 0 is the silent green this
//!   whole issue is about.
//! * **Knobs** (`--progress`, `--timeout`, `--retry`, `--parallel`,
//!   `--max-parallel`, `--resource-timeout`, `--rollback-on-failure`,
//!   `--force-unlock`, `--trace`) say HOW the reviewed delta executes and are
//!   passed straight through. There was never a reason not to.
//! * **Re-planners** (`--force`, `--force-tag`, `--refresh`) are REFUSED.
//!   They clear the lock entries the planner reads, so they change what the
//!   delta IS, and `--force` additionally defeats the scope outright:
//!   `PlanScope` demotes out-of-scope changes to `NoOp` so `triggers` still
//!   fire, and `should_skip_single` skips a `NoOp` only `if !cfg.force`.
//!
//! Every test drives `CARGO_BIN_EXE_forjar`, because what was wrong is what an
//! operator and a CI job observe.

#[path = "common/plan_project.rs"]
mod plan_project;

use plan_project::{combined, forjar, planned, project, Project};
use std::path::Path;

// ── selectors narrow the reviewed delta ──

/// RED-1 — THE REPORTED DEFECT. `-r alpha` on a two-resource plan converged
/// `bravo` as well, at exit 0.
#[test]
fn a_resource_selector_narrows_a_plan_apply() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (p, plan_path) = planned(dir.path());

    let out = p.apply_plan(&plan_path, &["-r", "alpha"]);
    let text = combined(&out);
    assert!(out.status.success(), "{text}");
    assert_eq!(
        p.converged(),
        (true, false),
        "-r alpha must converge alpha and nothing else: {text}"
    );
}

/// RED-2 — `-t` was dropped too, and a tag matching NOTHING in the plan still
/// converged everything.
#[test]
fn a_tag_selector_narrows_a_plan_apply() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (p, plan_path) = planned(dir.path());

    let out = p.apply_plan(&plan_path, &["-t", "t_bravo"]);
    let text = combined(&out);
    assert!(out.status.success(), "{text}");
    assert_eq!(p.converged(), (false, true), "{text}");
}

#[test]
fn a_group_selector_narrows_a_plan_apply() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (p, plan_path) = planned(dir.path());

    let out = p.apply_plan(&plan_path, &["-g", "ga"]);
    let text = combined(&out);
    assert!(out.status.success(), "{text}");
    assert_eq!(p.converged(), (true, false), "{text}");
}

/// A selector that intersects the reviewed plan in NOTHING is an error, not an
/// apply of nothing at exit 0. `-t nosuchtag` converged both resources before.
#[test]
fn a_selector_that_selects_nothing_in_the_plan_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (p, plan_path) = planned(dir.path());

    let out = p.apply_plan(&plan_path, &["-t", "nosuchtag"]);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "converging nothing must not exit 0: {text}"
    );
    assert!(text.contains("-t nosuchtag"), "{text}");
    assert!(text.contains("narrow"), "{text}");
    assert_eq!(p.converged(), (false, false), "{text}");
}

/// …and the selectors intersect one another, not only the plan.
#[test]
fn selectors_intersect_with_one_another_on_a_plan_apply() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (p, plan_path) = planned(dir.path());

    let out = p.apply_plan(&plan_path, &["-r", "alpha", "-t", "t_bravo"]);
    let text = combined(&out);
    assert!(!out.status.success(), "{text}");
    assert_eq!(p.converged(), (false, false), "{text}");
}

/// A selector may narrow the reviewed delta and never widen it. `plan -r alpha`
/// then `apply --plan-file -r bravo` must not converge `bravo`.
#[test]
fn a_selector_cannot_widen_a_plan_beyond_what_was_reviewed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("narrow.json");
    let o = forjar()
        .args(["plan", "-f"])
        .arg(&p.cfg)
        .arg("--state-dir")
        .arg(&p.state)
        .args(["-r", "alpha"])
        .arg("--out")
        .arg(&plan_path)
        .output()
        .expect("plan");
    assert!(o.status.success(), "{}", combined(&o));

    let out = p.apply_plan(&plan_path, &["-r", "bravo"]);
    let text = combined(&out);
    assert!(!out.status.success(), "{text}");
    assert_eq!(
        p.converged(),
        (false, false),
        "a plan that never named bravo must not converge it: {text}"
    );
}

/// The `--dry-run` preview must show the SAME narrowed set the real run would
/// converge, or the preview is not a preview.
#[test]
fn the_dry_run_preview_is_narrowed_by_the_same_selectors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (p, plan_path) = planned(dir.path());

    let out = p.apply_plan(&plan_path, &["-r", "alpha", "--dry-run"]);
    let text = combined(&out);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("alpha"), "{text}");
    assert!(
        !text.contains("bravo"),
        "the preview must not list what -r excluded: {text}"
    );
    assert!(text.contains("1 reviewed change(s)"), "{text}");
    assert_eq!(p.converged(), (false, false), "a preview converges nothing");
}

// ── the flags that cannot be honoured are refused, not dropped ──

/// `--force` defeats `PlanScope` entirely, so it cannot be honoured on a
/// reviewed plan. Before this fix it was accepted and ignored.
#[test]
fn force_is_refused_on_a_plan_apply_rather_than_ignored() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (p, plan_path) = planned(dir.path());

    let out = p.apply_plan(&plan_path, &["--force"]);
    let text = combined(&out);
    assert!(!out.status.success(), "{text}");
    assert!(text.contains("--force"), "{text}");
    assert!(text.contains("--plan-file"), "{text}");
    assert!(text.contains("Nothing was done"), "{text}");
    assert_eq!(p.converged(), (false, false), "{text}");
}

#[test]
fn refresh_is_refused_on_a_plan_apply() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (p, plan_path) = planned(dir.path());
    let out = p.apply_plan(&plan_path, &["--refresh"]);
    assert!(!out.status.success(), "{}", combined(&out));
    assert!(combined(&out).contains("--refresh"));
}

#[test]
fn force_tag_is_refused_on_a_plan_apply() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (p, plan_path) = planned(dir.path());
    let out = p.apply_plan(&plan_path, &["--force-tag", "t_alpha"]);
    assert!(!out.status.success(), "{}", combined(&out));
    assert!(combined(&out).contains("--force-tag"));
}

/// The refusal happens BEFORE anything runs, not after a partial apply.
#[test]
fn a_refused_flag_stops_the_run_before_any_resource_is_touched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (p, plan_path) = planned(dir.path());
    let out = p.apply_plan(&plan_path, &["--force", "--refresh"]);
    let text = combined(&out);
    assert!(!out.status.success(), "{text}");
    assert!(
        !text.contains("Plan applied:"),
        "no apply summary may be printed: {text}"
    );
    assert_eq!(p.converged(), (false, false), "{text}");
}

// ── the knobs reach the executor ──

/// `--progress` printed nothing on a plan apply, because `progress: false` was
/// hard-coded. The counter is the observable proof the field is now fed.
#[test]
fn progress_reaches_the_executor_on_a_plan_apply() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (p, plan_path) = planned(dir.path());

    let out = p.apply_plan(&plan_path, &["--progress"]);
    let text = combined(&out);
    assert!(out.status.success(), "{text}");
    assert!(
        text.contains("[1/") || text.contains("[2/"),
        "--progress must print its counter: {text}"
    );
}

/// `--trace` prints the generated script. It reached `cmd_apply_from_plan` as
/// `verbose` and was then dropped again into `trace: false`.
#[test]
fn trace_reaches_the_executor_on_a_plan_apply() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (p, plan_path) = planned(dir.path());

    let quiet = combined(&p.apply_plan(&plan_path, &["--dry-run"]));
    let loud = combined(&p.apply_plan(&plan_path, &["--trace"]));
    assert!(
        loud.len() > quiet.len(),
        "--trace must add transport detail:\nquiet={quiet}\nloud={loud}"
    );
    assert!(loud.contains("Executing saved plan"), "{loud}");
}

/// A project whose `bravo` cannot be written, so the reviewed plan is
/// guaranteed to fail halfway — after `alpha` has already rewritten the lock.
///
/// `/dev/null/...` is a path no `mkdir -p` can create, which is what makes the
/// failure independent of permissions, umask and who runs the suite.
fn project_that_half_fails(dir: &Path) -> Project {
    let p = project(dir);
    let cfg_text = std::fs::read_to_string(&p.cfg).expect("read cfg");
    std::fs::write(
        &p.cfg,
        cfg_text.replace(
            &p.bravo.display().to_string(),
            "/dev/null/forjar-rollback/bravo.txt",
        ),
    )
    .expect("rewrite cfg");
    p
}

/// The content hash the lock records for `alpha`.
///
/// Not the lock's raw bytes: the `details` map serialises in an arbitrary key
/// order, so two byte-identical states can render differently and a byte
/// comparison would fail for a reason that has nothing to do with rollback.
/// `alpha` is the only converged resource in this fixture, so the single
/// `hash:` line is its.
fn alpha_lock_hash(state: &Path) -> Option<String> {
    let text = std::fs::read_to_string(forjar::core::state::lock_file_path(state, "web")).ok()?;
    text.lines()
        .find(|l| l.trim_start().starts_with("hash: blake3:"))
        .map(|l| l.trim().to_string())
}

/// Seed a lock, save a plan over the remaining work, and run it with `extra`.
/// Returns the lock before the plan apply and the lock after.
fn half_failing_run(dir: &Path, extra: &[&str]) -> (Option<String>, Option<String>, String) {
    let p = project_that_half_fails(dir);
    // A first apply of `alpha` alone, so a lock EXISTS to be rolled back to and
    // the comparison is not "absent vs present".
    let seed = forjar()
        .args(["apply", "-f"])
        .arg(&p.cfg)
        .arg("--state-dir")
        .arg(&p.state)
        .args(["--yes", "-r", "alpha"])
        .output()
        .expect("seed");
    assert!(seed.status.success(), "seed: {}", combined(&seed));
    // Change alpha's content so the plan has real work for it as well as for
    // the resource that will fail.
    let cfg_text = std::fs::read_to_string(&p.cfg).expect("read cfg");
    std::fs::write(
        &p.cfg,
        cfg_text.replace("content: \"A\"", "content: \"A2\""),
    )
    .expect("rewrite cfg");

    let plan_path = dir.join("p.json");
    p.save_plan(&plan_path);
    let before = alpha_lock_hash(&p.state);
    assert!(before.is_some(), "the seed must have written a lock");

    let out = p.apply_plan(&plan_path, extra);
    let text = combined(&out);
    assert!(!out.status.success(), "the plan must fail: {text}");
    (before, alpha_lock_hash(&p.state), text)
}

/// The CONTROL. Without `--rollback-on-failure` the successful half of a failed
/// plan stays in the lock — which is what makes the treatment below observable
/// rather than vacuous.
#[test]
fn without_rollback_a_half_failed_plan_leaves_its_successful_half_in_the_lock() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (before, after, text) = half_failing_run(dir.path(), &[]);
    assert_ne!(
        before, after,
        "the control must move alpha's recorded hash: {text}"
    );
}

/// `--rollback-on-failure` is the flag the issue named: an operator who
/// believed it was armed on a plan apply was wrong, silently, because
/// `rollback_on_failure: false` was hard-coded in the `ApplyConfig` literal.
///
/// Armed, it snapshots the pre-apply locks and restores them when any resource
/// fails, so the same half-failing plan must leave the lock exactly as it found
/// it.
#[test]
fn rollback_on_failure_is_armed_on_a_plan_apply() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (before, after, text) = half_failing_run(dir.path(), &["--rollback-on-failure"]);
    assert_eq!(
        before, after,
        "--rollback-on-failure must restore the pre-apply lock: {text}"
    );
}

/// `--retry`, `--timeout`, `--resource-timeout`, `--max-parallel`, `--parallel`
/// and `--force-unlock` have no observable print of their own on a clean run,
/// so what is proved here is that each REACHES the run rather than tripping the
/// refusal list, and that the reviewed delta still converges exactly.
///
/// A flag silently dropped and a flag correctly honoured look identical on a
/// successful apply; the guards that keep them apart are
/// `every_knob_is_read_from_its_own_flag` and
/// `the_plan_paths_apply_config_hard_codes_only_what_is_refused` in
/// `src/cli/tests_dispatch_apply_b.rs` — one reads the struct the dispatcher
/// builds, the other reads the `ApplyConfig` literal itself.
#[test]
fn the_remaining_knobs_are_accepted_and_the_reviewed_delta_still_converges() {
    for knob in [
        vec!["--retry", "2"],
        vec!["--timeout", "30"],
        vec!["--resource-timeout", "30"],
        vec!["--max-parallel", "2"],
        vec!["--parallel"],
        vec!["--force-unlock"],
    ] {
        let dir = tempfile::tempdir().expect("tempdir");
        let (p, plan_path) = planned(dir.path());
        let out = p.apply_plan(&plan_path, &knob);
        let text = combined(&out);
        assert!(out.status.success(), "{knob:?}: {text}");
        assert_eq!(p.converged(), (true, true), "{knob:?}: {text}");
    }
}

/// GH-208: every flag in the `--dry-run` family means "change nothing".
/// `--plan-file` was handed `args.dry_run` alone, so `--dry-run-json` converged.
#[test]
fn the_whole_dry_run_family_previews_a_plan_apply() {
    for flag in ["--dry-run", "--dry-run-json", "--dry-run-summary"] {
        let dir = tempfile::tempdir().expect("tempdir");
        let (p, plan_path) = planned(dir.path());
        let out = p.apply_plan(&plan_path, &[flag]);
        let text = combined(&out);
        assert!(out.status.success(), "{flag}: {text}");
        assert_eq!(
            p.converged(),
            (false, false),
            "{flag} must converge nothing: {text}"
        );
    }
}

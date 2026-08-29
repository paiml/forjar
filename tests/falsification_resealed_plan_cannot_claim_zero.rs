//! Refs #358 — a plan file that claims "no changes" must be checkable, and an
//! unkeyed seal cannot check it.
//!
//! # The claim this file exists to falsify
//!
//! The seal work claimed `plan_seal::check_body_partition` "still refuses a
//! zero-the-counters edit whose author ALSO recomputed the seal". It does not,
//! and the adversary below is fifteen lines. `check_body_partition` asserts
//! that the four counters partition the change list; `0/0/0/0` over an EMPTY
//! change list partitions trivially. The doc comment was precise about what it
//! catches — a plan that "claims 0 changes while listing several" — and the
//! attack simply empties the list.
//!
//! The seal is an unkeyed BLAKE3 hash. Anyone who can run `forjar` can compute
//! one, so no arrangement of it can distinguish a plan forjar issued from a
//! plan an adversary issued. That is stated honestly in `core::plan_seal`, and
//! it is why nothing here tries to close the hole with more hashing.
//!
//! # What CAN close it, with no secret at all
//!
//! Re-plan and compare. `cmd_apply_from_plan` already holds the config and the
//! state directory, so the planner is nearly free to run, and a plan file's
//! claim about a `(machine, resource)` pair is checkable against what the
//! planner says about that same pair RIGHT NOW. The adversary cannot make the
//! real planner return `NoOp` while a create is genuinely pending, so this
//! defeats them completely — and it needs no key, no clock and no trust.
//!
//! Every test here drives `CARGO_BIN_EXE_forjar`, because the thing that was
//! wrong is the exit code an operator or a CI job reads.

#[path = "common/plan_forge.rs"]
mod plan_forge;

use forjar::core::types::PlanAction;
use plan_forge::{body, change, combined, read_plan, reseal};
use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

struct Project {
    cfg: PathBuf,
    state: PathBuf,
    managed: PathBuf,
}

/// One localhost file resource, so an apply is real but harmless.
fn project(dir: &Path) -> Project {
    let managed = dir.join("managed.txt");
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\n\
             name: resealed-zero\n\
             machines:\n\
             \x20 localhost:\n\
             \x20   hostname: localhost\n\
             \x20   addr: 127.0.0.1\n\
             resources:\n\
             \x20 managed:\n\
             \x20   type: file\n\
             \x20   machine: localhost\n\
             \x20   path: {}\n\
             \x20   content: \"sealed\"\n",
            managed.display()
        ),
    )
    .expect("write config");
    Project {
        cfg,
        state: dir.join("state"),
        managed,
    }
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

fn apply_plan(p: &Project, plan: &Path) -> std::process::Output {
    forjar()
        .args(["apply", "-f"])
        .arg(&p.cfg)
        .arg("--state-dir")
        .arg(&p.state)
        .arg("--plan-file")
        .arg(plan)
        .arg("--yes")
        .output()
        .expect("run apply")
}

fn apply_all(p: &Project) -> std::process::Output {
    forjar()
        .args(["apply", "-f"])
        .arg(&p.cfg)
        .arg("--state-dir")
        .arg(&p.state)
        .arg("--yes")
        .output()
        .expect("run apply")
}

/// RED-1: the reported defect verbatim. An empty change list with all four
/// counters at zero, re-sealed.
///
/// `check_body_partition` cannot fire — `0/0/0/0` over an empty list partitions
/// trivially — and every leg of the seal verifies, because the adversary
/// recomputed the one leg their edit moved.
///
/// MEASURED before the fix: `Plan has no changes to apply.` exit 0, with the
/// create still pending.
#[test]
fn a_resealed_empty_plan_cannot_certify_that_there_is_nothing_to_do() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&p, &plan_path).status.success());

    reseal(&plan_path, &body("resealed-zero", vec![], &[]));

    let out = apply_plan(&p, &plan_path);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "a plan claiming zero changes while a create is pending must not exit 0: {text}"
    );
    assert!(
        !text.contains("Plan has no changes to apply."),
        "the forged claim must not be repeated back as a benign sentence: {text}"
    );
    assert!(
        text.contains("PLAN_STALE"),
        "the refusal must name the check that caught it: {text}"
    );
    assert!(
        !p.managed.exists(),
        "nothing should have been converged from a refused plan"
    );
}

/// RED-2: the same attack with the change list KEPT and the pending create
/// relabelled `no_op`, so the counters still partition their list.
///
/// This is the shape that survives an "is the list empty?" test, and it is
/// refused for the same reason: the planner says `create`.
#[test]
fn a_resealed_plan_cannot_relabel_a_pending_create_as_a_no_op() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&p, &plan_path).status.success());

    reseal(
        &plan_path,
        &body(
            "resealed-zero",
            vec![change(
                "managed",
                "localhost",
                PlanAction::NoOp,
                "managed: already converged",
            )],
            &["managed"],
        ),
    );

    let out = apply_plan(&p, &plan_path);
    let text = combined(&out);
    assert!(!out.status.success(), "{text}");
    assert!(text.contains("PLAN_STALE"), "{text}");
    assert!(!p.managed.exists(), "{text}");
}

/// GREEN GUARD. Re-planning must not become "refuse every zero". A stack that
/// is genuinely converged plans to nothing, and applying that plan is a
/// legitimate, successful no-op.
#[test]
fn an_honest_zero_change_plan_still_applies_cleanly() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());

    let seed = apply_all(&p);
    assert!(seed.status.success(), "seed apply: {}", combined(&seed));

    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&p, &plan_path).status.success());
    assert_eq!(
        read_plan(&plan_path)["to_create"],
        0,
        "the converged stack must genuinely plan to zero changes"
    );

    let out = apply_plan(&p, &plan_path);
    let text = combined(&out);
    assert!(
        out.status.success(),
        "an honest zero plan must apply: {text}"
    );
    assert!(text.contains("Plan has no changes to apply."), "{text}");
}

/// GREEN GUARD. Re-planning must not become "refuse every plan": an untouched
/// sealed plan with a real create still converges it.
#[test]
fn an_untouched_sealed_plan_still_converges_what_it_named() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(plan_out(&p, &plan_path).status.success());

    let out = apply_plan(&p, &plan_path);
    let text = combined(&out);
    assert!(out.status.success(), "{text}");
    assert_eq!(
        std::fs::read_to_string(&p.managed).expect("managed"),
        "sealed"
    );
}

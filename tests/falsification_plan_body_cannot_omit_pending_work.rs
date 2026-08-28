//! Refs #358 — a plan body cannot make work disappear by not mentioning it.
//!
//! # The claim this file exists to falsify
//!
//! The previous round closed the re-sealing adversary and said so. It closed
//! ONE SHAPE of it. `check_an_empty_body_is_honest` keyed off
//! `plan.changes.is_empty()` — a syntactic accident, because what decides
//! whether anything executes is `PlanScope::from_plan`, which skips `NoOp`.
//!
//! So run the same attack against a PARTIALLY CONVERGED stack, which is every
//! real deployment. `bravo` converged, `alpha` still pending. Instead of
//! emptying the change list, delete the one pending line and keep the honest
//! `no_op` line beside it. The counters still partition (0/0/0/1), so
//! `plan_seal::check_body_partition` is silent; the list is not empty, so the
//! empty-body check is silent; the scope IS empty, so nothing executes.
//! Measured against the built binary before the fix:
//!
//! ```text
//!   $ forjar apply --plan-file forged_delete.json --yes
//!   Plan has no changes to apply.
//!   exit=0
//!   alpha STILL-PENDING
//! ```
//!
//! — #358's defect verbatim, reached through the fix for #358.
//!
//! # Why the obvious repair is wrong
//!
//! "Use `scope.is_empty()`, the predicate already in the file three lines
//! below." That refuses the attack — and refuses this, which is legitimate and
//! exits 0 today:
//!
//! ```text
//!   $ forjar plan -r bravo --out narrow.json     # bravo already converged
//!   $ forjar apply --plan-file narrow.json --yes
//!   Plan has no changes to apply.
//! ```
//!
//! Both documents are `changes: [bravo no_op]`, counters `0/0/0/1`, scope
//! empty. They are byte-identical. No predicate over the document can separate
//! them, which is why the plan file now RECORDS the selectors it was written
//! under and `apply --plan-file` re-plans through those. Every test below
//! carries its legitimate twin for exactly that reason.

#[path = "common/plan_forge.rs"]
mod plan_forge;
#[path = "common/plan_project.rs"]
mod plan_project;

use forjar::core::plan_selectors::PlanSelectors;
use forjar::core::types::PlanAction;
use plan_forge::{body, change, read_plan, reseal, reseal_as};
use plan_project::{combined, project};

/// The honest `no_op` line the forgery keeps, so the body still looks like a
/// plan over a converged resource.
fn honest_bravo_noop() -> forjar::core::types::PlannedChange {
    change("bravo", "web", PlanAction::NoOp, "bravo: no changes")
}

/// RED-1 — THE REPORTED EVASION. A partially converged stack, the pending
/// `create` line deleted, the honest `no_op` line kept.
///
/// Nothing about this body is "empty", and the counters partition it, so both
/// of the previous rounds' predicates are silent. It has to be caught by the
/// work the body FAILS to name.
#[test]
fn a_deleted_pending_line_cannot_certify_that_there_is_nothing_to_do() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    p.converge_bravo();

    let plan_path = dir.path().join("p.json");
    assert!(p.plan(&plan_path, &[]).status.success());
    assert_eq!(
        read_plan(&plan_path)["to_create"],
        1,
        "the honest plan must have one pending create to delete"
    );

    reseal(
        &plan_path,
        &body("partial", vec![honest_bravo_noop()], &["alpha", "bravo"]),
    );

    let out = p.apply_plan(&plan_path, &[]);
    let text = combined(&out);
    assert!(
        !out.status.success(),
        "a plan that omits a pending create must not exit 0: {text}"
    );
    assert!(
        !text.contains("Plan has no changes to apply."),
        "the forged claim must not be repeated back as a benign sentence: {text}"
    );
    assert!(text.contains("PLAN_STALE"), "{text}");
    assert!(
        text.contains("alpha on web"),
        "the refusal must name the work that was omitted: {text}"
    );
    assert!(!p.alpha.exists(), "nothing may be converged from a refusal");
}

/// The CONTROL for RED-1. Every seal leg verifies on that document, so the
/// refusal is demonstrably the re-plan check and not the hashing.
#[test]
fn the_deleted_line_forgery_passes_every_seal_check() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    p.converge_bravo();

    let plan_path = dir.path().join("p.json");
    assert!(p.plan(&plan_path, &[]).status.success());
    reseal(
        &plan_path,
        &body("partial", vec![honest_bravo_noop()], &["alpha", "bravo"]),
    );

    let text = combined(&p.apply_plan(&plan_path, &[]));
    for seal_code in ["PLAN_HASH_MISMATCH", "PLAN_MALFORMED", "PLAN_EXPIRED"] {
        assert!(
            !text.contains(seal_code),
            "{seal_code} must not appear — the seal is intact and the refusal \
             must come from re-planning: {text}"
        );
    }
}

/// RED-2 — the same deletion out of a plan that is still BUSY afterwards, so
/// not one emptiness predicate applies at any point.
///
/// Both resources pending; delete `bravo`'s line and keep `alpha`'s. The apply
/// would converge `alpha`, print `Plan applied: 1 converged`, exit 0, and leave
/// `bravo` untouched with nothing said about it.
#[test]
fn a_deleted_line_out_of_a_busy_plan_is_refused_too() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());

    let plan_path = dir.path().join("p.json");
    assert!(p.plan(&plan_path, &[]).status.success());
    reseal(
        &plan_path,
        &body(
            "partial",
            vec![change(
                "alpha",
                "web",
                PlanAction::Create,
                "alpha: create alpha.txt",
            )],
            &["alpha", "bravo"],
        ),
    );

    let out = p.apply_plan(&plan_path, &[]);
    let text = combined(&out);
    assert!(!out.status.success(), "{text}");
    assert!(text.contains("PLAN_STALE"), "{text}");
    assert!(text.contains("bravo on web"), "{text}");
    assert!(
        !p.alpha.exists() && !p.bravo.exists(),
        "a refused plan converges nothing at all: {text}"
    );
}

/// GREEN GUARD — THE LEGITIMATE TWIN of RED-1, and the reason the naive
/// `scope.is_empty()` repair is wrong.
///
/// `plan -r bravo --out` over a converged `bravo` writes `changes: [bravo
/// no_op]`, counters `0/0/0/1`, empty scope — the same document RED-1 forges.
/// It must still apply cleanly, or every idempotent CI loop over a filtered
/// plan starts failing.
#[test]
fn a_legitimately_filtered_plan_over_a_converged_resource_still_applies() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    p.converge_bravo();

    let plan_path = dir.path().join("narrow.json");
    assert!(p.plan(&plan_path, &["-r", "bravo"]).status.success());
    let doc = read_plan(&plan_path);
    assert_eq!(doc["unchanged"], 1, "the twin's shape: {doc}");
    assert_eq!(doc["to_create"], 0, "the twin's shape: {doc}");
    assert_eq!(doc["selectors"]["resource"], "bravo", "recorded: {doc}");

    let out = p.apply_plan(&plan_path, &[]);
    let text = combined(&out);
    assert!(
        out.status.success(),
        "a filtered plan asking for nothing must still exit 0: {text}"
    );
    assert!(text.contains("Plan has no changes to apply."), "{text}");
}

/// …and it says what it is leaving undone.
///
/// The seal is unkeyed, so a forgery CAN declare itself narrow and reach this
/// path honestly. What it can no longer do is reach it quietly: the sentence an
/// operator reads names the pending work outside the filter.
#[test]
fn a_filtered_plan_that_applies_nothing_discloses_the_work_it_skips() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    p.converge_bravo();

    let plan_path = dir.path().join("narrow.json");
    assert!(p.plan(&plan_path, &["-r", "bravo"]).status.success());

    let text = combined(&p.apply_plan(&plan_path, &[]));
    assert!(text.contains("this plan is filtered"), "{text}");
    assert!(text.contains("-r bravo"), "{text}");
    assert!(
        text.contains("alpha on web"),
        "the work outside the filter must be named: {text}"
    );
}

/// GREEN GUARD — an UNFILTERED plan over a fully converged stack says nothing
/// about work outside a filter it does not have.
#[test]
fn an_unfiltered_converged_plan_applies_without_a_disclosure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let seed = p.apply(&[]);
    assert!(seed.status.success(), "{}", combined(&seed));

    let plan_path = dir.path().join("p.json");
    assert!(p.plan(&plan_path, &[]).status.success());
    let out = p.apply_plan(&plan_path, &[]);
    let text = combined(&out);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("Plan has no changes to apply."), "{text}");
    assert!(!text.contains("this plan is filtered"), "{text}");
}

/// GREEN GUARD — an untouched whole-stack plan still converges everything it
/// named. The refusals must not become "refuse every plan".
#[test]
fn an_untouched_plan_still_converges_both_resources() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    let plan_path = dir.path().join("p.json");
    assert!(p.plan(&plan_path, &[]).status.success());

    let out = p.apply_plan(&plan_path, &[]);
    let text = combined(&out);
    assert!(out.status.success(), "{text}");
    assert!(p.alpha.exists() && p.bravo.exists(), "{text}");
}

/// The forger's remaining move, and its cost. Re-labelling the forged document
/// as a `-r bravo` plan DOES make the re-plan agree — the planner really does
/// find nothing pending for `bravo`. It is no longer invisible: the document has
/// to carry the claim, `apply --plan-file` prints it, and the work outside it is
/// named. That is the honest limit of an unkeyed seal, recorded as a test so
/// nobody reads the fix as authentication.
#[test]
fn a_forgery_that_claims_to_be_narrow_must_declare_it_and_is_disclosed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    p.converge_bravo();

    let plan_path = dir.path().join("p.json");
    assert!(p.plan(&plan_path, &[]).status.success());
    reseal_as(
        &plan_path,
        &body("partial", vec![honest_bravo_noop()], &["bravo"]),
        &PlanSelectors::new(None, Some("bravo"), None, None),
    );

    let out = p.apply_plan(&plan_path, &[]);
    let text = combined(&out);
    assert!(out.status.success(), "a self-declared narrow plan: {text}");
    assert!(
        text.contains("this plan is filtered") && text.contains("alpha on web"),
        "it must say what it is not doing: {text}"
    );
    assert!(!p.alpha.exists());
}

/// …and it cannot make that claim WITHOUT re-sealing: the selector record is
/// inside the diff leg, so editing it alone is a `PLAN_HASH_MISMATCH`.
#[test]
fn editing_only_the_selectors_of_an_honest_plan_breaks_its_seal() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = project(dir.path());
    p.converge_bravo();

    let plan_path = dir.path().join("p.json");
    assert!(p.plan(&plan_path, &[]).status.success());
    let mut doc = read_plan(&plan_path);
    doc["selectors"]["resource"] = serde_json::json!("bravo");
    std::fs::write(
        &plan_path,
        serde_json::to_string_pretty(&doc).expect("render"),
    )
    .expect("write");

    let out = p.apply_plan(&plan_path, &[]);
    let text = combined(&out);
    assert!(!out.status.success(), "{text}");
    assert!(text.contains("PLAN_HASH_MISMATCH"), "{text}");
    assert!(text.contains("diff leg"), "{text}");
}

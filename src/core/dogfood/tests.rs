//! FJ-038: tests for the dogfood gate itself.
//!
//! These guard the property that makes the gate worth having: it cannot go
//! quiet. A gate that silently stops covering things is worse than no gate,
//! because it reports GO with authority.

use super::*;

#[test]
fn every_resource_type_appears_in_all_types() {
    // ALL_TYPES drives the run. A variant missing from it is silently never
    // dogfooded, no matter what `coverage` says about it.
    //
    // `ResourceType` has no reflection, so this compares against the Display
    // round-trip set the enum itself pins in resource_enums.rs.
    let listed: Vec<String> = ALL_TYPES.iter().map(ToString::to_string).collect();
    for name in [
        "package",
        "file",
        "service",
        "mount",
        "user",
        "docker",
        "pepita",
        "network",
        "cron",
        "recipe",
        "model",
        "gpu",
        "task",
        "wasm_bundle",
        "image",
        "build",
        "github_release",
        "overlay_interface",
        "disk_budget",
        "backup_sync",
    ] {
        assert!(
            listed.iter().any(|l| l == name),
            "resource type `{name}` is missing from ALL_TYPES, so it is never dogfooded"
        );
    }
    assert_eq!(
        listed.len(),
        21,
        "ALL_TYPES has {} entries; update this test when a type is added",
        listed.len()
    );
}

#[test]
fn no_type_is_silently_uncovered() {
    // Every type resolves to Exercised or NotApplicable-with-a-reason. The
    // exhaustive match in `coverage` makes this a compile-time guarantee; this
    // test pins the runtime half — that no reason is an empty string.
    for t in ALL_TYPES {
        match coverage(t) {
            Coverage::Exercised => {}
            Coverage::NotApplicable(why) => assert!(
                !why.trim().is_empty(),
                "{t} is NotApplicable with no reason — state the debt or exercise it"
            ),
        }
    }
}

#[test]
fn the_resources_that_shipped_broken_are_exercised() {
    // disk_budget and backup_sync each shipped a release-breaking bug that only
    // real data or the real external tool could expose. They must never be
    // downgraded to NotApplicable to make a dogfood run pass.
    for t in [ResourceType::DiskBudget, ResourceType::BackupSync] {
        assert_eq!(
            coverage(&t),
            Coverage::Exercised,
            "{t} must stay exercised — it shipped broken precisely because it was not"
        );
    }
}

#[test]
fn exercised_types_all_have_a_real_exercise() {
    // `run_for` returns a failing outcome for an Exercised type with no
    // implementation, rather than silently passing.
    for t in ALL_TYPES {
        if coverage(t) != Coverage::Exercised {
            continue;
        }
        let o = exercises::run_for(t);
        assert!(
            !o.detail.contains("has no exercise"),
            "{t} is declared Exercised but has no exercise implemented"
        );
    }
}

#[test]
fn not_applicable_debt_is_reportable() {
    let debt = not_applicable();
    assert!(
        !debt.is_empty(),
        "some types are legitimately host-mutating"
    );
    for (name, why) in &debt {
        assert!(!name.is_empty());
        assert!(!why.is_empty(), "{name} carries an empty reason");
    }
}

#[test]
fn disk_budget_exercise_detects_both_real_cargo_layouts() {
    // The regression that shipped as 1.13.1: both-markers matched neither real
    // layout. This runs the actual exercise, which builds both shapes on disk.
    let o = exercises::run_for(&ResourceType::DiskBudget);
    assert!(
        o.passed,
        "disk_budget dogfood exercise failed: {}",
        o.detail
    );
    assert!(o.detail.contains("4 real shapes"), "{}", o.detail);
}

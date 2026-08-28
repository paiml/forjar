//! Unit tests for seal/verify: one per `SealError` variant, plus the boundary
//! cases that make "expired" and "tampered" mean something definite.

use super::tests_digest::{cfg, plan_of};
use super::*;

const T0: u64 = 1_000_000;

fn seeded_state() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let path = crate::core::state::lock_file_path(d.path(), "alpha");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "machine: alpha\nresources: {}\n").unwrap();
    d
}

#[test]
fn an_untouched_seal_verifies() {
    let d = seeded_state();
    let config = cfg("one");
    let plan = plan_of(2);
    let sealed = seal_at(&plan, &config, d.path(), None, T0).unwrap();
    assert_eq!(sealed.version, SEAL_VERSION);
    assert_eq!(
        verify_at(&sealed, &plan, &config, d.path(), T0 + 10_000_000),
        Ok(()),
        "with no TTL requested, age alone never invalidates a plan"
    );
}

#[test]
fn two_seals_of_the_same_inputs_are_identical() {
    let d = seeded_state();
    let config = cfg("one");
    let plan = plan_of(2);
    let a = seal_at(&plan, &config, d.path(), None, T0).unwrap();
    for _ in 0..50 {
        let b = seal_at(&plan, &config, d.path(), None, T0).unwrap();
        assert_eq!(a, b, "sealing is deterministic — no RNG, no clock read");
    }
    let later = seal_at(&plan, &config, d.path(), None, T0 + 1).unwrap();
    assert_eq!(a.config_hash, later.config_hash);
    assert_eq!(a.state_hash, later.state_hash);
    assert_eq!(a.diff_hash, later.diff_hash);
    assert_ne!(a.seal, later.seal, "sealed_at is part of the seal");
}

#[test]
fn a_changed_config_names_the_config_leg() {
    let d = seeded_state();
    let plan = plan_of(1);
    let sealed = seal_at(&plan, &cfg("one"), d.path(), None, T0).unwrap();
    let err = verify_at(&sealed, &plan, &cfg("two"), d.path(), T0).unwrap_err();
    assert_eq!(err.code(), "PLAN_HASH_MISMATCH");
    assert!(matches!(
        err,
        SealError::PlanHashMismatch {
            leg: Leg::Config,
            ..
        }
    ));
    assert!(err.to_string().contains("config leg"), "{err}");
}

#[test]
fn a_lock_rewritten_after_sealing_names_the_state_leg() {
    let d = seeded_state();
    let config = cfg("one");
    let plan = plan_of(1);
    let sealed = seal_at(&plan, &config, d.path(), None, T0).unwrap();

    std::fs::write(
        crate::core::state::lock_file_path(d.path(), "alpha"),
        "machine: alpha\nresources: {a: tampered}\n",
    )
    .unwrap();

    let err = verify_at(&sealed, &plan, &config, d.path(), T0).unwrap_err();
    assert!(matches!(
        err,
        SealError::PlanHashMismatch {
            leg: Leg::State,
            ..
        }
    ));
    assert!(err.to_string().contains("state lock changed"), "{err}");
}

#[test]
fn an_edited_body_names_the_diff_leg() {
    let d = seeded_state();
    let config = cfg("one");
    let sealed = seal_at(&plan_of(2), &config, d.path(), None, T0).unwrap();
    let err = verify_at(&sealed, &plan_of(1), &config, d.path(), T0).unwrap_err();
    assert!(matches!(
        err,
        SealError::PlanHashMismatch { leg: Leg::Diff, .. }
    ));
}

#[test]
fn moving_the_expiry_is_a_hash_mismatch_not_a_longer_life() {
    let d = seeded_state();
    let config = cfg("one");
    let plan = plan_of(1);
    let mut sealed = seal_at(&plan, &config, d.path(), Some(60), T0).unwrap();
    sealed.ttl_secs = MAX_TTL_SECS;

    let err = verify_at(&sealed, &plan, &config, d.path(), T0).unwrap_err();
    assert_eq!(err.code(), "PLAN_HASH_MISMATCH");
    assert!(
        matches!(err, SealError::PlanHashMismatch { leg: Leg::Seal, .. }),
        "the validity window is INSIDE the composition"
    );
}

#[test]
fn backdating_the_seal_is_a_hash_mismatch() {
    let d = seeded_state();
    let config = cfg("one");
    let plan = plan_of(1);
    let mut sealed = seal_at(&plan, &config, d.path(), Some(60), T0).unwrap();
    sealed.sealed_at_unix = T0 + 10_000;
    assert!(matches!(
        verify_at(&sealed, &plan, &config, d.path(), T0).unwrap_err(),
        SealError::PlanHashMismatch { leg: Leg::Seal, .. }
    ));
}

#[test]
fn an_expired_plan_is_rejected_without_touching_the_clock() {
    let d = seeded_state();
    let config = cfg("one");
    let plan = plan_of(1);
    let sealed = seal_at(&plan, &config, d.path(), Some(900), T0).unwrap();

    assert!(verify_at(&sealed, &plan, &config, d.path(), T0 + 899).is_ok());
    assert!(
        verify_at(&sealed, &plan, &config, d.path(), T0 + 900).is_ok(),
        "expiry is inclusive of sealed_at + ttl"
    );
    let err = verify_at(&sealed, &plan, &config, d.path(), T0 + 901).unwrap_err();
    assert_eq!(err.code(), "PLAN_EXPIRED");
    assert!(err.to_string().contains("expired at 1000900"), "{err}");
}

#[test]
fn an_unknown_seal_version_is_refused_outright() {
    let d = seeded_state();
    let config = cfg("one");
    let plan = plan_of(1);
    let mut sealed = seal_at(&plan, &config, d.path(), None, T0).unwrap();
    sealed.version = "forjar-plan-seal-v99".to_string();
    let err = verify_at(&sealed, &plan, &config, d.path(), T0).unwrap_err();
    assert_eq!(err.code(), "PLAN_VERSION_UNKNOWN");
}

#[test]
fn counters_that_contradict_the_change_list_are_malformed() {
    let d = seeded_state();
    let config = cfg("one");
    let mut plan = plan_of(2);
    plan.to_create = 0;

    // Sealed over the LIE, so every hash matches — the structural check is the
    // only thing standing between a re-sealed edit and a silent no-op apply.
    let sealed = seal_at(&plan, &config, d.path(), None, T0).unwrap();
    let err = verify_at(&sealed, &plan, &config, d.path(), T0).unwrap_err();
    assert_eq!(err.code(), "PLAN_MALFORMED");
    assert!(err.to_string().contains("to_create"), "{err}");
}

#[test]
fn every_counter_field_is_checked() {
    let base = plan_of(1);
    for mutate in [
        |p: &mut crate::core::types::ExecutionPlan| p.to_create += 1,
        |p: &mut crate::core::types::ExecutionPlan| p.to_update += 1,
        |p: &mut crate::core::types::ExecutionPlan| p.to_destroy += 1,
        |p: &mut crate::core::types::ExecutionPlan| p.unchanged += 1,
    ] {
        let mut plan = base.clone();
        mutate(&mut plan);
        assert_eq!(
            check_body_partition(&plan).unwrap_err().code(),
            "PLAN_MALFORMED"
        );
    }
    assert!(check_body_partition(&base).is_ok());
}

#[test]
fn ttl_is_clamped_and_zero_means_no_expiry() {
    assert_eq!(clamp_ttl(None), TTL_NO_EXPIRY);
    assert_eq!(clamp_ttl(Some(0)), TTL_NO_EXPIRY);
    assert_eq!(clamp_ttl(Some(1)), MIN_TTL_SECS);
    assert_eq!(clamp_ttl(Some(u64::MAX)), MAX_TTL_SECS);
    assert_eq!(clamp_ttl(Some(DEFAULT_TTL_SECS)), DEFAULT_TTL_SECS);
}

#[test]
fn the_clamped_ttl_is_what_gets_sealed() {
    let d = seeded_state();
    let sealed = seal_at(&plan_of(1), &cfg("one"), d.path(), Some(5), T0).unwrap();
    assert_eq!(
        sealed.ttl_secs, MIN_TTL_SECS,
        "the ACTUAL lifetime is sealed, never the raw request"
    );
}

#[test]
fn seal_uses_the_system_clock_and_verify_agrees_with_it() {
    let d = seeded_state();
    let config = cfg("one");
    let plan = plan_of(1);
    let sealed = seal(&plan, &config, d.path(), Some(MAX_TTL_SECS)).unwrap();
    assert!(sealed.sealed_at_unix > 1_700_000_000, "a real wall clock");
    assert!(verify(&sealed, &plan, &config, d.path()).is_ok());
}

#[test]
fn a_seal_document_with_an_unknown_field_is_refused() {
    let json = r#"{"version":"forjar-plan-seal-v1","plan_id":"x","config_hash":"a",
        "state_hash":"b","diff_hash":"c","sealed_at_unix":1,"ttl_secs":0,
        "seal":"d","extra":true}"#;
    assert!(serde_json::from_str::<PlanSeal>(json).is_err());
}

#[test]
fn leg_names_are_stable() {
    assert_eq!(Leg::Config.name(), "config");
    assert_eq!(Leg::State.name(), "state");
    assert_eq!(Leg::Diff.name(), "diff");
    assert_eq!(Leg::Seal.name(), "seal");
}

#[test]
fn an_unreadable_state_dir_entry_is_an_error_not_a_silent_absence() {
    // A directory where a lock file should be reads as a directory, not
    // NotFound — that must surface rather than hash as "no lock".
    let d = tempfile::tempdir().unwrap();
    let path = crate::core::state::lock_file_path(d.path(), "alpha");
    std::fs::create_dir_all(&path).unwrap();
    let err = digest::state_leg(&cfg("one"), d.path()).unwrap_err();
    assert!(err.contains("cannot read"), "{err}");
}

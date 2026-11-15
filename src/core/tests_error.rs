//! Unit tests of `error.rs` (split out to keep that file under the 500-line limit).

use super::*;

/// The exact text `forjar apply` produced against gx10. It is the reason
/// this module exists: a deterministic bashrs rejection, wrapped by
/// `resource_ops` in the words "transport error", which the prose
/// classifier read as a retryable connection failure.
const MEASURED_GX10_I8_FAILURE: &str =
    "transport error: I8 violation — script failed bashrs validation: bashrs lint errors: ...";

#[test]
fn the_measured_gx10_failure_is_validation_not_connection() {
    assert_eq!(
        classify_untyped(MEASURED_GX10_I8_FAILURE),
        ErrorClass::Validation
    );
    assert_eq!(
        ForjarError::from_untyped(MEASURED_GX10_I8_FAILURE).exit_code(),
        3,
        "an I8 bashrs rejection is deterministic — exiting 4 tells CI to retry \
         a failure that cannot succeed"
    );
}

/// Falsification: without the declared marker the SAME string classifies as
/// connection/4. This is what makes the fix a fix, and not luck.
#[test]
fn prose_alone_gets_the_measured_failure_wrong() {
    assert_eq!(
        legacy_prose_class(MEASURED_GX10_I8_FAILURE),
        ErrorClass::Connection,
        "if the prose heuristic ever got this right on its own, this test \
         would be measuring nothing"
    );
    assert_eq!(legacy_prose_class(MEASURED_GX10_I8_FAILURE).exit_code(), 4);
}

/// The EXECUTION half of KANI-FVS-002.
///
/// Kani proves `ErrorClass::exit_code` is variant-determined and injective,
/// allocation-free. It deliberately never constructs a `ForjarError`, because
/// that carries a `String` and modelling the allocator to prove a one-line
/// delegation costs 117 minutes for nothing (see kani_proofs_backup_sync).
///
/// So the delegation is proved HERE, by building real errors whose messages
/// differ in length, content, and — critically — in whether they contain the
/// very words the OLD prose classifier keyed on. If `exit_code` ever stopped
/// reading the variant, this is what catches it.
#[test]
fn the_exit_code_ignores_the_message_entirely() {
    let messages = [
        "",
        "x",
        "SSH connection refused",        // old heuristic: 4
        "YAML parse error at line 3",    // old heuristic: 3
        "drift detected on 2 resources", // old heuristic: 10
        "some resources failed",         // old heuristic: 2
        "transport error: I8 violation — script failed bashrs validation",
        &"very long message ".repeat(200),
    ];
    for class in [
        ErrorClass::Other,
        ErrorClass::Partial,
        ErrorClass::Validation,
        ErrorClass::Connection,
        ErrorClass::Drift,
    ] {
        let expected = class.exit_code();
        for m in &messages {
            let e = ForjarError::new(class, *m);
            assert_eq!(
                e.exit_code(),
                expected,
                "class {class:?} with message {m:?} produced exit {} — the code \
                 must come from the VARIANT, and this message is one the old \
                 prose classifier would have keyed on",
                e.exit_code()
            );
        }
    }
}

#[test]
fn exit_codes_are_the_published_contract() {
    assert_eq!(ErrorClass::Other.exit_code(), 1);
    assert_eq!(ErrorClass::Partial.exit_code(), 2);
    assert_eq!(ErrorClass::Validation.exit_code(), 3);
    assert_eq!(ErrorClass::Connection.exit_code(), 4);
    assert_eq!(ErrorClass::Drift.exit_code(), 10);
}

#[test]
fn a_declared_class_beats_the_prose_of_its_own_message() {
    // The message says every connection-ish word there is; the variant says
    // validation. The variant wins, because the producer knew.
    let e = ForjarError::validation("SSH connection transport all at once");
    assert_eq!(e.class(), ErrorClass::Validation);
    assert_eq!(e.exit_code(), 3);
}

#[test]
fn every_declared_marker_round_trips() {
    for (marker, class) in DECLARED_MARKERS {
        assert_eq!(
            classify_untyped(marker),
            *class,
            "marker {marker:?} does not classify as {class}"
        );
        // And it survives being wrapped by a caller, which is the whole point.
        let wrapped = format!("transport error: {marker}: details here");
        assert_eq!(classify_untyped(&wrapped), *class);
    }
}

#[test]
fn into_untyped_preserves_the_declared_class() {
    let lowered = ForjarError::validation(format!(
        "{I8_VALIDATION_MARKER}: bashrs lint errors:\n[Error] SC2135: x"
    ))
    .into_untyped();
    assert_eq!(ForjarError::from_untyped(lowered).exit_code(), 3);
}

#[test]
fn legacy_meanings_are_unchanged() {
    // Same inputs, same codes as the pre-taxonomy classifier in `main`.
    assert_eq!(
        classify_untyped("validation error: bad field").exit_code(),
        3
    );
    assert_eq!(
        classify_untyped("YAML parse error at line 3").exit_code(),
        3
    );
    assert_eq!(classify_untyped("SSH handshake failed").exit_code(), 4);
    assert_eq!(classify_untyped("connection refused").exit_code(), 4);
    assert_eq!(classify_untyped("transport timeout").exit_code(), 4);
    assert_eq!(classify_untyped("partial apply").exit_code(), 2);
    assert_eq!(classify_untyped("some resources failed").exit_code(), 2);
    assert_eq!(
        classify_untyped("drift detected in 2 resources").exit_code(),
        10
    );
    assert_eq!(classify_untyped("something else entirely").exit_code(), 1);
}

#[test]
fn constructors_carry_their_class() {
    assert_eq!(ForjarError::other("x").class(), ErrorClass::Other);
    assert_eq!(ForjarError::partial("x").class(), ErrorClass::Partial);
    assert_eq!(ForjarError::validation("x").class(), ErrorClass::Validation);
    assert_eq!(ForjarError::connection("x").class(), ErrorClass::Connection);
    assert_eq!(ForjarError::drift("x").class(), ErrorClass::Drift);
    assert_eq!(
        ForjarError::new(ErrorClass::Drift, "x").class(),
        ErrorClass::Drift
    );
}

#[test]
fn display_is_the_message_and_nothing_else() {
    // `main` prints "error: {e}" — the taxonomy must not change what a user
    // reads, only what the process exits with.
    let e = ForjarError::connection("ssh: connect to host gx10 port 22: No route to host");
    assert_eq!(
        e.to_string(),
        "ssh: connect to host gx10 port 22: No route to host"
    );
    assert_eq!(e.message(), e.to_string());
    assert_eq!(
        ForjarError::other("boom").into_message(),
        "boom".to_string()
    );
}

#[test]
fn class_labels_are_stable() {
    assert_eq!(ErrorClass::Validation.label(), "validation");
    assert_eq!(ErrorClass::Connection.to_string(), "connection");
    assert_eq!(ErrorClass::Partial.to_string(), "partial");
    assert_eq!(ErrorClass::Drift.to_string(), "drift");
    assert_eq!(ErrorClass::Other.to_string(), "other");
}

#[test]
fn errors_are_std_errors() {
    fn as_std(e: &dyn std::error::Error) -> String {
        e.to_string()
    }
    assert_eq!(as_std(&ForjarError::other("boom")), "boom");
}

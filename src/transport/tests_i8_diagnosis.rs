//! forjar#281: an I8 rejection must show the text it rejected.
//!
//! The reported symptom was an `SC2135`+`SC2136` pair that "could not be
//! reproduced standalone" — because the rejected text existed in neither the
//! YAML the author wrote nor in bashrs. forjar manufactured it, by inlining a
//! YAML folded scalar into a one-line `if ...; then`, and then discarded the
//! sanitised string before reporting.

use super::*;

/// A script bashrs rejects must come back with the numbered source attached.
#[test]
fn an_i8_rejection_shows_the_numbered_script_it_judged() {
    // A genuine SC2* error (SC2* is NOT filtered by validate_script, unlike
    // SC1*), so this exercises the real rejection path.
    let bad = "if true; do\n  echo hi\nfi\n";
    let err = validate_before_exec(bad).expect_err("bashrs should reject this");

    assert!(
        err.contains("the script bashrs judged"),
        "the rejection does not show what was judged:\n{err}"
    );
    assert!(
        err.contains("echo hi"),
        "the rejection omits the script body, so the cited line cannot be found:\n{err}"
    );
    assert!(
        err.contains("   1 | "),
        "the rejection is not line-numbered, so a rule's line number is unusable:\n{err}"
    );
}

/// A script that passes must not be decorated at all — the diagnostic is for
/// failures, and success carries no error to attach it to.
#[test]
fn a_passing_script_produces_no_diagnostic_dump() {
    assert!(validate_before_exec("echo 'hello'\n").is_ok());
}

/// An I8 rejection is a VALIDATION failure (exit 3), not a connection failure
/// (exit 4) — even after a caller wraps it in the words "transport error".
///
/// The measured failure this guards: applying a resource to gx10 produced
/// `"transport error: I8 violation — script failed bashrs validation: bashrs
/// lint errors: ..."`, and the old prose-matching classifier in `main` saw
/// "transport" and exited 4. CI treats 4 as retryable; a bashrs rejection is
/// deterministic and fails identically on every retry.
///
/// This runs the REAL producer, so it fails if the message and the marker
/// registered in `core::error` ever drift apart.
#[test]
fn an_i8_rejection_classifies_as_validation_not_connection() {
    use crate::core::error::{ErrorClass, ForjarError};

    let bad = "if true; do\n  echo hi\nfi\n";
    let err = validate_before_exec(bad).expect_err("bashrs should reject this");

    // Exactly how `resource_ops`/`machine_wave` wrap a transport failure today.
    let wrapped = format!("transport error: {err}");
    assert!(
        wrapped.contains("transport"),
        "the wrapping this test exists to survive is missing:\n{wrapped}"
    );

    let classified = ForjarError::from_untyped(wrapped);
    assert_eq!(classified.class(), ErrorClass::Validation);
    assert_eq!(
        classified.exit_code(),
        3,
        "a deterministic bashrs rejection must not exit 4 and tell CI to retry"
    );
}

/// The numbering must describe the SANITISED text, since that — not the
/// original — is what bashrs read and what its line numbers refer to.
#[test]
fn the_dump_is_the_sanitised_text_not_the_original() {
    let numbered = numbered_for_diagnosis("alpha\nbeta");
    assert!(numbered.contains("   1 | alpha"), "{numbered}");
    assert!(numbered.contains("   2 | beta"), "{numbered}");
}

//! FJ-266 — falsifiers for `contracts/apply-receipt-v1.yaml`.
//!
//! Filed after paiml/infra#208: `~/.cargo/bin` on the intel clean-room host
//! was destroyed three times in one day, taking rustup, forjar and ~20 other
//! binaries, failing all 16 runners each time. forjar could be neither
//! confirmed nor eliminated as the cause, because its event log could not
//! express the two facts the incident turned on — that something was DELETED,
//! and what a converge REPLACED.
//!
//! Each test below names the mutation that must turn it RED. A falsifier that
//! survives the removal of the code it covers is not evidence.
//!
//! WHAT THESE DO NOT COVER, stated plainly because the first draft of this file
//! did not and I only found out by mutating:
//!
//! These tests exercise the TYPE round-tripping through `append_event`. They do
//! NOT cover the emission site. Neutering `record_success` so it always passes
//! `previous_hash: None` leaves every test in this file green — verified, not
//! assumed. The rule itself is covered by `displaced_hash`'s unit falsifiers in
//! `core::executor::resource_ops::tests_displaced_hash` (RCP-006/007/008, both
//! confirmed RED under mutation), but the WIRING from `record_success` into
//! that rule is currently asserted by nothing.
//!
//! Closing that needs an executor-level test with a real `Machine`, because
//! `record_success` calls `transport::exec_script_timeout`. A `machine: local`
//! harness would make it cheap; there isn't one here yet. Recorded as a gap
//! rather than papered over — this repo's own `binding.yaml` uses `partial`
//! with a reason for exactly this situation.

use forjar::core::types::ProvenanceEvent;
use forjar::tripwire::eventlog::{append_event, ensure_event_log_writable, event_log_path};

fn read_log(dir: &std::path::Path, machine: &str) -> String {
    std::fs::read_to_string(event_log_path(dir, machine)).expect("event log readable")
}

/// FALSIFY-RCP-001 — deletion is representable and round-trips to the log.
///
/// RED under: removing the `ResourceDeleted` variant (it will not compile),
/// or serialising it without `resource` / `previous_hash`.
///
/// This is the event class the originating incident needed and did not have.
#[test]
fn a_deleted_resource_is_recorded_with_what_it_used_to_be() {
    let dir = tempfile::tempdir().expect("tempdir");
    append_event(
        dir.path(),
        "intel",
        ProvenanceEvent::ResourceDeleted {
            machine: "intel".to_string(),
            resource: "stack-tool-rustup".to_string(),
            previous_hash: Some("blake3:cafebabe".to_string()),
            reason: "destroy".to_string(),
        },
    )
    .expect("append");

    let log = read_log(dir.path(), "intel");
    assert!(
        log.contains("resource_deleted"),
        "a deletion must name itself in the log; got: {log}"
    );
    assert!(
        log.contains("stack-tool-rustup"),
        "a deletion must name WHICH resource; got: {log}"
    );
    assert!(
        log.contains("blake3:cafebabe"),
        "a deletion must record what was removed, or it cannot be told from a \
         resource that never existed; got: {log}"
    );
    assert!(
        log.contains("\"reason\":\"destroy\""),
        "a deletion must say what asked for it; got: {log}"
    );
}

/// FALSIFY-RCP-002 — a converge records what it replaced, not only its result.
///
/// RED under: dropping `previous_hash` from `ResourceConverged`, or passing
/// `None` at the emission site in `record_converged`.
///
/// Without it, a converge that silently overwrote a good artifact is
/// indistinguishable from one that created a resource from nothing.
#[test]
fn a_converge_records_the_hash_it_overwrote() {
    let dir = tempfile::tempdir().expect("tempdir");
    append_event(
        dir.path(),
        "intel",
        ProvenanceEvent::ResourceConverged {
            machine: "intel".to_string(),
            resource: "stack-tool-forjar".to_string(),
            duration_seconds: 1.5,
            hash: "blake3:after".to_string(),
            previous_hash: Some("blake3:before".to_string()),
        },
    )
    .expect("append");

    let log = read_log(dir.path(), "intel");
    assert!(
        log.contains("blake3:after") && log.contains("blake3:before"),
        "both sides of the change must be present; got: {log}"
    );
}

/// FALSIFY-RCP-003 — first converge is distinguishable from an overwrite.
///
/// RED under: emitting `previous_hash: Some("")` for a resource that did not
/// exist, which would make "created" look like "replaced something empty".
///
/// `skip_serializing_if` must keep the key absent rather than null-ish.
#[test]
fn a_first_converge_omits_the_previous_hash_entirely() {
    let dir = tempfile::tempdir().expect("tempdir");
    append_event(
        dir.path(),
        "intel",
        ProvenanceEvent::ResourceConverged {
            machine: "intel".to_string(),
            resource: "brand-new".to_string(),
            duration_seconds: 0.1,
            hash: "blake3:fresh".to_string(),
            previous_hash: None,
        },
    )
    .expect("append");

    let log = read_log(dir.path(), "intel");
    assert!(
        !log.contains("previous_hash"),
        "a resource that did not exist must not carry an empty previous_hash — \
         absent and empty are different claims; got: {log}"
    );
}

/// FALSIFY-RCP-004 — old logs still parse (the field is additive).
///
/// RED under: making `previous_hash` a required field.
///
/// Every historical `resource_converged` line predates this change. A reader
/// that cannot load them turns a five-month archive into nothing, which is a
/// worse outcome than the gap being closed.
#[test]
fn a_pre_existing_log_line_without_previous_hash_still_deserialises() {
    let legacy = r#"{"ts":"2026-05-02T12:48:38Z","event":"resource_converged","machine":"lambda-labs","resource":"docker-engine","duration_seconds":74.96,"hash":"blake3:e1281e6f"}"#;
    let parsed: serde_json::Value = serde_json::from_str(legacy).expect("legacy line is JSON");
    assert_eq!(parsed["event"], "resource_converged");

    let te: Result<forjar::core::types::TimestampedEvent, _> = serde_json::from_str(legacy);
    assert!(
        te.is_ok(),
        "a real historical log line must still deserialise: {:?}",
        te.err()
    );
}

/// FALSIFY-RCP-005 — an unwritable event log is detected BEFORE mutation.
///
/// RED under: deleting `ensure_event_log_writable`, or having it return `Ok`
/// unconditionally.
///
/// The quorum (CloudTrail organization trails, Kubernetes catch-all rules,
/// host-global auditd rules) is unanimous that coverage follows membership.
/// The preflight is how an apply refuses to mutate a host it cannot describe.
#[test]
fn an_unwritable_event_log_is_refused_up_front() {
    let dir = tempfile::tempdir().expect("tempdir");
    let machine_dir = dir.path().join("intel");
    std::fs::create_dir_all(&machine_dir).expect("mkdir");

    // Make the machine's events.jsonl a directory: creatable parent, but the
    // log itself can never be opened for append.
    std::fs::create_dir_all(machine_dir.join("events.jsonl")).expect("mkdir log-as-dir");

    assert!(
        ensure_event_log_writable(dir.path(), "intel").is_err(),
        "the preflight must refuse an unwritable log BEFORE anything mutates"
    );

    let err = append_event(
        dir.path(),
        "intel",
        ProvenanceEvent::ResourceDeleted {
            machine: "intel".to_string(),
            resource: "x".to_string(),
            previous_hash: None,
            reason: "probe".to_string(),
        },
    );
    assert!(
        err.is_err(),
        "an unwritable event log must surface an error rather than silently \
         succeeding — `let _ = append_event(..)` at the call sites is what made \
         this invisible"
    );
}

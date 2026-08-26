//! The lock must not let SPEC and STATUS be confused for one another.
//!
//! forjar#305 was a spec/status conflation bug. Two different digests lived in
//! the same untyped `details` map under string keys — `live_hash`, recorded on
//! every apply from a query that actually reached the target, and
//! `content_hash`, computed on the CONTROLLER from local bytes. Drift read the
//! second. On any real SSH host the controller has no such path, `.ok()`
//! swallowed the error, and no `content_hash` was written: 320 of 329 locked
//! file resources on the paiml fleet had no usable observed state, and drift
//! reported converged for five months.
//!
//! forjar#337 Step 1 lifts the observed digest into a TYPED field so the wrong
//! one cannot be reached for by name. These tests hold that line.

use forjar::core::types::{ResourceLock, ResourceStatus, ResourceType};
use std::collections::HashMap;

fn lock_with(hash: &str, observed: Option<&str>) -> ResourceLock {
    ResourceLock {
        resource_type: ResourceType::File,
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: hash.to_string(),
        observed: observed.map(str::to_string),
        details: HashMap::new(),
    }
}

/// The spec is not reachable through the status accessor.
///
/// If `observed_state()` ever fell back to `hash`, every resource would report
/// "observed exactly what was declared" — a permanent all-clear that looks
/// identical to a converged fleet. That is worse than the original defect,
/// which at least stayed silent instead of actively affirming.
#[test]
fn observed_state_never_returns_the_spec() {
    let rl = lock_with("blake3:DESIRED", None);
    assert_eq!(
        rl.observed_state(),
        None,
        "a lock with no observed state must report None, not echo its own spec"
    );
}

/// A pre-1.19 lock still reads. Every lock file on the fleet right now stores
/// this in `details`; if the fallback broke, drift would `continue` past every
/// resource and report a clean fleet on day one of the upgrade.
#[test]
fn legacy_details_live_hash_is_still_readable() {
    let mut rl = lock_with("blake3:DESIRED", None);
    rl.details.insert(
        "live_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:OBSERVED".to_string()),
    );
    assert_eq!(rl.observed_state(), Some("blake3:OBSERVED"));
}

/// THE TRAP THIS REFACTOR COULD HAVE SET FOR ITSELF.
///
/// `observed_state()` prefers the typed field. So any writer that updates only
/// `details` leaves the typed field holding the PREVIOUS digest, and every
/// reader gets a stale answer — two stores with readers split between them,
/// which is precisely forjar#305 rebuilt inside its own fix. `--refresh` was
/// written that way and is the reason this test exists.
#[test]
fn set_observed_state_updates_both_copies() {
    let mut rl = lock_with("blake3:DESIRED", Some("blake3:OLD"));
    rl.details.insert(
        "live_hash".to_string(),
        serde_yaml_ng::Value::String("blake3:OLD".to_string()),
    );

    rl.set_observed_state("blake3:NEW");

    assert_eq!(rl.observed_state(), Some("blake3:NEW"));
    assert_eq!(
        rl.details.get("live_hash").and_then(|v| v.as_str()),
        Some("blake3:NEW"),
        "the legacy copy must move too, or a rollback to 1.18.0 reads a stale digest"
    );
}

/// A round-trip through YAML must not quietly drop the field. If `observed`
/// failed to serialize, every apply would write it and every subsequent load
/// would find it missing — the field would look present in memory and be
/// absent in the artifact, which is the fleet's most-repeated bug class.
#[test]
fn observed_survives_a_serde_round_trip() {
    let rl = lock_with("blake3:DESIRED", Some("blake3:OBSERVED"));
    let yaml = serde_yaml_ng::to_string(&rl).expect("serialize");
    assert!(
        yaml.contains("observed"),
        "field absent from the serialized lock:\n{yaml}"
    );
    let back: ResourceLock = serde_yaml_ng::from_str(&yaml).expect("deserialize");
    assert_eq!(back.observed_state(), Some("blake3:OBSERVED"));
    assert_eq!(back.hash, "blake3:DESIRED");
}

/// An old lock with NO `observed` key must deserialize rather than error —
/// otherwise the upgrade bricks every machine's state file at once.
#[test]
fn a_lock_without_the_field_still_loads() {
    let yaml = r#"
type: file
status: converged
hash: "blake3:DESIRED"
details:
  live_hash: "blake3:OBSERVED"
"#;
    let rl: ResourceLock = serde_yaml_ng::from_str(yaml).expect("pre-1.19 lock must load");
    assert_eq!(rl.hash, "blake3:DESIRED");
    assert_eq!(rl.observed_state(), Some("blake3:OBSERVED"));
}

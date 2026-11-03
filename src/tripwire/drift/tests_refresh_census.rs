//! forjar#415: what a refreshed plan still plans from the lock alone.
//!
//! `plan --refresh` discloses the entries it did not consult. Counting only
//! unanswered queries left out observed entries a detector SKIPPED (a changed
//! observation mask, say), which were planned from the lock in silence.

use super::census::{DriftCensus, SkipReason};
use crate::core::types::{ForjarConfig, ResourceType, StateLock};

fn lock() -> StateLock {
    let entry = |id: &str, observed: bool| {
        let details = if observed {
            "    details:\n      live_hash: \"blake3:aa\"\n"
        } else {
            "    details: {}\n"
        };
        format!("  {id}:\n    type: file\n    status: converged\n    hash: \"h\"\n{details}")
    };
    let resources: String = [
        ("inspected", true),
        ("mask_changed", true),
        ("no_observation", false),
        ("unanswered", false),
        ("out_of_scope", true),
        ("never_censused", true),
    ]
    .iter()
    .map(|(id, o)| entry(id, *o))
    .collect();
    serde_yaml_ng::from_str(&format!(
        "schema: \"1.0\"\nmachine: m\nhostname: m\ngenerated_at: now\n\
         generator: test\nblake3_version: \"1\"\nresources:\n{resources}"
    ))
    .expect("lock yaml")
}

fn declared() -> ForjarConfig {
    let resources: String = [
        "inspected",
        "mask_changed",
        "no_observation",
        "unanswered",
        "never_censused",
    ]
    .iter()
    .map(|id| format!("  {id}: {{ type: file, machine: m, path: /tmp/{id}, content: x }}\n"))
    .collect();
    serde_yaml_ng::from_str(&format!(
        "version: \"1.0\"\nname: t\nmachines: {{ m: {{ hostname: m, addr: 127.0.0.1 }} }}\n\
         resources:\n{resources}"
    ))
    .expect("config yaml")
}

#[test]
fn a_refresh_discloses_skipped_observations_and_unanswered_queries() {
    let f = ResourceType::File;
    let mut census = DriftCensus::new();
    census.inspected("inspected", &f);
    census.skipped("mask_changed", &f, SkipReason::ObservationMaskChanged);
    census.skipped("no_observation", &f, SkipReason::NoObservedState);
    census.unmeasured("unanswered", &f);
    census.skipped("out_of_scope", &f, SkipReason::NotInConfig);
    // unanswered (no observed state, but the target was asked),
    // mask_changed (observed, skipped), never_censused (observed, never looked
    // at). Not: inspected, no_observation, out_of_scope.
    assert_eq!(
        census.unconsulted_observations(&lock(), &declared().resources),
        3
    );
}

#[test]
fn an_inspected_lock_is_fully_consulted() {
    let f = ResourceType::File;
    let mut census = DriftCensus::new();
    for id in [
        "inspected",
        "mask_changed",
        "never_censused",
        "out_of_scope",
    ] {
        census.inspected(id, &f);
    }
    assert_eq!(
        census.unconsulted_observations(&lock(), &declared().resources),
        0
    );
}

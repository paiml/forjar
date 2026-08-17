//! FJ-035: `overlay_hosts` is part of the desired-state hash, and its hash does
//! not depend on insertion order.
//!
//! These two properties were `#[kani::proof]` harnesses
//! (`proof_overlay_hosts_hash_order_independent`,
//! `proof_overlay_hosts_distinguishes_hash`) until 2026-08-17. Neither had ever
//! produced a verdict: both drive `hash_desired_state`, so both reach a real
//! BLAKE3 hash, and a bounded model checker cannot verify through one — the
//! default SIMD build fails outright with `call to foreign "C" function
//! syscall`, and blake3's portable `pure` build was measured at 29.1 GB of RSS
//! still running after 36 minutes on a comparable harness. They also each built
//! a `HashMap` with `format!`ted keys, which is expensive in a model for the
//! same reason.
//!
//! Converting them to tests is not a downgrade. A proof that cannot run proves
//! nothing; these execute in microseconds on every `cargo test`, and they cover
//! several concrete key pairs where the harness covered a bounded symbolic
//! range it never actually explored.

use super::hash_desired_state;
use crate::core::types::{Resource, ResourceType};
use std::collections::HashMap;

/// An overlay resource with the given host entries.
fn overlay(hosts: Option<HashMap<String, String>>) -> Resource {
    Resource {
        resource_type: ResourceType::OverlayInterface,
        overlay_ip: Some("10.42.0.11/24".into()),
        overlay_hosts: hosts,
        ..Default::default()
    }
}

fn hosts(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn overlay_hosts_hash_is_insertion_order_independent() {
    // The canonicalisation must sort, or two operators writing the same fleet
    // in a different order would see spurious diffs forever. Several pairs,
    // because a HashMap's iteration order varies with its contents — one pair
    // could agree by luck.
    for (a, b) in [("h1", "h2"), ("alpha", "beta"), ("z9", "a0"), ("m", "mm")] {
        let fwd = hosts(&[(a, "10.0.0.1"), (b, "10.0.0.2")]);
        let rev = hosts(&[(b, "10.0.0.2"), (a, "10.0.0.1")]);

        assert_eq!(
            hash_desired_state(&overlay(Some(fwd))),
            hash_desired_state(&overlay(Some(rev))),
            "overlay_hosts hash must be insertion-order independent ({a}, {b})"
        );
    }
}

#[test]
fn overlay_hosts_presence_changes_the_hash() {
    // THE MATERIAL FIX this property guards: if hosts were not a hash
    // component, `plan` would report NoOp over a changed /etc/hosts block and
    // apply would never write it.
    let with = overlay(Some(hosts(&[("h1", "10.0.0.1")])));
    let without = overlay(None);

    assert_ne!(
        hash_desired_state(&with),
        hash_desired_state(&without),
        "overlay_hosts presence must change the desired-state hash"
    );
}

#[test]
fn overlay_hosts_content_changes_the_hash() {
    // Not covered by either original harness: same KEY, different ADDRESS.
    // Order-independence is implemented by sorting, and a sort that keyed on
    // the hostname alone would pass both properties above while silently
    // ignoring a re-pointed host.
    let a = overlay(Some(hosts(&[("h1", "10.0.0.1")])));
    let b = overlay(Some(hosts(&[("h1", "10.0.0.99")])));

    assert_ne!(
        hash_desired_state(&a),
        hash_desired_state(&b),
        "changing a host's address must change the desired-state hash"
    );
}

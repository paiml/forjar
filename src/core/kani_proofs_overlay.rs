//! FJ-035: Kani bounded-model proofs for the overlay_interface resource.
//! Split from kani_proofs.rs to keep each file under the 500-line file-health
//! limit (CLAUDE.md). Gated behind `#[cfg(kani)]`; normal builds ignore them.
//! Run with: `cargo kani --harness proof_overlay_hosts_hash_order_independent`.

// ── Overlay Interface Proofs (FJ-035 / overlay-interface-v1) ─────────

/// FJ-035: overlay_hosts canonicalization is order-independent — two resources
/// with the SAME host entries inserted in a DIFFERENT order hash identically.
/// Bounded proof behind the `hash_overlay_hosts_sensitive` order-independence
/// invariant.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(4)]
fn proof_overlay_hosts_hash_order_independent() {
    use super::planner::hash_desired_state;
    use super::types::{Resource, ResourceType};
    use std::collections::HashMap;

    let a0: u8 = kani::any();
    let a1: u8 = kani::any();
    kani::assume(a0 != a1); // distinct keys

    let mut fwd = HashMap::new();
    fwd.insert(format!("h{a0}"), format!("10.0.0.{a0}"));
    fwd.insert(format!("h{a1}"), format!("10.0.0.{a1}"));

    let mut rev = HashMap::new();
    rev.insert(format!("h{a1}"), format!("10.0.0.{a1}"));
    rev.insert(format!("h{a0}"), format!("10.0.0.{a0}"));

    let mut r1 = Resource::default();
    r1.resource_type = ResourceType::OverlayInterface;
    r1.overlay_ip = Some("10.42.0.11/24".into());
    r1.overlay_hosts = Some(fwd);

    let mut r2 = Resource::default();
    r2.resource_type = ResourceType::OverlayInterface;
    r2.overlay_ip = Some("10.42.0.11/24".into());
    r2.overlay_hosts = Some(rev);

    assert_eq!(
        hash_desired_state(&r1),
        hash_desired_state(&r2),
        "overlay_hosts hash must be insertion-order independent"
    );
}

/// FJ-035: overlay_hosts is a hash component — a resource WITH hosts and an
/// otherwise-identical resource WITHOUT hosts must hash differently (so plan
/// never false-reports NoOp on a changed /etc/hosts block). Bounded proof
/// behind the MATERIAL FIX.
#[cfg(kani)]
#[kani::proof]
#[kani::unwind(4)]
fn proof_overlay_hosts_distinguishes_hash() {
    use super::planner::hash_desired_state;
    use super::types::{Resource, ResourceType};
    use std::collections::HashMap;

    let k: u8 = kani::any();

    let mut with_hosts = HashMap::new();
    with_hosts.insert(format!("h{k}"), format!("10.0.0.{k}"));

    let mut r_with = Resource::default();
    r_with.resource_type = ResourceType::OverlayInterface;
    r_with.overlay_ip = Some("10.42.0.11/24".into());
    r_with.overlay_hosts = Some(with_hosts);

    let mut r_without = Resource::default();
    r_without.resource_type = ResourceType::OverlayInterface;
    r_without.overlay_ip = Some("10.42.0.11/24".into());

    assert_ne!(
        hash_desired_state(&r_with),
        hash_desired_state(&r_without),
        "overlay_hosts presence must change the desired-state hash"
    );
}

// Production function proofs live in kani_production_proofs.rs

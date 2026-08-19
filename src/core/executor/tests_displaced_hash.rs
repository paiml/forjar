//! FJ-266 — falsifiers for `resource_ops::displaced_hash`.
//!
//! Split out of `resource_ops.rs` rather than inlined: this repo caps source
//! files at 500 lines and the module was already at 494. The sibling
//! `tests_*.rs` layout is the convention every other executor test follows.

use crate::core::types::ResourceLock;
use crate::core::types::{ResourceStatus, ResourceType};
use crate::tripwire::eventlog::displaced_hash;
use std::collections::HashMap;

fn lock_with(hash: &str) -> ResourceLock {
    ResourceLock {
        resource_type: ResourceType::File,
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: hash.to_string(),
        details: HashMap::new(),
    }
}

/// FALSIFY-RCP-006 — a converge over an existing resource reports what it
/// replaced. RED under: `displaced_hash` returning `None` unconditionally.
#[test]
fn an_overwrite_reports_the_hash_it_displaced() {
    assert_eq!(
        displaced_hash(Some(lock_with("blake3:old"))),
        Some("blake3:old".to_string())
    );
}

/// FALSIFY-RCP-007 — a first converge reports nothing displaced.
/// RED under: `displaced_hash` returning `Some(_)` for a fresh resource.
#[test]
fn a_first_converge_displaces_nothing() {
    assert_eq!(displaced_hash(None), None);
}

/// FALSIFY-RCP-008 — an empty stored hash is not a previous state.
/// RED under: dropping the `.filter(|h| !h.is_empty())`.
///
/// A recorded failure stores `hash: String::new()`. Reporting that as
/// `Some("")` would claim the resource previously held empty content,
/// which is a different fact from "there is no recorded previous state".
#[test]
fn an_empty_stored_hash_is_not_a_previous_state() {
    assert_eq!(displaced_hash(Some(lock_with(""))), None);
}

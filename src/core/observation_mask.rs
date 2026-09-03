//! forjar#360: which fields of an observation `lifecycle.ignore_drift` suppresses.
//!
//! # Why a mask over the stdout and not a per-field lock schema
//!
//! The issue asks for `ResourceLock.observed` to become a map of field to
//! observed value — a lock schema change, a migration, and a change to every
//! per-resource state-query generator. It is not needed. The generators ALREADY
//! emit field-shaped output (`file`: `owner=noah group=noah mode=644 size=19`;
//! `service`: `active=`/`enabled=`), and the digest is taken over that text. So
//! the field list can be honoured by dropping the named `key=value` tokens
//! BEFORE hashing, leaving the observation a digest exactly as it is today.
//! No schema bump — which also avoids `cli::lock_core`'s two hard-coded
//! `schema != "1" && schema != "1.0"` checks.
//!
//! # The mask has to be applied at every writer, not just the reader
//!
//! There are THREE places that hash a state query's stdout into the observed
//! state, and they must agree or the comparison is between two different
//! questions:
//!
//! 1. `core::executor::resource_ops::record_success` — the apply baseline;
//! 2. `cli::apply_variants::refreshed_live_hash` — `apply --refresh` and
//!    `--refresh-only`, which RE-BASELINE the same digest;
//! 3. `tripwire::drift::check_nonfile_drift` — the comparison.
//!
//! Masking (1) and (3) alone is not a fix: one `--refresh` then writes an
//! unmasked digest and the very next `drift` reports FALSE drift on precisely
//! the field the operator asked forjar to ignore. Pinned by
//! `tests/falsification_ignore_drift_names_one_field.rs::
//! refresh_does_not_rebaseline_an_unmasked_observation`.
//!
//! # Why the vocabulary starts narrow, and why `content` is not in it
//!
//! `mask_observation` drops `key=value` TOKENS. A file's content hash is a bare
//! line with no `=`, and so is the `MISSING` existence sentinel — a "drop the
//! line without an `=`" rule cannot tell them apart, and would quietly erode
//! existence detection while claiming to ignore content. Content suppression
//! needs an explicit existence marker in the generator, which is a separate
//! increment. Until then `ignore_drift: ["content"]` stays a hard error, which
//! is also what keeps
//! `falsification_ignore_drift_is_not_an_off_switch::
//! a_narrowed_ignore_drift_never_reaches_the_tripwire` honest.

use crate::core::types::{Resource, ResourceLock, ResourceType};

/// `details` key recording the mask an observation was taken under.
///
/// Provenance, not decoration: adding `ignore_drift: ["mode"]` to an
/// already-converged resource leaves a baseline that was hashed WITHOUT the
/// mask. Comparing a masked live reading against it manufactures drift on the
/// very field being ignored — and since forjar#307 drift feeds the apply gate.
/// Drift skips the resource until the next apply re-baselines it.
pub const MASK_KEY: &str = "observed_mask";

/// A file's observation: `owner=U group=G mode=M size=S` plus a content hash.
const FILE_FIELDS: &[&str] = &["owner", "group", "mode", "size"];

/// A service's observation: `active=X` and `enabled=Y`, one per line.
const SERVICE_FIELDS: &[&str] = &["active", "enabled"];

/// The `ignore_drift` entries forjar can honour for this resource type.
///
/// `None` means the type has no per-field observation at all, so every narrowed
/// entry stays a validation error for it — the forjar#335 refusal, unchanged.
pub fn vocabulary(resource_type: &ResourceType) -> Option<&'static [&'static str]> {
    match resource_type {
        ResourceType::File => Some(FILE_FIELDS),
        ResourceType::Service => Some(SERVICE_FIELDS),
        _ => None,
    }
}

/// Entries of this resource's `ignore_drift` that name a maskable field.
///
/// Sorted and deduped so the recorded provenance is stable across a reordered
/// config. The wildcard is excluded: `["*"]` suppresses the whole resource
/// through `should_ignore_drift`, which never reaches the observation.
pub fn ignored_fields(resource: &Resource) -> Vec<String> {
    let Some(lifecycle) = resource.lifecycle.as_ref() else {
        return Vec::new();
    };
    let vocab = vocabulary(&resource.resource_type).unwrap_or(&[]);
    let mut fields: Vec<String> = lifecycle
        .ignore_drift
        .iter()
        .filter(|f| vocab.contains(&f.as_str()))
        .cloned()
        .collect();
    fields.sort();
    fields.dedup();
    fields
}

/// The provenance string for this resource's mask; empty when nothing is masked.
pub fn mask_key(resource: &Resource) -> String {
    ignored_fields(resource).join(",")
}

/// True when `token` is a `key=value` whose key is masked.
fn is_masked(token: &str, ignored: &[String]) -> bool {
    match token.split_once('=') {
        Some((key, _)) => ignored.iter().any(|f| f == key),
        None => false,
    }
}

/// Drop the masked `key=value` tokens from a state query's stdout.
///
/// TOKEN-ANCHORED, never a substring replace: `mode=644` must not be found
/// inside a path or a hash. A line that consists only of masked tokens is
/// dropped; a line with no `=` at all (a content hash, the `MISSING`
/// sentinel) is never touched.
///
/// An empty mask returns the input verbatim, so every resource that does not
/// declare `ignore_drift` hashes exactly the bytes it always has — this change
/// moves no digest on the fleet.
pub fn mask_observation(stdout: &str, ignored: &[String]) -> String {
    if ignored.is_empty() {
        return stdout.to_string();
    }
    let mut out = String::with_capacity(stdout.len());
    for line in stdout.lines() {
        let kept: Vec<&str> = line
            .split_whitespace()
            .filter(|t| !is_masked(t, ignored))
            .collect();
        if kept.is_empty() && !line.trim().is_empty() {
            continue;
        }
        out.push_str(&kept.join(" "));
        out.push('\n');
    }
    out
}

/// Mask a state query's stdout for one resource.
pub fn masked_for(stdout: &str, resource: &Resource) -> String {
    mask_observation(stdout, &ignored_fields(resource))
}

/// Record the mask a freshly written observation was taken under.
pub fn record_mask(details: &mut std::collections::HashMap<String, serde_yaml_ng::Value>, key: &str) {
    if key.is_empty() {
        details.remove(MASK_KEY);
    } else {
        details.insert(
            MASK_KEY.to_string(),
            serde_yaml_ng::Value::String(key.to_string()),
        );
    }
}

/// The mask the lock entry's observation was taken under, `""` when none.
pub fn recorded_mask(lock: &ResourceLock) -> &str {
    lock.detail_str(MASK_KEY).unwrap_or("")
}

#[cfg(test)]
mod tests;

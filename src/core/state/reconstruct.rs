//! FJ-1280: Event-sourced state reconstruction.
//!
//! Replays `events.jsonl` up to a given timestamp to rebuild a `StateLock`
//! at any point in time. Enables point-in-time recovery and audit.

use crate::core::types::{
    ProvenanceEvent, ResourceLock, ResourceStatus, ResourceType, StateLock, TimestampedEvent,
};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::Path;

/// Reconstruct the state of a machine at a given point in time by replaying events.
///
/// Reads `state/<machine>/events.jsonl` and replays all events up to (and including)
/// the given ISO 8601 timestamp. Returns the reconstructed `StateLock`.
pub fn reconstruct_at(
    state_dir: &Path,
    machine: &str,
    timestamp: &str,
) -> Result<StateLock, String> {
    let event_path = state_dir.join(machine).join("events.jsonl");
    if !event_path.exists() {
        return Err(format!(
            "no event log for machine '{}' at {}",
            machine,
            event_path.display()
        ));
    }

    let content =
        std::fs::read_to_string(&event_path).map_err(|e| format!("cannot read event log: {e}"))?;

    let mut resources: IndexMap<String, ResourceLock> = IndexMap::new();
    let mut hostname = machine.to_string();
    let mut last_ts = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let te: TimestampedEvent =
            serde_json::from_str(line).map_err(|e| format!("parse event: {e}"))?;

        // Stop replaying if we've passed the target timestamp
        if te.ts.as_str() > timestamp {
            break;
        }
        last_ts = te.ts.clone();

        apply_event(&mut resources, &mut hostname, &te);
    }

    Ok(StateLock {
        schema: "1.0".to_string(),
        machine: machine.to_string(),
        hostname,
        generated_at: if last_ts.is_empty() {
            timestamp.to_string()
        } else {
            last_ts
        },
        generator: format!("forjar {} (reconstructed)", env!("CARGO_PKG_VERSION")),
        blake3_version: "1.8".to_string(),
        resources,
    })
}

/// Fold one replayed event into the state being rebuilt.
///
/// Decides which `ProvenanceEvent` variants carry state — a converge or a
/// failure replaces a resource's lock entry, a drift edits the entry already
/// there, an apply-start renames the host — and leaves every other variant
/// inert. Exists so `reconstruct_at` reads as the replay loop it is (read a
/// line, stop at the cut-off, fold it in) with the per-variant detail in one
/// place instead of inlined in the loop body.
fn apply_event(
    resources: &mut IndexMap<String, ResourceLock>,
    hostname: &mut String,
    te: &TimestampedEvent,
) {
    match &te.event {
        ProvenanceEvent::ResourceConverged {
            resource,
            hash,
            duration_seconds,
            ..
        } => {
            let (res_type, res_id) = parse_resource_ref(resource);
            resources.insert(
                res_id,
                ResourceLock {
                    resource_type: res_type,
                    status: ResourceStatus::Converged,
                    applied_at: Some(te.ts.clone()),
                    duration_seconds: Some(*duration_seconds),
                    hash: hash.clone(),
                    observed: None,
                    details: HashMap::new(),
                },
            );
        }
        ProvenanceEvent::ResourceFailed {
            resource, error, ..
        } => {
            let (res_type, res_id) = parse_resource_ref(resource);
            resources.insert(
                res_id,
                ResourceLock {
                    resource_type: res_type,
                    status: ResourceStatus::Failed,
                    applied_at: Some(te.ts.clone()),
                    duration_seconds: None,
                    hash: String::new(),
                    observed: None,
                    details: HashMap::from([(
                        "error".to_string(),
                        serde_yaml_ng::Value::String(error.clone()),
                    )]),
                },
            );
        }
        ProvenanceEvent::DriftDetected {
            resource,
            actual_hash,
            ..
        } => {
            if let Some(entry) = resources.get_mut(resource) {
                entry.status = ResourceStatus::Drifted;
                entry.hash = actual_hash.clone();
            }
        }
        ProvenanceEvent::ApplyStarted { machine: m, .. } => {
            *hostname = m.clone();
        }
        // Other events don't affect state reconstruction
        _ => {}
    }
}

/// Parse a resource reference string. Resources in events use their ID directly.
/// Returns (resource_type, resource_id). Type defaults to Package if unknown.
fn parse_resource_ref(resource: &str) -> (ResourceType, String) {
    // Resource type is not stored in event log — default to Package
    // The actual type would be resolved from config context if needed
    (ResourceType::Package, resource.to_string())
}

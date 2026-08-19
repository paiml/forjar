//! `forjar history --resource <NAME>` — per-resource change history.
//!
//! Split out of `history.rs` so both modules stay small.
//!
//! Dogfood #208 (`history-resource-filter-claims-no-event-logs`): this used to
//! read `state/events/*.jsonl`, a directory forjar never writes, and reported
//! "No event logs found." at rc=0 while `state/<machine>/events.jsonl` held the
//! resource's events all along.

use super::helpers::*;
use crate::core::types;
use std::path::Path;

// FJ-357: Show change history for a specific resource
/// Extract the `resource` field of a provenance event, if it has one.
///
/// Dogfood #208 (history-resource-filter-claims-no-event-logs): matching with
/// `line.contains(resource)` would also match a resource name appearing inside
/// an error string or a hash. Compare the typed field instead.
pub(crate) fn event_resource(event: &types::ProvenanceEvent) -> Option<&str> {
    match event {
        types::ProvenanceEvent::ResourceStarted { resource, .. }
        | types::ProvenanceEvent::ResourceConverged { resource, .. }
        | types::ProvenanceEvent::ResourceFailed { resource, .. }
        | types::ProvenanceEvent::DriftDetected { resource, .. }
        | types::ProvenanceEvent::SecretAccessed { resource, .. } => Some(resource.as_str()),
        _ => None,
    }
}

/// Collect this resource's events from every machine's `events.jsonl`.
///
/// Dogfood #208: this used to read `state/events/*.jsonl` — a directory forjar
/// never writes — so `history --resource <name>` printed "No event logs found."
/// at rc=0 even when `state/<machine>/events.jsonl` held that resource's
/// events. Read the machine event logs that `history` itself already reads.
pub(crate) fn collect_resource_events(
    state_dir: &Path,
    machine_filter: Option<&str>,
    resource: &str,
) -> Result<Vec<types::TimestampedEvent>, String> {
    let mut entries = super::history::load_machine_events(state_dir, machine_filter)?;
    entries.retain(|te| event_resource(&te.event) == Some(resource));
    Ok(entries)
}

pub(crate) fn cmd_history_resource(
    state_dir: &Path,
    machine_filter: Option<&str>,
    resource: &str,
    limit: usize,
    json: bool,
) -> Result<(), String> {
    if !state_dir.exists() {
        return Err(format!(
            "state directory {} does not exist — run `forjar apply` first",
            state_dir.display()
        ));
    }

    let mut entries = collect_resource_events(state_dir, machine_filter, resource)?;

    entries.sort_by(|a, b| a.ts.cmp(&b.ts));
    if entries.len() > limit {
        entries = entries.split_off(entries.len() - limit);
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&entries).map_err(|e| format!("JSON error: {e}"))?
        );
    } else {
        println!("History for resource '{}':\n", bold(resource));
        if entries.is_empty() {
            println!("  (no events found)");
        } else {
            for entry in &entries {
                println!("  {} {}", entry.ts, describe_resource_event(&entry.event));
            }
        }
    }

    Ok(())
}

/// One-line description of a resource-scoped provenance event.
fn describe_resource_event(event: &types::ProvenanceEvent) -> String {
    match event {
        types::ProvenanceEvent::ResourceStarted {
            machine, action, ..
        } => format!("started   {machine} ({action})"),
        types::ProvenanceEvent::ResourceConverged {
            machine,
            duration_seconds,
            hash,
            ..
        } => format!("converged {machine} ({duration_seconds:.3}s, {hash})"),
        types::ProvenanceEvent::ResourceFailed { machine, error, .. } => {
            format!("FAILED    {machine} — {error}")
        }
        types::ProvenanceEvent::DriftDetected {
            machine,
            expected_hash,
            actual_hash,
            ..
        } => format!("drift     {machine} (expected {expected_hash}, actual {actual_hash})"),
        other => format!("{other:?}"),
    }
}

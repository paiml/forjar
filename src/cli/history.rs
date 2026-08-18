//! History commands.
#![allow(clippy::manual_is_multiple_of)] // i64 doesn't support is_multiple_of

use super::helpers_time::*;
use crate::core::types;
use crate::tripwire::eventlog;
use std::path::Path;

/// Print (or emit as JSON) the apply history for one or all machines.
///
/// Reads every `state/<machine>/events.jsonl`, sorts newest-first, applies the
/// optional `--since` window and shows the `apply_started`/`apply_completed`
/// pairs. Per-resource history lives in [`super::history_resource`].
/// FJ-266: does this event appear in `forjar history`?
///
/// Extracted so a falsifier can assert the REAL rule. The first attempt at
/// that test duplicated this `matches!` inline and therefore passed against a
/// build where `ResourceDeleted` had been removed from it — a test that could
/// not fail, written into the fix for tests that could not fail.
pub(crate) fn is_history_event(event: &types::ProvenanceEvent) -> bool {
    matches!(
        event,
        types::ProvenanceEvent::ApplyStarted { .. }
            | types::ProvenanceEvent::ApplyCompleted { .. }
            | types::ProvenanceEvent::ResourceDeleted { .. }
    )
}

pub(crate) fn cmd_history(
    state_dir: &Path,
    machine_filter: Option<&str>,
    limit: usize,
    json: bool,
    since: Option<&str>,
) -> Result<(), String> {
    let mut all_events = load_machine_events(state_dir, machine_filter)?;

    // Sort by timestamp descending (most recent first)
    all_events.sort_by(|a, b| b.ts.cmp(&a.ts));

    // FJ-284: --since time filter
    if let Some(since_str) = since {
        let cutoff_str = compute_cutoff_iso8601(since_str)?;
        all_events.retain(|e| e.ts.as_str() >= cutoff_str.as_str());
    }

    // Filter to the run-level events, then limit.
    //
    // FJ-266: ResourceDeleted is included deliberately. It is a resource-level
    // event, not a run-level one, but a deletion is the single fact an incident
    // is most likely to be looking for — and before this it was written only to
    // destroy-log.jsonl, which this command never read. A removal that produces
    // silence at the surface an investigator queries is indistinguishable from
    // a removal that never happened (paiml/infra#208).
    let apply_events: Vec<&types::TimestampedEvent> = all_events
        .iter()
        .filter(|e| is_history_event(&e.event))
        .take(limit)
        .collect();

    if json {
        output_history_json(&all_events, &apply_events, since, limit)?;
    } else if apply_events.is_empty() {
        println!("No apply history found. Run `forjar apply` first.");
    } else {
        print_apply_events(&apply_events);
    }

    Ok(())
}

/// Machine directories under `state_dir` that hold an event log.
fn machines_with_event_logs(state_dir: &Path, filter: Option<&str>) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(state_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|name| filter.is_none_or(|f| name == f))
        .filter(|name| eventlog::event_log_path(state_dir, name).exists())
        .collect()
}

/// Parse every JSONL line of one machine's event log, skipping malformed rows.
fn parse_event_log(path: &Path) -> Result<Vec<types::TimestampedEvent>, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<types::TimestampedEvent>(l).ok())
        .collect())
}

/// Load every event from every in-scope machine's `events.jsonl`.
pub(crate) fn load_machine_events(
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Result<Vec<types::TimestampedEvent>, String> {
    if !state_dir.exists() {
        return Err(format!(
            "cannot read state dir {}: not found",
            state_dir.display()
        ));
    }
    let mut all = Vec::new();
    for name in machines_with_event_logs(state_dir, machine_filter) {
        all.extend(parse_event_log(&eventlog::event_log_path(
            state_dir, &name,
        ))?);
    }
    Ok(all)
}

/// Convert epoch seconds to ISO 8601 date string (manual UTC formatting).
fn epoch_secs_to_iso8601(d: u64) -> String {
    let secs_in_day = 86400u64;
    let mut days = d / secs_in_day;
    let rem = d % secs_in_day;
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let ss = rem % 60;
    // Gregorian calendar from days since 1970-01-01
    let mut y = 1970i64;
    loop {
        let dy = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if days < dy {
            break;
        }
        days -= dy;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mdays = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut mo = 0usize;
    while mo < 12 && days >= mdays[mo] {
        days -= mdays[mo];
        mo += 1;
    }
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        y,
        mo + 1,
        days + 1,
        hh,
        mm,
        ss
    )
}

/// Compute a cutoff ISO 8601 string from a duration string (e.g. "1h", "30m").
fn compute_cutoff_iso8601(since_str: &str) -> Result<String, String> {
    let secs = parse_duration_secs(since_str)?;
    let now = std::time::SystemTime::now();
    let cutoff = now
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("time error: {e}"))?
        .as_secs()
        .saturating_sub(secs);
    Ok(epoch_secs_to_iso8601(cutoff))
}

/// Output history as structured JSON.
fn output_history_json(
    all_events: &[types::TimestampedEvent],
    apply_events: &[&types::TimestampedEvent],
    since: Option<&str>,
    limit: usize,
) -> Result<(), String> {
    let total_events = all_events.len();
    let started = apply_events
        .iter()
        .filter(|e| matches!(e.event, types::ProvenanceEvent::ApplyStarted { .. }))
        .count();
    let completed = apply_events
        .iter()
        .filter(|e| matches!(e.event, types::ProvenanceEvent::ApplyCompleted { .. }))
        .count();
    let output = serde_json::json!({
        "total_events": total_events,
        "apply_started": started,
        "apply_completed": completed,
        "since": since,
        "limit": limit,
        "events": apply_events,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|e| format!("JSON error: {e}"))?
    );
    Ok(())
}

/// Print apply events in human-readable text format.
fn print_apply_events(apply_events: &[&types::TimestampedEvent]) {
    for event in apply_events {
        match &event.event {
            types::ProvenanceEvent::ApplyStarted {
                machine, run_id, ..
            } => {
                println!("{} started  {} ({})", event.ts, machine, run_id);
            }
            types::ProvenanceEvent::ResourceDeleted {
                machine,
                resource,
                previous_hash,
                reason,
            } => {
                let was = previous_hash.as_deref().unwrap_or("<no recorded hash>");
                println!(
                    "{} DELETED  {machine} — {resource} (reason={reason}, was {was})",
                    event.ts
                );
            }
            types::ProvenanceEvent::ApplyCompleted {
                machine,
                run_id,
                resources_converged,
                resources_unchanged,
                resources_failed,
                total_seconds,
            } => {
                println!(
                    "{} complete {} ({}) — {} converged, {} unchanged, {} failed ({:.1}s)",
                    event.ts,
                    machine,
                    run_id,
                    resources_converged,
                    resources_unchanged,
                    resources_failed,
                    total_seconds
                );
            }
            _ => {}
        }
    }
}

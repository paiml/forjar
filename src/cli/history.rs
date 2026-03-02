//! History commands.

use super::helpers::*;
use super::helpers_time::*;
use crate::core::types;
use crate::tripwire::eventlog;
use std::path::Path;

pub(crate) fn cmd_history(
    state_dir: &Path,
    machine_filter: Option<&str>,
    limit: usize,
    json: bool,
    since: Option<&str>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;

    let mut all_events: Vec<types::TimestampedEvent> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(filter) = machine_filter {
            if name != filter {
                continue;
            }
        }
        if !entry.path().is_dir() {
            continue;
        }

        let log_path = eventlog::event_log_path(state_dir, &name);
        if !log_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&log_path)
            .map_err(|e| format!("cannot read {}: {}", log_path.display(), e))?;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<types::TimestampedEvent>(line) {
                all_events.push(event);
            }
        }
    }

    // Sort by timestamp descending (most recent first)
    all_events.sort_by(|a, b| b.ts.cmp(&a.ts));

    // FJ-284: --since time filter
    if let Some(since_str) = since {
        let cutoff_str = compute_cutoff_iso8601(since_str)?;
        all_events.retain(|e| e.ts.as_str() >= cutoff_str.as_str());
    }

    // Filter to apply_started/apply_completed events for summary, then limit
    let apply_events: Vec<&types::TimestampedEvent> = all_events
        .iter()
        .filter(|e| {
            matches!(
                e.event,
                types::ProvenanceEvent::ApplyStarted { .. }
                    | types::ProvenanceEvent::ApplyCompleted { .. }
            )
        })
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
        .map_err(|e| format!("time error: {}", e))?
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
        serde_json::to_string_pretty(&output).map_err(|e| format!("JSON error: {}", e))?
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

// FJ-357: Show change history for a specific resource
/// Collect matching event lines from JSONL log files for a specific resource.
fn collect_resource_events(log_dir: &Path, resource: &str) -> Result<Vec<String>, String> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(log_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "jsonl") {
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            for line in content.lines() {
                if line.contains(resource) {
                    entries.push(line.to_string());
                }
            }
        }
    }
    Ok(entries)
}

pub(crate) fn cmd_history_resource(
    state_dir: &Path,
    resource: &str,
    limit: usize,
    json: bool,
) -> Result<(), String> {
    let log_dir = state_dir.join("events");
    if !log_dir.exists() {
        if json {
            println!("[]");
        } else {
            println!("No event logs found.");
        }
        return Ok(());
    }

    let mut entries = collect_resource_events(&log_dir, resource)?;

    entries.sort();
    if entries.len() > limit {
        entries = entries.split_off(entries.len() - limit);
    }

    if json {
        println!("[{}]", entries.join(","));
    } else {
        println!("History for resource '{}':\n", bold(resource));
        if entries.is_empty() {
            println!("  (no events found)");
        } else {
            for entry in &entries {
                println!("  {}", entry);
            }
        }
    }

    Ok(())
}

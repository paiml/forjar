//! FJ-015: Append-only JSONL provenance event log.

use crate::core::types::{ProvenanceEvent, TimestampedEvent};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Generate an ISO 8601 timestamp.
pub fn now_iso8601() -> String {
    // Manual implementation — no chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // Simple UTC conversion (good enough, no TZ complexity)
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Days since epoch to Y-M-D (simplified Gregorian)
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let year_days = if is_leap(y) { 366 } else { 365 };
        if remaining < year_days {
            break;
        }
        remaining -= year_days;
        y += 1;
    }
    let leap = is_leap(y);
    let month_days = [
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
    let mut m = 0;
    for (i, &md) in month_days.iter().enumerate() {
        if remaining < md as i64 {
            m = i + 1;
            break;
        }
        remaining -= md as i64;
    }
    let d = remaining + 1;

    format!(
        "{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}Z"
    )
}

pub(crate) fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Generate a run ID.
pub fn generate_run_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("r-{:012x}", nanos & 0xFFFF_FFFF_FFFF)
}

/// Derive the event log path for a machine.
pub fn event_log_path(state_dir: &Path, machine: &str) -> PathBuf {
    state_dir.join(machine).join("events.jsonl")
}

/// Append an event to the machine's event log.
pub fn append_event(state_dir: &Path, machine: &str, event: ProvenanceEvent) -> Result<(), String> {
    let path = event_log_path(state_dir, machine);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create state dir: {e}"))?;
    }

    let te = TimestampedEvent {
        ts: now_iso8601(),
        event,
    };
    let json = serde_json::to_string(&te).map_err(|e| format!("JSON serialize error: {e}"))?;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("cannot open event log {}: {}", path.display(), e))?;

    writeln!(file, "{json}").map_err(|e| format!("write error: {e}"))?;

    Ok(())
}

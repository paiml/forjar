//! FJ-015: Append-only JSONL provenance event log.

use crate::core::types::{ProvenanceEvent, ResourceLock, TimestampedEvent};
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

    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
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

/// FJ-266: assert the event log is writable before an apply mutates anything.
///
/// Coverage has to be a property of being managed, not a separate opt-in —
/// the reference quorum (CloudTrail organization trails, Kubernetes catch-all
/// audit rules, host-global auditd rules) is unanimous on that. Call this in
/// the apply preflight so an unwritable log stops the run while stopping is
/// still free, rather than being discovered after the host has changed.
pub fn ensure_event_log_writable(state_dir: &std::path::Path, machine: &str) -> Result<(), String> {
    let path = event_log_path(state_dir, machine);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("event log dir {} is not creatable: {e}", parent.display()))?;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map(|_| ())
        .map_err(|e| format!("event log {} is not appendable: {e}", path.display()))
}

/// FJ-266: append a provenance event, and SAY SO if the append fails.
///
/// This was `let _ = append_event(..)`. A full disk, a read-only state dir or
/// a bad permission silently produced an apply that mutated the host and
/// recorded nothing — and an absent event is indistinguishable from an apply
/// that never ran, which is precisely the ambiguity that left paiml/infra#208
/// unattributable across three toolchain deletions in one day.
///
/// A warning, not a hard failure: aborting an in-flight apply on a write error
/// is a behaviour change for a just-shipped 1.14.0 and is the maintainers'
/// call. `tripwire::eventlog::ensure_event_log_writable` is the preflight that
/// can refuse BEFORE mutation instead.
pub fn log_tripwire(
    state_dir: &std::path::Path,
    machine: &str,
    tripwire: bool,
    event: ProvenanceEvent,
) {
    if !tripwire {
        return;
    }
    if let Err(e) = append_event(state_dir, machine, event) {
        eprintln!("warning: provenance event NOT recorded for '{machine}': {e}");
        eprintln!("         this apply is mutating state the event log will not describe");
    }
}

/// FJ-266: the hash a converge displaced, or `None` if it created something new.
///
/// Extracted so the "what did this replace?" rule is a named unit with its own
/// falsifier rather than an inline expression. An empty stored hash means the
/// prior lock entry carried no digest (e.g. a recorded failure), which is NOT
/// the same claim as "there was a previous state with these bytes" — so it
/// collapses to `None` rather than to `Some("")`.
pub fn displaced_hash(previous: Option<ResourceLock>) -> Option<String> {
    previous.map(|p| p.hash).filter(|h| !h.is_empty())
}

/// FJ-266: refuse to apply to any machine whose provenance log is unwritable.
///
/// Coverage has to follow membership, not a separate opt-in — the reference
/// quorum (CloudTrail organization trails, Kubernetes catch-all audit rules,
/// host-global auditd rules) is unanimous on that. Checked BEFORE any resource
/// is touched, because the alternative is discovering it after the host has
/// already changed and the record of that change is the thing that failed.
///
/// Skipped when tripwire is off: the operator has explicitly said they do not
/// want a provenance log, and refusing then would block a supported
/// configuration rather than protect anything.
pub fn ensure_machines_loggable<'a>(
    state_dir: &Path,
    machines: impl Iterator<Item = &'a String>,
    machine_filter: Option<&str>,
    tripwire: bool,
) -> Result<(), String> {
    if !tripwire {
        return Ok(());
    }
    for machine in machines {
        if machine_filter.is_some_and(|f| f != machine.as_str()) {
            continue;
        }
        ensure_event_log_writable(state_dir, machine).map_err(|e| {
            format!(
                "refusing to apply: {e}\n  This apply would change '{machine}' without \
                 being able to record what it changed. Fix the state dir, or pass \
                 --no-tripwire to proceed deliberately without a provenance log."
            )
        })?;
    }
    Ok(())
}

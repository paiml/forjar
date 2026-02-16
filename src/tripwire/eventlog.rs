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
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, minutes, seconds
    )
}

fn is_leap(y: i64) -> bool {
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
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create state dir: {}", e))?;
    }

    let te = TimestampedEvent {
        ts: now_iso8601(),
        event,
    };
    let json = serde_json::to_string(&te).map_err(|e| format!("JSON serialize error: {}", e))?;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("cannot open event log {}: {}", path.display(), e))?;

    writeln!(file, "{}", json).map_err(|e| format!("write error: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj015_now_iso8601() {
        let ts = now_iso8601();
        assert!(ts.starts_with("20"));
        assert!(ts.ends_with('Z'));
        assert!(ts.contains('T'));
    }

    #[test]
    fn test_fj015_generate_run_id() {
        let id = generate_run_id();
        assert!(id.starts_with("r-"));
        assert!(id.len() > 4);
    }

    #[test]
    fn test_fj015_event_log_path() {
        let p = event_log_path(Path::new("/state"), "lambda");
        assert_eq!(p, PathBuf::from("/state/lambda/events.jsonl"));
    }

    #[test]
    fn test_fj015_append_event() {
        let dir = tempfile::tempdir().unwrap();
        let event = ProvenanceEvent::ApplyStarted {
            machine: "test".to_string(),
            run_id: "r-abc".to_string(),
            forjar_version: "0.1.0".to_string(),
        };
        append_event(dir.path(), "test", event).unwrap();

        let content = std::fs::read_to_string(dir.path().join("test/events.jsonl")).unwrap();
        assert!(content.contains("apply_started"));
        assert!(content.contains("r-abc"));
    }

    #[test]
    fn test_fj015_append_multiple() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..3 {
            let event = ProvenanceEvent::ResourceConverged {
                machine: "m".to_string(),
                resource: format!("r{}", i),
                duration_seconds: 1.0,
                hash: "blake3:xxx".to_string(),
            };
            append_event(dir.path(), "m", event).unwrap();
        }
        let content = std::fs::read_to_string(dir.path().join("m/events.jsonl")).unwrap();
        let lines: Vec<_> = content.lines().collect();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_fj015_is_leap() {
        // BH-MUT-0001: Each assertion kills a specific mutation of
        // `(y % 4 == 0 && y % 100 != 0) || y % 400 == 0`

        // Divisible by 400 → leap (kills: remove `|| y % 400 == 0`)
        assert!(is_leap(2000));
        assert!(is_leap(1600));

        // Divisible by 100 but NOT 400 → NOT leap (kills: flip `y % 100 != 0`)
        assert!(!is_leap(1900));
        assert!(!is_leap(2100));

        // Divisible by 4 but NOT 100 → leap (kills: flip `y % 4 == 0`, flip `&&` to `||`)
        assert!(is_leap(2024));
        assert!(is_leap(2028));
        assert!(is_leap(1996));

        // NOT divisible by 4 → NOT leap (kills: negate entire expression)
        assert!(!is_leap(2023));
        assert!(!is_leap(2025));
        assert!(!is_leap(2026));
    }
}

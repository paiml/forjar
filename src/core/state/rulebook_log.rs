//! FJ-3107: Rulebook event log — JSONL append for event-driven automation.
//!
//! Records triggered rulebook events and their outcomes to `rulebook-events.jsonl`
//! in the state directory for audit and debugging.

use crate::core::types::{EventType, InfraEvent};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// A logged rulebook trigger event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulebookLogEntry {
    /// ISO 8601 timestamp of when the event was processed.
    pub timestamp: String,
    /// Name of the rulebook that matched.
    pub rulebook: String,
    /// Event type that triggered the rulebook.
    pub event_type: EventType,
    /// Machine where the event originated.
    #[serde(default)]
    pub machine: Option<String>,
    /// Action type executed (apply, destroy, script, notify).
    pub action_type: String,
    /// Whether the action succeeded.
    pub success: bool,
    /// Error message if action failed.
    #[serde(default)]
    pub error: Option<String>,
}

const LOG_FILENAME: &str = "rulebook-events.jsonl";

/// Append a log entry to the rulebook event log.
pub fn append_entry(state_dir: &Path, entry: &RulebookLogEntry) -> Result<(), String> {
    let path = state_dir.join(LOG_FILENAME);
    let line = serde_json::to_string(entry).map_err(|e| format!("serialize rulebook log: {e}"))?;

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("open {}: {e}", path.display()))?;

    writeln!(file, "{line}").map_err(|e| format!("write {}: {e}", path.display()))
}

/// Read all log entries from the rulebook event log.
pub fn read_entries(state_dir: &Path) -> Result<Vec<RulebookLogEntry>, String> {
    let path = state_dir.join(LOG_FILENAME);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut entries = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: RulebookLogEntry =
            serde_json::from_str(line).map_err(|e| format!("parse log entry: {e}"))?;
        entries.push(entry);
    }
    Ok(entries)
}

/// Create a log entry from an event, rulebook name, and action result.
pub fn make_entry(
    event: &InfraEvent,
    rulebook_name: &str,
    action_type: &str,
    success: bool,
    error: Option<String>,
) -> RulebookLogEntry {
    RulebookLogEntry {
        timestamp: event.timestamp.clone(),
        rulebook: rulebook_name.to_string(),
        event_type: event.event_type.clone(),
        machine: event.machine.clone(),
        action_type: action_type.to_string(),
        success,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn sample_event() -> InfraEvent {
        InfraEvent {
            event_type: EventType::FileChanged,
            timestamp: "2026-03-09T12:00:00Z".into(),
            machine: Some("web-01".into()),
            payload: HashMap::new(),
        }
    }

    #[test]
    fn append_and_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let entry = make_entry(&sample_event(), "config-repair", "apply", true, None);
        append_entry(dir.path(), &entry).unwrap();

        let entries = read_entries(dir.path()).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].rulebook, "config-repair");
        assert_eq!(entries[0].action_type, "apply");
        assert!(entries[0].success);
        assert!(entries[0].error.is_none());
    }

    #[test]
    fn multiple_entries() {
        let dir = TempDir::new().unwrap();
        let e1 = make_entry(&sample_event(), "rule-a", "apply", true, None);
        let e2 = make_entry(
            &sample_event(),
            "rule-b",
            "script",
            false,
            Some("exit code 1".into()),
        );
        append_entry(dir.path(), &e1).unwrap();
        append_entry(dir.path(), &e2).unwrap();

        let entries = read_entries(dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].rulebook, "rule-a");
        assert_eq!(entries[1].rulebook, "rule-b");
        assert!(!entries[1].success);
        assert_eq!(entries[1].error.as_deref(), Some("exit code 1"));
    }

    #[test]
    fn read_empty_log() {
        let dir = TempDir::new().unwrap();
        let entries = read_entries(dir.path()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn make_entry_fields() {
        let event = InfraEvent {
            event_type: EventType::CronFired,
            timestamp: "2026-03-09T00:00:00Z".into(),
            machine: None,
            payload: HashMap::new(),
        };
        let entry = make_entry(&event, "cleanup", "script", true, None);
        assert_eq!(entry.event_type, EventType::CronFired);
        assert!(entry.machine.is_none());
        assert_eq!(entry.rulebook, "cleanup");
    }

    #[test]
    fn entry_serde_json_roundtrip() {
        let entry = make_entry(&sample_event(), "test", "notify", true, None);
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: RulebookLogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.rulebook, "test");
        assert_eq!(parsed.action_type, "notify");
    }

    #[test]
    fn log_file_is_jsonl() {
        let dir = TempDir::new().unwrap();
        let entry = make_entry(&sample_event(), "r1", "apply", true, None);
        append_entry(dir.path(), &entry).unwrap();
        append_entry(dir.path(), &entry).unwrap();

        let content = std::fs::read_to_string(dir.path().join("rulebook-events.jsonl")).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        // Each line is valid JSON
        for line in &lines {
            serde_json::from_str::<RulebookLogEntry>(line).unwrap();
        }
    }
}

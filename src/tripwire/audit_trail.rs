//! Reading the append-only provenance log back (paiml/forjar#356).
//!
//! `eventlog.rs` beside this file WRITES the log — one JSON object per line,
//! appended, never rewritten. Nothing read it back except
//! `cli::fleet_reporting::cmd_audit`, which parsed it inline and `println!`'d
//! as it went. So the one record forjar keeps specifically to be consulted
//! after the fact could be consulted only by a human looking at a terminal: no
//! other transport and no caller of the library could ask what had happened on
//! a machine.
//!
//! Separated from the writer rather than added to it because the two have
//! different failure modes. A write that loses a line has lost provenance; a
//! read that meets a torn line must still return every line before it.

use crate::core::types::TimestampedEvent;
use std::path::Path;

use super::eventlog::event_log_path;

/// Parse one machine's JSONL log.
///
/// A line that does not deserialise is SKIPPED rather than failing the read.
/// The log is append-only and written a line at a time, so a torn final write
/// must not make every earlier event unreadable — losing the whole trail is a
/// worse answer than losing its last line.
fn read_machine_log(log_path: &Path) -> Result<Vec<TimestampedEvent>, String> {
    let content = std::fs::read_to_string(log_path)
        .map_err(|e| format!("cannot read {}: {}", log_path.display(), e))?;
    Ok(content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<TimestampedEvent>(l).ok())
        .collect())
}

/// Read the provenance trail under `state_dir`, newest first.
///
/// Returns `(machine, event)` pairs, at most `limit` of them.
///
/// Ordering is newest-first with the machine name breaking ties. `read_dir`
/// returns entries in no order the OS defines — on the filesystems forjar runs
/// on it is a hash order, not creation order — and a stable sort on the
/// timestamp alone would let that leak in whenever two events share one, so two
/// calls over unchanged state could disagree.
///
/// An unreadable `state_dir` is an ERROR. "I could not find your state"
/// reported as "nothing happened" is the substitution that let `forjar_drift`
/// certify a tampered machine as clean (GH-208), and an empty audit trail is
/// the same defect wearing a different name.
pub fn collect_events(
    state_dir: &Path,
    machine_filter: Option<&str>,
    limit: usize,
) -> Result<Vec<(String, TimestampedEvent)>, String> {
    let entries = std::fs::read_dir(state_dir)
        .map_err(|e| format!("cannot read state dir {}: {}", state_dir.display(), e))?;

    let mut all: Vec<(String, TimestampedEvent)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if machine_filter.is_some_and(|f| f != name) {
            continue;
        }
        if !entry.path().is_dir() {
            continue;
        }
        let log_path = event_log_path(state_dir, &name);
        if !log_path.exists() {
            continue;
        }
        for ev in read_machine_log(&log_path)? {
            all.push((name.clone(), ev));
        }
    }

    all.sort_by(|a, b| b.1.ts.cmp(&a.1.ts).then_with(|| a.0.cmp(&b.0)));
    all.truncate(limit);
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_log(dir: &Path, machine: &str, body: &str) {
        let md = dir.join(machine);
        std::fs::create_dir_all(&md).unwrap();
        std::fs::write(md.join("events.jsonl"), body).unwrap();
    }

    fn ev(ts: &str, resource: &str) -> String {
        format!(
            "{{\"ts\":\"{ts}\",\"event\":\"resource_started\",\"machine\":\"m\",\
             \"resource\":\"{resource}\",\"action\":\"create\"}}"
        )
    }

    #[test]
    fn a_missing_state_dir_is_an_error_not_an_empty_trail() {
        let d = tempfile::tempdir().unwrap();
        let err = collect_events(&d.path().join("nope"), None, 10).unwrap_err();
        assert!(err.contains("cannot read state dir"), "{err}");
    }

    #[test]
    fn a_torn_last_line_does_not_lose_the_lines_before_it() {
        let d = tempfile::tempdir().unwrap();
        write_log(
            d.path(),
            "local",
            &format!(
                "{}\n{{\"ts\":\"2026-01-01T00:00:01Z\",\"ev",
                ev("2026-01-01T00:00:00Z", "a")
            ),
        );
        let got = collect_events(d.path(), None, 10).unwrap();
        assert_eq!(
            got.len(),
            1,
            "the intact line was thrown away with the torn one"
        );
    }

    #[test]
    fn entries_come_back_newest_first() {
        let d = tempfile::tempdir().unwrap();
        write_log(
            d.path(),
            "local",
            &format!(
                "{}\n{}\n",
                ev("2026-01-01T00:00:00Z", "old"),
                ev("2026-01-01T00:00:09Z", "new")
            ),
        );
        let got = collect_events(d.path(), None, 10).unwrap();
        assert_eq!(got[0].1.ts, "2026-01-01T00:00:09Z");
    }

    #[test]
    fn the_machine_filter_excludes_the_others() {
        let d = tempfile::tempdir().unwrap();
        write_log(
            d.path(),
            "a",
            &format!("{}\n", ev("2026-01-01T00:00:00Z", "x")),
        );
        write_log(
            d.path(),
            "b",
            &format!("{}\n", ev("2026-01-01T00:00:01Z", "y")),
        );
        let got = collect_events(d.path(), Some("b"), 10).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, "b");
    }

    #[test]
    fn a_directory_with_no_log_is_skipped_not_an_error() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("empty-machine")).unwrap();
        std::fs::write(d.path().join("stray-file"), "x").unwrap();
        assert!(collect_events(d.path(), None, 10).unwrap().is_empty());
    }

    #[test]
    fn the_limit_keeps_the_newest() {
        let d = tempfile::tempdir().unwrap();
        write_log(
            d.path(),
            "local",
            &format!(
                "{}\n{}\n{}\n",
                ev("2026-01-01T00:00:00Z", "a"),
                ev("2026-01-01T00:00:01Z", "b"),
                ev("2026-01-01T00:00:02Z", "c")
            ),
        );
        let got = collect_events(d.path(), None, 2).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].1.ts, "2026-01-01T00:00:02Z");
        assert_eq!(got[1].1.ts, "2026-01-01T00:00:01Z");
    }
}

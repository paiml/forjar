//! Tail loop for `forjar logs --follow`.
//!
//! Dogfood #208 (family #211, `logs-follow-does-not-follow-and-ignores-run`):
//! `--follow` printed "Press Ctrl+C to stop." and returned in ~5 ms without
//! streaming anything. The banner is not the feature; this module is.
//!
//! The loop polls the run directory, emits any bytes appended to `*.log` files
//! since the last poll, and announces log files that appear mid-run. It is
//! driven by a [`Clock`] so tests can bound the number of polls without
//! sleeping or spawning a process.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Decides whether the tail loop should keep polling.
///
/// Production uses [`Forever`]; tests use [`Bounded`] so a follow loop can be
/// asserted on deterministically.
pub(crate) trait Clock {
    /// Return true to run another poll iteration.
    fn should_continue(&mut self) -> bool;
    /// Wait between polls.
    fn sleep(&mut self, dur: Duration);
}

/// Never stops — the real `--follow` behaviour (ends on Ctrl+C).
pub(crate) struct Forever;

impl Clock for Forever {
    fn should_continue(&mut self) -> bool {
        true
    }
    fn sleep(&mut self, dur: Duration) {
        std::thread::sleep(dur);
    }
}

/// Runs a fixed number of poll iterations, without sleeping.
#[cfg(test)]
pub(crate) struct Bounded {
    /// Remaining iterations.
    pub(crate) remaining: usize,
}

#[cfg(test)]
impl Clock for Bounded {
    fn should_continue(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        true
    }
    fn sleep(&mut self, _dur: Duration) {}
}

/// Byte offsets already streamed, keyed by log file path.
type Offsets = HashMap<PathBuf, u64>;

/// List `*.log` files in a run directory, sorted for stable output.
fn log_files(run_dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = match std::fs::read_dir(run_dir) {
        Ok(entries) => entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "log"))
            .collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    files
}

/// Emit whatever has been appended to `path` since `offsets` last saw it.
///
/// Returns the number of new bytes emitted.
pub(crate) fn emit_new_bytes(path: &Path, offsets: &mut Offsets, json: bool) -> u64 {
    let content = match std::fs::read(path) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    let seen = offsets.entry(path.to_path_buf()).or_insert(0);
    let len = content.len() as u64;
    // A truncated/rewritten file restarts from zero rather than going silent.
    if len < *seen {
        *seen = 0;
    }
    if len == *seen {
        return 0;
    }
    let fresh = String::from_utf8_lossy(&content[*seen as usize..]).to_string();
    let new_bytes = len - *seen;
    *seen = len;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if json {
        let obj = serde_json::json!({
            "action": "follow",
            "status": "append",
            "file": name,
            "bytes": new_bytes,
            "content": fresh,
        });
        println!("{}", serde_json::to_string(&obj).unwrap_or_default());
    } else {
        for line in fresh.lines() {
            println!("[{name}] {line}");
        }
    }
    new_bytes
}

/// Poll `run_dir` until `clock` says to stop, streaming appended log bytes.
///
/// Returns the total number of bytes streamed (used by tests; production
/// ignores it because the loop only ends on Ctrl+C).
pub(crate) fn tail_run_dir(
    run_dir: &Path,
    json: bool,
    clock: &mut dyn Clock,
    interval: Duration,
) -> u64 {
    let mut offsets: Offsets = HashMap::new();
    let mut total = 0u64;
    while clock.should_continue() {
        for path in log_files(run_dir) {
            total += emit_new_bytes(&path, &mut offsets, json);
        }
        clock.sleep(interval);
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_streams_bytes_appended_between_polls() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let log = tmp.path().join("a.create.log");
        std::fs::write(&log, "line one\n").expect("write");

        // Two polls: the first drains the existing 9 bytes.
        let streamed = tail_run_dir(
            tmp.path(),
            false,
            &mut Bounded { remaining: 1 },
            Duration::from_millis(0),
        );
        assert_eq!(streamed, 9, "first poll must stream the existing content");
    }

    #[test]
    fn tail_does_not_re_emit_already_streamed_bytes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let log = tmp.path().join("a.create.log");
        std::fs::write(&log, "hello\n").expect("write");
        let streamed = tail_run_dir(
            tmp.path(),
            false,
            &mut Bounded { remaining: 4 },
            Duration::from_millis(0),
        );
        assert_eq!(streamed, 6, "content must be streamed exactly once");
    }

    #[test]
    fn emit_new_bytes_resumes_after_truncation() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let log = tmp.path().join("a.create.log");
        std::fs::write(&log, "abcdef").expect("write");
        let mut offsets = Offsets::new();
        assert_eq!(emit_new_bytes(&log, &mut offsets, false), 6);
        std::fs::write(&log, "xy").expect("rewrite");
        assert_eq!(
            emit_new_bytes(&log, &mut offsets, false),
            2,
            "a truncated log must restart rather than go silent"
        );
    }

    #[test]
    fn tail_ignores_non_log_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        std::fs::write(tmp.path().join("meta.yaml"), "run_id: r-1\n").expect("write");
        let streamed = tail_run_dir(
            tmp.path(),
            false,
            &mut Bounded { remaining: 2 },
            Duration::from_millis(0),
        );
        assert_eq!(streamed, 0);
    }
}

//! FJ-050: Syscall tracing provenance — renacer-compatible trace capture.
//!
//! Generates renacer-compatible trace records during apply execution.
//! Each resource apply produces a trace span with:
//! - W3C-compatible trace/span IDs
//! - Lamport logical clock for causal ordering
//! - Duration, exit code, stdout/stderr capture
//! - Resource metadata (type, machine, action)
//!
//! Trace output is JSONL format, compatible with renacer's SpanRecord schema.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Trace span for a single resource apply operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    /// W3C-compatible trace ID (hex string, 32 chars)
    pub trace_id: String,

    /// Span ID (hex string, 16 chars)
    pub span_id: String,

    /// Parent span ID (the apply run span)
    pub parent_span_id: Option<String>,

    /// Span name (e.g., "apply:nginx-config")
    pub name: String,

    /// Start time as ISO 8601
    pub start_time: String,

    /// Duration in microseconds
    pub duration_us: u64,

    /// Exit code (0 = success)
    pub exit_code: i32,

    /// Resource type
    pub resource_type: String,

    /// Target machine
    pub machine: String,

    /// Action taken (create/update/delete/noop)
    pub action: String,

    /// BLAKE3 content hash after apply
    pub content_hash: Option<String>,

    /// Lamport logical clock value
    pub logical_clock: u64,
}

/// Active trace session for an apply run.
pub struct TraceSession {
    trace_id: String,
    run_span_id: String,
    clock: u64,
    start: Instant,
    spans: Vec<TraceSpan>,
}

impl TraceSession {
    /// Start a new trace session for an apply run.
    pub fn start(run_id: &str) -> Self {
        // Derive trace ID from run_id (deterministic for reproducibility)
        let trace_id = format!("{:0>32}", format!("{:x}", hash_str(run_id)));
        let run_span_id = format!("{:0>16}", format!("{:x}", hash_str(&format!("{}-root", run_id))));

        Self {
            trace_id,
            run_span_id,
            clock: 0,
            start: Instant::now(),
            spans: Vec::new(),
        }
    }

    /// Get the trace ID.
    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    /// Get the root span ID.
    pub fn run_span_id(&self) -> &str {
        &self.run_span_id
    }

    /// Tick the logical clock and return the new value.
    pub fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }

    /// Record a resource apply span.
    #[allow(clippy::too_many_arguments)]
    pub fn record_span(
        &mut self,
        resource_id: &str,
        resource_type: &str,
        machine: &str,
        action: &str,
        duration: Duration,
        exit_code: i32,
        content_hash: Option<&str>,
    ) {
        let clock = self.tick();
        let span_id = format!(
            "{:0>16}",
            format!("{:x}", hash_str(&format!("{}-{}-{}", self.trace_id, resource_id, clock)))
        );

        let span = TraceSpan {
            trace_id: self.trace_id.clone(),
            span_id,
            parent_span_id: Some(self.run_span_id.clone()),
            name: format!("apply:{}", resource_id),
            start_time: crate::tripwire::eventlog::now_iso8601(),
            duration_us: duration.as_micros() as u64,
            exit_code,
            resource_type: resource_type.to_string(),
            machine: machine.to_string(),
            action: action.to_string(),
            content_hash: content_hash.map(String::from),
            logical_clock: clock,
        };

        self.spans.push(span);
    }

    /// Record a NoOp span (resource already converged).
    pub fn record_noop(&mut self, resource_id: &str, resource_type: &str, machine: &str) {
        self.record_span(
            resource_id,
            resource_type,
            machine,
            "noop",
            Duration::ZERO,
            0,
            None,
        );
    }

    /// Get all recorded spans.
    pub fn spans(&self) -> &[TraceSpan] {
        &self.spans
    }

    /// Total elapsed time for the session.
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    /// Finalize and create the root span.
    pub fn finalize(&mut self) -> TraceSpan {
        let clock = self.tick();
        let total_duration = self.start.elapsed();

        let failed = self.spans.iter().any(|s| s.exit_code != 0);

        TraceSpan {
            trace_id: self.trace_id.clone(),
            span_id: self.run_span_id.clone(),
            parent_span_id: None,
            name: "forjar:apply".to_string(),
            start_time: crate::tripwire::eventlog::now_iso8601(),
            duration_us: total_duration.as_micros() as u64,
            exit_code: if failed { 1 } else { 0 },
            resource_type: "root".to_string(),
            machine: "all".to_string(),
            action: "apply".to_string(),
            content_hash: None,
            logical_clock: clock,
        }
    }
}

/// Write trace spans to a JSONL file.
pub fn write_trace(state_dir: &Path, machine: &str, session: &TraceSession) -> Result<(), String> {
    let path = trace_path(state_dir, machine);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create trace dir: {}", e))?;
    }

    let mut output = String::new();
    for span in &session.spans {
        let json =
            serde_json::to_string(span).map_err(|e| format!("trace serialize error: {}", e))?;
        output.push_str(&json);
        output.push('\n');
    }

    std::fs::write(&path, output)
        .map_err(|e| format!("cannot write trace {}: {}", path.display(), e))?;

    Ok(())
}

/// Read trace spans from a JSONL file.
pub fn read_trace(state_dir: &Path, machine: &str) -> Result<Vec<TraceSpan>, String> {
    let path = trace_path(state_dir, machine);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read trace {}: {}", path.display(), e))?;

    let mut spans = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let span: TraceSpan =
            serde_json::from_str(line).map_err(|e| format!("trace parse error: {}", e))?;
        spans.push(span);
    }

    Ok(spans)
}

/// Derive the trace file path for a machine.
pub fn trace_path(state_dir: &Path, machine: &str) -> PathBuf {
    state_dir.join(machine).join("trace.jsonl")
}

/// Simple string hash for deriving IDs (not cryptographic).
fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325; // FNV offset basis
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3); // FNV prime
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj050_session_start() {
        let session = TraceSession::start("r-abc123");
        assert!(!session.trace_id().is_empty());
        assert!(!session.run_span_id().is_empty());
        assert_eq!(session.spans().len(), 0);
    }

    #[test]
    fn test_fj050_record_span() {
        let mut session = TraceSession::start("r-test");
        session.record_span(
            "nginx-config",
            "file",
            "web1",
            "create",
            Duration::from_millis(150),
            0,
            Some("blake3:abc123"),
        );
        assert_eq!(session.spans().len(), 1);
        let span = &session.spans()[0];
        assert_eq!(span.name, "apply:nginx-config");
        assert_eq!(span.resource_type, "file");
        assert_eq!(span.machine, "web1");
        assert_eq!(span.action, "create");
        assert_eq!(span.exit_code, 0);
        assert_eq!(span.duration_us, 150_000);
        assert_eq!(span.content_hash.as_deref(), Some("blake3:abc123"));
        assert_eq!(span.logical_clock, 1);
    }

    #[test]
    fn test_fj050_record_noop() {
        let mut session = TraceSession::start("r-test");
        session.record_noop("unchanged", "package", "localhost");
        let span = &session.spans()[0];
        assert_eq!(span.action, "noop");
        assert_eq!(span.duration_us, 0);
        assert_eq!(span.exit_code, 0);
    }

    #[test]
    fn test_fj050_logical_clock_ordering() {
        let mut session = TraceSession::start("r-test");
        session.record_noop("r1", "file", "m1");
        session.record_noop("r2", "file", "m1");
        session.record_noop("r3", "file", "m1");

        assert_eq!(session.spans()[0].logical_clock, 1);
        assert_eq!(session.spans()[1].logical_clock, 2);
        assert_eq!(session.spans()[2].logical_clock, 3);
    }

    #[test]
    fn test_fj050_trace_id_consistency() {
        let mut session = TraceSession::start("r-test");
        session.record_noop("r1", "file", "m1");
        session.record_noop("r2", "file", "m1");

        let trace_id = session.trace_id().to_string();
        assert_eq!(session.spans()[0].trace_id, trace_id);
        assert_eq!(session.spans()[1].trace_id, trace_id);
    }

    #[test]
    fn test_fj050_parent_span_id() {
        let mut session = TraceSession::start("r-test");
        session.record_noop("r1", "file", "m1");

        let run_span = session.run_span_id().to_string();
        assert_eq!(
            session.spans()[0].parent_span_id.as_deref(),
            Some(run_span.as_str())
        );
    }

    #[test]
    fn test_fj050_finalize_root_span() {
        let mut session = TraceSession::start("r-test");
        session.record_noop("r1", "file", "m1");
        let root = session.finalize();

        assert_eq!(root.name, "forjar:apply");
        assert!(root.parent_span_id.is_none());
        assert_eq!(root.exit_code, 0);
        assert_eq!(root.resource_type, "root");
    }

    #[test]
    fn test_fj050_finalize_failed_run() {
        let mut session = TraceSession::start("r-test");
        session.record_span("bad", "package", "m1", "create", Duration::from_secs(1), 1, None);
        let root = session.finalize();
        assert_eq!(root.exit_code, 1, "root should be failed when child failed");
    }

    #[test]
    fn test_fj050_span_serde_roundtrip() {
        let span = TraceSpan {
            trace_id: "00000000000000000000000000abcdef".to_string(),
            span_id: "0000000000123456".to_string(),
            parent_span_id: Some("0000000000000001".to_string()),
            name: "apply:test".to_string(),
            start_time: "2026-02-25T12:00:00Z".to_string(),
            duration_us: 5000,
            exit_code: 0,
            resource_type: "file".to_string(),
            machine: "localhost".to_string(),
            action: "create".to_string(),
            content_hash: Some("blake3:abc".to_string()),
            logical_clock: 1,
        };

        let json = serde_json::to_string(&span).unwrap();
        let back: TraceSpan = serde_json::from_str(&json).unwrap();
        assert_eq!(span.trace_id, back.trace_id);
        assert_eq!(span.span_id, back.span_id);
        assert_eq!(span.name, back.name);
        assert_eq!(span.duration_us, back.duration_us);
    }

    #[test]
    fn test_fj050_write_and_read_trace() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = TraceSession::start("r-test");
        session.record_noop("r1", "file", "m1");
        session.record_span("r2", "package", "m1", "create", Duration::from_millis(50), 0, None);

        write_trace(dir.path(), "m1", &session).unwrap();

        let spans = read_trace(dir.path(), "m1").unwrap();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].name, "apply:r1");
        assert_eq!(spans[1].name, "apply:r2");
    }

    #[test]
    fn test_fj050_read_trace_empty() {
        let dir = tempfile::tempdir().unwrap();
        let spans = read_trace(dir.path(), "nonexistent").unwrap();
        assert!(spans.is_empty());
    }

    #[test]
    fn test_fj050_trace_path() {
        let p = trace_path(Path::new("/state"), "web1");
        assert_eq!(p, PathBuf::from("/state/web1/trace.jsonl"));
    }

    #[test]
    fn test_fj050_hash_str_deterministic() {
        let h1 = hash_str("hello");
        let h2 = hash_str("hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_fj050_hash_str_different_inputs() {
        let h1 = hash_str("hello");
        let h2 = hash_str("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_fj050_unique_span_ids() {
        let mut session = TraceSession::start("r-test");
        session.record_noop("r1", "file", "m1");
        session.record_noop("r2", "file", "m1");
        session.record_noop("r3", "file", "m1");

        let ids: Vec<&str> = session.spans().iter().map(|s| s.span_id.as_str()).collect();
        let unique: std::collections::HashSet<&&str> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len(), "all span IDs must be unique");
    }

    #[test]
    fn test_fj050_session_deterministic() {
        let s1 = TraceSession::start("r-same");
        let s2 = TraceSession::start("r-same");
        assert_eq!(s1.trace_id(), s2.trace_id(), "same run ID → same trace ID");
    }

    #[test]
    fn test_fj050_session_different_runs() {
        let s1 = TraceSession::start("r-one");
        let s2 = TraceSession::start("r-two");
        assert_ne!(s1.trace_id(), s2.trace_id());
    }

    #[test]
    fn test_fj050_multiple_failures() {
        let mut session = TraceSession::start("r-test");
        session.record_span("ok", "file", "m1", "create", Duration::from_millis(10), 0, None);
        session.record_span("bad1", "package", "m1", "create", Duration::from_millis(20), 1, None);
        session.record_span("bad2", "service", "m1", "create", Duration::from_millis(30), 2, None);
        let root = session.finalize();
        assert_eq!(root.exit_code, 1);
    }
}

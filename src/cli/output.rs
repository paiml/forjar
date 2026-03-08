//! FJ-2920: Output abstraction for CLI handlers.
//!
//! Enables testable handlers by injecting an OutputWriter.
//! Production code uses StdoutWriter, tests use TestWriter,
//! benchmarks use NullWriter.
//!
//! Enables testable handlers by injecting an OutputWriter.
//! Production code uses StdoutWriter, tests use TestWriter,
//! benchmarks use NullWriter.

use std::io::Write;

/// Output trait for CLI commands — separates data (stdout) from status (stderr).
#[allow(dead_code)]
pub(crate) trait OutputWriter {
    /// Progress message → stderr.
    fn status(&mut self, msg: &str);
    /// Data output → stdout (the "result" of the command).
    fn result(&mut self, msg: &str);
    /// Warning message → stderr with ⚠ prefix.
    fn warning(&mut self, msg: &str);
    /// Error message → stderr with ✗ prefix.
    fn error(&mut self, msg: &str);
    /// Success message → stderr with ✓ prefix.
    fn success(&mut self, msg: &str);
    /// Flush all buffered output.
    fn flush(&mut self);
}

/// Production writer: status/warning/error → stderr, result → stdout.
pub(crate) struct StdoutWriter;

impl OutputWriter for StdoutWriter {
    fn status(&mut self, msg: &str) {
        eprintln!("{msg}");
    }

    fn result(&mut self, msg: &str) {
        println!("{msg}");
    }

    fn warning(&mut self, msg: &str) {
        let icon = super::colors::yellow("⚠");
        eprintln!("{icon} {msg}");
    }

    fn error(&mut self, msg: &str) {
        let icon = super::colors::red("✗");
        eprintln!("{icon} {msg}");
    }

    fn success(&mut self, msg: &str) {
        let icon = super::colors::green("✓");
        eprintln!("{icon} {msg}");
    }

    fn flush(&mut self) {
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
    }
}

/// Test writer: captures all output for assertions.
#[cfg(test)]
pub(crate) struct TestWriter {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}

#[cfg(test)]
impl TestWriter {
    pub fn new() -> Self {
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    /// Get all stdout lines joined.
    pub fn stdout_text(&self) -> String {
        self.stdout.join("\n")
    }

    /// Get all stderr lines joined.
    pub fn stderr_text(&self) -> String {
        self.stderr.join("\n")
    }
}

#[cfg(test)]
impl OutputWriter for TestWriter {
    fn status(&mut self, msg: &str) {
        self.stderr.push(msg.to_string());
    }

    fn result(&mut self, msg: &str) {
        self.stdout.push(msg.to_string());
    }

    fn warning(&mut self, msg: &str) {
        self.stderr.push(format!("⚠ {msg}"));
    }

    fn error(&mut self, msg: &str) {
        self.stderr.push(format!("✗ {msg}"));
    }

    fn success(&mut self, msg: &str) {
        self.stderr.push(format!("✓ {msg}"));
    }

    fn flush(&mut self) {}
}

/// Null writer: discards all output (for benchmarks).
#[allow(dead_code)]
pub(crate) struct NullWriter;

impl OutputWriter for NullWriter {
    fn status(&mut self, _msg: &str) {}
    fn result(&mut self, _msg: &str) {}
    fn warning(&mut self, _msg: &str) {}
    fn error(&mut self, _msg: &str) {}
    fn success(&mut self, _msg: &str) {}
    fn flush(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdout_writer_does_not_panic() {
        let mut w = StdoutWriter;
        w.status("status msg");
        w.result("result data");
        w.warning("warn msg");
        w.error("error msg");
        w.success("success msg");
        w.flush();
    }

    #[test]
    fn test_test_writer_captures() {
        let mut w = TestWriter::new();
        w.result("line 1");
        w.result("line 2");
        w.status("progress");
        w.warning("uh oh");
        w.error("failed");
        w.success("done");

        assert_eq!(w.stdout.len(), 2);
        assert_eq!(w.stderr.len(), 4);
        assert_eq!(w.stdout_text(), "line 1\nline 2");
        assert!(w.stderr_text().contains("⚠ uh oh"));
        assert!(w.stderr_text().contains("✗ failed"));
        assert!(w.stderr_text().contains("✓ done"));
    }

    #[test]
    fn test_null_writer_discards() {
        let mut w = NullWriter;
        w.result("data");
        w.status("status");
        w.warning("warn");
        w.error("err");
        w.success("ok");
        w.flush();
        // No assertions — just verify it doesn't panic
    }

    #[test]
    fn test_dyn_dispatch() {
        let mut w: Box<dyn OutputWriter> = Box::new(TestWriter::new());
        w.result("hello");
        w.status("working");
        w.flush();
    }
}

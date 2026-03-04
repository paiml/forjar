//! FJ-105: Progress bars and spinners for long-running operations.
//!
//! Lightweight ASCII progress indicators without external dependencies.
//! Supports spinner animation and progress bars with ETA estimation.

use std::io::Write;
use std::time::{Duration, Instant};

/// Spinner animation frames.
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// A text-based spinner.
pub struct Spinner {
    message: String,
    frame: usize,
    start: Instant,
    active: bool,
}

impl Spinner {
    pub fn new(message: &str) -> Self {
        Spinner {
            message: message.to_string(),
            frame: 0,
            start: Instant::now(),
            active: true,
        }
    }

    /// Advance the spinner one frame (call in a loop).
    pub fn tick(&mut self) {
        if !self.active {
            return;
        }
        let frame = SPINNER_FRAMES[self.frame % SPINNER_FRAMES.len()];
        let elapsed = self.start.elapsed().as_secs();
        eprint!("\r{frame} {} ({elapsed}s)", self.message);
        let _ = std::io::stderr().flush();
        self.frame += 1;
    }

    /// Stop the spinner with a final message.
    pub fn finish(&mut self, message: &str) {
        self.active = false;
        let elapsed = self.start.elapsed().as_secs();
        eprintln!("\r✓ {message} ({elapsed}s)");
    }

    /// Stop the spinner with an error.
    pub fn fail(&mut self, message: &str) {
        self.active = false;
        let elapsed = self.start.elapsed().as_secs();
        eprintln!("\r✗ {message} ({elapsed}s)");
    }
}

/// A text-based progress bar.
pub struct ProgressBar {
    total: usize,
    current: usize,
    width: usize,
    message: String,
    start: Instant,
}

impl ProgressBar {
    pub fn new(total: usize, message: &str) -> Self {
        ProgressBar {
            total,
            current: 0,
            width: 40,
            message: message.to_string(),
            start: Instant::now(),
        }
    }

    /// Set current progress.
    pub fn set(&mut self, current: usize) {
        self.current = current.min(self.total);
        self.render();
    }

    /// Increment by one.
    pub fn inc(&mut self) {
        self.set(self.current + 1);
    }

    /// Render the progress bar.
    fn render(&self) {
        let pct = if self.total > 0 {
            self.current * 100 / self.total
        } else {
            100
        };
        let filled = self.width * self.current / self.total.max(1);
        let empty = self.width - filled;

        let bar: String = "█".repeat(filled) + &"░".repeat(empty);
        let eta = self.estimate_eta();

        eprint!(
            "\r{} [{bar}] {}/{} ({pct}%) {eta}",
            self.message, self.current, self.total
        );
        let _ = std::io::stderr().flush();
    }

    fn estimate_eta(&self) -> String {
        if self.current == 0 {
            return "ETA: --".to_string();
        }
        let elapsed = self.start.elapsed();
        let per_item = elapsed / self.current as u32;
        let remaining = (self.total - self.current) as u32;
        let eta = per_item * remaining;
        format_duration(eta)
    }

    /// Finish the progress bar.
    pub fn finish(&self) {
        let elapsed = self.start.elapsed();
        eprintln!(
            "\r{} [{bar}] {}/{} (100%) done in {elapsed}",
            self.message,
            self.total,
            self.total,
            bar = "█".repeat(self.width),
            elapsed = format_duration(elapsed),
        );
    }
}

fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("ETA: {secs}s")
    } else {
        format!("ETA: {}m {}s", secs / 60, secs % 60)
    }
}

/// A multi-resource progress tracker.
#[derive(Debug, serde::Serialize)]
pub struct ProgressReport {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub elapsed_secs: f64,
}

/// Track progress across multiple resources.
pub fn track_progress(total: usize) -> ProgressTracker {
    ProgressTracker {
        total,
        completed: 0,
        failed: 0,
        skipped: 0,
        start: Instant::now(),
    }
}

pub struct ProgressTracker {
    total: usize,
    completed: usize,
    failed: usize,
    skipped: usize,
    start: Instant,
}

impl ProgressTracker {
    pub fn complete(&mut self) {
        self.completed += 1;
    }

    pub fn fail(&mut self) {
        self.failed += 1;
    }

    pub fn skip(&mut self) {
        self.skipped += 1;
    }

    pub fn report(&self) -> ProgressReport {
        ProgressReport {
            total: self.total,
            completed: self.completed,
            failed: self.failed,
            skipped: self.skipped,
            elapsed_secs: self.start.elapsed().as_secs_f64(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_lifecycle() {
        let mut s = Spinner::new("testing");
        s.tick();
        s.tick();
        s.finish("done");
        assert!(!s.active);
    }

    #[test]
    fn test_spinner_fail() {
        let mut s = Spinner::new("testing");
        s.fail("error occurred");
        assert!(!s.active);
    }

    #[test]
    fn test_progress_bar_lifecycle() {
        let mut pb = ProgressBar::new(10, "items");
        pb.inc();
        pb.inc();
        pb.set(5);
        assert_eq!(pb.current, 5);
        pb.set(10);
        pb.finish();
    }

    #[test]
    fn test_progress_bar_zero_total() {
        let pb = ProgressBar::new(0, "empty");
        pb.finish();
    }

    #[test]
    fn test_progress_tracker() {
        let mut t = track_progress(5);
        t.complete();
        t.complete();
        t.fail();
        t.skip();
        let r = t.report();
        assert_eq!(r.total, 5);
        assert_eq!(r.completed, 2);
        assert_eq!(r.failed, 1);
        assert_eq!(r.skipped, 1);
    }

    #[test]
    fn test_progress_report_serde() {
        let r = ProgressReport {
            total: 10,
            completed: 8,
            failed: 1,
            skipped: 1,
            elapsed_secs: 2.5,
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"total\":10"));
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "ETA: 30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "ETA: 1m 30s");
    }

    #[test]
    fn test_spinner_frames() {
        assert_eq!(SPINNER_FRAMES.len(), 10);
    }
}

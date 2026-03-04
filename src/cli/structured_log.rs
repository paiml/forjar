//! FJ-104: Structured logging framework.
//!
//! Provides structured log output with levels (DEBUG, INFO, WARN, ERROR),
//! structured spans, and subscriber-based output. Outputs as JSON when
//! `--log-json` is set, otherwise human-readable format.
//!
//! No external dependencies — pure Rust implementation.

use std::sync::atomic::{AtomicU8, Ordering};
use std::time::SystemTime;

/// Log level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum Level {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

impl std::fmt::Display for Level {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Level::Debug => write!(f, "DEBUG"),
            Level::Info => write!(f, "INFO"),
            Level::Warn => write!(f, "WARN"),
            Level::Error => write!(f, "ERROR"),
        }
    }
}

impl Level {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(Level::Debug),
            "info" => Ok(Level::Info),
            "warn" | "warning" => Ok(Level::Warn),
            "error" => Ok(Level::Error),
            _ => Err(format!("unknown log level: {s}")),
        }
    }
}

/// Global log level filter.
static LOG_LEVEL: AtomicU8 = AtomicU8::new(1); // default: Info
/// Global JSON mode flag.
static LOG_JSON: AtomicU8 = AtomicU8::new(0); // default: human

/// Set the global log level.
pub fn set_level(level: Level) {
    LOG_LEVEL.store(level as u8, Ordering::Relaxed);
}

/// Set JSON output mode.
pub fn set_json(json: bool) {
    LOG_JSON.store(u8::from(json), Ordering::Relaxed);
}

/// Get current log level.
pub fn current_level() -> Level {
    match LOG_LEVEL.load(Ordering::Relaxed) {
        0 => Level::Debug,
        1 => Level::Info,
        2 => Level::Warn,
        _ => Level::Error,
    }
}

/// Check if a level is enabled.
pub fn is_enabled(level: Level) -> bool {
    (level as u8) >= LOG_LEVEL.load(Ordering::Relaxed)
}

/// A structured log event.
#[derive(Debug, serde::Serialize)]
pub struct LogEvent {
    pub timestamp: String,
    pub level: Level,
    pub message: String,
    pub target: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<(String, String)>,
}

/// Emit a structured log event.
pub fn log_event(level: Level, target: &str, message: &str, fields: &[(&str, &str)]) {
    if !is_enabled(level) {
        return;
    }

    let event = LogEvent {
        timestamp: format!("{:?}", SystemTime::now()),
        level,
        message: message.to_string(),
        target: target.to_string(),
        fields: fields.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
    };

    if LOG_JSON.load(Ordering::Relaxed) != 0 {
        emit_json(&event);
    } else {
        emit_human(&event);
    }
}

fn emit_json(event: &LogEvent) {
    if let Ok(json) = serde_json::to_string(event) {
        eprintln!("{json}");
    }
}

fn emit_human(event: &LogEvent) {
    let fields_str = if event.fields.is_empty() {
        String::new()
    } else {
        let pairs: Vec<String> = event.fields.iter().map(|(k, v)| format!("{k}={v}")).collect();
        format!(" {}", pairs.join(" "))
    };
    eprintln!("[{}] {} {}{}", event.level, event.target, event.message, fields_str);
}

/// A log span for tracking execution context.
#[derive(Debug)]
pub struct Span {
    pub name: String,
    pub fields: Vec<(String, String)>,
}

impl Span {
    pub fn new(name: &str) -> Self {
        log_event(Level::Debug, name, "span:enter", &[]);
        Span {
            name: name.to_string(),
            fields: Vec::new(),
        }
    }

    pub fn with_field(mut self, key: &str, value: &str) -> Self {
        self.fields.push((key.to_string(), value.to_string()));
        self
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        log_event(Level::Debug, &self.name, "span:exit", &[]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_display() {
        assert_eq!(format!("{}", Level::Debug), "DEBUG");
        assert_eq!(format!("{}", Level::Error), "ERROR");
    }

    #[test]
    fn test_level_from_str() {
        assert_eq!(Level::from_str("debug").unwrap(), Level::Debug);
        assert_eq!(Level::from_str("INFO").unwrap(), Level::Info);
        assert_eq!(Level::from_str("warning").unwrap(), Level::Warn);
        assert!(Level::from_str("bad").is_err());
    }

    #[test]
    fn test_level_ordering() {
        assert!(Level::Debug < Level::Info);
        assert!(Level::Info < Level::Warn);
        assert!(Level::Warn < Level::Error);
    }

    #[test]
    fn test_set_and_get_level() {
        set_level(Level::Warn);
        assert_eq!(current_level(), Level::Warn);
        assert!(!is_enabled(Level::Debug));
        assert!(!is_enabled(Level::Info));
        assert!(is_enabled(Level::Warn));
        assert!(is_enabled(Level::Error));
        set_level(Level::Info); // restore default
    }

    #[test]
    fn test_log_event_no_panic() {
        set_level(Level::Debug);
        log_event(Level::Info, "test", "hello", &[("key", "value")]);
        set_level(Level::Info);
    }

    #[test]
    fn test_log_event_json_mode() {
        set_json(true);
        log_event(Level::Info, "test::json", "structured output", &[]);
        set_json(false);
    }

    #[test]
    fn test_log_event_serde() {
        let event = LogEvent {
            timestamp: "now".to_string(),
            level: Level::Info,
            message: "test".to_string(),
            target: "test".to_string(),
            fields: vec![("k".to_string(), "v".to_string())],
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"level\":\"Info\""));
    }

    #[test]
    fn test_span_lifecycle() {
        set_level(Level::Debug);
        {
            let _span = Span::new("test-span").with_field("id", "42");
            // span is active here
        }
        // span dropped — exit logged
        set_level(Level::Info);
    }
}

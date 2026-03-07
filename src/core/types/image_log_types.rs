//! FJ-2301: Image build log types — per-layer output capture and search.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2301: Per-layer build log entry for image builds.
///
/// Captures stdout/stderr from each layer build step, stored alongside
/// the OCI layout for debugging failed builds.
///
/// # Examples
///
/// ```
/// use forjar::core::types::LayerBuildLog;
///
/// let log = LayerBuildLog::new("ml-deps", 1, 47.3);
/// assert_eq!(log.layer_name, "ml-deps");
/// assert_eq!(log.layer_index, 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerBuildLog {
    /// Layer name from the image definition.
    pub layer_name: String,
    /// Layer index in the build plan (0-based).
    pub layer_index: u32,
    /// Duration of this layer build in seconds.
    pub duration_secs: f64,
    /// Exit code of the build command.
    #[serde(default)]
    pub exit_code: Option<i32>,
    /// Whether this layer was served from cache (no log content).
    #[serde(default)]
    pub cached: bool,
    /// Log file path (relative to image build dir).
    #[serde(default)]
    pub log_path: Option<String>,
    /// Truncated tail of stderr (for summary display).
    #[serde(default)]
    pub stderr_tail: Option<String>,
    /// Total bytes of log output.
    #[serde(default)]
    pub log_bytes: u64,
}

impl LayerBuildLog {
    /// Create a new log entry for a completed layer.
    pub fn new(name: &str, index: u32, duration_secs: f64) -> Self {
        Self {
            layer_name: name.to_string(),
            layer_index: index,
            duration_secs,
            exit_code: None,
            cached: false,
            log_path: None,
            stderr_tail: None,
            log_bytes: 0,
        }
    }

    /// Create a cached layer entry (no log output).
    pub fn cached(name: &str, index: u32) -> Self {
        Self {
            cached: true,
            duration_secs: 0.0,
            ..Self::new(name, index, 0.0)
        }
    }

    /// Whether the layer build succeeded.
    pub fn succeeded(&self) -> bool {
        self.exit_code.is_none_or(|c| c == 0) || self.cached
    }

    /// Log file path for a layer.
    pub fn default_log_path(image_name: &str, layer_index: u32) -> String {
        format!("state/builds/{image_name}/layer-{layer_index}.log")
    }
}

impl fmt::Display for LayerBuildLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.cached {
            write!(
                f,
                "  Layer {}: {} (cached)",
                self.layer_index, self.layer_name
            )
        } else {
            let status = if self.succeeded() { "ok" } else { "FAIL" };
            write!(
                f,
                "  Layer {}: {} ({status}, {:.1}s, {} bytes)",
                self.layer_index, self.layer_name, self.duration_secs, self.log_bytes,
            )
        }
    }
}

/// FJ-2301: Complete image build log collection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageBuildLog {
    /// Image reference being built.
    pub image_ref: String,
    /// Per-layer logs in build order.
    pub layers: Vec<LayerBuildLog>,
    /// Manifest assembly log path.
    #[serde(default)]
    pub manifest_log: Option<String>,
    /// Registry push log path (if --push).
    #[serde(default)]
    pub push_log: Option<String>,
    /// Total build duration in seconds.
    pub total_duration_secs: f64,
}

impl ImageBuildLog {
    /// Build directory for image logs.
    pub fn build_dir(image_name: &str) -> String {
        format!("state/builds/{image_name}")
    }

    /// Whether all layers succeeded.
    pub fn all_succeeded(&self) -> bool {
        self.layers.iter().all(|l| l.succeeded())
    }

    /// Count of cached layers.
    pub fn cached_count(&self) -> usize {
        self.layers.iter().filter(|l| l.cached).count()
    }

    /// Count of failed layers.
    pub fn failed_count(&self) -> usize {
        self.layers.iter().filter(|l| !l.succeeded()).count()
    }

    /// Total log bytes across all layers.
    pub fn total_log_bytes(&self) -> u64 {
        self.layers.iter().map(|l| l.log_bytes).sum()
    }
}

impl fmt::Display for ImageBuildLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Build: {} ({:.1}s)",
            self.image_ref, self.total_duration_secs
        )?;
        for layer in &self.layers {
            writeln!(f, "{layer}")?;
        }
        if self.failed_count() > 0 {
            write!(f, "  {} layer(s) FAILED", self.failed_count())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_build_log_new() {
        let log = LayerBuildLog::new("packages", 0, 12.5);
        assert_eq!(log.layer_name, "packages");
        assert_eq!(log.layer_index, 0);
        assert!(!log.cached);
        assert!(log.succeeded());
    }

    #[test]
    fn layer_build_log_cached() {
        let log = LayerBuildLog::cached("system", 0);
        assert!(log.cached);
        assert!(log.succeeded());
        let s = log.to_string();
        assert!(s.contains("cached"));
    }

    #[test]
    fn layer_build_log_failed() {
        let log = LayerBuildLog {
            exit_code: Some(1),
            stderr_tail: Some("error: compilation failed".into()),
            ..LayerBuildLog::new("build", 1, 30.0)
        };
        assert!(!log.succeeded());
        let s = log.to_string();
        assert!(s.contains("FAIL"));
    }

    #[test]
    fn layer_build_log_display() {
        let log = LayerBuildLog {
            log_bytes: 4096,
            ..LayerBuildLog::new("ml-deps", 2, 47.3)
        };
        let s = log.to_string();
        assert!(s.contains("ml-deps"));
        assert!(s.contains("47.3s"));
        assert!(s.contains("4096 bytes"));
    }

    #[test]
    fn layer_build_log_default_path() {
        let path = LayerBuildLog::default_log_path("training-image", 2);
        assert_eq!(path, "state/builds/training-image/layer-2.log");
    }

    #[test]
    fn image_build_log_all_succeeded() {
        let log = ImageBuildLog {
            image_ref: "app:1.0".into(),
            layers: vec![
                LayerBuildLog::cached("base", 0),
                LayerBuildLog::new("code", 1, 5.0),
            ],
            manifest_log: None,
            push_log: None,
            total_duration_secs: 5.2,
        };
        assert!(log.all_succeeded());
        assert_eq!(log.cached_count(), 1);
        assert_eq!(log.failed_count(), 0);
    }

    #[test]
    fn image_build_log_with_failure() {
        let log = ImageBuildLog {
            image_ref: "app:1.0".into(),
            layers: vec![
                LayerBuildLog::cached("base", 0),
                LayerBuildLog {
                    exit_code: Some(1),
                    ..LayerBuildLog::new("build", 1, 30.0)
                },
            ],
            manifest_log: None,
            push_log: None,
            total_duration_secs: 30.5,
        };
        assert!(!log.all_succeeded());
        assert_eq!(log.failed_count(), 1);
    }

    #[test]
    fn image_build_log_total_bytes() {
        let log = ImageBuildLog {
            image_ref: "x".into(),
            layers: vec![
                LayerBuildLog {
                    log_bytes: 1000,
                    ..LayerBuildLog::new("a", 0, 1.0)
                },
                LayerBuildLog {
                    log_bytes: 2000,
                    ..LayerBuildLog::new("b", 1, 2.0)
                },
            ],
            manifest_log: None,
            push_log: None,
            total_duration_secs: 3.0,
        };
        assert_eq!(log.total_log_bytes(), 3000);
    }

    #[test]
    fn image_build_log_display() {
        let log = ImageBuildLog {
            image_ref: "training:2.0".into(),
            layers: vec![LayerBuildLog::cached("base", 0)],
            manifest_log: None,
            push_log: None,
            total_duration_secs: 0.5,
        };
        let s = log.to_string();
        assert!(s.contains("training:2.0"));
        assert!(s.contains("0.5s"));
    }

    #[test]
    fn image_build_log_build_dir() {
        assert_eq!(ImageBuildLog::build_dir("my-app"), "state/builds/my-app");
    }

    #[test]
    fn image_build_log_serde() {
        let log = ImageBuildLog {
            image_ref: "test:1".into(),
            layers: vec![LayerBuildLog::new("code", 0, 1.0)],
            manifest_log: Some("manifest.log".into()),
            push_log: None,
            total_duration_secs: 1.0,
        };
        let json = serde_json::to_string(&log).unwrap();
        let parsed: ImageBuildLog = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.layers.len(), 1);
        assert_eq!(parsed.manifest_log.as_deref(), Some("manifest.log"));
    }
}

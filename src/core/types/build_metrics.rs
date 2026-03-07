//! FJ-2403: Build metrics types — binary size tracking, reproducibility.
//!
//! Tracks binary sizes, dependency counts, and build metadata across
//! releases for size regression detection.

use serde::{Deserialize, Serialize};

/// FJ-2403: Build metrics for a forjar release.
///
/// # Examples
///
/// ```
/// use forjar::core::types::BuildMetrics;
///
/// let metrics = BuildMetrics::current();
/// assert!(!metrics.version.is_empty());
/// assert!(metrics.target.contains("linux") || metrics.target.contains("darwin") || metrics.target.contains("windows"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildMetrics {
    /// Forjar version.
    pub version: String,
    /// Build target triple (e.g., x86_64-unknown-linux-gnu).
    pub target: String,
    /// Build profile (debug or release).
    pub profile: String,
    /// Binary size in bytes (if known).
    #[serde(default)]
    pub binary_size: Option<u64>,
    /// Number of direct dependencies.
    #[serde(default)]
    pub dependency_count: Option<u32>,
    /// Rust toolchain version.
    #[serde(default)]
    pub rust_version: Option<String>,
    /// Whether build used `--locked` (reproducible).
    #[serde(default)]
    pub locked: bool,
    /// Whether LTO was enabled.
    #[serde(default)]
    pub lto: bool,
    /// ISO 8601 build timestamp.
    #[serde(default)]
    pub built_at: Option<String>,
}

impl BuildMetrics {
    /// Collect current build metrics from compile-time environment.
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            target: std::env::consts::ARCH.to_string() + "-" + std::env::consts::OS,
            profile: if cfg!(debug_assertions) {
                "debug".into()
            } else {
                "release".into()
            },
            binary_size: None,
            dependency_count: None,
            rust_version: option_env!("RUSTC_VERSION").map(|s| s.to_string()),
            locked: false,
            lto: cfg!(not(debug_assertions)),
            built_at: None,
        }
    }

    /// Check if binary size exceeds a threshold (in bytes).
    pub fn exceeds_size(&self, max_bytes: u64) -> bool {
        self.binary_size.is_some_and(|s| s > max_bytes)
    }

    /// Calculate size change percentage from a previous build.
    pub fn size_change_pct(&self, previous: &Self) -> Option<f64> {
        match (self.binary_size, previous.binary_size) {
            (Some(current), Some(prev)) if prev > 0 => {
                Some(((current as f64 - prev as f64) / prev as f64) * 100.0)
            }
            _ => None,
        }
    }

    /// Format a human-readable summary.
    pub fn format_summary(&self) -> String {
        let mut out = format!(
            "Build: {} ({} {})\n",
            self.version, self.target, self.profile
        );
        if let Some(size) = self.binary_size {
            let mb = size as f64 / (1024.0 * 1024.0);
            out.push_str(&format!("  Binary size: {mb:.1} MB ({size} bytes)\n"));
        }
        if let Some(deps) = self.dependency_count {
            out.push_str(&format!("  Dependencies: {deps}\n"));
        }
        if let Some(ref rust) = self.rust_version {
            out.push_str(&format!("  Rust: {rust}\n"));
        }
        out.push_str(&format!("  LTO: {}, Locked: {}\n", self.lto, self.locked));
        out
    }
}

/// FJ-2403/E17: Image build metrics collected during `forjar build`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageBuildMetrics {
    /// Image tag (e.g., "myapp:latest").
    pub tag: String,
    /// Number of layers in the built image.
    pub layer_count: usize,
    /// Total compressed image size in bytes.
    pub total_size: u64,
    /// Per-layer metrics.
    pub layers: Vec<LayerMetric>,
    /// Build duration in seconds.
    pub duration_secs: f64,
    /// ISO 8601 timestamp of the build.
    pub built_at: String,
    /// Forjar version used for the build.
    pub forjar_version: String,
    /// Build target architecture.
    pub target_arch: String,
}

/// Per-layer build metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerMetric {
    /// Number of files in the layer.
    pub file_count: u32,
    /// Uncompressed layer size in bytes.
    pub uncompressed_size: u64,
    /// Compressed layer size in bytes.
    pub compressed_size: u64,
}

impl ImageBuildMetrics {
    /// Write metrics to `build-metrics.json` in the output directory.
    pub fn write_to(&self, dir: &std::path::Path) -> Result<(), String> {
        let path = dir.join("build-metrics.json");
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("serialize build metrics: {e}"))?;
        std::fs::write(path, json).map_err(|e| format!("write build metrics: {e}"))
    }
}

/// FJ-2403: Binary size threshold for regression detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeThreshold {
    /// Maximum allowed binary size in bytes.
    pub max_bytes: u64,
    /// Maximum allowed growth percentage from previous release.
    pub max_growth_pct: f64,
}

impl Default for SizeThreshold {
    fn default() -> Self {
        Self {
            max_bytes: 10 * 1024 * 1024, // 10 MB
            max_growth_pct: 10.0,        // 10%
        }
    }
}

impl SizeThreshold {
    /// Check if a build violates the threshold.
    pub fn check(&self, current: &BuildMetrics, previous: Option<&BuildMetrics>) -> Vec<String> {
        let mut violations = Vec::new();

        if current.exceeds_size(self.max_bytes) {
            let size = current.binary_size.unwrap_or(0);
            let max_mb = self.max_bytes as f64 / (1024.0 * 1024.0);
            let cur_mb = size as f64 / (1024.0 * 1024.0);
            violations.push(format!(
                "binary size {cur_mb:.1} MB exceeds threshold {max_mb:.1} MB"
            ));
        }

        if let Some(prev) = previous {
            if let Some(pct) = current.size_change_pct(prev) {
                if pct > self.max_growth_pct {
                    violations.push(format!(
                        "binary size grew {pct:.1}% (threshold: {:.1}%)",
                        self.max_growth_pct
                    ));
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_metrics_current() {
        let m = BuildMetrics::current();
        assert_eq!(m.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(m.profile, "debug"); // tests run in debug
    }

    #[test]
    fn build_metrics_exceeds_size() {
        let mut m = BuildMetrics::current();
        m.binary_size = Some(15_000_000);
        assert!(m.exceeds_size(10_000_000));
        assert!(!m.exceeds_size(20_000_000));
    }

    #[test]
    fn build_metrics_no_size() {
        let m = BuildMetrics::current();
        assert!(!m.exceeds_size(10_000_000));
    }

    #[test]
    fn build_metrics_size_change() {
        let mut current = BuildMetrics::current();
        current.binary_size = Some(11_000_000);
        let mut prev = BuildMetrics::current();
        prev.binary_size = Some(10_000_000);
        let pct = current.size_change_pct(&prev).unwrap();
        assert!((pct - 10.0).abs() < 0.1);
    }

    #[test]
    fn build_metrics_size_change_no_data() {
        let m = BuildMetrics::current();
        assert!(m.size_change_pct(&m).is_none());
    }

    #[test]
    fn build_metrics_format_summary() {
        let mut m = BuildMetrics::current();
        m.binary_size = Some(8_500_000);
        m.dependency_count = Some(45);
        let summary = m.format_summary();
        assert!(summary.contains("8.1 MB"));
        assert!(summary.contains("Dependencies: 45"));
    }

    #[test]
    fn build_metrics_serde_roundtrip() {
        let mut m = BuildMetrics::current();
        m.binary_size = Some(5_000_000);
        m.locked = true;
        let json = serde_json::to_string(&m).unwrap();
        let parsed: BuildMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.binary_size, Some(5_000_000));
        assert!(parsed.locked);
    }

    #[test]
    fn size_threshold_defaults() {
        let t = SizeThreshold::default();
        assert_eq!(t.max_bytes, 10 * 1024 * 1024);
        assert_eq!(t.max_growth_pct, 10.0);
    }

    #[test]
    fn size_threshold_check_pass() {
        let t = SizeThreshold::default();
        let mut m = BuildMetrics::current();
        m.binary_size = Some(8_000_000); // 8MB < 10MB
        let violations = t.check(&m, None);
        assert!(violations.is_empty());
    }

    #[test]
    fn size_threshold_check_exceed() {
        let t = SizeThreshold::default();
        let mut m = BuildMetrics::current();
        m.binary_size = Some(15_000_000);
        let violations = t.check(&m, None);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("exceeds threshold"));
    }

    #[test]
    fn size_threshold_check_growth() {
        let t = SizeThreshold::default();
        let mut current = BuildMetrics::current();
        current.binary_size = Some(9_000_000); // under absolute limit
        let mut prev = BuildMetrics::current();
        prev.binary_size = Some(7_000_000); // 28% growth
        let violations = t.check(&current, Some(&prev));
        assert_eq!(violations.len(), 1);
        assert!(violations[0].contains("grew"));
    }

    #[test]
    fn size_threshold_check_both_violations() {
        let t = SizeThreshold::default();
        let mut current = BuildMetrics::current();
        current.binary_size = Some(15_000_000);
        let mut prev = BuildMetrics::current();
        prev.binary_size = Some(7_000_000);
        let violations = t.check(&current, Some(&prev));
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn image_build_metrics_serde_roundtrip() {
        let m = ImageBuildMetrics {
            tag: "myapp:v1".into(),
            layer_count: 2,
            total_size: 1024,
            layers: vec![
                LayerMetric {
                    file_count: 3,
                    uncompressed_size: 2000,
                    compressed_size: 800,
                },
                LayerMetric {
                    file_count: 1,
                    uncompressed_size: 500,
                    compressed_size: 224,
                },
            ],
            duration_secs: 1.5,
            built_at: "2026-03-07T00:00:00Z".into(),
            forjar_version: "0.1.0".into(),
            target_arch: "x86_64".into(),
        };
        let json = serde_json::to_string(&m).unwrap();
        let parsed: ImageBuildMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tag, "myapp:v1");
        assert_eq!(parsed.layer_count, 2);
        assert_eq!(parsed.layers.len(), 2);
        assert_eq!(parsed.layers[0].file_count, 3);
    }

    #[test]
    fn image_build_metrics_write_to_tempdir() {
        let dir = std::env::temp_dir().join("forjar-test-ibm");
        let _ = std::fs::create_dir_all(&dir);
        let m = ImageBuildMetrics {
            tag: "test:latest".into(),
            layer_count: 1,
            total_size: 512,
            layers: vec![LayerMetric {
                file_count: 2,
                uncompressed_size: 512,
                compressed_size: 256,
            }],
            duration_secs: 0.3,
            built_at: "2026-03-07T00:00:00Z".into(),
            forjar_version: env!("CARGO_PKG_VERSION").into(),
            target_arch: "x86_64".into(),
        };
        m.write_to(&dir).unwrap();
        let content = std::fs::read_to_string(dir.join("build-metrics.json")).unwrap();
        assert!(content.contains("test:latest"));
        assert!(content.contains("512"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

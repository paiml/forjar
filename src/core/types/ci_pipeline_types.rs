//! FJ-2400/2403: CI pipeline types — reproducible builds, MSRV, feature matrix.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2403: Reproducible build configuration.
///
/// # Examples
///
/// ```
/// use forjar::core::types::ReproBuildConfig;
///
/// let config = ReproBuildConfig::default();
/// assert!(config.locked);
/// assert!(config.lto);
/// assert_eq!(config.codegen_units, 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproBuildConfig {
    /// Use `--locked` to pin Cargo.lock.
    #[serde(default = "default_true_ci")]
    pub locked: bool,
    /// Disable incremental compilation.
    #[serde(default = "default_true_ci")]
    pub no_incremental: bool,
    /// Enable LTO.
    #[serde(default = "default_true_ci")]
    pub lto: bool,
    /// Codegen units (1 for reproducibility).
    #[serde(default = "default_one_ci")]
    pub codegen_units: u32,
    /// Panic strategy ("abort" for reproducibility).
    #[serde(default = "default_abort")]
    pub panic: String,
    /// SOURCE_DATE_EPOCH value (0 = not set).
    #[serde(default)]
    pub source_date_epoch: Option<u64>,
}

fn default_true_ci() -> bool {
    true
}
fn default_one_ci() -> u32 {
    1
}
fn default_abort() -> String {
    "abort".into()
}

impl Default for ReproBuildConfig {
    fn default() -> Self {
        Self {
            locked: true,
            no_incremental: true,
            lto: true,
            codegen_units: 1,
            panic: "abort".into(),
            source_date_epoch: None,
        }
    }
}

impl ReproBuildConfig {
    /// Check if this config meets reproducibility requirements.
    pub fn is_reproducible(&self) -> bool {
        self.locked && self.no_incremental && self.lto && self.codegen_units == 1
    }

    /// Generate cargo build arguments.
    pub fn cargo_args(&self) -> Vec<String> {
        let mut args = vec!["build".into(), "--release".into()];
        if self.locked {
            args.push("--locked".into());
        }
        args
    }

    /// Generate environment variables.
    pub fn env_vars(&self) -> Vec<(String, String)> {
        let mut vars = vec![];
        if self.no_incremental {
            vars.push(("CARGO_INCREMENTAL".into(), "0".into()));
        }
        if let Some(epoch) = self.source_date_epoch {
            vars.push(("SOURCE_DATE_EPOCH".into(), epoch.to_string()));
        }
        vars
    }
}

/// FJ-2403: MSRV (Minimum Supported Rust Version) enforcement.
///
/// # Examples
///
/// ```
/// use forjar::core::types::MsrvCheck;
///
/// let check = MsrvCheck::new("1.88.0");
/// assert!(check.satisfies("1.88.0"));
/// assert!(check.satisfies("1.89.0"));
/// assert!(!check.satisfies("1.87.0"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsrvCheck {
    /// Required minimum version (e.g., "1.88.0").
    pub required: String,
    /// Parsed version components.
    major: u32,
    minor: u32,
    patch: u32,
}

impl MsrvCheck {
    /// Create from a version string.
    pub fn new(version: &str) -> Self {
        let parts: Vec<u32> = version.split('.').filter_map(|p| p.parse().ok()).collect();
        Self {
            required: version.to_string(),
            major: parts.first().copied().unwrap_or(0),
            minor: parts.get(1).copied().unwrap_or(0),
            patch: parts.get(2).copied().unwrap_or(0),
        }
    }

    /// Check if a given version satisfies the MSRV.
    pub fn satisfies(&self, actual: &str) -> bool {
        let parts: Vec<u32> = actual.split('.').filter_map(|p| p.parse().ok()).collect();
        let (a_major, a_minor, a_patch) = (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        );
        (a_major, a_minor, a_patch) >= (self.major, self.minor, self.patch)
    }
}

/// FJ-2403: Feature flag matrix configuration for CI testing.
///
/// # Examples
///
/// ```
/// use forjar::core::types::FeatureMatrix;
///
/// let matrix = FeatureMatrix::new(vec!["encryption", "container-test"]);
/// assert_eq!(matrix.combinations().len(), 4); // 2^2 combinations
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureMatrix {
    /// Available feature flags.
    pub features: Vec<String>,
}

impl FeatureMatrix {
    /// Create from feature names.
    pub fn new(features: Vec<&str>) -> Self {
        Self {
            features: features.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Generate all 2^n feature combinations.
    pub fn combinations(&self) -> Vec<Vec<String>> {
        let n = self.features.len();
        let count = 1_usize << n;
        let mut result = Vec::with_capacity(count);
        for mask in 0..count {
            let mut combo = Vec::new();
            for (i, feature) in self.features.iter().enumerate() {
                if mask & (1 << i) != 0 {
                    combo.push(feature.clone());
                }
            }
            result.push(combo);
        }
        result
    }

    /// Generate cargo test commands for each combination.
    pub fn cargo_commands(&self) -> Vec<String> {
        self.combinations()
            .iter()
            .map(|combo| {
                if combo.is_empty() {
                    "cargo test --no-default-features".into()
                } else {
                    format!(
                        "cargo test --no-default-features --features {}",
                        combo.join(","),
                    )
                }
            })
            .collect()
    }
}

/// FJ-2400: Purification benchmark result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurificationBenchmark {
    /// Resource type.
    pub resource_type: String,
    /// Validation time in microseconds.
    pub validate_us: f64,
    /// Purification time in microseconds.
    pub purify_us: f64,
    /// Number of scripts benchmarked.
    pub sample_count: u32,
}

impl PurificationBenchmark {
    /// Purification overhead as a ratio (purify / validate).
    pub fn overhead_ratio(&self) -> f64 {
        if self.validate_us == 0.0 {
            return 0.0;
        }
        self.purify_us / self.validate_us
    }
}

impl fmt::Display for PurificationBenchmark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: validate={:.0}us purify={:.0}us ratio={:.1}x (n={})",
            self.resource_type,
            self.validate_us,
            self.purify_us,
            self.overhead_ratio(),
            self.sample_count,
        )
    }
}

/// FJ-2401: Model integrity check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelIntegrityCheck {
    /// Model name.
    pub model_name: String,
    /// Expected BLAKE3 hash.
    pub expected_hash: String,
    /// Actual BLAKE3 hash after download.
    pub actual_hash: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Whether the hashes match.
    pub valid: bool,
}

impl ModelIntegrityCheck {
    /// Create from expected and actual hashes.
    pub fn check(model: &str, expected: &str, actual: &str, size: u64) -> Self {
        Self {
            model_name: model.to_string(),
            expected_hash: expected.to_string(),
            actual_hash: actual.to_string(),
            size_bytes: size,
            valid: expected == actual,
        }
    }

    /// Size in megabytes.
    pub fn size_mb(&self) -> f64 {
        self.size_bytes as f64 / (1024.0 * 1024.0)
    }
}

impl fmt::Display for ModelIntegrityCheck {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.valid { "OK" } else { "MISMATCH" };
        write!(
            f,
            "[{status}] {} ({:.1} MB) blake3:{}",
            self.model_name,
            self.size_mb(),
            &self.actual_hash[..8.min(self.actual_hash.len())],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repro_build_config_default() {
        let c = ReproBuildConfig::default();
        assert!(c.locked);
        assert!(c.no_incremental);
        assert!(c.lto);
        assert_eq!(c.codegen_units, 1);
        assert!(c.is_reproducible());
    }

    #[test]
    fn repro_build_config_not_reproducible() {
        let c = ReproBuildConfig {
            locked: false,
            ..Default::default()
        };
        assert!(!c.is_reproducible());
    }

    #[test]
    fn repro_build_config_cargo_args() {
        let c = ReproBuildConfig::default();
        let args = c.cargo_args();
        assert!(args.contains(&"--locked".to_string()));
        assert!(args.contains(&"--release".to_string()));
    }

    #[test]
    fn repro_build_config_env_vars() {
        let c = ReproBuildConfig {
            source_date_epoch: Some(1234567890),
            ..Default::default()
        };
        let vars = c.env_vars();
        assert!(vars.iter().any(|(k, _)| k == "CARGO_INCREMENTAL"));
        assert!(vars
            .iter()
            .any(|(k, v)| k == "SOURCE_DATE_EPOCH" && v == "1234567890"));
    }

    #[test]
    fn msrv_check_satisfies() {
        let m = MsrvCheck::new("1.88.0");
        assert!(m.satisfies("1.88.0"));
        assert!(m.satisfies("1.89.0"));
        assert!(m.satisfies("2.0.0"));
        assert!(!m.satisfies("1.87.0"));
        assert!(!m.satisfies("1.87.9"));
    }

    #[test]
    fn msrv_check_patch() {
        let m = MsrvCheck::new("1.88.1");
        assert!(!m.satisfies("1.88.0"));
        assert!(m.satisfies("1.88.1"));
        assert!(m.satisfies("1.88.2"));
    }

    #[test]
    fn feature_matrix_combinations() {
        let m = FeatureMatrix::new(vec!["a", "b"]);
        let combos = m.combinations();
        assert_eq!(combos.len(), 4);
        assert!(combos.contains(&vec![]));
        assert!(combos.contains(&vec!["a".to_string()]));
        assert!(combos.contains(&vec!["b".to_string()]));
        assert!(combos.contains(&vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn feature_matrix_single() {
        let m = FeatureMatrix::new(vec!["encryption"]);
        assert_eq!(m.combinations().len(), 2);
    }

    #[test]
    fn feature_matrix_empty() {
        let m = FeatureMatrix::new(vec![]);
        assert_eq!(m.combinations().len(), 1);
        assert_eq!(m.combinations()[0], Vec::<String>::new());
    }

    #[test]
    fn feature_matrix_cargo_commands() {
        let m = FeatureMatrix::new(vec!["encryption"]);
        let cmds = m.cargo_commands();
        assert!(cmds
            .iter()
            .any(|c| c.contains("--no-default-features") && !c.contains("--features")));
        assert!(cmds.iter().any(|c| c.contains("--features encryption")));
    }

    #[test]
    fn purification_benchmark_display() {
        let b = PurificationBenchmark {
            resource_type: "file".into(),
            validate_us: 50.0,
            purify_us: 150.0,
            sample_count: 100,
        };
        assert!((b.overhead_ratio() - 3.0).abs() < 0.01);
        let s = b.to_string();
        assert!(s.contains("file"));
        assert!(s.contains("3.0x"));
    }

    #[test]
    fn model_integrity_check_valid() {
        let c = ModelIntegrityCheck::check("llama3", "abc", "abc", 1024 * 1024);
        assert!(c.valid);
        assert!((c.size_mb() - 1.0).abs() < 0.01);
    }

    #[test]
    fn model_integrity_check_invalid() {
        let c = ModelIntegrityCheck::check("llama3", "abc", "def", 5_000_000_000);
        assert!(!c.valid);
        let s = c.to_string();
        assert!(s.contains("[MISMATCH]"));
    }

    #[test]
    fn model_integrity_display() {
        let c = ModelIntegrityCheck::check("gpt2", "abcdef01", "abcdef01", 500_000_000);
        let s = c.to_string();
        assert!(s.contains("[OK]"));
        assert!(s.contains("gpt2"));
        assert!(s.contains("476.8 MB"));
    }

    #[test]
    fn repro_build_config_serde() {
        let c = ReproBuildConfig::default();
        let json = serde_json::to_string(&c).unwrap();
        let parsed: ReproBuildConfig = serde_json::from_str(&json).unwrap();
        assert!(parsed.is_reproducible());
    }
}

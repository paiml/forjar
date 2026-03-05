//! FJ-2402: WASM deployment types — build config, size budgets, deploy targets.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2402: WASM optimization level.
///
/// # Examples
///
/// ```
/// use forjar::core::types::WasmOptLevel;
///
/// let level = WasmOptLevel::MinSize;
/// assert_eq!(level.flag(), "-Oz");
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WasmOptLevel {
    /// `-O1` — fast compile, minimal optimization.
    Fast,
    /// `-O2` — balanced (CI default).
    Balanced,
    /// `-O3` — max speed for compute-heavy apps.
    MaxSpeed,
    /// `-Oz` — minimum size (production default).
    #[default]
    MinSize,
}

impl WasmOptLevel {
    /// wasm-opt flag for this level.
    pub fn flag(self) -> &'static str {
        match self {
            Self::Fast => "-O1",
            Self::Balanced => "-O2",
            Self::MaxSpeed => "-O3",
            Self::MinSize => "-Oz",
        }
    }
}

impl fmt::Display for WasmOptLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.flag())
    }
}

/// FJ-2402: WASM build configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmBuildConfig {
    /// Crate path to compile.
    pub crate_path: String,
    /// wasm-pack target (`web`, `bundler`, `nodejs`).
    #[serde(default = "default_wasm_target")]
    pub target: String,
    /// Optimization level.
    #[serde(default)]
    pub opt_level: WasmOptLevel,
    /// Output directory.
    #[serde(default = "default_dist_dir")]
    pub dist_dir: String,
    /// Whether to run wasm-opt after wasm-pack.
    #[serde(default = "crate::core::types::default_true")]
    pub optimize: bool,
}

fn default_wasm_target() -> String {
    "web".into()
}

fn default_dist_dir() -> String {
    "dist".into()
}

/// FJ-2402: WASM size budget for binary size regression detection.
///
/// # Examples
///
/// ```
/// use forjar::core::types::WasmSizeBudget;
///
/// let budget = WasmSizeBudget {
///     core_kb: 100,
///     widgets_kb: 150,
///     full_app_kb: 500,
/// };
/// assert!(budget.check_core(95_000)); // 95 KB < 100 KB budget
/// assert!(!budget.check_core(120_000)); // 120 KB > 100 KB budget
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmSizeBudget {
    /// Max core WASM size in KB.
    pub core_kb: u64,
    /// Max widget WASM size in KB.
    pub widgets_kb: u64,
    /// Max full app WASM size in KB.
    pub full_app_kb: u64,
}

impl Default for WasmSizeBudget {
    fn default() -> Self {
        Self {
            core_kb: 100,
            widgets_kb: 150,
            full_app_kb: 500,
        }
    }
}

impl WasmSizeBudget {
    /// Check if a core WASM binary is within budget (bytes).
    pub fn check_core(&self, size_bytes: u64) -> bool {
        size_bytes <= self.core_kb * 1024
    }

    /// Check if a full app WASM binary is within budget (bytes).
    pub fn check_full_app(&self, size_bytes: u64) -> bool {
        size_bytes <= self.full_app_kb * 1024
    }
}

/// FJ-2402: CDN deploy target.
///
/// # Examples
///
/// ```
/// use forjar::core::types::CdnTarget;
///
/// let target = CdnTarget::S3 {
///     bucket: "my-bucket".into(),
///     region: Some("us-east-1".into()),
///     distribution: Some("E1234567".into()),
/// };
/// assert_eq!(target.name(), "s3");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CdnTarget {
    /// AWS S3 + CloudFront.
    S3 {
        bucket: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        region: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        distribution: Option<String>,
    },
    /// Cloudflare Pages.
    Cloudflare { project: String },
    /// Local directory copy.
    Local { path: String },
}

impl CdnTarget {
    /// Short name for the target type.
    pub fn name(&self) -> &'static str {
        match self {
            Self::S3 { .. } => "s3",
            Self::Cloudflare { .. } => "cloudflare",
            Self::Local { .. } => "local",
        }
    }
}

impl fmt::Display for CdnTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::S3 { bucket, .. } => write!(f, "s3://{bucket}"),
            Self::Cloudflare { project } => write!(f, "cloudflare:{project}"),
            Self::Local { path } => write!(f, "local:{path}"),
        }
    }
}

/// FJ-2402: Cache-Control headers per file extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePolicy {
    /// File extension (e.g., ".wasm", ".html").
    pub extension: String,
    /// Cache-Control header value.
    pub cache_control: String,
    /// TTL description (human-readable).
    pub ttl: String,
}

impl CachePolicy {
    /// Default cache policies for WASM deployments.
    pub fn defaults() -> Vec<Self> {
        vec![
            Self {
                extension: ".wasm".into(),
                cache_control: "public, max-age=31536000, immutable".into(),
                ttl: "1 year".into(),
            },
            Self {
                extension: ".js".into(),
                cache_control: "public, max-age=31536000, immutable".into(),
                ttl: "1 year".into(),
            },
            Self {
                extension: ".css".into(),
                cache_control: "public, max-age=604800".into(),
                ttl: "7 days".into(),
            },
            Self {
                extension: ".html".into(),
                cache_control: "no-cache, must-revalidate".into(),
                ttl: "0 (always fresh)".into(),
            },
        ]
    }
}

/// FJ-2402: WASM build result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmBuildResult {
    /// Output WASM file path.
    pub wasm_path: String,
    /// WASM binary size in bytes (after optimization).
    pub wasm_size: u64,
    /// JS glue file size in bytes.
    pub js_size: u64,
    /// Total bundle size in bytes.
    pub total_size: u64,
    /// Build duration in seconds.
    pub duration_secs: f64,
    /// Optimization level used.
    pub opt_level: WasmOptLevel,
}

impl WasmBuildResult {
    /// WASM size in KB (rounded).
    pub fn wasm_kb(&self) -> u64 {
        self.wasm_size / 1024
    }

    /// Total bundle size in KB (rounded).
    pub fn total_kb(&self) -> u64 {
        self.total_size / 1024
    }
}

impl fmt::Display for WasmBuildResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "WASM: {} KB, JS: {} KB, Total: {} KB ({} {:.1}s)",
            self.wasm_kb(),
            self.js_size / 1024,
            self.total_kb(),
            self.opt_level,
            self.duration_secs,
        )
    }
}

/// FJ-2402: Bundle size drift detection.
///
/// Compares current build size against budget and previous build to detect
/// regressions. Used to alert when WASM exceeds budget or grows too fast.
///
/// # Examples
///
/// ```
/// use forjar::core::types::{WasmSizeBudget, BundleSizeDrift};
///
/// let budget = WasmSizeBudget::default();
/// let drift = BundleSizeDrift::check(&budget, 90 * 1024, Some(85 * 1024));
/// assert!(drift.is_ok());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleSizeDrift {
    /// Current build size in bytes.
    pub current_bytes: u64,
    /// Previous build size in bytes (if available).
    pub previous_bytes: Option<u64>,
    /// Budget limit in bytes.
    pub budget_bytes: u64,
    /// Whether current size exceeds budget.
    pub exceeds_budget: bool,
    /// Whether growth from previous build exceeds 20%.
    pub exceeds_growth_limit: bool,
}

impl BundleSizeDrift {
    /// Check bundle size against budget and previous build.
    pub fn check(budget: &WasmSizeBudget, current_bytes: u64, previous_bytes: Option<u64>) -> Self {
        let budget_bytes = budget.core_kb * 1024;
        let exceeds_budget = current_bytes > budget_bytes;
        let exceeds_growth_limit = previous_bytes
            .map(|prev| prev > 0 && current_bytes > prev + prev / 5) // >20% growth
            .unwrap_or(false);
        Self {
            current_bytes,
            previous_bytes,
            budget_bytes,
            exceeds_budget,
            exceeds_growth_limit,
        }
    }

    /// Whether the bundle is within all limits.
    pub fn is_ok(&self) -> bool {
        !self.exceeds_budget && !self.exceeds_growth_limit
    }

    /// Current size in KB.
    pub fn current_kb(&self) -> u64 {
        self.current_bytes / 1024
    }
}

impl fmt::Display for BundleSizeDrift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (cur, bud) = (self.current_kb(), self.budget_bytes / 1024);
        if self.exceeds_budget {
            write!(f, "BUDGET EXCEEDED: {cur} KB > {bud} KB budget")?;
        } else {
            write!(f, "OK: {cur} KB / {bud} KB budget")?;
        }
        if let Some(prev) = self.previous_bytes {
            let p = prev / 1024;
            let pct = if prev > 0 { ((cur as f64 - p as f64) / p as f64 * 100.0) as i32 } else { 0 };
            write!(f, " (prev: {p} KB, {pct:+}%)")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_opt_level_flags() {
        assert_eq!(WasmOptLevel::Fast.flag(), "-O1");
        assert_eq!(WasmOptLevel::Balanced.flag(), "-O2");
        assert_eq!(WasmOptLevel::MaxSpeed.flag(), "-O3");
        assert_eq!(WasmOptLevel::MinSize.flag(), "-Oz");
    }

    #[test]
    fn wasm_opt_level_default() {
        assert_eq!(WasmOptLevel::default(), WasmOptLevel::MinSize);
    }

    #[test]
    fn wasm_opt_level_serde_roundtrip() {
        for level in [
            WasmOptLevel::Fast,
            WasmOptLevel::Balanced,
            WasmOptLevel::MaxSpeed,
            WasmOptLevel::MinSize,
        ] {
            let json = serde_json::to_string(&level).unwrap();
            let parsed: WasmOptLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, parsed);
        }
    }

    #[test]
    fn wasm_size_budget_defaults() {
        let budget = WasmSizeBudget::default();
        assert_eq!(budget.core_kb, 100);
        assert_eq!(budget.full_app_kb, 500);
    }

    #[test]
    fn wasm_size_budget_check() {
        let budget = WasmSizeBudget::default();
        assert!(budget.check_core(50 * 1024));
        assert!(budget.check_core(100 * 1024));
        assert!(!budget.check_core(101 * 1024));
        assert!(budget.check_full_app(500 * 1024));
        assert!(!budget.check_full_app(501 * 1024));
    }

    #[test]
    fn cdn_target_s3_display() {
        let t = CdnTarget::S3 {
            bucket: "my-bucket".into(),
            region: Some("us-east-1".into()),
            distribution: None,
        };
        assert_eq!(t.name(), "s3");
        assert_eq!(t.to_string(), "s3://my-bucket");
    }

    #[test]
    fn cdn_target_cloudflare_display() {
        let t = CdnTarget::Cloudflare {
            project: "my-app".into(),
        };
        assert_eq!(t.name(), "cloudflare");
        assert_eq!(t.to_string(), "cloudflare:my-app");
    }

    #[test]
    fn cdn_target_local_display() {
        let t = CdnTarget::Local {
            path: "/var/www".into(),
        };
        assert_eq!(t.name(), "local");
        assert_eq!(t.to_string(), "local:/var/www");
    }

    #[test]
    fn cdn_target_serde_roundtrip() {
        let t = CdnTarget::S3 {
            bucket: "b".into(),
            region: Some("r".into()),
            distribution: Some("d".into()),
        };
        let json = serde_json::to_string(&t).unwrap();
        let parsed: CdnTarget = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name(), "s3");
    }

    #[test]
    fn cache_policy_defaults() {
        let policies = CachePolicy::defaults();
        assert_eq!(policies.len(), 4);
        assert_eq!(policies[0].extension, ".wasm");
        assert!(policies[0].cache_control.contains("immutable"));
        assert_eq!(policies[3].extension, ".html");
        assert!(policies[3].cache_control.contains("no-cache"));
    }

    #[test]
    fn wasm_build_result_display() {
        let result = WasmBuildResult {
            wasm_path: "dist/app.wasm".into(),
            wasm_size: 500 * 1024,
            js_size: 20 * 1024,
            total_size: 520 * 1024,
            duration_secs: 12.5,
            opt_level: WasmOptLevel::MinSize,
        };
        assert_eq!(result.wasm_kb(), 500);
        assert_eq!(result.total_kb(), 520);
        let s = result.to_string();
        assert!(s.contains("500 KB"));
        assert!(s.contains("-Oz"));
    }

    #[test]
    fn wasm_build_config_serde() {
        let yaml = r#"
crate_path: crates/presentar
target: web
opt_level: minsize
dist_dir: dist
optimize: true
"#;
        let config: WasmBuildConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.crate_path, "crates/presentar");
        assert_eq!(config.opt_level, WasmOptLevel::MinSize);
        assert!(config.optimize);
    }

    #[test]
    fn bundle_drift_scenarios() {
        let budget = WasmSizeBudget::default();
        // Within budget
        let d = BundleSizeDrift::check(&budget, 90 * 1024, Some(85 * 1024));
        assert!(d.is_ok());
        // Exceeds budget
        let d = BundleSizeDrift::check(&budget, 110 * 1024, Some(90 * 1024));
        assert!(d.exceeds_budget && !d.is_ok());
        assert!(d.to_string().contains("BUDGET EXCEEDED"));
        // Growth >20% (within budget)
        let big = WasmSizeBudget { core_kb: 200, ..Default::default() };
        let d = BundleSizeDrift::check(&big, 130 * 1024, Some(100 * 1024));
        assert!(!d.exceeds_budget && d.exceeds_growth_limit && !d.is_ok());
        // No previous
        let d = BundleSizeDrift::check(&budget, 90 * 1024, None);
        assert!(d.is_ok());
    }
}

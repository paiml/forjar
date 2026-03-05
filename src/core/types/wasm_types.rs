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
}

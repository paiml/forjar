//! FJ-2105: Distribution types — load, push, FAR export, multi-arch.
//!
//! Types for `forjar build --load/--push/--far`: registry push state,
//! multi-arch image index, and build output reporting.

use serde::{Deserialize, Serialize};

/// FJ-2105: Distribution target for built images.
///
/// # Examples
///
/// ```
/// use forjar::core::types::DistTarget;
///
/// let target = DistTarget::Push {
///     registry: "myregistry.io".into(),
///     name: "myapp".into(),
///     tag: "1.0.0".into(),
/// };
/// assert!(matches!(target, DistTarget::Push { .. }));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DistTarget {
    /// Load into local Docker/Podman daemon.
    Load {
        /// Container runtime (docker or podman).
        runtime: String,
    },
    /// Push to OCI registry.
    Push {
        /// Registry hostname.
        registry: String,
        /// Image name (without tag).
        name: String,
        /// Image tag.
        tag: String,
    },
    /// Export as FAR archive.
    Far {
        /// Output file path.
        output_path: String,
    },
}

impl DistTarget {
    /// Human-readable description of the target.
    pub fn description(&self) -> String {
        match self {
            Self::Load { runtime } => format!("{runtime} load"),
            Self::Push { registry, name, tag } => format!("{registry}/{name}:{tag}"),
            Self::Far { output_path } => format!("FAR → {output_path}"),
        }
    }
}

/// FJ-2105: Registry push result for a single blob or manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushResult {
    /// What was pushed (layer, config, manifest).
    pub kind: PushKind,
    /// Digest of the pushed content.
    pub digest: String,
    /// Size in bytes.
    pub size: u64,
    /// Whether the blob already existed (skip upload).
    pub existed: bool,
    /// Upload duration in seconds (0 if existed).
    pub duration_secs: f64,
}

/// Kind of content pushed to registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PushKind {
    /// Layer blob.
    Layer,
    /// Config blob.
    Config,
    /// Manifest.
    Manifest,
    /// Image index (multi-arch).
    Index,
}

/// FJ-2105: Multi-arch build matrix entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchBuild {
    /// Platform string (e.g., "linux/amd64").
    pub platform: String,
    /// OS.
    pub os: String,
    /// Architecture.
    pub architecture: String,
    /// Optional variant (e.g., "v8" for arm64).
    #[serde(default)]
    pub variant: Option<String>,
    /// Manifest digest for this platform.
    #[serde(default)]
    pub manifest_digest: Option<String>,
    /// Build duration in seconds.
    #[serde(default)]
    pub duration_secs: Option<f64>,
}

impl ArchBuild {
    /// Create a linux/amd64 entry.
    pub fn linux_amd64() -> Self {
        Self {
            platform: "linux/amd64".into(),
            os: "linux".into(),
            architecture: "amd64".into(),
            variant: None,
            manifest_digest: None,
            duration_secs: None,
        }
    }

    /// Create a linux/arm64 entry.
    pub fn linux_arm64() -> Self {
        Self {
            platform: "linux/arm64".into(),
            os: "linux".into(),
            architecture: "arm64".into(),
            variant: Some("v8".into()),
            manifest_digest: None,
            duration_secs: None,
        }
    }
}

/// FJ-2105: Complete build output report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildReport {
    /// Image name:tag.
    pub image_ref: String,
    /// Final image digest.
    pub digest: String,
    /// Total image size (compressed).
    pub total_size: u64,
    /// Number of layers.
    pub layer_count: u32,
    /// Total build duration in seconds.
    pub duration_secs: f64,
    /// Per-layer build details.
    pub layers: Vec<LayerReport>,
    /// Distribution results (if --load/--push/--far).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub distribution: Vec<DistResult>,
    /// Multi-arch builds (if --platform).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub architectures: Vec<ArchBuild>,
}

impl BuildReport {
    /// Total size in MB.
    pub fn size_mb(&self) -> f64 {
        self.total_size as f64 / (1024.0 * 1024.0)
    }

    /// Format a compact build summary.
    pub fn format_summary(&self) -> String {
        let mut out = format!(
            "Image: {} ({:.1} MB, {} layers)\n",
            self.image_ref,
            self.size_mb(),
            self.layer_count,
        );
        out.push_str(&format!("Digest: {}\n", self.digest));
        out.push_str(&format!("Built in {:.1}s\n", self.duration_secs));
        for layer in &self.layers {
            let status = if layer.cached { "cached" } else { "new" };
            out.push_str(&format!(
                "  Layer {}: {} ({status}, {:.1}s)\n",
                layer.index, layer.name, layer.duration_secs,
            ));
        }
        for dist in &self.distribution {
            out.push_str(&format!(
                "  Distributed: {} ({:.1}s)\n",
                dist.target, dist.duration_secs,
            ));
        }
        out
    }
}

/// Per-layer build report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerReport {
    /// Layer index (0-based).
    pub index: u32,
    /// Layer name from config.
    pub name: String,
    /// BLAKE3 store hash.
    pub store_hash: String,
    /// Compressed size in bytes.
    pub size: u64,
    /// Whether this layer was served from cache.
    pub cached: bool,
    /// Build duration in seconds.
    pub duration_secs: f64,
}

/// Distribution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistResult {
    /// Target description.
    pub target: String,
    /// Whether distribution succeeded.
    pub success: bool,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Error message if failed.
    #[serde(default)]
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dist_target_load_description() {
        let t = DistTarget::Load { runtime: "docker".into() };
        assert_eq!(t.description(), "docker load");
    }

    #[test]
    fn dist_target_push_description() {
        let t = DistTarget::Push {
            registry: "ghcr.io".into(),
            name: "myorg/myapp".into(),
            tag: "v1".into(),
        };
        assert_eq!(t.description(), "ghcr.io/myorg/myapp:v1");
    }

    #[test]
    fn dist_target_far_description() {
        let t = DistTarget::Far { output_path: "/tmp/out.far".into() };
        assert_eq!(t.description(), "FAR → /tmp/out.far");
    }

    #[test]
    fn arch_build_linux_amd64() {
        let a = ArchBuild::linux_amd64();
        assert_eq!(a.platform, "linux/amd64");
        assert_eq!(a.os, "linux");
        assert!(a.variant.is_none());
    }

    #[test]
    fn arch_build_linux_arm64() {
        let a = ArchBuild::linux_arm64();
        assert_eq!(a.platform, "linux/arm64");
        assert_eq!(a.variant.as_deref(), Some("v8"));
    }

    #[test]
    fn build_report_size_mb() {
        let report = sample_report();
        assert!((report.size_mb() - 47.7).abs() < 0.1);
    }

    #[test]
    fn build_report_format_summary() {
        let report = sample_report();
        let s = report.format_summary();
        assert!(s.contains("myregistry.io/app:1.0"));
        assert!(s.contains("47.7 MB"));
        assert!(s.contains("3 layers"));
        assert!(s.contains("cached"));
        assert!(s.contains("new"));
    }

    #[test]
    fn build_report_serde_roundtrip() {
        let report = sample_report();
        let json = serde_json::to_string(&report).unwrap();
        let parsed: BuildReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.layer_count, 3);
        assert_eq!(parsed.layers.len(), 3);
    }

    #[test]
    fn dist_target_serde_roundtrip() {
        let t = DistTarget::Push {
            registry: "r".into(),
            name: "n".into(),
            tag: "t".into(),
        };
        let json = serde_json::to_string(&t).unwrap();
        let parsed: DistTarget = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, DistTarget::Push { .. }));
    }

    #[test]
    fn push_result_existed() {
        let r = PushResult {
            kind: PushKind::Layer,
            digest: "sha256:abc".into(),
            size: 1000,
            existed: true,
            duration_secs: 0.0,
        };
        assert!(r.existed);
        assert_eq!(r.kind, PushKind::Layer);
    }

    #[test]
    fn push_result_new_upload() {
        let r = PushResult {
            kind: PushKind::Manifest,
            digest: "sha256:xyz".into(),
            size: 512,
            existed: false,
            duration_secs: 0.5,
        };
        assert!(!r.existed);
        assert_eq!(r.kind, PushKind::Manifest);
    }

    fn sample_report() -> BuildReport {
        BuildReport {
            image_ref: "myregistry.io/app:1.0".into(),
            digest: "sha256:abc123".into(),
            total_size: 50_000_000,
            layer_count: 3,
            duration_secs: 48.5,
            layers: vec![
                LayerReport {
                    index: 0,
                    name: "system-packages".into(),
                    store_hash: "blake3:aaa".into(),
                    size: 45_000_000,
                    cached: true,
                    duration_secs: 0.2,
                },
                LayerReport {
                    index: 1,
                    name: "ml-deps".into(),
                    store_hash: "blake3:bbb".into(),
                    size: 4_900_000,
                    cached: false,
                    duration_secs: 47.3,
                },
                LayerReport {
                    index: 2,
                    name: "app-code".into(),
                    store_hash: "blake3:ccc".into(),
                    size: 100_000,
                    cached: false,
                    duration_secs: 0.01,
                },
            ],
            distribution: vec![],
            architectures: vec![],
        }
    }
}

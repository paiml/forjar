//! FJ-2101: Container build types — OCI layer building, deterministic tar, dual digest.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2101: OCI layer building configuration.
///
/// # Examples
///
/// ```
/// use forjar::core::types::{OciLayerConfig, OciCompression};
///
/// let config = OciLayerConfig::default();
/// assert_eq!(config.compression, OciCompression::Gzip);
/// assert!(config.deterministic);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLayerConfig {
    /// Compression algorithm for layer tarballs.
    #[serde(default)]
    pub compression: OciCompression,
    /// Whether to produce deterministic tarballs.
    #[serde(default = "default_true_container")]
    pub deterministic: bool,
    /// Epoch mtime for deterministic builds (default: 0 = Unix epoch).
    #[serde(default)]
    pub epoch_mtime: u64,
    /// File sorting strategy for deterministic tar.
    #[serde(default)]
    pub sort_order: TarSortOrder,
}

impl Default for OciLayerConfig {
    fn default() -> Self {
        Self {
            compression: OciCompression::Gzip,
            deterministic: true,
            epoch_mtime: 0,
            sort_order: TarSortOrder::Lexicographic,
        }
    }
}

fn default_true_container() -> bool {
    true
}

/// Compression algorithm for OCI layer tarballs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OciCompression {
    /// No compression.
    None,
    /// gzip compression (default, widest compatibility).
    #[default]
    Gzip,
    /// zstd compression (faster, better ratio).
    Zstd,
}

impl fmt::Display for OciCompression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Gzip => write!(f, "gzip"),
            Self::Zstd => write!(f, "zstd"),
        }
    }
}

/// File sorting strategy for deterministic tar archives.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TarSortOrder {
    /// Lexicographic by full path (default).
    #[default]
    Lexicographic,
    /// Directory-first, then lexicographic within each directory.
    DirectoryFirst,
}

/// FJ-2101: Dual digest result — BLAKE3 + SHA-256 computed in a single pass.
///
/// OCI registries require SHA-256 digests, but forjar uses BLAKE3 internally.
/// Computing both in one pass avoids re-reading the data.
///
/// # Examples
///
/// ```
/// use forjar::core::types::DualDigest;
///
/// let d = DualDigest {
///     blake3: "abc123".into(),
///     sha256: "def456".into(),
///     size_bytes: 1024,
/// };
/// assert_eq!(d.oci_digest(), "sha256:def456");
/// assert_eq!(d.forjar_digest(), "blake3:abc123");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualDigest {
    /// BLAKE3 hex digest (forjar internal).
    pub blake3: String,
    /// SHA-256 hex digest (OCI standard).
    pub sha256: String,
    /// Content size in bytes.
    pub size_bytes: u64,
}

impl DualDigest {
    /// OCI-format digest string (`sha256:<hex>`).
    pub fn oci_digest(&self) -> String {
        format!("sha256:{}", self.sha256)
    }

    /// Forjar-format digest string (`blake3:<hex>`).
    pub fn forjar_digest(&self) -> String {
        format!("blake3:{}", self.blake3)
    }
}

impl fmt::Display for DualDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "blake3:{} sha256:{} ({}B)",
            &self.blake3[..8.min(self.blake3.len())],
            &self.sha256[..8.min(self.sha256.len())],
            self.size_bytes,
        )
    }
}

/// FJ-2101: Layer cache entry in the BLAKE3 content-addressed store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerCacheEntry {
    /// BLAKE3 hash of the layer content.
    pub content_hash: String,
    /// SHA-256 hash for OCI compatibility.
    pub oci_digest: String,
    /// Compressed layer size.
    pub compressed_size: u64,
    /// Uncompressed layer size.
    pub uncompressed_size: u64,
    /// Compression algorithm used.
    pub compression: OciCompression,
    /// Path in the store (relative to store root).
    pub store_path: String,
}

/// FJ-2104: Image build plan — multi-layer build strategy.
///
/// # Examples
///
/// ```
/// use forjar::core::types::{ImageBuildPlan, LayerStrategy};
///
/// let plan = ImageBuildPlan {
///     tag: "myapp:latest".into(),
///     base_image: Some("ubuntu:22.04".into()),
///     layers: vec![
///         LayerStrategy::Packages { names: vec!["nginx".into(), "curl".into()] },
///         LayerStrategy::Files { paths: vec!["/etc/nginx/nginx.conf".into()] },
///     ],
///     labels: vec![("maintainer".into(), "team@example.com".into())],
///     entrypoint: None,
/// };
/// assert_eq!(plan.layer_count(), 2);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageBuildPlan {
    /// Image tag (name:tag).
    pub tag: String,
    /// Base image reference (None for scratch).
    #[serde(default)]
    pub base_image: Option<String>,
    /// Ordered layer strategies.
    pub layers: Vec<LayerStrategy>,
    /// Image labels (key-value pairs).
    #[serde(default)]
    pub labels: Vec<(String, String)>,
    /// Entrypoint command.
    #[serde(default)]
    pub entrypoint: Option<Vec<String>>,
}

impl ImageBuildPlan {
    /// Number of layers in this build plan.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Whether this image builds from scratch (no base).
    pub fn is_scratch(&self) -> bool {
        self.base_image.is_none()
    }
}

/// FJ-2104: Layer strategy — how to build each layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum LayerStrategy {
    /// Package layer: install packages via package manager.
    Packages {
        /// Package names to install.
        names: Vec<String>,
    },
    /// Files layer: copy specific files into the image.
    Files {
        /// File paths to include.
        paths: Vec<String>,
    },
    /// Build layer: run a build command, capture overlay diff.
    Build {
        /// Build command to execute.
        command: String,
        /// Working directory inside the container.
        #[serde(default)]
        workdir: Option<String>,
    },
    /// Derivation layer: forjar store derivation output.
    Derivation {
        /// Store path of the derivation.
        store_path: String,
    },
}

/// FJ-2105: Base image reference with resolution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseImageRef {
    /// Image reference (e.g., "ubuntu:22.04", "docker.io/library/nginx:1.25").
    pub reference: String,
    /// Resolved manifest digest (after pull).
    #[serde(default)]
    pub manifest_digest: Option<String>,
    /// Platform (e.g., "linux/amd64").
    #[serde(default)]
    pub platform: Option<String>,
    /// Whether the image has been pulled and stored locally.
    #[serde(default)]
    pub resolved: bool,
}

impl BaseImageRef {
    /// Create a new unresolved base image reference.
    pub fn new(reference: &str) -> Self {
        Self {
            reference: reference.to_string(),
            manifest_digest: None,
            platform: None,
            resolved: false,
        }
    }

    /// Parse registry and repository from the reference.
    pub fn registry(&self) -> &str {
        if let Some(slash) = self.reference.find('/') {
            let maybe_registry = &self.reference[..slash];
            if maybe_registry.contains('.') || maybe_registry.contains(':') {
                return maybe_registry;
            }
        }
        "docker.io"
    }
}

/// FJ-2105: OCI image build result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciBuildResult {
    /// Image tag.
    pub tag: String,
    /// Manifest digest (sha256).
    pub manifest_digest: String,
    /// Number of layers.
    pub layer_count: usize,
    /// Total compressed image size.
    pub total_size: u64,
    /// Build duration in seconds.
    pub duration_secs: f64,
    /// Path to the OCI layout directory.
    pub layout_path: String,
}

impl OciBuildResult {
    /// Image size in megabytes.
    pub fn size_mb(&self) -> f64 {
        self.total_size as f64 / (1024.0 * 1024.0)
    }
}

impl fmt::Display for OciBuildResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({} layers, {:.1} MB, {:.1}s)",
            self.tag,
            self.layer_count,
            self.size_mb(),
            self.duration_secs,
        )
    }
}

/// FJ-2106: Overlay-to-OCI whiteout conversion mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WhiteoutEntry {
    /// File deletion: `.wh.<name>` in OCI layer.
    FileDelete { path: String },
    /// Opaque directory: `.wh..wh..opq` marker.
    OpaqueDir { path: String },
}

impl WhiteoutEntry {
    /// Convert to OCI whiteout filename.
    pub fn oci_path(&self) -> String {
        match self {
            Self::FileDelete { path } => {
                if let Some(pos) = path.rfind('/') {
                    format!("{}/.wh.{}", &path[..pos], &path[pos + 1..])
                } else {
                    format!(".wh.{path}")
                }
            }
            Self::OpaqueDir { path } => {
                format!("{path}/.wh..wh..opq")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oci_layer_config_default() {
        let c = OciLayerConfig::default();
        assert_eq!(c.compression, OciCompression::Gzip);
        assert!(c.deterministic);
        assert_eq!(c.epoch_mtime, 0);
        assert_eq!(c.sort_order, TarSortOrder::Lexicographic);
    }

    #[test]
    fn oci_compression_display() {
        assert_eq!(OciCompression::None.to_string(), "none");
        assert_eq!(OciCompression::Gzip.to_string(), "gzip");
        assert_eq!(OciCompression::Zstd.to_string(), "zstd");
    }

    #[test]
    fn oci_compression_serde() {
        let c = OciCompression::Zstd;
        let json = serde_json::to_string(&c).unwrap();
        let parsed: OciCompression = serde_json::from_str(&json).unwrap();
        assert_eq!(c, parsed);
    }

    #[test]
    fn dual_digest_oci_format() {
        let d = DualDigest {
            blake3: "abcdef0123456789".into(),
            sha256: "deadbeef01234567".into(),
            size_bytes: 4096,
        };
        assert_eq!(d.oci_digest(), "sha256:deadbeef01234567");
        assert_eq!(d.forjar_digest(), "blake3:abcdef0123456789");
    }

    #[test]
    fn dual_digest_display() {
        let d = DualDigest {
            blake3: "abcdef0123456789".into(),
            sha256: "deadbeef01234567".into(),
            size_bytes: 4096,
        };
        let s = d.to_string();
        assert!(s.contains("blake3:abcdef01"));
        assert!(s.contains("sha256:deadbeef"));
        assert!(s.contains("4096B"));
    }

    #[test]
    fn layer_cache_entry_serde() {
        let e = LayerCacheEntry {
            content_hash: "blake3:abc".into(),
            oci_digest: "sha256:def".into(),
            compressed_size: 1024,
            uncompressed_size: 4096,
            compression: OciCompression::Gzip,
            store_path: "store/abc".into(),
        };
        let json = serde_json::to_string(&e).unwrap();
        let parsed: LayerCacheEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.compressed_size, 1024);
    }

    #[test]
    fn image_build_plan_layer_count() {
        let plan = ImageBuildPlan {
            tag: "app:v1".into(),
            base_image: Some("ubuntu:22.04".into()),
            layers: vec![
                LayerStrategy::Packages { names: vec!["nginx".into()] },
                LayerStrategy::Files { paths: vec!["/etc/app.conf".into()] },
            ],
            labels: vec![],
            entrypoint: None,
        };
        assert_eq!(plan.layer_count(), 2);
        assert!(!plan.is_scratch());
    }

    #[test]
    fn image_build_plan_scratch() {
        let plan = ImageBuildPlan {
            tag: "static:latest".into(),
            base_image: None,
            layers: vec![LayerStrategy::Files { paths: vec!["/app".into()] }],
            labels: vec![],
            entrypoint: Some(vec!["/app".into()]),
        };
        assert!(plan.is_scratch());
    }

    #[test]
    fn base_image_ref_registry() {
        let r = BaseImageRef::new("ubuntu:22.04");
        assert_eq!(r.registry(), "docker.io");
        assert!(!r.resolved);

        let r = BaseImageRef::new("ghcr.io/org/app:v1");
        assert_eq!(r.registry(), "ghcr.io");

        let r = BaseImageRef::new("localhost:5000/myimage");
        assert_eq!(r.registry(), "localhost:5000");
    }

    #[test]
    fn oci_build_result_display() {
        let r = OciBuildResult {
            tag: "app:v1".into(),
            manifest_digest: "sha256:abc".into(),
            layer_count: 3,
            total_size: 50 * 1024 * 1024,
            duration_secs: 12.5,
            layout_path: "out/oci".into(),
        };
        let s = r.to_string();
        assert!(s.contains("app:v1"));
        assert!(s.contains("3 layers"));
        assert!(s.contains("50.0 MB"));
    }

    #[test]
    fn whiteout_file_delete() {
        let w = WhiteoutEntry::FileDelete {
            path: "etc/nginx/old.conf".into(),
        };
        assert_eq!(w.oci_path(), "etc/nginx/.wh.old.conf");
    }

    #[test]
    fn whiteout_file_delete_root() {
        let w = WhiteoutEntry::FileDelete {
            path: "orphan.txt".into(),
        };
        assert_eq!(w.oci_path(), ".wh.orphan.txt");
    }

    #[test]
    fn whiteout_opaque_dir() {
        let w = WhiteoutEntry::OpaqueDir {
            path: "var/cache".into(),
        };
        assert_eq!(w.oci_path(), "var/cache/.wh..wh..opq");
    }

    #[test]
    fn layer_strategy_serde() {
        let pkg = LayerStrategy::Packages { names: vec!["nginx".into()] };
        let json = serde_json::to_string(&pkg).unwrap();
        assert!(json.contains("packages"));

        let build = LayerStrategy::Build {
            command: "make".into(),
            workdir: Some("/src".into()),
        };
        let json = serde_json::to_string(&build).unwrap();
        assert!(json.contains("build"));
    }
}

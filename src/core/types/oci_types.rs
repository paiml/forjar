//! FJ-2101: OCI image types — layout, manifest, config, layer.
//!
//! Types for daemonless OCI image construction from forjar resources.
//! Implements OCI Image Spec v1.1 and Docker compat manifest.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// FJ-2101: OCI image manifest (OCI Image Manifest v1.1).
///
/// # Examples
///
/// ```
/// use forjar::core::types::OciManifest;
///
/// let manifest = OciManifest::new("sha256:abc123".into(), vec![]);
/// assert_eq!(manifest.schema_version, 2);
/// assert_eq!(manifest.media_type, "application/vnd.oci.image.manifest.v1+json");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciManifest {
    /// Always 2 for OCI.
    pub schema_version: u32,
    /// Media type of this manifest.
    pub media_type: String,
    /// Reference to image config blob.
    pub config: OciDescriptor,
    /// Ordered list of layer descriptors (bottom to top).
    pub layers: Vec<OciDescriptor>,
    /// Optional annotations.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub annotations: HashMap<String, String>,
}

impl OciManifest {
    /// Create a new manifest with a config digest and layers.
    pub fn new(config_digest: String, layers: Vec<OciDescriptor>) -> Self {
        let layer_count = layers.len();
        let result = Self {
            schema_version: 2,
            media_type: "application/vnd.oci.image.manifest.v1+json".into(),
            config: OciDescriptor {
                media_type: "application/vnd.oci.image.config.v1+json".into(),
                digest: config_digest,
                size: 0,
                annotations: HashMap::new(),
            },
            layers,
            annotations: HashMap::new(),
        };
        debug_assert_eq!(result.schema_version, 2, "OCI manifest schema must be 2");
        debug_assert_eq!(
            result.layers.len(),
            layer_count,
            "layer count must be preserved"
        );
        // FJ-2200 G4: OCI manifest media type correctness
        debug_assert_eq!(
            result.media_type, "application/vnd.oci.image.manifest.v1+json",
            "OCI manifest media type must match spec"
        );
        debug_assert_eq!(
            result.config.media_type, "application/vnd.oci.image.config.v1+json",
            "OCI config media type must match spec"
        );
        result
    }

    /// Total compressed size of all layers.
    pub fn total_layer_size(&self) -> u64 {
        self.layers.iter().map(|l| l.size).sum()
    }
}

/// OCI content descriptor — references a blob by digest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciDescriptor {
    /// Media type of the referenced content.
    pub media_type: String,
    /// Digest in `algorithm:hex` format (e.g., `sha256:abc123...`).
    pub digest: String,
    /// Size of the referenced content in bytes.
    pub size: u64,
    /// Optional annotations.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub annotations: HashMap<String, String>,
}

impl OciDescriptor {
    /// Create a gzip-compressed layer descriptor.
    pub fn gzip_layer(digest: String, size: u64) -> Self {
        Self {
            media_type: "application/vnd.oci.image.layer.v1.tar+gzip".into(),
            digest,
            size,
            annotations: HashMap::new(),
        }
    }

    /// Create a zstd-compressed layer descriptor (OCI 1.1).
    pub fn zstd_layer(digest: String, size: u64) -> Self {
        Self {
            media_type: "application/vnd.oci.image.layer.v1.tar+zstd".into(),
            digest,
            size,
            annotations: HashMap::new(),
        }
    }
}

/// FJ-2101: OCI image configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciImageConfig {
    /// Architecture (e.g., amd64, arm64).
    pub architecture: String,
    /// OS (e.g., linux).
    pub os: String,
    /// Config section with runtime parameters.
    #[serde(default)]
    pub config: OciRuntimeConfig,
    /// Rootfs section with diff IDs.
    pub rootfs: OciRootfs,
    /// Build history entries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<OciHistoryEntry>,
}

impl OciImageConfig {
    /// Create a minimal Linux amd64 config.
    pub fn linux_amd64(diff_ids: Vec<String>) -> Self {
        let id_count = diff_ids.len();
        let result = Self {
            architecture: "amd64".into(),
            os: "linux".into(),
            config: OciRuntimeConfig::default(),
            rootfs: OciRootfs {
                rootfs_type: "layers".into(),
                diff_ids,
            },
            history: Vec::new(),
        };
        debug_assert_eq!(
            result.rootfs.rootfs_type, "layers",
            "rootfs type must be layers"
        );
        debug_assert_eq!(
            result.layer_count(),
            id_count,
            "diff_id count must be preserved"
        );
        result
    }

    /// Number of layers.
    pub fn layer_count(&self) -> usize {
        self.rootfs.diff_ids.len()
    }
}

/// OCI runtime configuration (entrypoint, env, etc.).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OciRuntimeConfig {
    /// Entrypoint command.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entrypoint: Vec<String>,
    /// Default command arguments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cmd: Vec<String>,
    /// Environment variables.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<String>,
    /// Working directory.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
    /// User to run as.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// Exposed ports.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub exposed_ports: HashMap<String, serde_json::Value>,
    /// Labels.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub labels: HashMap<String, String>,
}

/// OCI rootfs descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciRootfs {
    /// Always "layers".
    #[serde(rename = "type")]
    pub rootfs_type: String,
    /// SHA-256 DiffIDs of uncompressed layers.
    pub diff_ids: Vec<String>,
}

/// OCI history entry for build provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciHistoryEntry {
    /// ISO 8601 timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// Command that created this layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    /// Whether this is an empty (metadata-only) layer.
    #[serde(default)]
    pub empty_layer: bool,
    /// Human-readable comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// FJ-2101: OCI layout index (index.json at root of OCI layout directory).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciIndex {
    /// Always 2.
    pub schema_version: u32,
    /// Manifest descriptors.
    pub manifests: Vec<OciDescriptor>,
    /// Optional annotations.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub annotations: HashMap<String, String>,
}

impl OciIndex {
    /// Create an index with a single manifest.
    pub fn single(manifest_descriptor: OciDescriptor) -> Self {
        Self {
            schema_version: 2,
            manifests: vec![manifest_descriptor],
            annotations: HashMap::new(),
        }
    }
}

/// FJ-2102: Layer build result from direct assembly or pepita export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerBuildResult {
    /// SHA-256 digest of compressed content (OCI layer digest).
    pub digest: String,
    /// SHA-256 digest of uncompressed content (OCI DiffID).
    pub diff_id: String,
    /// BLAKE3 hash for forjar store address.
    pub store_hash: String,
    /// Compressed size in bytes.
    pub compressed_size: u64,
    /// Uncompressed size in bytes.
    pub uncompressed_size: u64,
    /// Compression algorithm used.
    pub compression: LayerCompression,
    /// Number of files in the layer.
    pub file_count: u32,
    /// Build path used (direct or pepita).
    pub build_path: LayerBuildPath,
}

impl LayerBuildResult {
    /// Compression ratio as a percentage (100% = no compression).
    pub fn compression_ratio(&self) -> f64 {
        if self.uncompressed_size == 0 {
            return 100.0;
        }
        (self.compressed_size as f64 / self.uncompressed_size as f64) * 100.0
    }

    /// Convert to an OCI descriptor.
    pub fn to_descriptor(&self) -> OciDescriptor {
        let media_type = match self.compression {
            LayerCompression::Gzip => "application/vnd.oci.image.layer.v1.tar+gzip",
            LayerCompression::Zstd => "application/vnd.oci.image.layer.v1.tar+zstd",
            LayerCompression::None => "application/vnd.oci.image.layer.v1.tar",
        };
        OciDescriptor {
            media_type: media_type.into(),
            digest: self.digest.clone(),
            size: self.compressed_size,
            annotations: HashMap::new(),
        }
    }
}

/// Layer compression algorithm.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LayerCompression {
    /// gzip — maximum compatibility.
    #[default]
    Gzip,
    /// zstd — better ratio, OCI 1.1+.
    Zstd,
    /// No compression.
    None,
}

/// Layer build path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerBuildPath {
    /// Path 1: direct assembly (file/package → tar).
    DirectAssembly,
    /// Path 2: pepita sandbox → export overlay upper.
    PepitaExport,
}

/// FJ-2104: Image build configuration from YAML resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageBuildConfig {
    /// Image name (e.g., `myregistry.io/myapp`).
    pub name: String,
    /// Image tag.
    pub tag: String,
    /// Base image reference.
    #[serde(default)]
    pub base: Option<String>,
    /// Determinism level.
    #[serde(default)]
    pub deterministic: DeterminismLevel,
    /// Enable layer caching.
    #[serde(default = "super::default_true")]
    pub cache: bool,
    /// Maximum number of layers.
    #[serde(default = "default_max_layers")]
    pub max_layers: u32,
    /// Compression algorithm.
    #[serde(default)]
    pub compress: LayerCompression,
}

fn default_max_layers() -> u32 {
    10
}

/// Determinism level for image builds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeterminismLevel {
    /// No restrictions (default).
    #[default]
    False,
    /// Network disabled.
    Network,
    /// Full lockdown: epoch timestamps, sanitized env, sorted entries.
    Strict,
    /// Alias for strict.
    True,
}

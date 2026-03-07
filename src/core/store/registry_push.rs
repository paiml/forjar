//! FJ-2105: OCI Distribution v1.1 registry push.
//!
//! Implements the push protocol: HEAD check for existing blobs,
//! blob upload (POST + PUT), and manifest PUT. Uses `curl` via
//! the transport layer (I8-validated).

use crate::core::types::{OciIndex, OciManifest, PushKind, PushResult};
use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

/// Registry push configuration.
#[derive(Debug, Clone)]
pub struct RegistryPushConfig {
    /// Registry hostname (e.g., "ghcr.io").
    pub registry: String,
    /// Image name (e.g., "myorg/myapp").
    pub name: String,
    /// Image tag (e.g., "v1.0").
    pub tag: String,
    /// Whether to check if blobs already exist before uploading.
    pub check_existing: bool,
}

/// A blob descriptor to push.
#[derive(Debug, Clone)]
pub struct BlobDescriptor {
    /// Content digest (sha256:...).
    pub digest: String,
    /// Size in bytes.
    pub size: u64,
    /// Path to the blob file on disk.
    pub path: std::path::PathBuf,
    /// What kind of content this is.
    pub kind: PushKind,
}

/// Check if a blob already exists in the registry via HEAD request.
///
/// OCI Distribution Spec v1.1: `HEAD /v2/{name}/blobs/{digest}`
/// Returns 200 if exists, 404 if not.
pub fn check_blob_exists(registry: &str, name: &str, digest: &str) -> Result<bool, String> {
    let url = format!("https://{registry}/v2/{name}/blobs/{digest}");
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "--head",
            &url,
        ])
        .output()
        .map_err(|e| format!("curl HEAD: {e}"))?;

    let status = String::from_utf8_lossy(&output.stdout);
    Ok(status.trim() == "200")
}

/// Generate the curl command for a HEAD blob check.
pub fn head_check_command(registry: &str, name: &str, digest: &str) -> String {
    format!(
        "curl -s -o /dev/null -w '%{{http_code}}' --head 'https://{registry}/v2/{name}/blobs/{digest}'"
    )
}

/// Generate the curl command for initiating a blob upload.
pub fn upload_initiate_command(registry: &str, name: &str) -> String {
    format!("curl -s -X POST -D - 'https://{registry}/v2/{name}/blobs/uploads/'")
}

/// Generate the curl command for completing a blob upload.
pub fn upload_complete_command(upload_url: &str, digest: &str, blob_path: &str) -> String {
    format!(
        "curl -s -X PUT -H 'Content-Type: application/octet-stream' \
         --data-binary '@{blob_path}' '{upload_url}?digest={digest}'"
    )
}

/// Generate the curl command for pushing a manifest.
pub fn manifest_put_command(registry: &str, name: &str, tag: &str, manifest_path: &str) -> String {
    format!(
        "curl -s -X PUT -H 'Content-Type: application/vnd.oci.image.manifest.v1+json' \
         --data-binary '@{manifest_path}' 'https://{registry}/v2/{name}/manifests/{tag}'"
    )
}

/// Push a single blob to the registry.
///
/// 1. Optionally HEAD-check if blob exists (skip if `check_existing` and exists)
/// 2. POST to initiate upload
/// 3. PUT to complete upload with digest
pub fn push_blob(config: &RegistryPushConfig, blob: &BlobDescriptor) -> Result<PushResult, String> {
    let start = Instant::now();

    // Step 1: Check if blob already exists
    if config.check_existing {
        let exists = check_blob_exists(&config.registry, &config.name, &blob.digest)?;
        if exists {
            return Ok(PushResult {
                kind: blob.kind,
                digest: blob.digest.clone(),
                size: blob.size,
                existed: true,
                duration_secs: 0.0,
            });
        }
    }

    // Step 2: Initiate upload
    let initiate_output = std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "-D",
            "-",
            &format!(
                "https://{}/v2/{}/blobs/uploads/",
                config.registry, config.name
            ),
        ])
        .output()
        .map_err(|e| format!("blob upload initiate: {e}"))?;

    let headers = String::from_utf8_lossy(&initiate_output.stdout);
    let upload_url = parse_location_header(&headers)
        .ok_or_else(|| "no Location header in upload response".to_string())?;

    // Step 3: Complete upload
    let blob_path = blob.path.display().to_string();
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "PUT",
            "-H",
            "Content-Type: application/octet-stream",
            "--data-binary",
            &format!("@{blob_path}"),
            &format!("{upload_url}?digest={}", blob.digest),
        ])
        .output()
        .map_err(|e| format!("blob upload complete: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "blob upload failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(PushResult {
        kind: blob.kind,
        digest: blob.digest.clone(),
        size: blob.size,
        existed: false,
        duration_secs: start.elapsed().as_secs_f64(),
    })
}

/// Push a manifest to the registry.
///
/// PUT /v2/{name}/manifests/{tag} with OCI manifest content type.
pub fn push_manifest(
    config: &RegistryPushConfig,
    manifest_json: &str,
    digest: &str,
) -> Result<PushResult, String> {
    let start = Instant::now();

    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "PUT",
            "-H",
            "Content-Type: application/vnd.oci.image.manifest.v1+json",
            "-d",
            manifest_json,
            &format!(
                "https://{}/v2/{}/manifests/{}",
                config.registry, config.name, config.tag
            ),
        ])
        .output()
        .map_err(|e| format!("manifest push: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "manifest push failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(PushResult {
        kind: PushKind::Manifest,
        digest: digest.to_string(),
        size: manifest_json.len() as u64,
        existed: false,
        duration_secs: start.elapsed().as_secs_f64(),
    })
}

/// Push a complete OCI image to a registry.
///
/// Follows OCI Distribution Spec v1.1:
/// 1. Push layer blobs (skip existing via HEAD check)
/// 2. Push config blob
/// 3. Push manifest
pub fn push_image(oci_dir: &Path, config: &RegistryPushConfig) -> Result<Vec<PushResult>, String> {
    let blobs_dir = oci_dir.join("blobs").join("sha256");
    if !blobs_dir.is_dir() {
        return Err(format!(
            "OCI blobs directory not found: {}",
            blobs_dir.display()
        ));
    }

    let index_path = oci_dir.join("index.json");
    if !index_path.exists() {
        return Err(format!(
            "OCI index.json not found: {}",
            index_path.display()
        ));
    }

    let blobs = discover_blobs(oci_dir)?;
    let mut results = Vec::new();

    // Push in correct order: layers first, then config, then manifests
    for kind in [PushKind::Layer, PushKind::Config, PushKind::Manifest] {
        for blob in blobs.iter().filter(|b| b.kind == kind) {
            let result = push_blob(config, blob)?;
            results.push(result);
        }
    }

    Ok(results)
}

/// Digest classification sets parsed from OCI index.json → manifest chain.
struct DigestClassification {
    manifests: HashSet<String>,
    configs: HashSet<String>,
}

/// Parse index.json and manifest blobs to classify digests by kind.
fn classify_digests_from_index(oci_dir: &Path) -> DigestClassification {
    let mut result = DigestClassification {
        manifests: HashSet::new(),
        configs: HashSet::new(),
    };
    let index_path = oci_dir.join("index.json");
    let index_json = match std::fs::read_to_string(&index_path) {
        Ok(s) => s,
        Err(_) => return result,
    };
    let index: OciIndex = match serde_json::from_str(&index_json) {
        Ok(i) => i,
        Err(_) => return result,
    };
    let blobs_dir = oci_dir.join("blobs").join("sha256");
    for m in &index.manifests {
        result.manifests.insert(m.digest.clone());
        let hash = m.digest.strip_prefix("sha256:").unwrap_or(&m.digest);
        let mf_path = blobs_dir.join(hash);
        if let Ok(mf_json) = std::fs::read_to_string(&mf_path) {
            if let Ok(manifest) = serde_json::from_str::<OciManifest>(&mf_json) {
                result.configs.insert(manifest.config.digest.clone());
            }
        }
    }
    result
}

/// Discover and classify all blobs in an OCI layout directory.
///
/// Parses index.json → manifest → identifies config and layer digests.
/// Blobs not referenced by any manifest default to Layer kind.
pub(crate) fn discover_blobs(oci_dir: &Path) -> Result<Vec<BlobDescriptor>, String> {
    let blobs_dir = oci_dir.join("blobs").join("sha256");
    if !blobs_dir.is_dir() {
        return Ok(Vec::new());
    }

    let classification = classify_digests_from_index(oci_dir);
    let mut blobs = Vec::new();
    let entries = std::fs::read_dir(&blobs_dir).map_err(|e| format!("read blobs dir: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("read blob entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let digest = format!("sha256:{name}");
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);

        let kind = if classification.manifests.contains(&digest) {
            PushKind::Manifest
        } else if classification.configs.contains(&digest) {
            PushKind::Config
        } else {
            PushKind::Layer
        };

        blobs.push(BlobDescriptor {
            digest,
            size,
            path,
            kind,
        });
    }

    Ok(blobs)
}

/// Parse the Location header from HTTP response headers.
pub(crate) fn parse_location_header(headers: &str) -> Option<String> {
    for line in headers.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("location:") {
            return Some(line[9..].trim().to_string());
        }
    }
    None
}

/// Validate a registry push config.
pub fn validate_push_config(config: &RegistryPushConfig) -> Vec<String> {
    let mut errors = Vec::new();
    if config.registry.is_empty() {
        errors.push("registry hostname is required".into());
    }
    if config.name.is_empty() {
        errors.push("image name is required".into());
    }
    if config.tag.is_empty() {
        errors.push("image tag is required".into());
    }
    if config.registry.contains("://") {
        errors.push("registry should be hostname only, not a URL".into());
    }
    errors
}

/// Format a push summary for CLI output.
pub fn format_push_summary(results: &[PushResult]) -> String {
    let mut out = String::new();
    let uploaded: Vec<_> = results.iter().filter(|r| !r.existed).collect();
    let skipped: Vec<_> = results.iter().filter(|r| r.existed).collect();

    out.push_str(&format!(
        "Push complete: {} uploaded, {} skipped (already exist)\n",
        uploaded.len(),
        skipped.len(),
    ));

    let total_bytes: u64 = uploaded.iter().map(|r| r.size).sum();
    let total_secs: f64 = uploaded.iter().map(|r| r.duration_secs).sum();
    if !uploaded.is_empty() {
        out.push_str(&format!(
            "  Uploaded {:.1} MB in {:.1}s\n",
            total_bytes as f64 / (1024.0 * 1024.0),
            total_secs,
        ));
    }

    for r in results {
        let status = if r.existed { "skip" } else { "push" };
        let kind = match r.kind {
            PushKind::Layer => "layer",
            PushKind::Config => "config",
            PushKind::Manifest => "manifest",
            PushKind::Index => "index",
        };
        out.push_str(&format!(
            "  [{status}] {kind}: {} ({} bytes)\n",
            r.digest, r.size
        ));
    }

    out
}

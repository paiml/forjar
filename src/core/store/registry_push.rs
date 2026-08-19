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
/// Refs #210: only 200 (exists) and 404 (absent) are answers. Anything else —
/// including the `000` curl prints when it never connected — is an error, not
/// an implicit "does not exist"; the old code read every failure as 404 and
/// marched on to an upload it could not perform.
pub fn check_blob_exists(registry: &str, name: &str, digest: &str) -> Result<bool, String> {
    let url =
        super::registry_push_http::registry_url(registry, &format!("v2/{name}/blobs/{digest}"));
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "--connect-timeout",
            super::registry_push_http::CONNECT_TIMEOUT_SECS,
            "--head",
            &url,
        ])
        .output()
        .map_err(|e| format!("curl HEAD: {e}"))?;

    let status = String::from_utf8_lossy(&output.stdout);
    match status.trim() {
        "200" => Ok(true),
        "404" => Ok(false),
        "000" | "" => Err(format!(
            "registry unreachable: no HTTP response from {url} \
             (curl exited {})",
            output.status
        )),
        other => {
            let code = other.parse::<u16>().unwrap_or(0);
            Err(super::registry_push_http::describe_status(
                &format!("HEAD {url}"),
                code,
            ))
        }
    }
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
/// `--fail-with-body`: see [`monolithic_put_args`] (Bug-hunt #4, Refs #154).
pub fn upload_complete_command(upload_url: &str, digest: &str, blob_path: &str) -> String {
    format!(
        "curl -s --fail-with-body -X PUT -H 'Content-Type: application/octet-stream' \
         --data-binary '@{blob_path}' '{upload_url}?digest={digest}'"
    )
}

/// Generate the curl command for pushing a manifest.
/// `--fail-with-body`: see [`manifest_put_args`] (Bug-hunt #4, Refs #154).
pub fn manifest_put_command(registry: &str, name: &str, tag: &str, manifest_path: &str) -> String {
    format!(
        "curl -s --fail-with-body -X PUT -H 'Content-Type: application/vnd.oci.image.manifest.v1+json' \
         --data-binary '@{manifest_path}' 'https://{registry}/v2/{name}/manifests/{tag}'"
    )
}

/// Push a single blob to the registry.
///
/// 1. Optionally HEAD-check if blob exists (skip if `check_existing` and exists)
/// 2. POST to initiate upload
/// 3. For blobs < 64MB: monolithic PUT
///    For blobs >= 64MB: chunked PATCH + PUT (E14)
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
    let upload_url = initiate_upload(&config.registry, &config.name)?;

    // Step 3: Upload blob (monolithic or chunked based on size)
    if blob.size >= CHUNKED_UPLOAD_THRESHOLD {
        push_blob_chunked(&upload_url, blob)?;
    } else {
        push_blob_monolithic(&upload_url, blob)?;
    }

    Ok(PushResult {
        kind: blob.kind,
        digest: blob.digest.clone(),
        size: blob.size,
        existed: false,
        duration_secs: start.elapsed().as_secs_f64(),
    })
}

/// Initiate a blob upload session. Returns the upload URL from Location header.
///
/// Refs #210: the status code is now the gate. A `Location` header alone is no
/// evidence of an upload session — `docker.io` answers this POST with a 301 to
/// its marketing site, and the old code took that redirect target as the
/// session URL, PUT the blob at a web page, got 200, and reported a push.
/// Only 202 Accepted opens a session.
fn initiate_upload(registry: &str, name: &str) -> Result<String, String> {
    let url =
        super::registry_push_http::registry_url(registry, &format!("v2/{name}/blobs/uploads/"));
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "POST",
            "-D",
            "-",
            "-o",
            "/dev/null",
            "--connect-timeout",
            super::registry_push_http::CONNECT_TIMEOUT_SECS,
            &url,
        ])
        .output()
        .map_err(|e| format!("blob upload initiate: {e}"))?;

    let headers = String::from_utf8_lossy(&output.stdout);
    let Some(code) = super::registry_push_http::parse_status_code(&headers) else {
        return Err(format!(
            "registry unreachable: no HTTP response to POST {url} (curl exited {})",
            output.status
        ));
    };
    if code != 202 {
        return Err(super::registry_push_http::describe_status(
            &format!("POST {url}"),
            code,
        ));
    }
    let location = parse_location_header(&headers).ok_or_else(|| {
        format!("POST {url} returned 202 with no Location header; no upload session to write to")
    })?;
    Ok(super::registry_push_http::resolve_location(
        registry, &location,
    ))
}

/// Build the curl argv for a monolithic blob PUT. Bug-hunt #4 (Refs #154):
/// `--fail-with-body` makes curl exit non-zero on HTTP >= 400 (401/404/413/5xx)
/// — without it a failed PUT was reported as a successful push while the
/// registry stored nothing. Extracted so the flag is unit-testable (no network).
pub(crate) fn monolithic_put_args(upload_url: &str, digest: &str, blob_path: &str) -> Vec<String> {
    vec![
        "-s".into(),
        "--fail-with-body".into(),
        "-X".into(),
        "PUT".into(),
        "-H".into(),
        "Content-Type: application/octet-stream".into(),
        "--data-binary".into(),
        format!("@{blob_path}"),
        super::registry_push_http::with_digest_query(upload_url, digest),
    ]
}

/// Monolithic PUT upload for small blobs (< 64 MB).
fn push_blob_monolithic(upload_url: &str, blob: &BlobDescriptor) -> Result<(), String> {
    let blob_path = blob.path.display().to_string();
    let args = monolithic_put_args(upload_url, &blob.digest, &blob_path);
    let output = std::process::Command::new("curl")
        .args(&args)
        .output()
        .map_err(|e| format!("blob upload complete: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "blob upload failed (HTTP error): {}",
            curl_error_detail(&output)
        ));
    }
    Ok(())
}

/// Build the curl argv for a manifest PUT. `--fail-with-body`: see
/// [`monolithic_put_args`] (Bug-hunt #4, Refs #154) — the manifest PUT is the
/// final, release-critical step, so a silent 401/404/5xx here is the worst case.
pub(crate) fn manifest_put_args(manifest_json: &str, url: &str) -> Vec<String> {
    vec![
        "-s".into(),
        "--fail-with-body".into(),
        "-X".into(),
        "PUT".into(),
        "-H".into(),
        "Content-Type: application/vnd.oci.image.manifest.v1+json".into(),
        "-d".into(),
        manifest_json.into(),
        url.into(),
    ]
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

    let url = super::registry_push_http::registry_url(
        &config.registry,
        &format!("v2/{}/manifests/{}", config.name, config.tag),
    );
    let args = manifest_put_args(manifest_json, &url);
    let output = std::process::Command::new("curl")
        .args(&args)
        .output()
        .map_err(|e| format!("manifest push: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "manifest push failed (HTTP error): {}",
            curl_error_detail(&output)
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

/// Verify the `curl` binary this module depends on is actually available.
///
/// GH-224. `curl` is an **undeclared runtime dependency** of `forjar build
/// --push`: nothing in Cargo.toml or the docs says you need it, and it is only
/// discovered when a push fails. Probing with `--version` (rather than scanning
/// PATH by hand) tests the thing that actually matters — that we can spawn it.
///
/// Returns Ok(()) when curl can be spawned, otherwise an actionable error.
pub(crate) fn require_curl() -> Result<(), String> {
    match std::process::Command::new("curl").arg("--version").output() {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(
            "`forjar build --push` requires the `curl` binary on PATH, and it was not found.\n\
             OCI registry requests (HEAD/POST/PUT) are made by shelling out to curl.\n\
             Install it and retry — e.g. `apt-get install -y curl` or `dnf install -y curl`."
                .to_string(),
        ),
        Err(e) => Err(format!("could not execute `curl`: {e}")),
    }
}

/// Push a complete OCI image to a registry.
///
/// Follows OCI Distribution Spec v1.1:
/// 1. Push layer blobs (skip existing via HEAD check)
/// 2. Push config blob
/// 3. Push manifest
pub fn push_image(oci_dir: &Path, config: &RegistryPushConfig) -> Result<Vec<PushResult>, String> {
    // GH-224: fail with a message that names the actual problem. Every registry
    // request in this module shells out to `curl`, so on a host without it the
    // first HEAD died as:
    //
    //   curl HEAD: No such file or directory (os error 2)
    //
    // which names neither curl nor "a required external binary is missing", and
    // reads like a network or registry fault. It was found by infra's clean-room
    // gate (a container with only declared deps) while GitHub CI stayed green,
    // because CI's image happens to ship curl.
    //
    // Checked once here rather than at each of the ~13 call sites: this is the
    // single funnel every push goes through, so one probe covers the CLI and any
    // library caller, and the message is emitted before a partial upload starts.
    require_curl()?;

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
    let manifests: Vec<&BlobDescriptor> = blobs
        .iter()
        .filter(|b| b.kind == PushKind::Manifest)
        .collect();

    // Refs #210: a push with no manifest PUT leaves the tag pointing wherever
    // it pointed before, so refuse rather than report a partial push.
    if manifests.is_empty() {
        return Err(format!(
            "no manifest found in OCI layout {}: index.json references none of the blobs present",
            oci_dir.display()
        ));
    }
    if manifests.len() > 1 {
        return Err(format!(
            "OCI layout {} holds {} manifests (a multi-arch image index); \
             forjar cannot push an index to a single tag",
            oci_dir.display(),
            manifests.len()
        ));
    }

    let mut results = Vec::new();

    // Push in correct order: layer blobs, then the config blob, ...
    for kind in [PushKind::Layer, PushKind::Config] {
        for blob in blobs.iter().filter(|b| b.kind == kind) {
            results.push(push_blob(config, blob)?);
        }
    }

    // ... then the manifest, which is PUT to the TAG — not uploaded as a blob.
    // Uploading the manifest bytes to /blobs/ (what this used to do) creates no
    // tag, so nothing could ever pull the image that was reported as pushed.
    let manifest = manifests[0];
    let manifest_json = std::fs::read_to_string(&manifest.path)
        .map_err(|e| format!("read manifest {}: {e}", manifest.path.display()))?;
    results.push(push_manifest(config, &manifest_json, &manifest.digest)?);

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

// Chunked upload lives in `registry_push_chunked` and the curl error detail in
// `registry_push_http` (Refs #210: this file has to stay under the 500-line
// health limit). Re-exported so `super::registry_push::*` callers are unchanged.
#[allow(unused_imports)]
pub(crate) use super::registry_push_chunked::{
    push_blob_chunked, CHUNKED_UPLOAD_THRESHOLD, CHUNK_SIZE,
};
pub(crate) use super::registry_push_http::curl_error_detail;

// `validate_push_config` and `format_push_summary` live in `registry_push_fmt`
// (split out to keep this file under the 500-line health limit). Re-exported so
// existing `super::registry_push::*` callers keep working unchanged.
pub use super::registry_push_fmt::{format_push_summary, validate_push_config};

//! FJ-2105/E14: chunked (PATCH) blob upload for large layers.
//!
//! Split out of `registry_push.rs` to keep that file under the 500-line health
//! limit (Refs #210). GH-228: the requests are ureq calls, not `curl`
//! subprocesses.

use super::registry_http::{self, RequestBody};
use super::registry_push::BlobDescriptor;
use super::registry_push_http::with_digest_query;

/// E14: Chunk size for chunked uploads (64 MB).
pub(crate) const CHUNKED_UPLOAD_THRESHOLD: u64 = 64 * 1024 * 1024;
/// E14: Chunk size for PATCH uploads (16 MB).
pub(crate) const CHUNK_SIZE: u64 = 16 * 1024 * 1024;

/// E14: Chunked PATCH upload for large blobs (>= 64 MB).
///
/// OCI Distribution Spec v1.1 chunked upload protocol:
/// 1. PATCH with Content-Range for each chunk
/// 2. PUT to complete with final digest
///
/// Bug-hunt #4 (Refs #154): every request is gated on a 2xx from the registry.
/// GH-228: each chunk streams the declared byte range of the blob and declares
/// its own `Content-Length`. That matters twice — without an explicit length
/// ureq falls back to chunked transfer-encoding, which some registries reject
/// on PATCH, and the previous `curl -r <range> --data-binary @file` sent the
/// **whole file** for every chunk, because `-r` is a download-side flag.
pub(crate) fn push_blob_chunked(upload_url: &str, blob: &BlobDescriptor) -> Result<(), String> {
    let on_disk = std::fs::metadata(&blob.path)
        .map_err(|e| format!("chunked upload: cannot stat {}: {e}", blob.path.display()))?
        .len();
    if on_disk != blob.size {
        return Err(format!(
            "chunked upload refused: {} holds {on_disk} bytes but the descriptor declares {}; \
             uploading it would leave the registry with a blob it must reject",
            blob.path.display(),
            blob.size
        ));
    }

    let agent = registry_http::agent_for_url(upload_url);
    let mut offset: u64 = 0;
    let mut current_url = upload_url.to_string();

    while offset < blob.size {
        let end = std::cmp::min(offset + CHUNK_SIZE, blob.size) - 1;
        let len = end - offset + 1;
        let range = format!("{offset}-{end}");

        let response = registry_http::send(
            &agent,
            "PATCH",
            &current_url,
            &[
                ("Content-Type", "application/octet-stream".to_string()),
                ("Content-Range", range.clone()),
                ("Content-Length", len.to_string()),
            ],
            RequestBody::FileRange {
                path: &blob.path,
                offset,
                len,
            },
        )
        .map_err(|e| format!("chunked upload failed at range {range}: {e}"))?;

        if !registry_http::is_success(response.status) {
            return Err(format!(
                "chunked upload failed at range {range} (HTTP {}): {}",
                response.status,
                registry_http::detail(&response)
            ));
        }

        // Follow the Location header for the next chunk URL. Out of scope here
        // (Refs #228): a *relative* Location is taken verbatim rather than
        // resolved against the registry host, so resumption still breaks for a
        // registry that answers with a path.
        if let Some(location) = response.location {
            current_url = location;
        }

        offset = end + 1;
    }

    finalize_chunked_upload(&agent, &current_url, blob)
}

/// Complete the session with a PUT carrying the blob digest.
fn finalize_chunked_upload(
    agent: &ureq::Agent,
    current_url: &str,
    blob: &BlobDescriptor,
) -> Result<(), String> {
    let url = with_digest_query(current_url, &blob.digest);
    let response = registry_http::send(
        agent,
        "PUT",
        &url,
        &[("Content-Type", "application/octet-stream".to_string())],
        RequestBody::Empty,
    )
    .map_err(|e| format!("chunked upload finalize failed: no HTTP response from {url} ({e})"))?;

    if !registry_http::is_success(response.status) {
        return Err(format!(
            "chunked upload finalize failed (HTTP {}): {}",
            response.status,
            registry_http::detail(&response)
        ));
    }
    Ok(())
}

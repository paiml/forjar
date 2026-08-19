//! FJ-2105/E14: chunked (PATCH) blob upload for large layers.
//!
//! Split out of `registry_push.rs` to keep that file under the 500-line health
//! limit (Refs #210).

use super::registry_push::BlobDescriptor;
use super::registry_push_http::{curl_error_detail, with_digest_query};

/// E14: Chunk size for chunked uploads (64 MB).
pub(crate) const CHUNKED_UPLOAD_THRESHOLD: u64 = 64 * 1024 * 1024;
/// E14: Chunk size for PATCH uploads (16 MB).
pub(crate) const CHUNK_SIZE: u64 = 16 * 1024 * 1024;

/// E14: Chunked PATCH upload for large blobs (>= 64 MB).
///
/// OCI Distribution Spec v1.1 chunked upload protocol:
/// 1. PATCH with Content-Range for each chunk
/// 2. PUT to complete with final digest
pub(crate) fn push_blob_chunked(upload_url: &str, blob: &BlobDescriptor) -> Result<(), String> {
    let blob_path = blob.path.display().to_string();
    let total_size = blob.size;
    let mut offset: u64 = 0;
    let mut current_url = upload_url.to_string();

    while offset < total_size {
        let end = std::cmp::min(offset + CHUNK_SIZE, total_size) - 1;
        let range = format!("{offset}-{end}");

        let output = std::process::Command::new("curl")
            .args([
                "-s",
                "--fail-with-body", // Bug-hunt #4 (Refs #154): gate on HTTP status.
                "-X",
                "PATCH",
                "-D",
                "-",
                "-H",
                "Content-Type: application/octet-stream",
                "-H",
                &format!("Content-Range: {range}"),
                "-H",
                &format!("Content-Length: {}", end - offset + 1),
                "-r",
                &range,
                "--data-binary",
                &format!("@{blob_path}"),
                &current_url,
            ])
            .output()
            .map_err(|e| format!("chunked upload PATCH: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "chunked upload failed at range {range} (HTTP error): {}",
                curl_error_detail(&output)
            ));
        }

        // Follow Location header for next chunk URL
        let headers = String::from_utf8_lossy(&output.stdout);
        if let Some(loc) = super::registry_push::parse_location_header(&headers) {
            current_url = loc;
        }

        offset = end + 1;
    }

    // Complete the upload with PUT + digest
    let output = std::process::Command::new("curl")
        .args([
            "-s",
            "--fail-with-body", // Bug-hunt #4 (Refs #154): gate on HTTP status.
            "-X",
            "PUT",
            "-H",
            "Content-Type: application/octet-stream",
            &with_digest_query(&current_url, &blob.digest),
        ])
        .output()
        .map_err(|e| format!("chunked upload finalize: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "chunked upload finalize failed (HTTP error): {}",
            curl_error_detail(&output)
        ));
    }
    Ok(())
}

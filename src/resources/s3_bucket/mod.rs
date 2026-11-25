//! PMAT-1242: `s3_bucket` — one private S3 bucket as declared state.
//!
//! # Why
//!
//! infra declares `s3://paiml-metrics-preview` (RMEDIA-METRICS-001 D5). forjar had no
//! bucket resource, so the alternative was a shell script around `aws`, which is the
//! shape forjar exists to replace. The `aws` CLI is shelled out to by the generated
//! script, as `cloud_instance` does; no new binary is added.
//!
//! # YAML
//!
//! ```yaml
//! metrics-preview:
//!   type: s3_bucket
//!   machine: lambda-labs
//!   s3_bucket: paiml-metrics-preview
//!   s3_region: us-east-1
//!   s3_private: true                 # block all four public-access flags
//!   s3_sse: AES256                   # SSE-S3
//!   s3_tls_only: true                # deny aws:SecureTransport=false
//!   s3_versioning: true
//!   s3_noncurrent_expire_days: 30
//! ```
//!
//! # Contract
//!
//! * `check_script` and `state_query_script` ASK AWS (`head-bucket`,
//!   `get-bucket-encryption`, `get-public-access-block`, `get-bucket-versioning`,
//!   `get-bucket-lifecycle-configuration`, `get-bucket-policy`) and compare to the
//!   declaration. The lock is never read. A failed read is divergence, never a pass.
//! * `apply_script` creates the bucket when `head-bucket` says 404 (a failed lookup
//!   is an ERROR, not "absent"), then re-asserts every declared property, then reads
//!   them back and fails unless AWS agrees. A second apply leaves AWS unchanged.
//! * `state: absent` is refused: forjar never deletes a bucket or its data.
//! * `put-bucket-lifecycle-configuration` and `put-bucket-policy` REPLACE the whole
//!   document, so a declared bucket owns its lifecycle rules and its policy.

mod script;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_fake;
#[cfg(test)]
mod tests_mutants;
#[cfg(test)]
mod tests_policy;

use crate::core::types::Resource;
pub use script::{apply_script, check_script, state_query_script};

/// A validated `s3_bucket` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3Bucket {
    pub bucket: String,
    pub region: String,
    pub tls_only: bool,
    pub versioning: bool,
    pub noncurrent_expire_days: Option<u32>,
}

/// Bucket name: 3-63 of `a-z 0-9 - .`, starting and ending alphanumeric, no `..`.
pub(crate) fn is_bucket_name(n: &str) -> bool {
    let edge = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    (3..=63).contains(&n.len())
        && n.starts_with(edge)
        && n.ends_with(edge)
        && n.chars().all(|c| edge(c) || c == '-' || c == '.')
        && !n.contains("..")
}

/// Region: `<2-3 letters>(-<letters>)+-<digit(s)>` such as `us-east-1`, `eu-south-2`.
pub(crate) fn is_region(r: &str) -> bool {
    let parts: Vec<&str> = r.split('-').collect();
    parts.len() >= 3
        && parts.len() <= 4
        && parts[..parts.len() - 1]
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 10 && p.chars().all(|c| c.is_ascii_lowercase()))
        && parts
            .last()
            .is_some_and(|d| !d.is_empty() && d.len() <= 2 && d.chars().all(|c| c.is_ascii_digit()))
}

/// Validate a declaration.
///
/// # Errors
///
/// Returns `Err` for a missing or malformed bucket or region, `s3_private: false`,
/// an `s3_sse` other than `AES256`, an expiry outside 1..=36500 days, or `state: absent`.
pub fn spec_of(resource: &Resource) -> Result<S3Bucket, String> {
    let s = &resource.s3;
    let bucket = s
        .bucket
        .as_deref()
        .filter(|b| !b.is_empty())
        .ok_or("s3_bucket requires `s3_bucket`")?;
    if !is_bucket_name(bucket) {
        return Err(format!("s3_bucket: `{bucket}` is not a valid bucket name"));
    }
    let region = s
        .region
        .as_deref()
        .filter(|r| !r.is_empty())
        .ok_or("s3_bucket requires `s3_region`")?;
    if !is_region(region) {
        return Err(format!("s3_bucket: `{region}` is not a valid region"));
    }
    if s.private == Some(false) {
        return Err(
            "s3_bucket: `s3_private: false` is refused; a public bucket is not supported".into(),
        );
    }
    if s.sse.as_deref().is_some_and(|v| v != "AES256") {
        return Err("s3_bucket: `s3_sse` must be AES256 (SSE-S3)".into());
    }
    if s.noncurrent_expire_days
        .is_some_and(|d| !(1..=36_500).contains(&d))
    {
        return Err("s3_bucket: `s3_noncurrent_expire_days` must be 1..=36500".into());
    }
    if resource.state.as_deref() == Some("absent") {
        return Err("s3_bucket: `state: absent` is refused; forjar never deletes a bucket".into());
    }
    Ok(S3Bucket {
        bucket: bucket.to_string(),
        region: region.to_string(),
        tls_only: s.tls_only.unwrap_or(true),
        versioning: s.versioning.unwrap_or(true),
        noncurrent_expire_days: s.noncurrent_expire_days,
    })
}

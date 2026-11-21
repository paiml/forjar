//! PMAT-1242: `s3_bucket` declaration fields — ONE private S3 bucket as declared state.
//!
//! Flattened into `Resource`, so the YAML stays flat (`s3_bucket:`, `s3_region:` …).
//! Every field skips serialising when unset, so no existing resource's desired-state
//! hash moves. `resources::s3_bucket::spec_of` refuses a bad declaration.

/// Declaration fields for an `s3_bucket` resource.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct S3Spec {
    /// Bucket name (3-63 chars, lowercase letters, digits, `-` and `.`).
    #[serde(rename = "s3_bucket", default, skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,

    /// AWS region, e.g. `us-east-1` (needs no LocationConstraint at create).
    #[serde(rename = "s3_region", default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    /// Block all four public-access flags. Defaults to true; `false` is refused.
    #[serde(
        rename = "s3_private",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub private: Option<bool>,

    /// Default encryption. Only `AES256` (SSE-S3) is supported; it is the default.
    #[serde(rename = "s3_sse", default, skip_serializing_if = "Option::is_none")]
    pub sse: Option<String>,

    /// Bucket policy denying every request with `aws:SecureTransport=false`.
    /// Defaults to true; `false` leaves the policy unmanaged.
    #[serde(
        rename = "s3_tls_only",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub tls_only: Option<bool>,

    /// Versioning `Enabled`. Defaults to true; `false` leaves it unmanaged
    /// (S3 can only suspend versioning, never remove it).
    #[serde(
        rename = "s3_versioning",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub versioning: Option<bool>,

    /// Expire non-current object versions after this many days (1..=36500).
    /// Unset leaves lifecycle unmanaged.
    #[serde(
        rename = "s3_noncurrent_expire_days",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub noncurrent_expire_days: Option<u32>,
}

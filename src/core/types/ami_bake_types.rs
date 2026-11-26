//! #1266 (SPOT-CAPACITY T2, slice 1): `ami_bake` declaration fields.
//!
//! ONE fleet AMI built from a stock base image, declared as state. Flattened
//! into `Resource`, so the YAML stays flat (`ami_base:`, `ami_region:` …).
//! Every field skips serialising when unset, so no existing resource's
//! desired-state hash moves. Slice 1 is declaration only: no AWS call, no
//! builder, no apply. `core::parser::ami_bake_validate` refuses a bad declaration.

/// Declaration fields for an `ami_bake` resource.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AmiBakeSpec {
    /// Stock base AMI id the builder starts from, e.g. `ami-0123456789abcdef0`.
    #[serde(rename = "ami_base", default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,

    /// Fleet image file loaded onto the builder (a path, never inline content).
    #[serde(
        rename = "ami_image_file",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub image_file: Option<String>,

    /// AWS region the AMI is registered in, e.g. `us-east-1`.
    #[serde(
        rename = "ami_region",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub region: Option<String>,

    /// Fleet digest the registered AMI is tagged with: `sha256:` + 64 hex.
    #[serde(
        rename = "ami_fleet_digest",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub fleet_digest: Option<String>,

    /// Declared old-AMI cleanup: keep this many newest bakes, deregister the rest.
    /// Must be at least 1, so a cleanup can never remove the only image.
    #[serde(
        rename = "ami_keep_last",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub keep_last: Option<u32>,
}

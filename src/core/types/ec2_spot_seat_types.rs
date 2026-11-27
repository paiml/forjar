//! forjar#1262: `ec2_spot_seat` declaration fields — one EC2 spot box that runs
//! GitHub runner seats from a pinned container image, managed only by forjar.
//!
//! Flattened into `Resource`, so the YAML stays flat (`spot_pin_file:` ...).
//! Every field skips serialising when unset, so no existing resource's
//! desired-state hash moves. `resources::ec2_spot_seat::spec_of` refuses a bad
//! declaration.

/// Declaration fields for an `ec2_spot_seat` resource.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpotSeatSpec {
    /// Which action this resource performs: `launch`, `image-load`, `seat-start`,
    /// `register` or `teardown`. Unset: `teardown` for `state: absent`, else `launch`.
    #[serde(
        rename = "spot_phase",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub phase: Option<String>,
    /// Path of the single image pin file (rows `image<TAB>key<TAB>value`).
    #[serde(
        rename = "spot_pin_file",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pin_file: Option<String>,
    /// Pin key holding the image id (default `id`).
    #[serde(
        rename = "spot_pin_id_key",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pin_id_key: Option<String>,
    /// Pin key holding the S3 key of the image tar (default `s3_key`).
    #[serde(
        rename = "spot_pin_s3_key_key",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pin_s3_key_key: Option<String>,
    /// Pin key holding the tar's sha256 (default `tar_sha256`).
    #[serde(
        rename = "spot_pin_sha256_key",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub pin_sha256_key: Option<String>,
    /// S3 bucket holding the image tar.
    #[serde(
        rename = "spot_image_bucket",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub image_bucket: Option<String>,
    /// Instance types to try, in order.
    #[serde(
        rename = "spot_instance_types",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub instance_types: Vec<String>,
    /// AWS region, e.g. `us-east-1`.
    #[serde(
        rename = "spot_region",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub region: Option<String>,
    /// Availability zone, e.g. `us-east-1a`.
    #[serde(rename = "spot_az", default, skip_serializing_if = "Option::is_none")]
    pub az: Option<String>,
    /// Subnet id (`subnet-...`) in that zone.
    #[serde(
        rename = "spot_subnet",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub subnet: Option<String>,
    /// AMI id of the host image.
    #[serde(rename = "spot_ami", default, skip_serializing_if = "Option::is_none")]
    pub ami: Option<String>,
    /// Box lifetime in minutes (1..=720).
    #[serde(
        rename = "spot_ttl_min",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ttl_min: Option<u32>,
    /// Seats per box (1..=64).
    #[serde(
        rename = "spot_seats",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub seats: Option<u32>,
    /// Value of the `Owner` tag; also the key every cap and teardown filters on.
    #[serde(
        rename = "spot_owner",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub owner: Option<String>,
    /// Hard cap on live boxes carrying the owner tag, this one included.
    #[serde(
        rename = "spot_max_boxes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_boxes: Option<u32>,
    /// Hard cap on the summed max price (USD/hour) of live boxes, this one included.
    #[serde(
        rename = "spot_max_usd_per_hour",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_usd_per_hour: Option<f64>,
    /// Max spot price (USD/hour) bid for this box; counts against the cap.
    #[serde(
        rename = "spot_price_usd_per_hour",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub price_usd_per_hour: Option<f64>,
    /// Market. Only `spot` exists; `on-demand` is a validation error.
    #[serde(
        rename = "spot_market",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub market: Option<String>,
    /// GitHub org the seats register in.
    #[serde(
        rename = "spot_runner_org",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub runner_org: Option<String>,
    /// Runner group name or numeric id.
    #[serde(
        rename = "spot_runner_group",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub runner_group: Option<String>,
    /// The one declared label list every seat carries.
    #[serde(
        rename = "spot_runner_labels",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub runner_labels: Vec<String>,
    /// Where seat JIT configs come from: `gh-api` (minted by `register`) or
    /// `dir:<path>` (files `seat-N.jit`).
    #[serde(
        rename = "spot_jit_source",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub jit_source: Option<String>,
    /// Off-box destination of the teardown evidence, `s3://bucket/prefix`.
    #[serde(
        rename = "spot_evidence_dest",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub evidence_dest: Option<String>,
}

//! FJ-861: `cloud_instance` declaration fields — spot instance or fleet as declared state.
//!
//! Inert by construction: nothing here talks to a cloud. `provider`, `image`
//! (by `@sha256` digest) and `tags` reuse the existing `Resource` fields; the
//! rest live here, `#[serde(flatten)]`-ed so the YAML stays flat.

/// Declaration fields for a `cloud_instance` resource (all optional at the
/// serde layer; `resources::cloud_instance::spec_of` refuses a bad declaration).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CloudSpec {
    /// Cloud region, e.g. `eu-south-2`.
    #[serde(
        rename = "cloud_region",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub region: Option<String>,

    /// Instance type, e.g. `c7a.xlarge`. Mutually exclusive with `cloud_instance_types`.
    #[serde(
        rename = "cloud_instance_type",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub instance_type: Option<String>,

    /// Candidate instance types tried in order across spot capacity fallback (#1265).
    /// Mutually exclusive with `cloud_instance_type`.
    #[serde(
        rename = "cloud_instance_types",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub instance_types: Option<Vec<String>>,

    /// Fleet size (1 to 6, default 1; #1265).
    #[serde(
        rename = "cloud_count",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub count: Option<u32>,

    /// Market type; only `spot` (or unset) is accepted — on-demand is refused (#1265).
    #[serde(
        rename = "cloud_market_type",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub market_type: Option<String>,

    /// Spot price ceiling in USD per hour, a decimal string (`0.20`).
    #[serde(
        rename = "cloud_max_price",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_price: Option<String>,

    /// Hard lifetime in minutes; the instance is destroyed after it.
    #[serde(
        rename = "cloud_max_lifetime_min",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_lifetime_min: Option<u32>,

    /// Daily budget cap in USD, a decimal string. Apply refuses without it.
    #[serde(
        rename = "cloud_budget_cap_usd_day",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub budget_cap_usd_day: Option<String>,

    /// Absolute path of the kill-switch file. While it exists, apply refuses.
    /// Apply refuses if none is declared.
    #[serde(
        rename = "cloud_kill_switch",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub kill_switch: Option<String>,

    /// Absolute path of the parity-gate file. Apply refuses unless it exists
    /// and holds the line `parity-gate: pass`.
    #[serde(
        rename = "cloud_parity_gate",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub parity_gate: Option<String>,

    /// Optional machine image pin: one whole AMI id (`ami-` + 8 or 17 hex).
    /// Absent: apply launches the default image, exactly as before.
    #[serde(rename = "cloud_ami", default, skip_serializing_if = "Option::is_none")]
    pub ami: Option<String>,

    /// Subnet the instance lands in, a whole `subnet-<hex>` id (#1033).
    #[serde(
        rename = "cloud_subnet_id",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub subnet_id: Option<String>,

    /// Security groups, 1 to 5 whole `sg-<hex>` ids (#1034).
    #[serde(
        rename = "cloud_security_group_ids",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub security_group_ids: Option<Vec<String>>,

    /// Availability zone; must be `cloud_region` plus one letter (#1035).
    /// Mutually exclusive with `cloud_availability_zones`.
    #[serde(
        rename = "cloud_availability_zone",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub availability_zone: Option<String>,

    /// Candidate availability zones tried in order; each must be `cloud_region` plus one letter (#1265).
    /// Mutually exclusive with `cloud_availability_zone`.
    #[serde(
        rename = "cloud_availability_zones",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub availability_zones: Option<Vec<String>>,

    /// Root volume size in GiB, 1 to 16384; absent keeps the image default.
    #[serde(
        rename = "cloud_root_volume_gb",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub root_volume_gb: Option<u32>,

    /// Root volume type, `gp2` or `gp3` (gp3 when only a size is set).
    #[serde(
        rename = "cloud_root_volume_type",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub root_volume_type: Option<String>,

    /// Optional data volume on /dev/sdf, size in GiB, 1 to 16384.
    #[serde(
        rename = "cloud_data_volume_gb",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub data_volume_gb: Option<u32>,

    /// Data volume type, `gp2` or `gp3` (default gp3).
    #[serde(
        rename = "cloud_data_volume_type",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub data_volume_type: Option<String>,

    /// Data volume provisioned IOPS (gp3: 3000 to 16000).
    #[serde(
        rename = "cloud_data_volume_iops",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub data_volume_iops: Option<u32>,

    /// Data volume throughput in MiB/s (gp3: 125 to 1000).
    #[serde(
        rename = "cloud_data_volume_throughput",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub data_volume_throughput: Option<u32>,

    /// When true, the declared `tags:` are also applied to the volumes at launch.
    #[serde(
        rename = "cloud_volume_tags",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub volume_tags: Option<bool>,

    /// Local FILE whose content is passed as the instance's user-data at apply
    /// time (absolute path, `[A-Za-z0-9._/-]`). Only the path is ever used by
    /// forjar; absent: the built-in two-line script.
    #[serde(
        rename = "cloud_user_data_file",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub user_data_file: Option<String>,
    /// IAM instance profile NAME (not an ARN): 1..128 of `[A-Za-z0-9+.@_-]`.
    /// IAM also allows `,` and `=`, but forjar refuses them: they are separators in
    /// the aws shorthand `Name=<value>`.
    #[serde(
        rename = "cloud_instance_profile",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub instance_profile: Option<String>,
}

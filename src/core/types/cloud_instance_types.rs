//! FJ-861: `cloud_instance` declaration fields — ONE spot instance as declared state.
//!
//! Inert by construction: nothing here talks to a cloud. `provider`, `image`
//! (by `@sha256` digest) and `tags` reuse the existing `Resource` fields; the
//! rest live here, `#[serde(flatten)]`-ed so the YAML stays flat.

/// Declaration fields for a `cloud_instance` resource (all optional at the
/// serde layer; `resources::cloud_instance::spec_of` refuses a bad declaration).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CloudSpec {
    /// Cloud region, e.g. `eu-south-2`.
    #[serde(rename = "cloud_region", default)]
    pub region: Option<String>,

    /// Instance type, e.g. `c7a.xlarge`.
    #[serde(rename = "cloud_instance_type", default)]
    pub instance_type: Option<String>,

    /// Spot price ceiling in USD per hour, a decimal string (`0.20`).
    #[serde(rename = "cloud_max_price", default)]
    pub max_price: Option<String>,

    /// Hard lifetime in minutes; the instance is destroyed after it.
    #[serde(rename = "cloud_max_lifetime_min", default)]
    pub max_lifetime_min: Option<u32>,

    /// Daily budget cap in USD, a decimal string. Apply refuses without it.
    #[serde(rename = "cloud_budget_cap_usd_day", default)]
    pub budget_cap_usd_day: Option<String>,

    /// Absolute path of the kill-switch file. While it exists, apply refuses.
    /// Apply refuses if none is declared.
    #[serde(rename = "cloud_kill_switch", default)]
    pub kill_switch: Option<String>,

    /// Absolute path of the parity-gate file. Apply refuses unless it exists
    /// and holds the line `parity-gate: pass`.
    #[serde(rename = "cloud_parity_gate", default)]
    pub parity_gate: Option<String>,
}

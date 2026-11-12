//! forjar#1000: `github_runner` declaration fields — a self-hosted GitHub
//! Actions runner seat, registered through a GitHub App.
//!
//! See `contracts/github-runner-v1.yaml` for what the resource guarantees.
//!
//! `owner`, `repo`, `name`, `group` and `version` are the `Resource` fields of
//! those names (all `Option<String>`); only the fields no other resource type
//! uses live here. Every field skips serialising when unset, so a resource of
//! any other type serialises exactly as it did before these existed and no
//! existing desired-state hash moves.

/// The `github_runner` fields that no other resource type shares.
///
/// `#[serde(flatten)]`-ed into `Resource`, so the YAML shape is flat
/// (`labels:` and `sha256:` are top level, as in the design).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunnerSpec {
    /// Where the seat registers: `org` or `repo`. A repo-scoped seat also needs
    /// `repo`; an org-scoped seat must not have one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,

    /// Labels the seat carries. Compared as a set. Must not be empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,

    /// sha256 of the pinned runner tarball. Required: there is no unpinned path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,

    /// Directory the runner works in (`_work`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_dir: Option<String>,

    /// Unix account the runner and its service unit run as.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// The GitHub App that mints registration tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<RunnerApp>,
}

/// The GitHub App a seat registers through.
///
/// `private_key` is supplied as `{{secrets.NAME}}` and is read on the control
/// host only; it never reaches the seat (contract C1).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RunnerApp {
    /// The App's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_id: Option<u64>,

    /// The installation to mint tokens for; looked up when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installation_id: Option<u64>,

    /// The App's private key, as a `{{secrets.*}}` reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
}

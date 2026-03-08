//! Resource enums: ResourceType and MachineTarget.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Resource type enum.
///
/// ```
/// use forjar::core::types::ResourceType;
/// assert_eq!(ResourceType::Package.to_string(), "package");
/// assert_eq!(ResourceType::default(), ResourceType::Package);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    /// System package (apt, cargo, uv).
    #[default]
    Package,
    /// File or directory.
    File,
    /// Systemd service unit.
    Service,
    /// Filesystem mount point.
    Mount,
    /// Unix user account.
    User,
    /// Docker container.
    Docker,
    /// Pepita namespace-isolated process.
    Pepita,
    /// Network/firewall rule.
    Network,
    /// Cron scheduled job.
    Cron,
    /// Nested recipe inclusion.
    Recipe,
    /// FJ-240: ML model resource type.
    Model,
    /// FJ-241: GPU hardware resource type.
    Gpu,
    /// ALB-027: Pipeline task resource type.
    Task,
    /// FJ-2402: WASM bundle for presentar app deployment.
    WasmBundle,
    /// FJ-2101: OCI container image resource.
    Image,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Package => write!(f, "package"),
            Self::File => write!(f, "file"),
            Self::Service => write!(f, "service"),
            Self::Mount => write!(f, "mount"),
            Self::User => write!(f, "user"),
            Self::Docker => write!(f, "docker"),
            Self::Pepita => write!(f, "pepita"),
            Self::Network => write!(f, "network"),
            Self::Cron => write!(f, "cron"),
            Self::Recipe => write!(f, "recipe"),
            Self::Model => write!(f, "model"),
            Self::Gpu => write!(f, "gpu"),
            Self::Task => write!(f, "task"),
            Self::WasmBundle => write!(f, "wasm_bundle"),
            Self::Image => write!(f, "image"),
        }
    }
}

/// Machine target — single machine or multiple.
///
/// # Examples
///
/// ```
/// use forjar::core::types::MachineTarget;
///
/// let single = MachineTarget::Single("web".to_string());
/// assert_eq!(single.to_vec(), vec!["web"]);
/// assert_eq!(single.to_string(), "web");
///
/// let multi = MachineTarget::Multiple(vec!["web".into(), "db".into()]);
/// assert_eq!(multi.to_vec(), vec!["web", "db"]);
/// assert_eq!(multi.to_string(), "[web, db]");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MachineTarget {
    /// A single machine name.
    Single(String),
    /// Multiple machine names.
    Multiple(Vec<String>),
}

impl Default for MachineTarget {
    fn default() -> Self {
        Self::Single("localhost".to_string())
    }
}

impl fmt::Display for MachineTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(s) => write!(f, "{s}"),
            Self::Multiple(v) => write!(f, "[{}]", v.join(", ")),
        }
    }
}

impl MachineTarget {
    /// Expand to a list of machine names.
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            Self::Single(s) => vec![s.clone()],
            Self::Multiple(v) => v.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_type_display_all_variants() {
        let cases = [
            (ResourceType::Package, "package"),
            (ResourceType::File, "file"),
            (ResourceType::Service, "service"),
            (ResourceType::Mount, "mount"),
            (ResourceType::User, "user"),
            (ResourceType::Docker, "docker"),
            (ResourceType::Pepita, "pepita"),
            (ResourceType::Network, "network"),
            (ResourceType::Cron, "cron"),
            (ResourceType::Recipe, "recipe"),
            (ResourceType::Model, "model"),
            (ResourceType::Gpu, "gpu"),
            (ResourceType::Task, "task"),
            (ResourceType::WasmBundle, "wasm_bundle"),
            (ResourceType::Image, "image"),
        ];
        for (variant, expected) in &cases {
            assert_eq!(variant.to_string(), *expected);
        }
    }

    #[test]
    fn test_resource_type_default() {
        assert_eq!(ResourceType::default(), ResourceType::Package);
    }

    #[test]
    fn test_resource_type_serde_roundtrip() {
        let yaml = "\"file\"";
        let rt: ResourceType = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(rt, ResourceType::File);
        let serialized = serde_yaml_ng::to_string(&rt).unwrap();
        assert!(serialized.trim() == "file");
    }

    #[test]
    fn test_machine_target_single() {
        let t = MachineTarget::Single("web".to_string());
        assert_eq!(t.to_string(), "web");
        assert_eq!(t.to_vec(), vec!["web"]);
    }

    #[test]
    fn test_machine_target_multiple() {
        let t = MachineTarget::Multiple(vec!["a".into(), "b".into()]);
        assert_eq!(t.to_string(), "[a, b]");
        assert_eq!(t.to_vec(), vec!["a", "b"]);
    }

    #[test]
    fn test_machine_target_default() {
        let d = MachineTarget::default();
        assert_eq!(d.to_vec(), vec!["localhost"]);
    }
}

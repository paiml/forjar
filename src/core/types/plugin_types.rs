//! FJ-3400: WASM resource provider plugin types.
//!
//! Defines the plugin manifest format, capability permissions, ABI version,
//! and content-addressed verification for resource provider plugins.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-3400: Plugin ABI version.
///
/// The ABI version determines which host functions are available
/// and the serialization format for state exchange.
pub const PLUGIN_ABI_VERSION: u32 = 1;

/// FJ-3402: Plugin manifest — declares a resource provider plugin.
///
/// Loaded from `plugin.yaml` in the plugin directory. The BLAKE3 hash
/// is verified against the actual `.wasm` file before loading.
///
/// # Examples
///
/// ```
/// use forjar::core::types::PluginManifest;
///
/// let yaml = r#"
/// name: k8s-deployment
/// version: "0.1.0"
/// description: "Manage Kubernetes Deployments"
/// abi_version: 1
/// wasm: k8s-deployment.wasm
/// blake3: "a7f3c2e100000000000000000000000000000000000000000000000000000000"
/// "#;
/// let manifest: PluginManifest = serde_yaml_ng::from_str(yaml).unwrap();
/// assert_eq!(manifest.name, "k8s-deployment");
/// assert_eq!(manifest.abi_version, 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin name (used in `type: "plugin:<name>"`).
    pub name: String,

    /// Plugin version (semver).
    pub version: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// ABI version this plugin targets.
    pub abi_version: u32,

    /// WASM module filename (relative to plugin directory).
    pub wasm: String,

    /// BLAKE3 hash of the WASM module for content-addressed verification.
    pub blake3: String,

    /// Capability permissions granted to this plugin.
    #[serde(default)]
    pub permissions: PluginPermissions,

    /// Resource schema (property validation).
    #[serde(default)]
    pub schema: Option<PluginSchema>,
}

impl PluginManifest {
    /// Verify the BLAKE3 hash of a WASM module against the manifest.
    pub fn verify_hash(&self, wasm_bytes: &[u8]) -> bool {
        let actual = blake3::hash(wasm_bytes).to_hex().to_string();
        actual == self.blake3
    }

    /// Check if the ABI version is compatible with the current host.
    pub fn is_abi_compatible(&self) -> bool {
        self.abi_version == PLUGIN_ABI_VERSION
    }

    /// Resource type string for use in forjar.yaml (`plugin:<name>`).
    pub fn resource_type(&self) -> String {
        format!("plugin:{}", self.name)
    }
}

impl fmt::Display for PluginManifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}@{} (ABI v{})",
            self.name, self.version, self.abi_version
        )
    }
}

/// Capability-based permission set for WASM plugins.
///
/// Plugins run in a sandbox and can only access resources declared here.
/// Follows the principle of least privilege.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginPermissions {
    /// Filesystem access permissions.
    #[serde(default)]
    pub fs: FsPermissions,

    /// Network access permissions.
    #[serde(default)]
    pub net: NetPermissions,

    /// External binary execution permissions.
    #[serde(default)]
    pub exec: ExecPermissions,

    /// Environment variable read permissions.
    #[serde(default)]
    pub env: EnvPermissions,
}

impl PluginPermissions {
    /// Whether the plugin has any permissions at all.
    pub fn is_empty(&self) -> bool {
        self.fs.read.is_empty()
            && self.fs.write.is_empty()
            && self.net.connect.is_empty()
            && self.exec.allow.is_empty()
            && self.env.read.is_empty()
    }
}

/// Filesystem permissions for a plugin.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FsPermissions {
    /// Paths the plugin can read.
    #[serde(default)]
    pub read: Vec<String>,
    /// Paths the plugin can write.
    #[serde(default)]
    pub write: Vec<String>,
}

/// Network permissions for a plugin.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetPermissions {
    /// Host:port pairs the plugin can connect to.
    #[serde(default)]
    pub connect: Vec<String>,
}

/// External binary execution permissions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecPermissions {
    /// Binary names the plugin is allowed to execute.
    #[serde(default)]
    pub allow: Vec<String>,
}

/// Environment variable read permissions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvPermissions {
    /// Environment variable names the plugin can read.
    #[serde(default)]
    pub read: Vec<String>,
}

/// FJ-3408: Plugin resource schema for property validation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginSchema {
    /// Required properties.
    #[serde(default)]
    pub required: Vec<String>,
    /// Property definitions (simplified JSON Schema subset).
    #[serde(default)]
    pub properties: indexmap::IndexMap<String, SchemaProperty>,
}

impl PluginSchema {
    /// Validate a resource's properties against this schema.
    ///
    /// Returns a list of validation errors. Empty means valid.
    pub fn validate(
        &self,
        properties: &indexmap::IndexMap<String, serde_yaml_ng::Value>,
    ) -> Vec<String> {
        let mut errors = Vec::new();

        // Check required fields
        for req in &self.required {
            if !properties.contains_key(req) {
                errors.push(format!("missing required property: {req}"));
            }
        }

        // Check property types
        for (name, value) in properties {
            if let Some(prop_schema) = self.properties.get(name) {
                if let Some(err) = prop_schema.validate_value(name, value) {
                    errors.push(err);
                }
            }
        }

        errors
    }
}

/// Schema property definition (JSON Schema subset).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaProperty {
    /// Property type: "string", "integer", "boolean", "array".
    #[serde(rename = "type", default)]
    pub prop_type: Option<String>,

    /// Default value.
    #[serde(default)]
    pub default: Option<serde_yaml_ng::Value>,

    /// For array types: item schema.
    #[serde(default)]
    pub items: Option<Box<SchemaProperty>>,
}

impl SchemaProperty {
    /// Validate a single value against this property schema.
    fn validate_value(&self, name: &str, value: &serde_yaml_ng::Value) -> Option<String> {
        let expected_type = self.prop_type.as_ref()?;
        let actual_ok = match expected_type.as_str() {
            "string" => value.is_string(),
            "integer" | "number" => value.is_number(),
            "boolean" => value.is_bool(),
            "array" => matches!(value, serde_yaml_ng::Value::Sequence(_)),
            _ => true,
        };
        if actual_ok {
            None
        } else {
            Some(format!(
                "property '{name}' expected type '{expected_type}', got {:?}",
                value
            ))
        }
    }
}

/// Plugin check/apply result status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginStatus {
    /// Resource is in desired state.
    Converged,
    /// Resource needs changes.
    Drifted,
    /// Resource does not exist.
    Missing,
    /// Error checking resource.
    Error,
}

impl fmt::Display for PluginStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Converged => write!(f, "converged"),
            Self::Drifted => write!(f, "drifted"),
            Self::Missing => write!(f, "missing"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Plugin apply outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginApplyOutcome {
    /// Whether the apply succeeded.
    pub success: bool,
    /// Status after apply.
    pub status: PluginStatus,
    /// Changes made (human-readable).
    #[serde(default)]
    pub changes: Vec<String>,
    /// Error message (if !success).
    #[serde(default)]
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_serde_roundtrip() {
        let yaml = r#"
name: k8s-deployment
version: "0.1.0"
description: "Manage K8s Deployments"
abi_version: 1
wasm: k8s-deployment.wasm
blake3: "a7f3c2e100000000000000000000000000000000000000000000000000000000"
permissions:
  fs:
    read: ["~/.kube/config"]
  exec:
    allow: ["kubectl"]
  env:
    read: ["KUBECONFIG"]
"#;
        let m: PluginManifest = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(m.name, "k8s-deployment");
        assert_eq!(m.abi_version, 1);
        assert!(m.is_abi_compatible());
        assert_eq!(m.resource_type(), "plugin:k8s-deployment");
        assert_eq!(m.permissions.fs.read, vec!["~/.kube/config"]);
        assert_eq!(m.permissions.exec.allow, vec!["kubectl"]);
        assert_eq!(m.permissions.env.read, vec!["KUBECONFIG"]);
    }

    #[test]
    fn manifest_display() {
        let m = PluginManifest {
            name: "test".into(),
            version: "1.0.0".into(),
            description: None,
            abi_version: 1,
            wasm: "test.wasm".into(),
            blake3: "abc".into(),
            permissions: PluginPermissions::default(),
            schema: None,
        };
        assert_eq!(m.to_string(), "test@1.0.0 (ABI v1)");
    }

    #[test]
    fn verify_hash_correct() {
        let data = b"fake wasm module bytes";
        let hash = blake3::hash(data).to_hex().to_string();
        let m = PluginManifest {
            name: "t".into(),
            version: "0.1.0".into(),
            description: None,
            abi_version: 1,
            wasm: "t.wasm".into(),
            blake3: hash,
            permissions: PluginPermissions::default(),
            schema: None,
        };
        assert!(m.verify_hash(data));
        assert!(!m.verify_hash(b"tampered bytes"));
    }

    #[test]
    fn abi_compatibility() {
        let m = PluginManifest {
            name: "t".into(),
            version: "0.1.0".into(),
            description: None,
            abi_version: PLUGIN_ABI_VERSION,
            wasm: "t.wasm".into(),
            blake3: "h".into(),
            permissions: PluginPermissions::default(),
            schema: None,
        };
        assert!(m.is_abi_compatible());

        let m2 = PluginManifest {
            abi_version: 99,
            ..m
        };
        assert!(!m2.is_abi_compatible());
    }

    #[test]
    fn permissions_empty() {
        let p = PluginPermissions::default();
        assert!(p.is_empty());
    }

    #[test]
    fn permissions_not_empty() {
        let mut p = PluginPermissions::default();
        p.fs.read.push("/etc/config".into());
        assert!(!p.is_empty());
    }

    #[test]
    fn schema_validate_required() {
        let schema = PluginSchema {
            required: vec!["name".into(), "image".into()],
            properties: indexmap::IndexMap::new(),
        };
        let mut props = indexmap::IndexMap::new();
        props.insert("name".into(), serde_yaml_ng::Value::String("app".into()));

        let errors = schema.validate(&props);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("image"));
    }

    #[test]
    fn schema_validate_types() {
        let mut properties = indexmap::IndexMap::new();
        properties.insert(
            "replicas".into(),
            SchemaProperty {
                prop_type: Some("integer".into()),
                default: None,
                items: None,
            },
        );
        let schema = PluginSchema {
            required: vec![],
            properties,
        };

        // Valid
        let mut props = indexmap::IndexMap::new();
        props.insert(
            "replicas".into(),
            serde_yaml_ng::Value::Number(serde_yaml_ng::Number::from(3)),
        );
        assert!(schema.validate(&props).is_empty());

        // Invalid type
        let mut props = indexmap::IndexMap::new();
        props.insert(
            "replicas".into(),
            serde_yaml_ng::Value::String("three".into()),
        );
        let errors = schema.validate(&props);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("expected type 'integer'"));
    }

    #[test]
    fn schema_validate_all_pass() {
        let schema = PluginSchema {
            required: vec!["name".into()],
            properties: indexmap::IndexMap::new(),
        };
        let mut props = indexmap::IndexMap::new();
        props.insert("name".into(), serde_yaml_ng::Value::String("ok".into()));
        assert!(schema.validate(&props).is_empty());
    }

    #[test]
    fn plugin_status_display() {
        assert_eq!(PluginStatus::Converged.to_string(), "converged");
        assert_eq!(PluginStatus::Drifted.to_string(), "drifted");
        assert_eq!(PluginStatus::Missing.to_string(), "missing");
        assert_eq!(PluginStatus::Error.to_string(), "error");
    }

    #[test]
    fn plugin_apply_outcome_serde() {
        let outcome = PluginApplyOutcome {
            success: true,
            status: PluginStatus::Converged,
            changes: vec!["created deployment".into()],
            error: None,
        };
        let json = serde_json::to_string(&outcome).unwrap();
        let parsed: PluginApplyOutcome = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.status, PluginStatus::Converged);
    }
}

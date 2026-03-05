//! FJ-2002: Extended generation model types.
//!
//! Enhances generation metadata with config tracking (BLAKE3 hash, git ref),
//! undo action recording, and resource delta summaries.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// FJ-2002: Extended generation metadata.
///
/// Stored as `.generation.yaml` in each generation directory. Extends the
/// basic generation/created_at with config hash, git ref, action type,
/// and per-machine resource deltas.
///
/// # Examples
///
/// ```
/// use forjar::core::types::GenerationMeta;
///
/// let meta = GenerationMeta::new(5, "2026-03-05T14:30:00Z".into());
/// assert_eq!(meta.generation, 5);
/// assert!(meta.config_hash.is_none());
/// assert_eq!(meta.action, "apply");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMeta {
    /// Generation number.
    pub generation: u32,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// BLAKE3 hash of the config file used for this generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_hash: Option<String>,
    /// Git commit SHA at apply time (for config recovery via `git show`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    /// Full config YAML snapshot (fallback when git ref unavailable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_snapshot: Option<String>,
    /// Action that created this generation: "apply", "undo", "destroy", "rollback".
    #[serde(default = "default_action")]
    pub action: String,
    /// Parent generation number (for undo chains).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_generation: Option<u32>,
    /// Operator identity (user@hostname).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,
    /// Forjar version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forjar_version: Option<String>,
    /// bashrs version used for script purification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bashrs_version: Option<String>,
    /// Per-machine resource delta summary.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub machines: HashMap<String, MachineDelta>,
}

fn default_action() -> String {
    "apply".to_string()
}

impl GenerationMeta {
    /// Create a new generation metadata with minimal required fields.
    pub fn new(generation: u32, created_at: String) -> Self {
        Self {
            generation,
            created_at,
            config_hash: None,
            git_ref: None,
            config_snapshot: None,
            action: "apply".into(),
            parent_generation: None,
            operator: None,
            forjar_version: None,
            bashrs_version: None,
            machines: HashMap::new(),
        }
    }

    /// Create an undo generation referencing the parent.
    pub fn new_undo(generation: u32, created_at: String, parent: u32) -> Self {
        Self {
            action: "undo".into(),
            parent_generation: Some(parent),
            ..Self::new(generation, created_at)
        }
    }

    /// Set config tracking fields from a config file path.
    pub fn with_config_hash(mut self, hash: String) -> Self {
        self.config_hash = Some(hash);
        self
    }

    /// Set git ref from current HEAD.
    pub fn with_git_ref(mut self, git_ref: String) -> Self {
        self.git_ref = Some(git_ref);
        self
    }

    /// Record a machine delta.
    pub fn record_machine(&mut self, machine: &str, delta: MachineDelta) {
        self.machines.insert(machine.to_string(), delta);
    }

    /// Serialize to YAML string.
    pub fn to_yaml(&self) -> Result<String, String> {
        serde_yaml_ng::to_string(self).map_err(|e| format!("cannot serialize generation meta: {e}"))
    }

    /// Deserialize from YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        serde_yaml_ng::from_str(yaml).map_err(|e| format!("cannot parse generation meta: {e}"))
    }

    /// Total number of changes across all machines.
    pub fn total_changes(&self) -> u32 {
        self.machines.values().map(|d| d.total_changes()).sum()
    }

    /// Whether this generation is an undo.
    pub fn is_undo(&self) -> bool {
        self.action == "undo"
    }
}

/// Per-machine resource delta in a generation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MachineDelta {
    /// Resources created in this generation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub created: Vec<String>,
    /// Resources updated in this generation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub updated: Vec<String>,
    /// Resources destroyed in this generation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub destroyed: Vec<String>,
    /// Resources unchanged (no-op).
    #[serde(default)]
    pub unchanged: u32,
}

impl MachineDelta {
    /// Total number of changes (create + update + destroy).
    pub fn total_changes(&self) -> u32 {
        (self.created.len() + self.updated.len() + self.destroyed.len()) as u32
    }

    /// Whether this machine had any changes.
    pub fn has_changes(&self) -> bool {
        self.total_changes() > 0
    }
}

/// FJ-2002: Get the current git HEAD ref, if in a git repository.
pub fn get_git_ref() -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

/// FJ-2002: Check if the git working tree is dirty (uncommitted changes).
pub fn git_is_dirty() -> bool {
    std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// FJ-2005: Destroy log entry for undo-destroy recovery.
///
/// Records the pre-destroy state of a resource so it can be recreated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestroyLogEntry {
    /// ISO 8601 timestamp of the destroy operation.
    pub timestamp: String,
    /// Machine name.
    pub machine: String,
    /// Resource identifier.
    pub resource_id: String,
    /// Resource type.
    pub resource_type: String,
    /// BLAKE3 hash of the resource's state before destruction.
    pub pre_hash: String,
    /// Generation number when destroyed.
    pub generation: u32,
    /// Key resource config for recreation (serialized YAML fragment).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_fragment: Option<String>,
    /// Whether recreation is reliable (true for files with inline content).
    #[serde(default)]
    pub reliable_recreate: bool,
}

impl DestroyLogEntry {
    /// Format as a JSONL line for appending to destroy-log.jsonl.
    pub fn to_jsonl(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| format!("cannot serialize destroy log entry: {e}"))
    }

    /// Parse from a JSONL line.
    pub fn from_jsonl(line: &str) -> Result<Self, String> {
        serde_json::from_str(line)
            .map_err(|e| format!("cannot parse destroy log entry: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_meta_new() {
        let meta = GenerationMeta::new(0, "2026-01-01T00:00:00Z".into());
        assert_eq!(meta.generation, 0);
        assert_eq!(meta.action, "apply");
        assert!(meta.config_hash.is_none());
        assert!(meta.git_ref.is_none());
        assert!(meta.machines.is_empty());
    }

    #[test]
    fn generation_meta_undo() {
        let meta = GenerationMeta::new_undo(3, "2026-01-01T00:00:00Z".into(), 1);
        assert_eq!(meta.action, "undo");
        assert_eq!(meta.parent_generation, Some(1));
        assert!(meta.is_undo());
    }

    #[test]
    fn generation_meta_with_config() {
        let meta = GenerationMeta::new(1, "ts".into())
            .with_config_hash("blake3:abc123".into())
            .with_git_ref("a1b2c3d".into());
        assert_eq!(meta.config_hash.as_deref(), Some("blake3:abc123"));
        assert_eq!(meta.git_ref.as_deref(), Some("a1b2c3d"));
    }

    #[test]
    fn generation_meta_yaml_roundtrip() {
        let mut meta = GenerationMeta::new(5, "2026-03-05T14:30:00Z".into());
        meta.config_hash = Some("blake3:abc".into());
        meta.operator = Some("noah@host".into());
        meta.record_machine("intel", MachineDelta {
            created: vec!["pkg-a".into()],
            updated: vec!["config-b".into()],
            destroyed: vec![],
            unchanged: 10,
        });

        let yaml = meta.to_yaml().unwrap();
        let parsed = GenerationMeta::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.generation, 5);
        assert_eq!(parsed.config_hash.as_deref(), Some("blake3:abc"));
        assert_eq!(parsed.machines.len(), 1);
        assert_eq!(parsed.machines["intel"].created, vec!["pkg-a"]);
    }

    #[test]
    fn generation_meta_total_changes() {
        let mut meta = GenerationMeta::new(1, "ts".into());
        meta.record_machine("a", MachineDelta {
            created: vec!["x".into()],
            updated: vec!["y".into()],
            destroyed: vec![],
            unchanged: 5,
        });
        meta.record_machine("b", MachineDelta {
            created: vec![],
            updated: vec![],
            destroyed: vec!["z".into()],
            unchanged: 3,
        });
        assert_eq!(meta.total_changes(), 3);
    }

    #[test]
    fn generation_meta_no_changes() {
        let meta = GenerationMeta::new(0, "ts".into());
        assert_eq!(meta.total_changes(), 0);
        assert!(!meta.is_undo());
    }

    #[test]
    fn machine_delta_total() {
        let delta = MachineDelta {
            created: vec!["a".into(), "b".into()],
            updated: vec!["c".into()],
            destroyed: vec!["d".into()],
            unchanged: 10,
        };
        assert_eq!(delta.total_changes(), 4);
        assert!(delta.has_changes());
    }

    #[test]
    fn machine_delta_empty() {
        let delta = MachineDelta::default();
        assert_eq!(delta.total_changes(), 0);
        assert!(!delta.has_changes());
    }

    #[test]
    fn destroy_log_entry_jsonl_roundtrip() {
        let entry = DestroyLogEntry {
            timestamp: "2026-03-05T14:30:00Z".into(),
            machine: "intel".into(),
            resource_id: "nginx-pkg".into(),
            resource_type: "package".into(),
            pre_hash: "blake3:aaa".into(),
            generation: 5,
            config_fragment: Some("state: present\nname: nginx".into()),
            reliable_recreate: false,
        };
        let line = entry.to_jsonl().unwrap();
        let parsed = DestroyLogEntry::from_jsonl(&line).unwrap();
        assert_eq!(parsed.resource_id, "nginx-pkg");
        assert_eq!(parsed.generation, 5);
        assert!(!parsed.reliable_recreate);
    }

    #[test]
    fn destroy_log_entry_reliable() {
        let entry = DestroyLogEntry {
            timestamp: "ts".into(),
            machine: "m".into(),
            resource_id: "config-file".into(),
            resource_type: "file".into(),
            pre_hash: "blake3:bbb".into(),
            generation: 2,
            config_fragment: Some("content: inline\npath: /etc/app.conf".into()),
            reliable_recreate: true,
        };
        assert!(entry.reliable_recreate);
    }

    #[test]
    fn get_git_ref_returns_something() {
        // This test runs in a git repo, so it should return Some
        let ref_opt = get_git_ref();
        assert!(ref_opt.is_some());
        let git_ref = ref_opt.unwrap();
        assert!(!git_ref.is_empty());
    }

    #[test]
    fn generation_meta_backward_compat() {
        // Old-format YAML (just generation + created_at) should parse
        let yaml = "generation: 3\ncreated_at: '2026-01-01T00:00:00Z'\n";
        let meta = GenerationMeta::from_yaml(yaml).unwrap();
        assert_eq!(meta.generation, 3);
        assert_eq!(meta.action, "apply"); // default
        assert!(meta.config_hash.is_none());
        assert!(meta.machines.is_empty());
    }

    #[test]
    fn generation_meta_skip_serializing_empty() {
        let meta = GenerationMeta::new(0, "ts".into());
        let yaml = meta.to_yaml().unwrap();
        // Should not contain optional empty fields
        assert!(!yaml.contains("config_hash"));
        assert!(!yaml.contains("git_ref"));
        assert!(!yaml.contains("machines"));
        assert!(!yaml.contains("parent_generation"));
    }
}

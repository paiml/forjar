//! Tests: Destroy — state cleanup.

#![allow(unused_imports)]
use super::commands::*;
use super::destroy::*;
use super::helpers::*;
use super::helpers_state::*;
use crate::core::{state, types};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    /// FJ-2005: cleanup_succeeded_entries removes only specified entries.
    #[test]
    fn cleanup_succeeded_entries_partial() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path();
        let machine_dir = state_dir.join("m1");
        std::fs::create_dir_all(&machine_dir).unwrap();

        let rl = |hash: &str| types::ResourceLock {
            resource_type: types::ResourceType::File,
            status: types::ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: hash.into(),
            details: std::collections::HashMap::new(),
        };
        let mut resources = indexmap::IndexMap::new();
        resources.insert("r1".into(), rl("h1"));
        resources.insert("r2".into(), rl("h2"));
        let lock = types::StateLock {
            schema: "1.0".into(),
            machine: "m1".into(),
            hostname: "m1".into(),
            generated_at: "now".into(),
            generator: "forjar".into(),
            blake3_version: "1.8".into(),
            resources,
        };
        let yaml = serde_yaml_ng::to_string(&lock).unwrap();
        std::fs::write(machine_dir.join("state.lock.yaml"), yaml).unwrap();

        let mut succeeded = std::collections::HashMap::new();
        succeeded.insert("m1".to_string(), vec!["r1".to_string()]);
        cleanup_succeeded_entries(state_dir, &succeeded);

        let remaining = std::fs::read_to_string(machine_dir.join("state.lock.yaml")).unwrap();
        let remaining_lock: types::StateLock = serde_yaml_ng::from_str(&remaining).unwrap();
        assert!(!remaining_lock.resources.contains_key("r1"));
        assert!(remaining_lock.resources.contains_key("r2"));
    }
}

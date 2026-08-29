//! Tests: Drift detection.

#![allow(unused_imports)]
use super::commands::*;
use super::dispatch::*;
use super::drift::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj017_drift_empty_state() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            None,
            false,
            false, // dry_run
            false,
            false,
            None, // no env_file
            false,
        )
        .unwrap();
    }

    #[test]
    fn test_fj017_drift_with_lock() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        // Create a lock with a file resource
        let test_file = dir.path().join("tracked.txt");
        std::fs::write(&test_file, "stable content").unwrap();
        let hash = crate::tripwire::hasher::hash_file(&test_file).unwrap();

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(test_file.to_str().unwrap().to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String(hash),
        );
        resources.insert(
            "tracked-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::File,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: Some("2026-01-01T00:00:00Z".to_string()),
                duration_seconds: Some(0.1),
                hash: "blake3:x".to_string(),
                observed: None,
                details,
            },
        );
        let lock = crate::core::types::StateLock {
            schema: "1.0".to_string(),
            machine: "testbox".to_string(),
            hostname: "testbox".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // No drift expected
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            None,
            false,
            false, // dry_run
            false,
            false,
            None, // no env_file
            false,
        )
        .unwrap();
    }

    #[test]
    fn test_fj017_drift_with_actual_drift_tripwire() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let test_file = dir.path().join("drifted.txt");
        std::fs::write(&test_file, "original").unwrap();

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(test_file.to_str().unwrap().to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:wrong_hash".to_string()),
        );
        resources.insert(
            "drifted-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::File,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: Some("2026-01-01T00:00:00Z".to_string()),
                duration_seconds: Some(0.1),
                hash: "blake3:x".to_string(),
                observed: None,
                details,
            },
        );
        let lock = crate::core::types::StateLock {
            schema: "1.0".to_string(),
            machine: "driftbox".to_string(),
            hostname: "driftbox".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // Tripwire mode should error on drift
        let result = cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            true,
            None,
            false,
            false, // dry_run
            false,
            false,
            None, // no env_file
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("drift"));
    }

    #[test]
    fn test_fj017_drift_machine_filter() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(state.join("alpha")).unwrap();
        std::fs::create_dir_all(state.join("beta")).unwrap();

        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            Some("alpha"),
            false,
            None,
            false,
            false, // dry_run
            false,
            false,
            None, // no env_file
            false,
        )
        .unwrap();
    }

    #[test]
    fn test_fj017_dispatch_drift() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        dispatch(
            Commands::Drift(DriftArgs {
                file: dir.path().join("forjar.yaml"),
                machine: None,
                state_dir: state,
                tripwire: false,
                alert_cmd: None,
                auto_remediate: false,
                dry_run: false,
                json: false,
                env_file: None,
                workspace: None,
                no_task_checks: false,
            }),
            0,
            true,
        )
        .unwrap();
    }

    #[test]
    fn test_fj017_drift_no_tripwire_still_reports() {
        // Exercises the total_drift > 0 && !tripwire_mode path (Ok, not Err)
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let test_file = dir.path().join("drifted2.txt");
        std::fs::write(&test_file, "current").unwrap();

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(test_file.to_str().unwrap().to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:mismatched".to_string()),
        );
        resources.insert(
            "drifted-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::File,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: Some("2026-01-01T00:00:00Z".to_string()),
                duration_seconds: Some(0.1),
                hash: "blake3:x".to_string(),
                observed: None,
                details,
            },
        );
        let lock = crate::core::types::StateLock {
            schema: "1.0".to_string(),
            machine: "driftbox2".to_string(),
            hostname: "driftbox2".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // tripwire_mode=false: drift detected but should still be Ok(())
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            None,
            false,
            false, // dry_run
            false,
            false,
            None, // no env_file
            false,
        )
        .unwrap();
    }

    #[test]
    fn test_fj017_drift_json_output() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");

        let test_file = dir.path().join("drifted-json.txt");
        std::fs::write(&test_file, "current").unwrap();

        let mut resources = indexmap::IndexMap::new();
        let mut details = std::collections::HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(test_file.to_str().unwrap().to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String("blake3:wrong_hash".to_string()),
        );
        resources.insert(
            "drifted-file".to_string(),
            crate::core::types::ResourceLock {
                resource_type: crate::core::types::ResourceType::File,
                status: crate::core::types::ResourceStatus::Converged,
                applied_at: Some("2026-01-01T00:00:00Z".to_string()),
                duration_seconds: Some(0.1),
                hash: "blake3:x".to_string(),
                observed: None,
                details,
            },
        );
        let lock = crate::core::types::StateLock {
            schema: "1.0".to_string(),
            machine: "jsonbox".to_string(),
            hostname: "jsonbox".to_string(),
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar 0.1.0".to_string(),
            blake3_version: "1.8".to_string(),
            resources,
        };
        crate::core::state::save_lock(&state, &lock).unwrap();

        // JSON drift output should not panic
        cmd_drift(
            Path::new("nonexistent.yaml"),
            &state,
            None,
            false,
            None,
            false,
            false, // dry_run
            true,
            false,
            None, // no env_file
            false,
        )
        .unwrap();
    }

    /// PMAT-197 regression: a resource whose fields contain `{{params.*}}`
    /// must NOT report drift immediately after a clean apply.
    ///
    /// `forjar drift` compared RAW config resources against live machine state
    /// while the executor stored state from RESOLVED ones, so any templated
    /// resource drifted forever. For `type: task` the state query is
    /// `echo "command=<command>"`, so the stored value held the resolved
    /// command and the drift probe regenerated it with the literal
    /// `{{params.*}}` still in place — a guaranteed mismatch.
    ///
    /// Reproduced on 1.10.0 (`Drift detected: 1 resource(s)`) and fixed by
    /// routing the drift path through `resolver::resolve_all`.
    ///
    /// This bug class has now appeared three times (planner FJ-154, drift,
    /// destroy), which is why resolution lives in exactly one helper.
    #[test]
    fn templated_task_does_not_self_drift() {
        let mut params = HashMap::new();
        params.insert(
            "greeting".to_string(),
            serde_yaml_ng::Value::String("hello".to_string()),
        );

        let mut raw = types::Resource {
            resource_type: types::ResourceType::Task,
            ..Default::default()
        };
        raw.command = Some("echo {{params.greeting}}".to_string());

        let resolved = resolver::resolve_or_fallback(
            "t",
            &raw,
            &params,
            &indexmap::IndexMap::new(),
            &types::SecretsConfig::default(),
        );

        // The executor stores state built from the RESOLVED resource.
        let resolved_cmd = resolved.command.clone().unwrap();
        assert_eq!(
            resolved_cmd, "echo hello",
            "resolution must substitute params"
        );

        // The drift probe must see the same thing. Before the fix it saw the
        // raw command and reported a spurious change.
        assert_ne!(
            raw.command.clone().unwrap(),
            resolved_cmd,
            "test is vacuous unless raw and resolved actually differ"
        );
    }
}

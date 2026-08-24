//! Tests: FJ-1378 Pre-apply drift gate.

#[cfg(test)]
mod tests {
    use crate::cli::apply::cmd_apply;
    use crate::core::state;
    use crate::core::types::{ResourceLock, ResourceStatus, ResourceType, StateLock};
    use indexmap::IndexMap;
    use std::collections::HashMap;

    /// Helper: create a minimal lock with a file resource that has content_hash.
    fn write_lock_with_file(
        state_dir: &std::path::Path,
        machine: &str,
        resource_id: &str,
        file_path: &str,
        content_hash: &str,
    ) {
        let machine_dir = state_dir.join(machine);
        std::fs::create_dir_all(&machine_dir).unwrap();

        let mut details = HashMap::new();
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(file_path.to_string()),
        );
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String(content_hash.to_string()),
        );

        let mut resources = IndexMap::new();
        resources.insert(
            resource_id.to_string(),
            ResourceLock {
                resource_type: ResourceType::File,
                status: ResourceStatus::Converged,
                applied_at: Some("2026-03-03T12:00:00Z".to_string()),
                duration_seconds: Some(0.01),
                hash: content_hash.to_string(),
                details,
            },
        );

        let lock = StateLock {
            schema: "v1".to_string(),
            machine: machine.to_string(),
            hostname: "localhost".to_string(),
            generated_at: "2026-03-03T12:00:00Z".to_string(),
            generator: "forjar-test".to_string(),
            blake3_version: "1.5.5".to_string(),
            resources,
        };

        state::save_lock(state_dir, &lock).unwrap();
    }

    #[test]
    fn test_fj1378_drift_is_reconciled_not_blocked() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        let file_path = dir.path().join("managed.txt");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Write managed file
        std::fs::write(&file_path, "original content").unwrap();
        let original_hash = crate::tripwire::hasher::hash_file(&file_path).unwrap();

        // Write config
        std::fs::write(
            &config_path,
            format!(
                r#"
version: "1.0"
name: test-drift
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  managed-file:
    type: file
    machine: local
    path: {}
    content: "original content"
policy:
  tripwire: true
"#,
                file_path.display()
            ),
        )
        .unwrap();

        // Write lock with original hash
        write_lock_with_file(
            &state_dir,
            "local",
            "managed-file",
            &file_path.to_string_lossy(),
            &original_hash,
        );

        // Simulate drift by modifying the file
        std::fs::write(&file_path, "DRIFTED content").unwrap();

        // APPLY MUST RECONCILE THE DRIFT, NOT REFUSE.
        //
        // This test asserted `result.is_err()` and that the error mentioned
        // "drift" — the FJ-1378 gate, which blocked the apply and told the
        // operator to re-run with `--force`. forjar#305 changed that, for two
        // reasons: converging observed drift is the one job an IaC apply
        // exists to do, and `--force` is not a repair — it empties the lock
        // map so EVERY resource re-applies, with no way to fix just the one
        // that moved.
        //
        // The assertion is now on the FILE. An exit code alone would pass for
        // a build that succeeded and wrote nothing, which is exactly the shape
        // the original defect had.
        let result = cmd_apply(
            &config_path,
            &state_dir,
            None,
            None,
            None,
            None,
            false, // force=false — drift should block
            false,
            false, // no_tripwire=false — drift check enabled
            &[],
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            false,
            None,
            false,
            false,
            0,
            true,  // yes — skip confirmation
            false,
            None,
            false,
            None,
            None,
            None,
            false,
            None,
            false,
            None, // telemetry_endpoint
            false, // refresh
            None, // force_tag
        &[],
        );
        assert!(
            result.is_ok(),
            "apply refused a drifted resource instead of reconciling it: {result:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&file_path).unwrap(),
            "original content",
            "apply did not restore the declared content — the drift survived"
        );
    }

    #[test]
    fn test_fj1378_drift_gate_force_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        let file_path = dir.path().join("managed2.txt");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Write managed file
        std::fs::write(&file_path, "original").unwrap();
        let original_hash = crate::tripwire::hasher::hash_file(&file_path).unwrap();

        std::fs::write(
            &config_path,
            format!(
                r#"
version: "1.0"
name: test-drift-force
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  managed-file:
    type: file
    machine: local
    path: {}
    content: "original"
policy:
  tripwire: true
"#,
                file_path.display()
            ),
        )
        .unwrap();

        write_lock_with_file(
            &state_dir,
            "local",
            "managed-file",
            &file_path.to_string_lossy(),
            &original_hash,
        );

        // Drift the file
        std::fs::write(&file_path, "DRIFTED").unwrap();

        // Apply with force=true should succeed (override drift gate)
        let result = cmd_apply(
            &config_path,
            &state_dir,
            None,
            None,
            None,
            None,
            true, // force=true — override drift
            false,
            false,
            &[],
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            false,
            None,
            false,
            false,
            0,
            true,
            false,
            None,
            false,
            None,
            None,
            None,
            false,
            None,
            false,
            None, // telemetry_endpoint
            false, // refresh
            None, // force_tag
        &[],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj1378_drift_gate_no_tripwire_skips() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        let file_path = dir.path().join("managed3.txt");
        std::fs::create_dir_all(&state_dir).unwrap();

        std::fs::write(&file_path, "original").unwrap();
        let original_hash = crate::tripwire::hasher::hash_file(&file_path).unwrap();

        std::fs::write(
            &config_path,
            format!(
                r#"
version: "1.0"
name: test-no-tripwire
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  managed-file:
    type: file
    machine: local
    path: {}
    content: "original"
policy:
  tripwire: true
"#,
                file_path.display()
            ),
        )
        .unwrap();

        write_lock_with_file(
            &state_dir,
            "local",
            "managed-file",
            &file_path.to_string_lossy(),
            &original_hash,
        );

        // Drift
        std::fs::write(&file_path, "DRIFTED").unwrap();

        // Apply with no_tripwire=true should succeed (drift check skipped)
        let result = cmd_apply(
            &config_path,
            &state_dir,
            None,
            None,
            None,
            None,
            false,
            false,
            true, // no_tripwire=true — skip drift check
            &[],
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            false,
            None,
            false,
            false,
            0,
            true,
            false,
            None,
            false,
            None,
            None,
            None,
            false,
            None,
            false,
            None, // telemetry_endpoint
            false, // refresh
            None, // force_tag
        &[],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj1378_drift_gate_no_drift_passes() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state_dir = dir.path().join("state");
        let file_path = dir.path().join("managed4.txt");
        std::fs::create_dir_all(&state_dir).unwrap();

        std::fs::write(&file_path, "original").unwrap();
        let original_hash = crate::tripwire::hasher::hash_file(&file_path).unwrap();

        std::fs::write(
            &config_path,
            format!(
                r#"
version: "1.0"
name: test-no-drift
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  managed-file:
    type: file
    machine: local
    path: {}
    content: "original"
policy:
  tripwire: true
"#,
                file_path.display()
            ),
        )
        .unwrap();

        write_lock_with_file(
            &state_dir,
            "local",
            "managed-file",
            &file_path.to_string_lossy(),
            &original_hash,
        );

        // No drift — file unchanged
        let result = cmd_apply(
            &config_path,
            &state_dir,
            None,
            None,
            None,
            None,
            false,
            false,
            false,
            &[],
            false,
            None,
            false,
            false,
            None,
            None,
            false,
            false,
            None,
            false,
            false,
            0,
            true,
            false,
            None,
            false,
            None,
            None,
            None,
            false,
            None,
            false,
            None, // telemetry_endpoint
            false, // refresh
            None, // force_tag
        &[],
        );
        assert!(result.is_ok());
    }
}

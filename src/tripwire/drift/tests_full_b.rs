//! Continuation of `tests_full.rs`, split to stay under the 500-line budget.
//! Same subject; the `_b` suffix is this repo's existing convention for it.

use super::tests_full::{make_service_resource, make_test_machine};
use super::*;
use crate::core::types::Machine;
use crate::tripwire::hasher;

#[test]
fn test_fj016_detect_drift_full_file_plus_service() {
    // Mixed: file resource (local) + service resource (live_hash) in same lock
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("mixed.txt");
    std::fs::write(&file, "stable").unwrap();
    let file_hash = hasher::hash_file(&file).unwrap();

    let mut config_resources = indexmap::IndexMap::new();
    config_resources.insert("my-svc".to_string(), make_service_resource(Some("nginx")));

    let machine = make_test_machine();

    // Run the real state query to get current live_hash
    let query =
        crate::core::codegen::state_query_script(config_resources.get("my-svc").unwrap()).unwrap();
    let output = crate::transport::exec_script(&machine, &query).unwrap();
    let svc_live_hash = hasher::hash_string_or_sentinel(&output.stdout);

    let mut lock_resources = indexmap::IndexMap::new();

    // File resource — no drift
    let mut file_details = std::collections::HashMap::new();
    file_details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(file.to_str().unwrap().to_string()),
    );
    file_details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String(file_hash),
    );
    lock_resources.insert(
        "my-file".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::File,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:desired".to_string(),
            observed: None,
            details: file_details,
        },
    );

    // Service resource — no drift (live_hash matches)
    let mut svc_details = std::collections::HashMap::new();
    svc_details.insert(
        "live_hash".to_string(),
        serde_yaml_ng::Value::String(svc_live_hash),
    );
    lock_resources.insert(
        "my-svc".to_string(),
        crate::core::types::ResourceLock {
            resource_type: ResourceType::Service,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: "blake3:desired".to_string(),
            observed: None,
            details: svc_details,
        },
    );

    let lock = StateLock {
        schema: "1.0".to_string(),
        machine: "test".to_string(),
        hostname: "test".to_string(),
        generated_at: "now".to_string(),
        generator: "test".to_string(),
        blake3_version: "1.8".to_string(),
        resources: lock_resources,
    };

    let findings = detect_drift_full(&lock, &machine, &config_resources);
    assert!(
        findings.is_empty(),
        "no drift expected when both file and service hashes match"
    );
}

// ── forjar: drift must ask the MACHINE, not the controller ───────────────────

/// `check_file_resource_drift` routed through the transport only for CONTAINER
/// transports. Every other machine — including plain SSH — fell to
/// `check_file_drift`, which takes no machine and hashes the CONTROLLER's
/// filesystem, then reports the answer as the remote host's state.
///
/// That is forjar#305's root cause, still live in the other arm. Measured
/// against a real SSH host before the fix:
///
///     file          : present on the CONTROLLER, ABSENT on intel
///     content_hash  : matches the controller's copy
///     drift(intel)  -> "No drift detected."
///
/// A false CLEAN over a file that does not exist on the target. The inverse is
/// equally reachable: a controller holding different bytes at the same path
/// yields a false DRIFT for a host that is perfectly converged.
///
/// A real two-host test cannot run in CI, so this uses an UNREACHABLE machine
/// instead. The logic is the same and the discrimination is exact: if the check
/// consults the controller it finds the file, sees a matching hash and reports
/// clean; if it consults the machine the transport fails and it must say so.
/// Silence here means it read the wrong filesystem.
#[test]
fn file_drift_consults_the_machine_not_the_controller() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("probe.txt");
    std::fs::write(&path, b"CONTROLLER-ONLY\n").unwrap();
    let expected = crate::tripwire::hasher::hash_file(&path).expect("hash");

    let mut rl = crate::core::types::ResourceLock {
        resource_type: ResourceType::File,
        status: ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: "blake3:whatever".to_string(),
        observed: None,
        details: std::collections::HashMap::new(),
    };
    rl.details.insert(
        "path".to_string(),
        serde_yaml_ng::Value::String(path.display().to_string()),
    );
    rl.details.insert(
        "content_hash".to_string(),
        serde_yaml_ng::Value::String(expected.clone()),
    );

    // `.invalid` is reserved by RFC 2606 and never resolves, so the transport
    // fails immediately rather than waiting out DRIFT_QUERY_TIMEOUT_SECS.
    let machine = Machine {
        hostname: "nowhere.invalid".to_string(),
        addr: "nowhere.invalid".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    };

    // The lock entry names the file; the checker takes that pair, not the entry.
    let (path_str, expected_hash) =
        super::file::locked_file_target(&rl).expect("the lock entry carries path + content_hash");
    let finding =
        super::file::check_file_resource_drift("f", path_str, expected_hash, Some(&machine));
    assert!(
        finding.is_some(),
        "drift reported CLEAN for an unreachable machine — it hashed the \
         controller's copy of the file and called that the machine's state"
    );

    // The control: with NO machine, the controller IS the only filesystem there
    // is, and reading it is the honest best effort rather than a wrong answer
    // about somewhere else. It must still report clean for a matching file.
    assert!(
        super::file::check_file_resource_drift("f", path_str, expected_hash, None).is_none(),
        "with no machine known, a file matching its content_hash must be clean"
    );
}

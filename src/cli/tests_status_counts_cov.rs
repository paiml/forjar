//! Coverage tests for status_counts.rs — convergence, failed, drift counts.

use super::status_counts::*;
use crate::core::types;
use std::collections::HashMap;

fn write_lock(state_dir: &std::path::Path, machine: &str, resources: Vec<(&str, types::ResourceStatus)>) {
    // discover_machines needs: state_dir/{machine}/state.lock.yaml
    let machine_dir = state_dir.join(machine);
    std::fs::create_dir_all(&machine_dir).unwrap();

    let mut res_map = indexmap::IndexMap::new();
    for (id, status) in &resources {
        res_map.insert(
            id.to_string(),
            types::ResourceLock {
                resource_type: types::ResourceType::File,
                status: status.clone(),
                applied_at: Some("2026-01-01T00:00:00Z".to_string()),
                duration_seconds: Some(0.1),
                hash: "blake3:aabbccdd".to_string(),
                details: HashMap::new(),
            },
        );
    }
    let lock = types::StateLock {
        schema: "1.0".to_string(),
        machine: machine.to_string(),
        hostname: machine.to_string(),
        generated_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "forjar 0.1.0".to_string(),
        blake3_version: "1.8".to_string(),
        resources: res_map.clone(),
    };

    // Save via state module for discover_machines
    let yaml = serde_yaml_ng::to_string(&lock).unwrap();
    std::fs::write(machine_dir.join("state.lock.yaml"), &yaml).unwrap();
    // Also write flat lock for tally_machine_health
    std::fs::write(state_dir.join(format!("{machine}.lock.yaml")), &yaml).unwrap();
}

// ── cmd_status_convergence_percentage ─────────────────────────────────

#[test]
fn convergence_all_converged() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web", vec![
        ("a", types::ResourceStatus::Converged),
        ("b", types::ResourceStatus::Converged),
    ]);
    assert!(cmd_status_convergence_percentage(dir.path(), None, false).is_ok());
}

#[test]
fn convergence_mixed_status() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web", vec![
        ("a", types::ResourceStatus::Converged),
        ("b", types::ResourceStatus::Failed),
        ("c", types::ResourceStatus::Drifted),
    ]);
    assert!(cmd_status_convergence_percentage(dir.path(), None, false).is_ok());
}

#[test]
fn convergence_json_output() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web", vec![
        ("a", types::ResourceStatus::Converged),
    ]);
    assert!(cmd_status_convergence_percentage(dir.path(), None, true).is_ok());
}

#[test]
fn convergence_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web", vec![
        ("a", types::ResourceStatus::Converged),
    ]);
    write_lock(dir.path(), "db", vec![
        ("b", types::ResourceStatus::Failed),
    ]);
    assert!(cmd_status_convergence_percentage(dir.path(), Some("web"), false).is_ok());
}

#[test]
fn convergence_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    assert!(cmd_status_convergence_percentage(dir.path(), None, false).is_ok());
}

// ── cmd_status_failed_count ──────────────────────────────────────────

#[test]
fn failed_count_none() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web", vec![
        ("a", types::ResourceStatus::Converged),
    ]);
    assert!(cmd_status_failed_count(dir.path(), None, false).is_ok());
}

#[test]
fn failed_count_some() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web", vec![
        ("a", types::ResourceStatus::Failed),
        ("b", types::ResourceStatus::Failed),
    ]);
    assert!(cmd_status_failed_count(dir.path(), None, false).is_ok());
}

#[test]
fn failed_count_json() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web", vec![
        ("a", types::ResourceStatus::Failed),
    ]);
    assert!(cmd_status_failed_count(dir.path(), None, true).is_ok());
}

#[test]
fn failed_count_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web", vec![
        ("a", types::ResourceStatus::Failed),
    ]);
    assert!(cmd_status_failed_count(dir.path(), Some("web"), false).is_ok());
}

// ── cmd_status_drift_count ───────────────────────────────────────────

#[test]
fn drift_count_none() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web", vec![
        ("a", types::ResourceStatus::Converged),
    ]);
    assert!(cmd_status_drift_count(dir.path(), None, false).is_ok());
}

#[test]
fn drift_count_some() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web", vec![
        ("a", types::ResourceStatus::Drifted),
        ("b", types::ResourceStatus::Drifted),
    ]);
    assert!(cmd_status_drift_count(dir.path(), None, false).is_ok());
}

#[test]
fn drift_count_json() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web", vec![
        ("a", types::ResourceStatus::Drifted),
    ]);
    assert!(cmd_status_drift_count(dir.path(), None, true).is_ok());
}

#[test]
fn drift_count_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web", vec![
        ("a", types::ResourceStatus::Drifted),
    ]);
    write_lock(dir.path(), "db", vec![
        ("b", types::ResourceStatus::Converged),
    ]);
    assert!(cmd_status_drift_count(dir.path(), Some("db"), true).is_ok());
}

// ── multi-machine scenarios ──────────────────────────────────────────

#[test]
fn multi_machine_convergence() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "web", vec![
        ("a", types::ResourceStatus::Converged),
        ("b", types::ResourceStatus::Converged),
    ]);
    write_lock(dir.path(), "db", vec![
        ("c", types::ResourceStatus::Failed),
        ("d", types::ResourceStatus::Converged),
    ]);
    write_lock(dir.path(), "cache", vec![
        ("e", types::ResourceStatus::Drifted),
    ]);
    assert!(cmd_status_convergence_percentage(dir.path(), None, false).is_ok());
    assert!(cmd_status_convergence_percentage(dir.path(), None, true).is_ok());
    assert!(cmd_status_failed_count(dir.path(), None, false).is_ok());
    assert!(cmd_status_drift_count(dir.path(), None, false).is_ok());
}

#[test]
fn convergence_json_multiple_machines() {
    let dir = tempfile::tempdir().unwrap();
    write_lock(dir.path(), "a", vec![
        ("x", types::ResourceStatus::Converged),
    ]);
    write_lock(dir.path(), "b", vec![
        ("y", types::ResourceStatus::Failed),
    ]);
    assert!(cmd_status_convergence_percentage(dir.path(), None, true).is_ok());
    assert!(cmd_status_failed_count(dir.path(), None, true).is_ok());
    assert!(cmd_status_drift_count(dir.path(), None, true).is_ok());
}

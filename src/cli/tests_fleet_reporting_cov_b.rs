//! Coverage tests for fleet_reporting.rs — compliance, export, suggest.

// ── cmd_compliance ───────────────────────────────────────────────────

#[test]
fn audit_with_event_log_json() {
    let state_dir = tempfile::tempdir().unwrap();
    let machine_dir = state_dir.path().join("web");
    std::fs::create_dir_all(&machine_dir).unwrap();
    let event = r#"{"ts":"2026-03-08T10:00:00Z","event":{"ApplyStarted":{"machine":"web","run_id":"run-1","config_hash":"abc"}}}"#;
    std::fs::write(machine_dir.join("events.jsonl"), format!("{event}\n")).unwrap();
    let result = super::fleet_reporting::cmd_audit(state_dir.path(), None, 20, true);
    assert!(result.is_ok());
}

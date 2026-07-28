//! Coverage tests for check.rs — pure filter/skip helpers.
//!
//! Split out of `tests_check_cov.rs` to keep both files under the 500-line
//! limit; these exercise the pure helpers with no config fixture.
use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}
#[test]
fn resource_filter_no_filters() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\n").unwrap();
    let (skip, count) = super::check::check_resource_filters("r1", &resource, None, None);
    assert!(!skip);
    assert!(!count);
}

#[test]
fn resource_filter_name_match() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\n").unwrap();
    let (skip, count) =
        super::check::check_resource_filters("cfg", &resource, Some("cfg"), None);
    assert!(!skip);
    assert!(!count);
}

#[test]
fn resource_filter_name_mismatch() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\n").unwrap();
    let (skip, count) =
        super::check::check_resource_filters("cfg", &resource, Some("other"), None);
    assert!(skip);
    assert!(!count);
}

#[test]
fn resource_filter_tag_match() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\ntags: [web, app]\n")
            .unwrap();
    let (skip, count) =
        super::check::check_resource_filters("cfg", &resource, None, Some("web"));
    assert!(!skip);
    assert!(!count);
}

#[test]
fn resource_filter_tag_mismatch() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\ntags: [db]\n").unwrap();
    let (skip, count) =
        super::check::check_resource_filters("cfg", &resource, None, Some("web"));
    assert!(skip);
    assert!(count);
}

// ── skip_machine helper ─────────────────────────────────────────────

#[test]
fn skip_machine_no_filter() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\n").unwrap();
    let machine = super::check::localhost_machine();
    assert!(!super::check::skip_machine("local", None, &resource, &machine));
}

#[test]
fn skip_machine_filter_match() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\n").unwrap();
    let machine = super::check::localhost_machine();
    assert!(!super::check::skip_machine(
        "local",
        Some("local"),
        &resource,
        &machine
    ));
}

#[test]
fn skip_machine_filter_mismatch() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\n").unwrap();
    let machine = super::check::localhost_machine();
    assert!(super::check::skip_machine(
        "local",
        Some("other"),
        &resource,
        &machine
    ));
}

#[test]
fn skip_machine_arch_mismatch() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\narch: [aarch64]\n")
            .unwrap();
    let machine = super::check::localhost_machine(); // arch is x86_64
    assert!(super::check::skip_machine(
        "local",
        None,
        &resource,
        &machine
    ));
}

#[test]
fn skip_machine_arch_match() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\narch: [x86_64]\n")
            .unwrap();
    let machine = super::check::localhost_machine();
    assert!(!super::check::skip_machine(
        "local",
        None,
        &resource,
        &machine
    ));
}

// ── localhost_machine ────────────────────────────────────────────────

#[test]
fn localhost_machine_fields() {
    let m = super::check::localhost_machine();
    assert_eq!(m.hostname, "localhost");
    assert_eq!(m.addr, "127.0.0.1");
    assert_eq!(m.user, "root");
    assert_eq!(m.arch, "x86_64");
    assert!(m.ssh_key.is_none());
    assert!(m.roles.is_empty());
}

// ── make_check_result ────────────────────────────────────────────────

#[test]
fn make_check_result_pass() {
    let r = super::check::make_check_result("nginx", "web", "pass", Some(0), String::new());
    assert_eq!(r.resource_id, "nginx");
    assert_eq!(r.machine, "web");
    assert_eq!(r.status, "pass");
    assert_eq!(r.exit_code, Some(0));
    assert!(r.detail.is_empty());
}

#[test]
fn make_check_result_fail() {
    let r = super::check::make_check_result(
        "nginx", "web", "fail", Some(1), "not running".to_string(),
    );
    assert_eq!(r.status, "fail");
    assert_eq!(r.exit_code, Some(1));
    assert_eq!(r.detail, "not running");
}

#[test]
fn make_check_result_error() {
    let r = super::check::make_check_result(
        "pg", "db", "error", None, "connection refused".to_string(),
    );
    assert_eq!(r.status, "error");
    assert!(r.exit_code.is_none());
}

// ── format_check_json ────────────────────────────────────────────────

#[test]
fn check_json_empty_results() {
    let results: Vec<super::check::CheckResult> = vec![];
    assert!(super::check::format_check_json("test", &results, 0, 0, 0).is_ok());
}

#[test]
fn check_json_all_pass() {
    let results = vec![
        super::check::make_check_result("nginx", "web", "pass", Some(0), String::new()),
    ];
    assert!(super::check::format_check_json("test", &results, 1, 0, 0).is_ok());
}

#[test]
fn check_json_with_failures() {
    let results = vec![
        super::check::make_check_result("nginx", "web", "pass", Some(0), String::new()),
        super::check::make_check_result(
            "redis", "db", "fail", Some(1), "not running".to_string(),
        ),
    ];
    assert!(super::check::format_check_json("test", &results, 1, 1, 0).is_ok());
}

#[test]
fn check_json_with_error() {
    let results = vec![
        super::check::make_check_result(
            "pg", "db", "error", None, "connection refused".to_string(),
        ),
    ];
    assert!(super::check::format_check_json("test", &results, 0, 1, 1).is_ok());
}

#[test]
fn check_json_with_skip() {
    let results: Vec<super::check::CheckResult> = vec![];
    assert!(super::check::format_check_json("test", &results, 2, 0, 3).is_ok());
}

// ── FJ-178: --state-dir on check ─────────────────────────────────────
//
// Hermetic: local /tmp-file resource (no ssh/network/docker). Verifies that
// `check --state-dir` consults the given state dir to report whether a checked
// resource is recorded in state, mirroring drift/status.

/// Config with a single `type: file` resource on the `local` machine, whose
/// target file already exists (so the live check passes regardless of state).
fn check_file_config(dir: &Path) -> std::path::PathBuf {
    let target = dir.join("fj178.txt");
    std::fs::write(&target, "hello").unwrap();
    write_config(
        dir,
        &format!(
            r#"
version: "1.0"
name: fj178
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: {}
    content: hello
"#,
            target.display()
        ),
    )
}

#[test]
fn state_dir_resource_in_state_present() {
    use crate::core::types;
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    super::test_fixtures::make_state_dir_with_lock(
        &state,
        "local",
        vec![("cfg", "blake3:aaa", types::ResourceStatus::Converged)],
    );
    let config = super::helpers::parse_and_validate(&check_file_config(dir.path())).unwrap();
    let cs = super::check::load_check_locks(&state, &config);
    assert_eq!(
        super::check::resource_in_state(&cs, "local", "cfg"),
        Some(true),
        "resource recorded in populated state dir should be in_state=Some(true)"
    );
}

#[test]
fn state_dir_absent_is_state_agnostic() {
    let dir = tempfile::tempdir().unwrap();
    let config = super::helpers::parse_and_validate(&check_file_config(dir.path())).unwrap();
    // A state dir that does not exist (the default-absent case) → None, so the
    // historical state-unaware behavior is preserved (no "missing" note).
    let missing = dir.path().join("does-not-exist");
    let cs = super::check::load_check_locks(&missing, &config);
    assert_eq!(
        super::check::resource_in_state(&cs, "local", "cfg"),
        None,
        "absent state dir should be state-agnostic (None)"
    );
}

#[test]
fn state_dir_empty_but_existing_reports_missing() {
    let empty = tempfile::tempdir().unwrap(); // exists, but no locks
    let dir = tempfile::tempdir().unwrap();
    let config = super::helpers::parse_and_validate(&check_file_config(dir.path())).unwrap();
    let cs = super::check::load_check_locks(empty.path(), &config);
    assert_eq!(
        super::check::resource_in_state(&cs, "local", "cfg"),
        Some(false),
        "existing-but-empty state dir should report the resource as missing"
    );
}

#[test]
fn state_dir_resource_not_recorded_reports_missing() {
    use crate::core::types;
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    // Lock exists for "local" but records a DIFFERENT resource → cfg missing.
    super::test_fixtures::make_state_dir_with_lock(
        &state,
        "local",
        vec![("other", "blake3:bbb", types::ResourceStatus::Converged)],
    );
    let config = super::helpers::parse_and_validate(&check_file_config(dir.path())).unwrap();
    let cs = super::check::load_check_locks(&state, &config);
    assert_eq!(
        super::check::resource_in_state(&cs, "local", "cfg"),
        Some(false),
        "resource absent from existing lock should be in_state=Some(false)"
    );
}

#[test]
fn check_with_populated_state_dir_succeeds() {
    use crate::core::types;
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    super::test_fixtures::make_state_dir_with_lock(
        &state,
        "local",
        vec![("cfg", "blake3:aaa", types::ResourceStatus::Converged)],
    );
    let config = check_file_config(dir.path());
    // Live check passes (file exists) and state is consulted from the tempdir.
    super::check::cmd_check(&config, None, None, None, &state, false, false).unwrap();
    // JSON path also exercised against the populated state dir.
    super::check::cmd_check(&config, None, None, None, &state, true, false).unwrap();
}

#[test]
fn check_with_empty_state_dir_still_succeeds() {
    let empty = tempfile::tempdir().unwrap(); // exists, but no recorded state
    let dir = tempfile::tempdir().unwrap();
    let config = check_file_config(dir.path());
    // A different, existing-but-empty state dir: live check still passes (file
    // exists), and the "not recorded in state" note is reported without
    // changing the exit status (Ok).
    super::check::cmd_check(&config, None, None, None, empty.path(), false, false).unwrap();
}

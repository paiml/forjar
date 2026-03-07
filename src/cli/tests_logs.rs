//! Tests for cli/logs.rs — real file I/O against run directories.

use super::logs::{cmd_logs, cmd_logs_gc, cmd_logs_follow};

/// Create a realistic run directory with meta.yaml and log files.
fn create_run(
    state_dir: &std::path::Path,
    machine: &str,
    run_id: &str,
    failed_count: u32,
) -> std::path::PathBuf {
    let run_dir = state_dir.join(machine).join("runs").join(run_id);
    std::fs::create_dir_all(&run_dir).unwrap();

    let meta = format!(
        r#"run_id: {run_id}
machine: {machine}
command: apply
generation: 5
started_at: "2026-03-05T14:30:00Z"
finished_at: "2026-03-05T14:30:04Z"
duration_secs: 4.0
resources:
  nginx:
    action: converged
    exit_code: 0
    duration_secs: 1.5
    failed: false
summary:
  total: 2
  converged: 1
  noop: 1
  failed: {failed_count}
  skipped: 0
"#
    );
    std::fs::write(run_dir.join("meta.yaml"), meta).unwrap();

    let log = r#"=== FORJAR TRANSPORT LOG ===
resource: nginx
type: package
action: apply
machine: intel
transport: local
started: 2026-03-05T14:30:01Z
script_hash: blake3:abc123

=== SCRIPT ===
apt-get install -y nginx

=== STDOUT ===
Reading package lists...
nginx is already the newest version.

=== STDERR ===

=== RESULT ===
exit_code: 0
duration_secs: 1.500
finished: 2026-03-05T14:30:02Z
"#;
    std::fs::write(run_dir.join("nginx.apply.log"), log).unwrap();
    std::fs::write(run_dir.join("nginx.script"), "apt-get install -y nginx\n").unwrap();

    run_dir
}

// ── discover + display ──

#[test]
fn logs_empty_state_dir() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_logs(dir.path(), None, None, None, false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_with_real_run() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-abc123", 0);
    let r = cmd_logs(dir.path(), None, None, None, false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_machine_filter_match() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-001", 0);
    create_run(dir.path(), "jetson", "r-002", 0);
    let r = cmd_logs(dir.path(), Some("intel"), None, None, false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_machine_filter_no_match() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-001", 0);
    let r = cmd_logs(dir.path(), Some("nonexistent"), None, None, false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_run_filter() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-001", 0);
    create_run(dir.path(), "intel", "r-002", 0);
    let r = cmd_logs(dir.path(), None, Some("r-001"), None, false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_resource_filter() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-001", 0);
    let r = cmd_logs(dir.path(), None, None, Some("nginx"), false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_resource_with_script() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-001", 0);
    let r = cmd_logs(dir.path(), None, None, Some("nginx"), false, true, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_failures_only_filters() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-ok", 0);
    create_run(dir.path(), "intel", "r-fail", 1);
    let r = cmd_logs(dir.path(), None, None, None, true, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_all_machines() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-001", 0);
    create_run(dir.path(), "jetson", "r-002", 0);
    // all_machines=true ignores machine filter
    let r = cmd_logs(dir.path(), Some("intel"), None, None, false, false, true, false);
    assert!(r.is_ok());
}

// ── JSON output ──

#[test]
fn logs_json_empty() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_logs(dir.path(), None, None, None, false, false, false, true);
    assert!(r.is_ok());
}

#[test]
fn logs_json_with_runs() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-001", 0);
    let r = cmd_logs(dir.path(), None, None, None, false, false, false, true);
    assert!(r.is_ok());
}

#[test]
fn logs_json_resource_filter() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-001", 0);
    let r = cmd_logs(dir.path(), None, None, Some("nginx"), false, false, false, true);
    assert!(r.is_ok());
}

#[test]
fn logs_json_resource_with_script() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-001", 0);
    let r = cmd_logs(dir.path(), None, None, Some("nginx"), false, true, false, true);
    assert!(r.is_ok());
}

// ── GC ──

#[test]
fn gc_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_logs_gc(dir.path(), false, false, false, None);
    assert!(r.is_ok());
}

#[test]
fn gc_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    // Create 12 runs (exceeds default keep_runs=10)
    for i in 0..12 {
        create_run(dir.path(), "intel", &format!("r-{i:03}"), 0);
    }
    let r = cmd_logs_gc(dir.path(), true, false, false, None);
    assert!(r.is_ok());
}

#[test]
fn gc_actual_delete() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..12 {
        create_run(dir.path(), "intel", &format!("r-{i:03}"), 0);
    }
    let r = cmd_logs_gc(dir.path(), false, false, false, None);
    assert!(r.is_ok());
}

#[test]
fn gc_keep_failed() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..12 {
        let failed = if i == 0 { 1 } else { 0 };
        create_run(dir.path(), "intel", &format!("r-{i:03}"), failed);
    }
    let r = cmd_logs_gc(dir.path(), false, true, false, None);
    assert!(r.is_ok());
}

#[test]
fn gc_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_logs_gc(dir.path(), false, false, true, None);
    assert!(r.is_ok());
}

#[test]
fn gc_json_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..12 {
        create_run(dir.path(), "intel", &format!("r-{i:03}"), 0);
    }
    let r = cmd_logs_gc(dir.path(), true, false, true, None);
    assert!(r.is_ok());
}

// ── Follow ──

#[test]
fn follow_empty() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_logs_follow(dir.path(), false);
    assert!(r.is_ok());
}

#[test]
fn follow_with_runs() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-latest", 0);
    let r = cmd_logs_follow(dir.path(), false);
    assert!(r.is_ok());
}

#[test]
fn follow_json_empty() {
    let dir = tempfile::tempdir().unwrap();
    let r = cmd_logs_follow(dir.path(), true);
    assert!(r.is_ok());
}

#[test]
fn follow_json_with_runs() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-latest", 0);
    let r = cmd_logs_follow(dir.path(), true);
    assert!(r.is_ok());
}

// ── Edge cases ──

#[test]
fn logs_ignores_images_dir() {
    let dir = tempfile::tempdir().unwrap();
    // Create an "images" directory that should be skipped
    std::fs::create_dir_all(dir.path().join("images/runs/r-001")).unwrap();
    std::fs::write(
        dir.path().join("images/runs/r-001/meta.yaml"),
        "run_id: r-001\nmachine: images\ncommand: build\n",
    ).unwrap();
    let r = cmd_logs(dir.path(), None, None, None, false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_skips_invalid_meta_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let run_dir = dir.path().join("intel/runs/r-bad");
    std::fs::create_dir_all(&run_dir).unwrap();
    std::fs::write(run_dir.join("meta.yaml"), "invalid: {{ yaml").unwrap();
    let r = cmd_logs(dir.path(), None, None, None, false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_skips_missing_meta_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let run_dir = dir.path().join("intel/runs/r-no-meta");
    std::fs::create_dir_all(&run_dir).unwrap();
    // No meta.yaml — should be silently skipped
    let r = cmd_logs(dir.path(), None, None, None, false, false, false, false);
    assert!(r.is_ok());
}

#[test]
fn logs_resource_no_log_file() {
    let dir = tempfile::tempdir().unwrap();
    create_run(dir.path(), "intel", "r-001", 0);
    // Request logs for a resource that has no log files
    let r = cmd_logs(dir.path(), None, None, Some("nonexistent-resource"), false, false, false, false);
    assert!(r.is_ok());
}

//! Coverage tests for cli/undo.rs — cmd_undo_destroy, load_generation_locks.

use crate::core::types;

fn make_destroy_entry(machine: &str, resource: &str, rtype: &str, reliable: bool) -> String {
    serde_json::to_string(&types::DestroyLogEntry {
        timestamp: "2026-01-01T00:00:00Z".into(),
        machine: machine.into(),
        resource_id: resource.into(),
        resource_type: rtype.into(),
        pre_hash: "abc123".into(),
        generation: 5,
        config_fragment: None,
        reliable_recreate: reliable,
    })
    .unwrap()
}

fn write_destroy_log(dir: &std::path::Path, entries: &[String]) {
    let content = entries.join("\n") + "\n";
    std::fs::write(dir.join("destroy-log.jsonl"), content).unwrap();
}

const LOCK_YAML: &str = r#"schema: "1"
machine: web1
hostname: web1
generated_at: "2025-01-01T00:00:00Z"
generator: forjar-test
blake3_version: "1.0"
resources:
  nginx:
    type: package
    status: converged
    hash: abc123
    applied_at: "2025-01-01T00:00:00Z"
    duration_seconds: 2.5
"#;

// ── cmd_undo_destroy ──

#[test]
fn undo_destroy_no_log() {
    let d = tempfile::tempdir().unwrap();
    let r = super::undo::cmd_undo_destroy(d.path(), None, false, false);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("no destroy-log.jsonl"));
}

#[test]
fn undo_destroy_empty_log() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("destroy-log.jsonl"), "\n").unwrap();
    let r = super::undo::cmd_undo_destroy(d.path(), None, false, false);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("no matching entries"));
}

#[test]
fn undo_destroy_dry_run_reliable() {
    let d = tempfile::tempdir().unwrap();
    let entries = vec![
        make_destroy_entry("web1", "nginx", "package", true),
        make_destroy_entry("web1", "config", "file", true),
    ];
    write_destroy_log(d.path(), &entries);
    let r = super::undo::cmd_undo_destroy(d.path(), None, false, true);
    assert!(r.is_ok());
}

#[test]
fn undo_destroy_dry_run_with_unreliable() {
    let d = tempfile::tempdir().unwrap();
    let entries = vec![
        make_destroy_entry("web1", "nginx", "package", true),
        make_destroy_entry("web1", "mysql", "package", false),
    ];
    write_destroy_log(d.path(), &entries);
    let r = super::undo::cmd_undo_destroy(d.path(), None, false, true);
    assert!(r.is_ok());
}

#[test]
fn undo_destroy_dry_run_force_includes_unreliable() {
    let d = tempfile::tempdir().unwrap();
    let entries = vec![
        make_destroy_entry("web1", "nginx", "package", true),
        make_destroy_entry("db1", "pg", "package", false),
    ];
    write_destroy_log(d.path(), &entries);
    let r = super::undo::cmd_undo_destroy(d.path(), None, true, true);
    assert!(r.is_ok());
}

#[test]
fn undo_destroy_machine_filter() {
    let d = tempfile::tempdir().unwrap();
    let entries = vec![
        make_destroy_entry("web1", "nginx", "package", true),
        make_destroy_entry("db1", "pg", "package", true),
    ];
    write_destroy_log(d.path(), &entries);
    let r = super::undo::cmd_undo_destroy(d.path(), Some("web1"), false, true);
    assert!(r.is_ok());
}

#[test]
fn undo_destroy_machine_filter_no_match() {
    let d = tempfile::tempdir().unwrap();
    let entries = vec![make_destroy_entry("web1", "nginx", "package", true)];
    write_destroy_log(d.path(), &entries);
    let r = super::undo::cmd_undo_destroy(d.path(), Some("zzz"), false, true);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("no matching entries"));
}

#[test]
fn undo_destroy_not_dry_run_prints_not_implemented() {
    let d = tempfile::tempdir().unwrap();
    let entries = vec![make_destroy_entry("web1", "nginx", "package", true)];
    write_destroy_log(d.path(), &entries);
    let r = super::undo::cmd_undo_destroy(d.path(), None, false, false);
    assert!(r.is_ok());
}

#[test]
fn undo_destroy_all_unreliable_no_force() {
    let d = tempfile::tempdir().unwrap();
    let entries = vec![
        make_destroy_entry("web1", "svc1", "service", false),
        make_destroy_entry("web1", "svc2", "service", false),
    ];
    write_destroy_log(d.path(), &entries);
    let r = super::undo::cmd_undo_destroy(d.path(), None, false, true);
    assert!(r.is_ok());
}

#[test]
fn undo_destroy_force_not_dry_run() {
    let d = tempfile::tempdir().unwrap();
    let entries = vec![
        make_destroy_entry("web1", "svc1", "service", false),
    ];
    write_destroy_log(d.path(), &entries);
    let r = super::undo::cmd_undo_destroy(d.path(), None, true, false);
    assert!(r.is_ok());
}

#[test]
fn undo_destroy_mixed_machines() {
    let d = tempfile::tempdir().unwrap();
    let entries = vec![
        make_destroy_entry("web1", "nginx", "package", true),
        make_destroy_entry("db1", "pg", "package", true),
        make_destroy_entry("cache1", "redis", "package", false),
    ];
    write_destroy_log(d.path(), &entries);
    let r = super::undo::cmd_undo_destroy(d.path(), None, true, true);
    assert!(r.is_ok());
}

// ── cmd_undo edge cases ──

#[test]
fn undo_no_generations_dir() {
    let d = tempfile::tempdir().unwrap();
    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg_path = cfg_dir.path().join("forjar.yaml");
    std::fs::write(&cfg_path, "version: '1.0'\nname: t\nmachines: {}\nresources: {}\n").unwrap();
    let r = super::undo::cmd_undo(&cfg_path, d.path(), 1, None, false, true);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("no generations found"));
}

// ── generation create + list + gc + rollback (filesystem-only) ──

fn setup_gen_state(state_dir: &std::path::Path) {
    std::fs::create_dir_all(state_dir.join("web1")).unwrap();
    std::fs::write(state_dir.join("web1/state.lock.yaml"), LOCK_YAML).unwrap();
}

#[test]
fn create_generation_basic() {
    let d = tempfile::tempdir().unwrap();
    setup_gen_state(d.path());
    let gen = super::generation::create_generation(d.path()).unwrap();
    assert_eq!(gen, 0);
    assert!(d.path().join("generations/0").exists());
    assert!(d.path().join("generations/current").exists());
}

#[test]
fn create_generation_increments() {
    let d = tempfile::tempdir().unwrap();
    setup_gen_state(d.path());
    let g0 = super::generation::create_generation(d.path()).unwrap();
    let g1 = super::generation::create_generation(d.path()).unwrap();
    assert_eq!(g0, 0);
    assert_eq!(g1, 1);
}

#[test]
fn list_generations_empty() {
    let d = tempfile::tempdir().unwrap();
    let r = super::generation::list_generations(d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn list_generations_empty_json() {
    let d = tempfile::tempdir().unwrap();
    let r = super::generation::list_generations(d.path(), true);
    assert!(r.is_ok());
}

#[test]
fn list_generations_with_data() {
    let d = tempfile::tempdir().unwrap();
    setup_gen_state(d.path());
    super::generation::create_generation(d.path()).unwrap();
    super::generation::create_generation(d.path()).unwrap();
    let r = super::generation::list_generations(d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn list_generations_with_data_json() {
    let d = tempfile::tempdir().unwrap();
    setup_gen_state(d.path());
    super::generation::create_generation(d.path()).unwrap();
    let r = super::generation::list_generations(d.path(), true);
    assert!(r.is_ok());
}

#[test]
fn current_generation_none() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("generations")).unwrap();
    let r = super::generation::current_generation(&d.path().join("generations"));
    assert!(r.is_none());
}

#[test]
fn current_generation_after_create() {
    let d = tempfile::tempdir().unwrap();
    setup_gen_state(d.path());
    super::generation::create_generation(d.path()).unwrap();
    super::generation::create_generation(d.path()).unwrap();
    let cur = super::generation::current_generation(&d.path().join("generations"));
    assert_eq!(cur, Some(1));
}

#[test]
fn gc_generations_no_dir() {
    let d = tempfile::tempdir().unwrap();
    super::generation::gc_generations(d.path(), 5, false);
}

#[test]
fn gc_generations_keeps_recent() {
    let d = tempfile::tempdir().unwrap();
    setup_gen_state(d.path());
    for _ in 0..5 {
        super::generation::create_generation(d.path()).unwrap();
    }
    super::generation::gc_generations(d.path(), 2, true);
    assert!(!d.path().join("generations/0").exists());
    assert!(!d.path().join("generations/1").exists());
    assert!(!d.path().join("generations/2").exists());
    assert!(d.path().join("generations/3").exists());
    assert!(d.path().join("generations/4").exists());
}

#[test]
fn gc_generations_nothing_to_gc() {
    let d = tempfile::tempdir().unwrap();
    setup_gen_state(d.path());
    super::generation::create_generation(d.path()).unwrap();
    super::generation::gc_generations(d.path(), 10, false);
    assert!(d.path().join("generations/0").exists());
}

#[test]
fn rollback_to_generation_no_yes() {
    let d = tempfile::tempdir().unwrap();
    let r = super::generation::rollback_to_generation(d.path(), 0, false);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("requires --yes"));
}

#[test]
fn rollback_to_generation_missing() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("generations")).unwrap();
    let r = super::generation::rollback_to_generation(d.path(), 99, true);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("does not exist"));
}

#[test]
fn rollback_to_generation_success() {
    let d = tempfile::tempdir().unwrap();
    setup_gen_state(d.path());
    super::generation::create_generation(d.path()).unwrap();
    // Modify state after gen 0
    std::fs::write(d.path().join("web1/state.lock.yaml"), "modified").unwrap();
    super::generation::create_generation(d.path()).unwrap();
    // Rollback to gen 0
    let r = super::generation::rollback_to_generation(d.path(), 0, true);
    assert!(r.is_ok());
    // Verify state was restored
    let content = std::fs::read_to_string(d.path().join("web1/state.lock.yaml")).unwrap();
    assert!(content.contains("nginx"));
}

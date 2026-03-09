//! Coverage tests for undo.rs — cmd_undo_resume, cmd_undo edge paths.

use crate::core::types;
use std::collections::HashMap;

// ── cmd_undo_resume: no partial undo ─────────────────────────────────

#[test]
fn undo_resume_no_partial() {
    let cfg_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let cfg_path = cfg_dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg_path,
        "version: '1.0'\nname: t\nmachines:\n  web:\n    hostname: w\n    addr: 127.0.0.1\nresources: {}\n",
    )
    .unwrap();

    let result = super::undo::cmd_undo_resume(&cfg_path, state_dir.path(), None, false, true);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no partial undo found"));
}

#[test]
fn undo_resume_dry_run_with_partial() {
    let cfg_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let cfg_path = cfg_dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg_path,
        "version: '1.0'\nname: t\nmachines:\n  web:\n    hostname: w\n    addr: 127.0.0.1\nresources: {}\n",
    )
    .unwrap();

    // Write a partial undo-progress.yaml
    let mut resources = HashMap::new();
    resources.insert(
        "nginx".to_string(),
        types::ResourceProgress {
            status: types::ResourceProgressStatus::Pending,
            at: None,
        },
    );
    resources.insert(
        "app".to_string(),
        types::ResourceProgress {
            status: types::ResourceProgressStatus::Completed,
            at: Some("2026-01-01T00:00:00Z".to_string()),
        },
    );
    let progress = types::UndoProgress {
        generation_from: 5,
        generation_to: 3,
        started_at: "2026-01-01T00:00:00Z".to_string(),
        status: types::UndoStatus::Partial,
        resources,
    };
    super::undo::write_undo_progress(state_dir.path(), "web", &progress);

    let result = super::undo::cmd_undo_resume(&cfg_path, state_dir.path(), None, true, true);
    assert!(result.is_ok());
}

#[test]
fn undo_resume_no_yes_flag() {
    let cfg_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let cfg_path = cfg_dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg_path,
        "version: '1.0'\nname: t\nmachines:\n  web:\n    hostname: w\n    addr: 127.0.0.1\nresources: {}\n",
    )
    .unwrap();

    // Write partial progress with failed resource
    let mut resources = HashMap::new();
    resources.insert(
        "nginx".to_string(),
        types::ResourceProgress {
            status: types::ResourceProgressStatus::Failed {
                error: "timeout".to_string(),
            },
            at: Some("2026-01-01T00:00:00Z".to_string()),
        },
    );
    let progress = types::UndoProgress {
        generation_from: 3,
        generation_to: 1,
        started_at: "2026-01-01T00:00:00Z".to_string(),
        status: types::UndoStatus::Partial,
        resources,
    };
    super::undo::write_undo_progress(state_dir.path(), "web", &progress);

    // Without --yes should fail
    let result = super::undo::cmd_undo_resume(&cfg_path, state_dir.path(), None, false, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires --yes"));
}

#[test]
fn undo_resume_machine_filter() {
    let cfg_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let cfg_path = cfg_dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg_path,
        "version: '1.0'\nname: t\nmachines:\n  web:\n    hostname: w\n    addr: 127.0.0.1\n  db:\n    hostname: d\n    addr: 127.0.0.1\nresources: {}\n",
    )
    .unwrap();

    // Write progress only for db
    let mut resources = HashMap::new();
    resources.insert(
        "pg".to_string(),
        types::ResourceProgress {
            status: types::ResourceProgressStatus::Pending,
            at: None,
        },
    );
    let progress = types::UndoProgress {
        generation_from: 2,
        generation_to: 1,
        started_at: "2026-01-01T00:00:00Z".to_string(),
        status: types::UndoStatus::Partial,
        resources,
    };
    super::undo::write_undo_progress(state_dir.path(), "db", &progress);

    // Filter to web — should find no partial
    let result =
        super::undo::cmd_undo_resume(&cfg_path, state_dir.path(), Some("web"), false, true);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no partial undo found"));

    // Filter to db — should find partial, dry_run succeeds
    let result =
        super::undo::cmd_undo_resume(&cfg_path, state_dir.path(), Some("db"), true, true);
    assert!(result.is_ok());
}

// ── cmd_undo: error paths ────────────────────────────────────────────

fn setup_generations(state_dir: &std::path::Path, count: u32) {
    let gen_dir = state_dir.join("generations");
    for i in 0..count {
        std::fs::create_dir_all(gen_dir.join(i.to_string())).unwrap();
    }
    // Create symlink for current → last generation
    if count > 0 {
        let current = gen_dir.join("current");
        std::os::unix::fs::symlink((count - 1).to_string(), &current).unwrap();
    }
}

#[test]
fn undo_too_many_generations() {
    let cfg_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let cfg_path = cfg_dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg_path,
        "version: '1.0'\nname: t\nmachines: {}\nresources: {}\n",
    )
    .unwrap();

    // Create 1 generation (current=0)
    setup_generations(state_dir.path(), 1);

    let result = super::undo::cmd_undo(&cfg_path, state_dir.path(), 5, None, false, true);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("cannot undo 5 generation(s)"),
        "expected 'cannot undo' error"
    );
}

#[test]
fn undo_target_gen_missing() {
    let cfg_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let cfg_path = cfg_dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg_path,
        "version: '1.0'\nname: t\nmachines: {}\nresources: {}\n",
    )
    .unwrap();

    // Create gen 2 only (target=1 won't exist)
    let gen_dir = state_dir.path().join("generations");
    std::fs::create_dir_all(gen_dir.join("2")).unwrap();
    std::os::unix::fs::symlink("2", gen_dir.join("current")).unwrap();

    let result = super::undo::cmd_undo(&cfg_path, state_dir.path(), 1, None, false, true);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("does not exist"),
        "expected 'does not exist' error"
    );
}

#[test]
fn undo_dry_run_no_changes() {
    let cfg_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let cfg_path = cfg_dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg_path,
        "version: '1.0'\nname: t\nmachines:\n  web:\n    hostname: w\n    addr: 127.0.0.1\nresources: {}\n",
    )
    .unwrap();

    // Create gen 0 and gen 1 with no machine locks
    setup_generations(state_dir.path(), 2);

    // Undo 1 → target=0, both empty, no changes
    let result = super::undo::cmd_undo(&cfg_path, state_dir.path(), 1, None, true, true);
    assert!(result.is_ok());
}

#[test]
fn undo_dry_run_with_changes() {
    let cfg_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let cfg_path = cfg_dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg_path,
        "version: '1.0'\nname: t\nmachines:\n  web:\n    hostname: w\n    addr: 127.0.0.1\nresources: {}\n",
    )
    .unwrap();

    let gen_dir = state_dir.path().join("generations");
    // Gen 0 has a lock with resources
    std::fs::create_dir_all(gen_dir.join("0/web")).unwrap();
    std::fs::write(
        gen_dir.join("0/web/state.lock.yaml"),
        "schema: '1'\nmachine: web\nhostname: w\ngenerated_at: t\ngenerator: g\nblake3_version: b\nresources:\n  nginx:\n    type: package\n    status: converged\n    hash: abc123\n",
    )
    .unwrap();
    // Gen 1 is empty
    std::fs::create_dir_all(gen_dir.join("1")).unwrap();
    std::os::unix::fs::symlink("1", gen_dir.join("current")).unwrap();

    // Undo 1 gen → dry_run shows changes
    let result = super::undo::cmd_undo(&cfg_path, state_dir.path(), 1, None, true, true);
    assert!(result.is_ok());
}

#[test]
fn undo_no_yes_flag() {
    let cfg_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let cfg_path = cfg_dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg_path,
        "version: '1.0'\nname: t\nmachines:\n  web:\n    hostname: w\n    addr: 127.0.0.1\nresources: {}\n",
    )
    .unwrap();

    let gen_dir = state_dir.path().join("generations");
    std::fs::create_dir_all(gen_dir.join("0/web")).unwrap();
    std::fs::write(
        gen_dir.join("0/web/state.lock.yaml"),
        "schema: '1'\nmachine: web\nhostname: w\ngenerated_at: t\ngenerator: g\nblake3_version: b\nresources:\n  nginx:\n    type: package\n    status: converged\n    hash: abc123\n",
    )
    .unwrap();
    std::fs::create_dir_all(gen_dir.join("1")).unwrap();
    std::os::unix::fs::symlink("1", gen_dir.join("current")).unwrap();

    // Without --yes, not dry_run → should err
    let result = super::undo::cmd_undo(&cfg_path, state_dir.path(), 1, None, false, false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires --yes"));
}

// ── init_undo_progress edge cases ────────────────────────────────────

#[test]
fn init_progress_empty_changes() {
    let progress = super::undo::init_undo_progress(3, 1, &[]);
    assert_eq!(progress.generation_from, 3);
    assert_eq!(progress.generation_to, 1);
    assert_eq!(progress.pending_count(), 0);
    assert_eq!(progress.completed_count(), 0);
    assert!(progress.resources.is_empty());
}

#[test]
fn init_progress_extracts_resource_ids() {
    let changes = vec![
        "  + nginx (web): will be created".to_string(),
        "  ~ app (web): will be updated".to_string(),
        "  - old (web): will be destroyed".to_string(),
    ];
    let progress = super::undo::init_undo_progress(10, 5, &changes);
    assert_eq!(progress.pending_count(), 3);
    assert!(progress.resources.contains_key("nginx"));
    assert!(progress.resources.contains_key("app"));
    assert!(progress.resources.contains_key("old"));
}

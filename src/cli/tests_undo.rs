//! Tests for FJ-2003/FJ-2005: undo and undo-destroy commands.

#[cfg(test)]
mod tests {
    use crate::core::types;
    use std::collections::HashMap;

    // ── diff_machine_locks ────────────────────────────────────────

    fn make_lock(resources: Vec<(&str, &str)>) -> types::StateLock {
        let header = "schema: '1'\nmachine: m\nhostname: h\ngenerated_at: t\ngenerator: g\nblake3_version: b\n";
        let mut entries = format!("{header}resources:\n");
        for (id, hash) in &resources {
            entries.push_str(&format!(
                "  {id}:\n    type: file\n    status: converged\n    hash: \"{hash}\"\n"
            ));
        }
        if resources.is_empty() {
            entries.push_str("  {}\n");
        }
        serde_yaml_ng::from_str(&entries).unwrap()
    }

    #[test]
    fn diff_empty_to_populated() {
        let target = make_lock(vec![("a", "h1"), ("b", "h2")]);
        let changes = crate::cli::undo::diff_machine_locks("m1", None, &target);
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().all(|c| c.contains("will be created")));
    }

    #[test]
    fn diff_no_changes() {
        let lock = make_lock(vec![("a", "h1")]);
        let changes = crate::cli::undo::diff_machine_locks("m1", Some(&lock), &lock);
        assert!(changes.is_empty());
    }

    #[test]
    fn diff_updated_resource() {
        let current = make_lock(vec![("a", "h1")]);
        let target = make_lock(vec![("a", "h2")]);
        let changes = crate::cli::undo::diff_machine_locks("m1", Some(&current), &target);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].contains("will be updated"));
    }

    #[test]
    fn diff_destroyed_resource() {
        let current = make_lock(vec![("a", "h1"), ("b", "h2")]);
        let target = make_lock(vec![("a", "h1")]);
        let changes = crate::cli::undo::diff_machine_locks("m1", Some(&current), &target);
        assert_eq!(changes.len(), 1);
        assert!(changes[0].contains("will be destroyed"));
    }

    #[test]
    fn diff_mixed_changes() {
        let current = make_lock(vec![("a", "h1"), ("c", "h3")]);
        let target = make_lock(vec![("a", "h2"), ("b", "new")]);
        let changes = crate::cli::undo::diff_machine_locks("m1", Some(&current), &target);
        // a: updated, b: created, c: destroyed
        assert_eq!(changes.len(), 3);
    }

    // ── compute_undo_diff ──────────────────────────────────────────

    #[test]
    fn compute_undo_diff_empty() {
        let current = HashMap::new();
        let target = HashMap::new();
        let changes = crate::cli::undo::compute_undo_diff(&current, &target);
        assert!(changes.is_empty());
    }

    #[test]
    fn compute_undo_diff_multi_machine() {
        let mut current = HashMap::new();
        current.insert("intel".to_string(), make_lock(vec![("a", "h1")]));

        let mut target = HashMap::new();
        target.insert("intel".to_string(), make_lock(vec![("a", "h2")]));
        target.insert("jetson".to_string(), make_lock(vec![("b", "h3")]));

        let changes = crate::cli::undo::compute_undo_diff(&current, &target);
        // intel: a updated, jetson: b created
        assert_eq!(changes.len(), 2);
    }

    // ── init_undo_progress ──────────────────────────────────────────

    #[test]
    fn init_progress_sets_all_pending() {
        let changes = vec![
            "  + pkg1 (m1): will be created".to_string(),
            "  ~ cfg1 (m1): will be updated".to_string(),
        ];
        let progress = crate::cli::undo::init_undo_progress(5, 3, &changes);
        assert_eq!(progress.generation_from, 5);
        assert_eq!(progress.generation_to, 3);
        assert_eq!(progress.pending_count(), 2);
        assert_eq!(progress.completed_count(), 0);
        assert!(matches!(progress.status, types::UndoStatus::InProgress));
    }

    // ── undo progress read/write roundtrip ────────────────────────

    #[test]
    fn progress_write_read_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path();
        let machine = "test-machine";

        let mut resources = HashMap::new();
        resources.insert("res1".to_string(), types::ResourceProgress {
            status: types::ResourceProgressStatus::Completed,
            at: Some("2026-01-01T00:00:00Z".to_string()),
        });
        let progress = types::UndoProgress {
            generation_from: 10,
            generation_to: 8,
            started_at: "2026-01-01T00:00:00Z".to_string(),
            status: types::UndoStatus::Partial,
            resources,
        };

        crate::cli::undo::write_undo_progress(state_dir, machine, &progress);
        let loaded = crate::cli::undo::read_undo_progress(state_dir, machine).unwrap();
        assert_eq!(loaded.generation_from, 10);
        assert_eq!(loaded.generation_to, 8);
        assert!(loaded.needs_resume());
        assert_eq!(loaded.completed_count(), 1);
    }

    #[test]
    fn progress_read_missing_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(crate::cli::undo::read_undo_progress(dir.path(), "missing").is_none());
    }

    // ── compute_rollback_changes ───────────────────────────────────

    #[test]
    fn rollback_no_changes() {
        let yaml = "name: test\nversion: '1'\nmachines: {}\nresources: {}";
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let changes = crate::cli::destroy::compute_rollback_changes(&config, &config, 1);
        assert!(changes.is_empty());
    }
}

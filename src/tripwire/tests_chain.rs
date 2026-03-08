//! Tests: FJ-1386 tamper-evident chain hashing.

#[cfg(test)]
mod tests {
    use crate::tripwire::chain::*;
    use std::io::Write;

    fn write_events(dir: &std::path::Path, machine: &str, lines: &[&str]) -> std::path::PathBuf {
        let machine_dir = dir.join(machine);
        std::fs::create_dir_all(&machine_dir).unwrap();
        let path = machine_dir.join("events.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        for line in lines {
            writeln!(f, "{line}").unwrap();
        }
        path
    }

    #[test]
    fn test_compute_chain_hash_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let events = write_events(
            dir.path(),
            "m1",
            &[
                r#"{"ts":"2026-01-01T00:00:00Z","event":"converged","resource":"pkg-a"}"#,
                r#"{"ts":"2026-01-01T00:01:00Z","event":"converged","resource":"pkg-b"}"#,
            ],
        );
        let h1 = compute_chain_hash(&events).unwrap();
        let h2 = compute_chain_hash(&events).unwrap();
        assert_eq!(h1, h2);
        assert!(!h1.is_empty());
    }

    #[test]
    fn test_chain_hash_changes_on_tamper() {
        let dir = tempfile::tempdir().unwrap();
        let events = write_events(
            dir.path(),
            "m1",
            &[
                r#"{"ts":"2026-01-01T00:00:00Z","event":"converged","resource":"pkg-a"}"#,
                r#"{"ts":"2026-01-01T00:01:00Z","event":"converged","resource":"pkg-b"}"#,
            ],
        );
        let h1 = compute_chain_hash(&events).unwrap();

        // Tamper with first line
        let events2 = write_events(
            dir.path(),
            "m1",
            &[
                r#"{"ts":"2026-01-01T00:00:00Z","event":"TAMPERED","resource":"pkg-a"}"#,
                r#"{"ts":"2026-01-01T00:01:00Z","event":"converged","resource":"pkg-b"}"#,
            ],
        );
        let h2 = compute_chain_hash(&events2).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_write_and_verify_chain() {
        let dir = tempfile::tempdir().unwrap();
        let events = write_events(
            dir.path(),
            "m1",
            &[r#"{"ts":"2026-01-01T00:00:00Z","event":"converged","resource":"pkg-a"}"#],
        );
        write_chain_sidecar(&events).unwrap();
        let result = verify_chain(&events).unwrap();
        assert_eq!(result.total_lines, 1);
        assert_eq!(result.verified, 1);
        assert!(result.failures.is_empty());
    }

    #[test]
    fn test_verify_detects_tamper() {
        let dir = tempfile::tempdir().unwrap();
        let events = write_events(
            dir.path(),
            "m1",
            &[r#"{"ts":"2026-01-01T00:00:00Z","event":"converged","resource":"pkg-a"}"#],
        );
        write_chain_sidecar(&events).unwrap();

        // Tamper
        let mut f = std::fs::File::create(&events).unwrap();
        writeln!(
            f,
            r#"{{"ts":"2026-01-01T00:00:00Z","event":"TAMPERED","resource":"pkg-a"}}"#
        )
        .unwrap();

        let result = verify_chain(&events).unwrap();
        assert!(!result.failures.is_empty());
        assert_eq!(result.verified, 0);
    }

    #[test]
    fn test_verify_no_sidecar_passes() {
        let dir = tempfile::tempdir().unwrap();
        let events = write_events(
            dir.path(),
            "m1",
            &[r#"{"ts":"2026-01-01T00:00:00Z","event":"converged","resource":"pkg-a"}"#],
        );
        // No sidecar written
        let result = verify_chain(&events).unwrap();
        assert!(result.failures.is_empty());
        assert_eq!(result.verified, 1);
    }

    #[test]
    fn test_empty_log() {
        let dir = tempfile::tempdir().unwrap();
        let events = write_events(dir.path(), "m1", &[]);
        let hash = compute_chain_hash(&events).unwrap();
        assert_eq!(hash, "genesis");
    }

    #[test]
    fn test_verify_all_chains() {
        let dir = tempfile::tempdir().unwrap();
        let e1 = write_events(
            dir.path(),
            "m1",
            &[r#"{"ts":"2026-01-01T00:00:00Z","event":"converged","resource":"a"}"#],
        );
        let e2 = write_events(
            dir.path(),
            "m2",
            &[r#"{"ts":"2026-01-01T00:00:00Z","event":"converged","resource":"b"}"#],
        );
        write_chain_sidecar(&e1).unwrap();
        write_chain_sidecar(&e2).unwrap();

        let results = verify_all_chains(dir.path());
        assert_eq!(results.len(), 2);
        for (_, v) in &results {
            assert!(v.failures.is_empty());
        }
    }

    #[test]
    fn test_verify_all_chains_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let results = verify_all_chains(dir.path());
        assert!(results.is_empty());
    }

    #[test]
    fn test_verify_all_chains_nonexistent_dir() {
        let results = verify_all_chains(std::path::Path::new("/tmp/nonexistent-chain-dir-xyz"));
        assert!(results.is_empty());
    }

    #[test]
    fn test_verify_all_chains_skips_files() {
        let dir = tempfile::tempdir().unwrap();
        // Create a regular file (not a directory) at top level
        std::fs::write(dir.path().join("not-a-machine"), "data").unwrap();
        let results = verify_all_chains(dir.path());
        assert!(results.is_empty(), "regular files should be skipped");
    }

    #[test]
    fn test_verify_all_chains_machine_without_events() {
        let dir = tempfile::tempdir().unwrap();
        // Create machine dir but no events.jsonl
        std::fs::create_dir(dir.path().join("orphan-machine")).unwrap();
        let results = verify_all_chains(dir.path());
        assert!(
            results.is_empty(),
            "machine dirs without events.jsonl skipped"
        );
    }

    #[test]
    fn test_compute_chain_hash_skips_blank_lines() {
        let dir = tempfile::tempdir().unwrap();
        let events = write_events(
            dir.path(),
            "m1",
            &[r#"{"event":"a"}"#, "", r#"{"event":"b"}"#],
        );
        let h1 = compute_chain_hash(&events).unwrap();
        // Without blank lines
        let events2 = write_events(dir.path(), "m2", &[r#"{"event":"a"}"#, r#"{"event":"b"}"#]);
        let h2 = compute_chain_hash(&events2).unwrap();
        assert_eq!(h1, h2, "blank lines should be skipped");
    }

    #[test]
    fn test_compute_chain_hash_nonexistent_file() {
        let result = compute_chain_hash(std::path::Path::new("/tmp/nonexistent-events.jsonl"));
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_all_chains_handles_unreadable_events() {
        let dir = tempfile::tempdir().unwrap();
        let machine_dir = dir.path().join("bad-machine");
        std::fs::create_dir_all(&machine_dir).unwrap();
        // Create events.jsonl as a directory (not a file) — read_to_string will fail
        std::fs::create_dir(machine_dir.join("events.jsonl")).unwrap();
        let results = verify_all_chains(dir.path());
        assert_eq!(results.len(), 1);
        let (name, v) = &results[0];
        assert_eq!(name, "bad-machine");
        assert!(!v.failures.is_empty(), "should record the read error");
        assert_eq!(v.total_lines, 0);
        assert_eq!(v.verified, 0);
        assert!(v.chain_hash.is_empty());
    }

    #[test]
    fn test_chain_incorporates_previous() {
        let dir = tempfile::tempdir().unwrap();
        // Single event
        let e1 = write_events(dir.path(), "m1", &[r#"{"event":"a"}"#]);
        let h1 = compute_chain_hash(&e1).unwrap();

        // Two events (first is same)
        let e2 = write_events(dir.path(), "m1", &[r#"{"event":"a"}"#, r#"{"event":"b"}"#]);
        let h2 = compute_chain_hash(&e2).unwrap();

        // Different because chain links previous
        assert_ne!(h1, h2);

        // Same first event but different second
        let e3 = write_events(dir.path(), "m1", &[r#"{"event":"a"}"#, r#"{"event":"c"}"#]);
        let h3 = compute_chain_hash(&e3).unwrap();
        assert_ne!(h2, h3);
    }
}

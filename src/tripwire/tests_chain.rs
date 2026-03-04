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

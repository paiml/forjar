//! Coverage tests for lock_security, lock_ops, lock_merge, lock_lifecycle, lock_core, lock_audit.

use super::lock_audit::*;
use super::lock_core::*;
use super::lock_lifecycle::*;
use super::lock_merge::*;
use super::lock_ops::*;
use super::lock_security::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn write_yaml(dir: &Path, name: &str, content: &str) {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
    }

    fn state_lock_yaml() -> &'static str {
        "schema: \"1\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789\"\n  g:\n    type: service\n    status: drifted\n    hash: \"1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef\"\n"
    }

    /// Create a temp dir with both flat ({m}.lock.yaml) and nested ({m}/state.lock.yaml) lock files.
    fn make_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        // Nested structure for discover_machines + state::load_lock
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock_yaml());
        // Flat structure for lock_lifecycle/lock_audit/lock_security functions
        write_yaml(dir.path(), "web.lock.yaml", state_lock_yaml());
        dir
    }

    fn make_dir_with_events() -> tempfile::TempDir {
        let dir = make_dir();
        let events = r#"{"timestamp":"2026-01-01T00:00:00Z","resource":"f","action":"apply"}
{"timestamp":"2026-02-01T00:00:00Z","resource":"g","action":"converge"}
"#;
        write_yaml(dir.path(), "web/events.jsonl", events);
        write_yaml(dir.path(), "web.events.jsonl", events);
        dir
    }

    fn minimal_config(dir: &Path) -> std::path::PathBuf {
        let p = dir.join("forjar.yaml");
        std::fs::write(&p, "version: \"1.0\"\nname: test\nmachines:\n  web:\n    hostname: web\n    addr: 127.0.0.1\nresources:\n  f:\n    type: file\n    machine: web\n    path: /tmp/f\n    content: hello\n").unwrap();
        p
    }

    // ── lock_security: cmd_lock_verify_sig ──

    #[test]
    fn test_lock_verify_sig_plain() {
        let dir = make_dir();
        let _ = cmd_lock_verify_sig(dir.path(), "test-key", false);
    }

    #[test]
    fn test_lock_verify_sig_json() {
        let dir = make_dir();
        let _ = cmd_lock_verify_sig(dir.path(), "test-key", true);
    }

    // ── lock_security: cmd_lock_compact_all ──

    #[test]
    fn test_lock_compact_all_plain() {
        let dir = make_dir();
        let _ = cmd_lock_compact_all(dir.path(), false, false);
    }

    #[test]
    fn test_lock_compact_all_json() {
        let dir = make_dir();
        let _ = cmd_lock_compact_all(dir.path(), true, true);
    }

    // ── lock_security: cmd_lock_audit_trail ──

    #[test]
    fn test_lock_audit_trail_plain() {
        let dir = make_dir_with_events();
        let _ = cmd_lock_audit_trail(dir.path(), None, false);
    }

    #[test]
    fn test_lock_audit_trail_json() {
        let dir = make_dir_with_events();
        let _ = cmd_lock_audit_trail(dir.path(), Some("web"), true);
    }

    // ── lock_security: cmd_lock_rotate_keys ──

    #[test]
    fn test_lock_rotate_keys_plain() {
        let dir = make_dir();
        let _ = cmd_lock_rotate_keys(dir.path(), "old-key", "new-key", false);
    }

    #[test]
    fn test_lock_rotate_keys_json() {
        let dir = make_dir();
        let _ = cmd_lock_rotate_keys(dir.path(), "old-key", "new-key", true);
    }

    // ── lock_security: cmd_lock_backup ──

    #[test]
    fn test_lock_backup_plain() {
        let dir = make_dir();
        let _ = cmd_lock_backup(dir.path(), false);
    }

    #[test]
    fn test_lock_backup_json() {
        let dir = make_dir();
        let _ = cmd_lock_backup(dir.path(), true);
    }

    // ── lock_security: cmd_lock_verify_chain ──

    #[test]
    fn test_lock_verify_chain_plain() {
        let dir = make_dir();
        let _ = cmd_lock_verify_chain(dir.path(), false);
    }

    #[test]
    fn test_lock_verify_chain_json() {
        let dir = make_dir();
        let _ = cmd_lock_verify_chain(dir.path(), true);
    }

    // ── lock_security: cmd_lock_stats ──

    #[test]
    fn test_lock_stats_plain() {
        let dir = make_dir();
        let _ = cmd_lock_stats(dir.path(), false);
    }

    #[test]
    fn test_lock_stats_json() {
        let dir = make_dir();
        let _ = cmd_lock_stats(dir.path(), true);
    }

    // ── lock_ops: cmd_lock_compact ──

    #[test]
    fn test_lock_compact_plain() {
        let dir = make_dir_with_events();
        let _ = cmd_lock_compact(dir.path(), false, false);
    }

    #[test]
    fn test_lock_compact_json() {
        let dir = make_dir_with_events();
        let _ = cmd_lock_compact(dir.path(), true, true);
    }

    // ── lock_ops: cmd_lock_verify ──

    #[test]
    fn test_lock_verify_plain() {
        let dir = make_dir();
        // Also create the lock.yaml file inside the subdir for lock_ops verify
        write_yaml(dir.path(), "web/lock.yaml", state_lock_yaml());
        let _ = cmd_lock_verify(dir.path(), false);
    }

    #[test]
    fn test_lock_verify_json() {
        let dir = make_dir();
        write_yaml(dir.path(), "web/lock.yaml", state_lock_yaml());
        let _ = cmd_lock_verify(dir.path(), true);
    }

    // ── lock_ops: cmd_lock_export ──

    #[test]
    fn test_lock_export_json() {
        let dir = make_dir();
        let _ = cmd_lock_export(dir.path(), "json", None);
    }

    #[test]
    fn test_lock_export_csv() {
        let dir = make_dir();
        let _ = cmd_lock_export(dir.path(), "csv", Some("web"));
    }

    // ── lock_ops: cmd_lock_gc ──

    #[test]
    fn test_lock_gc_plain() {
        let dir = make_dir();
        let cfg = minimal_config(dir.path());
        let _ = cmd_lock_gc(&cfg, dir.path(), false, false);
    }

    #[test]
    fn test_lock_gc_json() {
        let dir = make_dir();
        let cfg = minimal_config(dir.path());
        let _ = cmd_lock_gc(&cfg, dir.path(), true, true);
    }

    // ── lock_ops: cmd_lock_diff ──

    #[test]
    fn test_lock_diff_plain() {
        let a = make_dir();
        let b = make_dir();
        let _ = cmd_lock_diff(a.path(), b.path(), false);
    }

    #[test]
    fn test_lock_diff_json() {
        let a = make_dir();
        let b = tempfile::tempdir().unwrap();
        let _ = cmd_lock_diff(a.path(), b.path(), true);
    }

    // ── lock_merge: cmd_lock_merge ──

    #[test]
    fn test_lock_merge_plain() {
        let a = make_dir();
        let b = make_dir();
        let out = tempfile::tempdir().unwrap();
        let _ = cmd_lock_merge(a.path(), b.path(), out.path(), false);
    }

    #[test]
    fn test_lock_merge_json() {
        let a = make_dir();
        let b = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let _ = cmd_lock_merge(a.path(), b.path(), out.path(), true);
    }

    // ── lock_merge: cmd_lock_rebase ──

    #[test]
    fn test_lock_rebase_plain() {
        let dir = make_dir();
        let cfg = minimal_config(dir.path());
        let out = tempfile::tempdir().unwrap();
        let _ = cmd_lock_rebase(dir.path(), &cfg, out.path(), false);
    }

    #[test]
    fn test_lock_rebase_json() {
        let dir = make_dir();
        let cfg = minimal_config(dir.path());
        let out = tempfile::tempdir().unwrap();
        let _ = cmd_lock_rebase(dir.path(), &cfg, out.path(), true);
    }

    // ── lock_merge: cmd_lock_sign ──

    #[test]
    fn test_lock_sign_plain() {
        let dir = make_dir();
        // cmd_lock_sign looks for {subdir}/lock.yaml
        write_yaml(dir.path(), "web/lock.yaml", state_lock_yaml());
        let _ = cmd_lock_sign(dir.path(), "sign-key", false);
    }

    #[test]
    fn test_lock_sign_json() {
        let dir = make_dir();
        write_yaml(dir.path(), "web/lock.yaml", state_lock_yaml());
        let _ = cmd_lock_sign(dir.path(), "sign-key", true);
    }

    // ── lock_lifecycle: cmd_lock_compress ──

    #[test]
    fn test_lock_compress_plain() {
        let dir = make_dir();
        let _ = cmd_lock_compress(dir.path(), false);
    }

    #[test]
    fn test_lock_compress_json() {
        let dir = make_dir();
        let _ = cmd_lock_compress(dir.path(), true);
    }

    // ── lock_lifecycle: cmd_lock_archive ──

    #[test]
    fn test_lock_archive_plain() {
        let dir = make_dir_with_events();
        let _ = cmd_lock_archive(dir.path(), false);
    }

    #[test]
    fn test_lock_archive_json() {
        let dir = make_dir_with_events();
        let _ = cmd_lock_archive(dir.path(), true);
    }

    // ── lock_lifecycle: cmd_lock_snapshot ──

    #[test]
    fn test_lock_snapshot_plain() {
        let dir = make_dir();
        let _ = cmd_lock_snapshot(dir.path(), false);
    }

    #[test]
    fn test_lock_snapshot_json() {
        let dir = make_dir();
        let _ = cmd_lock_snapshot(dir.path(), true);
    }

    // ── lock_lifecycle: cmd_lock_defrag ──

    #[test]
    fn test_lock_defrag_plain() {
        let dir = make_dir();
        let _ = cmd_lock_defrag(dir.path(), false);
    }

    #[test]
    fn test_lock_defrag_json() {
        let dir = make_dir();
        let _ = cmd_lock_defrag(dir.path(), true);
    }

    // ── lock_core: cmd_lock_info ──

    #[test]
    fn test_lock_info_plain() {
        let dir = make_dir();
        let _ = cmd_lock_info(dir.path(), false);
    }

    #[test]
    fn test_lock_info_json() {
        let dir = make_dir();
        let _ = cmd_lock_info(dir.path(), true);
    }

    // ── lock_core: cmd_lock_prune ──

    #[test]
    fn test_lock_prune_dry() {
        let dir = make_dir();
        let cfg = minimal_config(dir.path());
        let _ = cmd_lock_prune(&cfg, dir.path(), false);
    }

    #[test]
    fn test_lock_prune_yes() {
        let dir = make_dir();
        let cfg = minimal_config(dir.path());
        let _ = cmd_lock_prune(&cfg, dir.path(), true);
    }

    // ── lock_audit: cmd_lock_history ──

    #[test]
    fn test_lock_history_plain() {
        let dir = make_dir();
        let _ = cmd_lock_history(dir.path(), false, 10);
    }

    #[test]
    fn test_lock_history_json() {
        let dir = make_dir();
        let _ = cmd_lock_history(dir.path(), true, 10);
    }

    // ── lock_audit: cmd_lock_audit ──

    #[test]
    fn test_lock_audit_plain() {
        let dir = make_dir();
        let _ = cmd_lock_audit(dir.path(), false);
    }

    #[test]
    fn test_lock_audit_json() {
        let dir = make_dir();
        let _ = cmd_lock_audit(dir.path(), true);
    }

    // ── lock_audit: cmd_lock_verify_hmac ──

    #[test]
    fn test_lock_verify_hmac_plain() {
        let dir = make_dir();
        let _ = cmd_lock_verify_hmac(dir.path(), false);
    }

    #[test]
    fn test_lock_verify_hmac_json() {
        let dir = make_dir();
        let _ = cmd_lock_verify_hmac(dir.path(), true);
    }

    // ── lock_audit: cmd_lock_restore ──

    #[test]
    fn test_lock_restore_plain() {
        let dir = make_dir();
        let _ = cmd_lock_restore(dir.path(), None, false);
    }

    #[test]
    fn test_lock_restore_json() {
        let dir = make_dir();
        let _ = cmd_lock_restore(dir.path(), Some("nonexistent"), true);
    }

    // ── lock_audit: cmd_lock_verify_schema ──

    #[test]
    fn test_lock_verify_schema_plain() {
        let dir = make_dir();
        let _ = cmd_lock_verify_schema(dir.path(), false);
    }

    #[test]
    fn test_lock_verify_schema_json() {
        let dir = make_dir();
        let _ = cmd_lock_verify_schema(dir.path(), true);
    }

    // ── lock_audit: cmd_lock_tag ──

    #[test]
    fn test_lock_tag_plain() {
        let dir = make_dir();
        let _ = cmd_lock_tag(dir.path(), "env", "prod", false);
    }

    #[test]
    fn test_lock_tag_json() {
        let dir = make_dir();
        let _ = cmd_lock_tag(dir.path(), "env", "staging", true);
    }

    // ── lock_audit: cmd_lock_migrate ──

    #[test]
    fn test_lock_migrate_plain() {
        let dir = make_dir();
        let _ = cmd_lock_migrate(dir.path(), "0.9", false);
    }

    #[test]
    fn test_lock_migrate_json() {
        let dir = make_dir();
        let _ = cmd_lock_migrate(dir.path(), "0.9", true);
    }

    // ── lock_core: cmd_lock_validate ──

    #[test]
    fn test_lock_validate_with_data_plain() {
        let dir = make_dir();
        let _ = cmd_lock_validate(dir.path(), false);
    }

    #[test]
    fn test_lock_validate_with_data_json() {
        let dir = make_dir();
        let _ = cmd_lock_validate(dir.path(), true);
    }

    // ── lock_core: cmd_lock_integrity ──

    #[test]
    fn test_lock_integrity_with_data_plain() {
        let dir = make_dir();
        let _ = cmd_lock_integrity(dir.path(), false);
    }

    #[test]
    fn test_lock_integrity_with_data_json() {
        let dir = make_dir();
        let _ = cmd_lock_integrity(dir.path(), true);
    }
}

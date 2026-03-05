//! Tests: Lock management.

#![allow(unused_imports)]
use super::commands::*;
use super::dispatch::*;
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::lock_audit::*;
use super::lock_core::*;
use super::lock_lifecycle::*;
use super::lock_repair::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj415_lock_export_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch(
            Commands::LockExport(LockExportArgs {
                state_dir: state,
                format: "json".to_string(),
                machine: None,
            }),
            0,
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj425_lock_gc_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let config = dir.path().join("forjar.yaml");
        std::fs::write(
            &config,
            r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources: {}
"#,
        )
        .unwrap();
        let result = dispatch(
            Commands::LockGc(LockGcArgs {
                file: config,
                state_dir: state,
                yes: false,
                json: true,
            }),
            0,
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj435_lock_diff_dispatch() {
        let cmd = Commands::LockDiff(LockDiffArgs {
            from: PathBuf::from("state-a"),
            to: PathBuf::from("state-b"),
            json: false,
        });
        match cmd {
            Commands::LockDiff(LockDiffArgs { from, to, json }) => {
                assert_eq!(from, PathBuf::from("state-a"));
                assert_eq!(to, PathBuf::from("state-b"));
                assert!(!json);
            }
            _ => panic!("expected LockDiff"),
        }
    }

    #[test]
    fn test_fj445_lock_merge_dispatch() {
        let cmd = Commands::LockMerge(LockMergeArgs {
            from: PathBuf::from("state-a"),
            to: PathBuf::from("state-b"),
            output: PathBuf::from("state-merged"),
            json: false,
        });
        match cmd {
            Commands::LockMerge(LockMergeArgs {
                from, to, output, ..
            }) => {
                assert_eq!(from, PathBuf::from("state-a"));
                assert_eq!(to, PathBuf::from("state-b"));
                assert_eq!(output, PathBuf::from("state-merged"));
            }
            _ => panic!("expected LockMerge"),
        }
    }

    #[test]
    fn test_fj455_lock_rebase_dispatch() {
        let cmd = Commands::LockRebase(LockRebaseArgs {
            from: PathBuf::from("old-state"),
            file: PathBuf::from("forjar.yaml"),
            output: PathBuf::from("new-state"),
            json: false,
        });
        match cmd {
            Commands::LockRebase(LockRebaseArgs {
                from, file, output, ..
            }) => {
                assert_eq!(from, PathBuf::from("old-state"));
                assert_eq!(file, PathBuf::from("forjar.yaml"));
                assert_eq!(output, PathBuf::from("new-state"));
            }
            _ => panic!("expected LockRebase"),
        }
    }

    #[test]
    fn test_fj465_lock_sign_dispatch() {
        let cmd = Commands::LockSign(LockSignArgs {
            state_dir: PathBuf::from("state"),
            key: "my-signing-key".to_string(),
            json: false,
        });
        match cmd {
            Commands::LockSign(LockSignArgs { key, .. }) => assert_eq!(key, "my-signing-key"),
            _ => panic!("expected LockSign"),
        }
    }

    #[test]
    fn test_fj475_lock_verify_sig_dispatch() {
        let cmd = Commands::LockVerifySig(LockVerifySigArgs {
            state_dir: PathBuf::from("state"),
            key: "my-verify-key".to_string(),
            json: false,
        });
        match cmd {
            Commands::LockVerifySig(LockVerifySigArgs { key, .. }) => {
                assert_eq!(key, "my-verify-key")
            }
            _ => panic!("expected LockVerifySig"),
        }
    }

    #[test]
    fn test_fj485_lock_compact_all_dispatch() {
        let cmd = Commands::LockCompactAll(LockCompactAllArgs {
            state_dir: PathBuf::from("state"),
            yes: false,
            json: false,
        });
        match cmd {
            Commands::LockCompactAll(LockCompactAllArgs { yes, .. }) => assert!(!yes),
            _ => panic!("expected LockCompactAll"),
        }
    }

    #[test]
    fn test_fj495_lock_audit_trail_dispatch() {
        let cmd = Commands::LockAuditTrail(LockAuditTrailArgs {
            state_dir: PathBuf::from("state"),
            machine: None,
            json: false,
        });
        match cmd {
            Commands::LockAuditTrail(LockAuditTrailArgs { machine, .. }) => {
                assert!(machine.is_none())
            }
            _ => panic!("expected LockAuditTrail"),
        }
    }

    #[test]
    fn test_fj505_lock_rotate_keys_dispatch() {
        let cmd = Commands::LockRotateKeys(LockRotateKeysArgs {
            state_dir: PathBuf::from("state"),
            old_key: "old-key".to_string(),
            new_key: "new-key".to_string(),
            json: false,
        });
        match cmd {
            Commands::LockRotateKeys(LockRotateKeysArgs {
                old_key, new_key, ..
            }) => {
                assert_eq!(old_key, "old-key");
                assert_eq!(new_key, "new-key");
            }
            _ => panic!("expected LockRotateKeys"),
        }
    }

    #[test]
    fn test_fj515_lock_backup_command() {
        let cmd = Commands::LockBackup(LockBackupArgs {
            state_dir: PathBuf::from("state"),
            json: false,
        });
        match cmd {
            Commands::LockBackup(LockBackupArgs { state_dir, .. }) => {
                assert_eq!(state_dir, PathBuf::from("state"))
            }
            _ => panic!("expected LockBackup"),
        }
    }

    #[test]
    fn test_fj525_lock_gc_already_exists() {
        // LockGc was added in an earlier phase — verify it still constructs
        let cmd = Commands::LockGc(LockGcArgs {
            file: PathBuf::from("forjar.yaml"),
            state_dir: PathBuf::from("state"),
            yes: false,
            json: true,
        });
        match cmd {
            Commands::LockGc(LockGcArgs { json, .. }) => assert!(json),
            _ => panic!("expected LockGc"),
        }
    }

    #[test]
    fn test_fj535_lock_verify_chain_command() {
        let cmd = Commands::LockVerifyChain(LockVerifyChainArgs {
            state_dir: PathBuf::from("state"),
            json: false,
        });
        match cmd {
            Commands::LockVerifyChain(LockVerifyChainArgs { state_dir, .. }) => {
                assert_eq!(state_dir, PathBuf::from("state"))
            }
            _ => panic!("expected LockVerifyChain"),
        }
    }

    #[test]
    fn test_fj545_lock_stats_command() {
        let cmd = Commands::LockStats(LockStatsArgs {
            state_dir: PathBuf::from("state"),
            json: true,
        });
        match cmd {
            Commands::LockStats(LockStatsArgs { json, .. }) => assert!(json),
            _ => panic!("expected LockStats"),
        }
    }

    #[test]
    fn test_fj555_lock_audit_command() {
        let cmd = Commands::LockAudit(LockAuditArgs {
            state_dir: PathBuf::from("state"),
            json: true,
        });
        match cmd {
            Commands::LockAudit(LockAuditArgs { json, .. }) => assert!(json),
            _ => panic!("expected LockAudit"),
        }
    }

    #[test]
    fn test_fj565_lock_compress_command() {
        let cmd = Commands::LockCompress(LockCompressArgs {
            state_dir: PathBuf::from("state"),
            json: true,
        });
        match cmd {
            Commands::LockCompress(LockCompressArgs { json, .. }) => assert!(json),
            _ => panic!("expected LockCompress"),
        }
    }

    #[test]
    fn test_fj575_lock_defrag_command() {
        let cmd = Commands::LockDefrag(LockDefragArgs {
            state_dir: PathBuf::from("state"),
            json: true,
        });
        match cmd {
            Commands::LockDefrag(LockDefragArgs { json, .. }) => assert!(json),
            _ => panic!("expected LockDefrag"),
        }
    }

    #[test]
    fn test_fj585_lock_normalize_command() {
        let cmd = Commands::LockNormalize(LockNormalizeArgs {
            state_dir: PathBuf::from("state"),
            json: true,
        });
        match cmd {
            Commands::LockNormalize(LockNormalizeArgs { json, .. }) => assert!(json),
            _ => panic!("expected LockNormalize"),
        }
    }

    #[test]
    fn test_fj596_lock_validate() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_validate(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj596_lock_validate_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_validate(dir.path(), true);
        assert!(result.is_ok());
    }

    // ── Phase 46 Tests: FJ-600→FJ-607 Security Hardening & Audit ──

    #[test]
    fn test_fj605_lock_verify_hmac() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_verify_hmac(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj605_lock_verify_hmac_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_verify_hmac(dir.path(), true);
        assert!(result.is_ok());
    }

    // ── Phase 47 Tests: FJ-610→FJ-617 Resource Intelligence & Analytics ──

    #[test]
    fn test_fj615_lock_archive() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_archive(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj615_lock_archive_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_archive(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj625_lock_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_snapshot(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj625_lock_snapshot_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_snapshot(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj635_lock_repair() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_repair(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj635_lock_repair_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_repair(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj645_lock_history() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_history(dir.path(), false, 20);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj675_lock_integrity() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_integrity(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj675_lock_integrity_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_integrity(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj685_lock_rehash() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_rehash(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj685_lock_rehash_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_rehash(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj695_lock_restore() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_restore(dir.path(), None, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj705_lock_verify_schema() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_verify_schema(dir.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj705_lock_verify_schema_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_verify_schema(dir.path(), true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj715_lock_tag() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_tag(dir.path(), "env", "prod", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj725_lock_migrate() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_migrate(dir.path(), "0.9", false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_fj725_lock_migrate_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = cmd_lock_migrate(dir.path(), "0.9", true);
        assert!(result.is_ok());
    }
}

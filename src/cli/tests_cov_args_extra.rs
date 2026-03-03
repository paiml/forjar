//! Tests: Coverage for remaining args structs (lock_core_args, lock_ops_args, misc_args, misc_ops_args, plan_args, state_args, commands/mod.rs).

use super::commands::*;
use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;

    // ── lock_core_args.rs (48 uncov) ──

    #[test]
    fn test_cov_lock_args_construct() {
        let a = LockArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("s"),
            env_file: None,
            workspace: None,
            verify: false,
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_prune_args_construct() {
        let a = LockPruneArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("s"),
            yes: true,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_info_args_construct() {
        let a = LockInfoArgs {
            state_dir: PathBuf::from("s"),
            json: true,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_compact_args_construct() {
        let a = LockCompactArgs {
            state_dir: PathBuf::from("s"),
            yes: false,
            json: true,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_gc_args_construct() {
        let a = LockGcArgs {
            file: PathBuf::from("f.yaml"),
            state_dir: PathBuf::from("s"),
            yes: false,
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_export_args_construct() {
        let a = LockExportArgs {
            state_dir: PathBuf::from("s"),
            format: "json".to_string(),
            machine: Some("m".to_string()),
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_verify_args_construct() {
        let a = LockVerifyArgs {
            state_dir: PathBuf::from("s"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_diff_args_construct() {
        let a = LockDiffArgs {
            from: PathBuf::from("a"),
            to: PathBuf::from("b"),
            json: true,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_merge_args_construct() {
        let a = LockMergeArgs {
            from: PathBuf::from("a"),
            to: PathBuf::from("b"),
            output: PathBuf::from("o"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_rebase_args_construct() {
        let a = LockRebaseArgs {
            from: PathBuf::from("a"),
            file: PathBuf::from("f.yaml"),
            output: PathBuf::from("o"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_sign_args_construct() {
        let a = LockSignArgs {
            state_dir: PathBuf::from("s"),
            key: "key".to_string(),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_verify_sig_args_construct() {
        let a = LockVerifySigArgs {
            state_dir: PathBuf::from("s"),
            key: "key".to_string(),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_compact_all_args_construct() {
        let a = LockCompactAllArgs {
            state_dir: PathBuf::from("s"),
            yes: true,
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_audit_trail_args_construct() {
        let a = LockAuditTrailArgs {
            state_dir: PathBuf::from("s"),
            machine: None,
            json: true,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_rotate_keys_args_construct() {
        let a = LockRotateKeysArgs {
            state_dir: PathBuf::from("s"),
            old_key: "old".to_string(),
            new_key: "new".to_string(),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_backup_args_construct() {
        let a = LockBackupArgs {
            state_dir: PathBuf::from("s"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    // ── lock_ops_args.rs (40 uncov) ──

    #[test]
    fn test_cov_lock_verify_chain_args_construct() {
        let a = LockVerifyChainArgs {
            state_dir: PathBuf::from("s"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_stats_args_construct() {
        let a = LockStatsArgs {
            state_dir: PathBuf::from("s"),
            json: true,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_audit_args_construct() {
        let a = LockAuditArgs {
            state_dir: PathBuf::from("s"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_compress_args_construct() {
        let a = LockCompressArgs {
            state_dir: PathBuf::from("s"),
            json: true,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_defrag_args_construct() {
        let a = LockDefragArgs {
            state_dir: PathBuf::from("s"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_normalize_args_construct() {
        let a = LockNormalizeArgs {
            state_dir: PathBuf::from("s"),
            json: true,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_validate_args_construct() {
        let a = LockValidateArgs {
            state_dir: PathBuf::from("s"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_verify_hmac_args_construct() {
        let a = LockVerifyHmacArgs {
            state_dir: PathBuf::from("s"),
            json: true,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_archive_args_construct() {
        let a = LockArchiveArgs {
            state_dir: PathBuf::from("s"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_snapshot_args_construct() {
        let a = LockSnapshotArgs {
            state_dir: PathBuf::from("s"),
            json: true,
        };
        let _ = format!("{:?}", a);
    }

    #[test]
    fn test_cov_lock_repair_args_construct() {
        let a = LockRepairArgs {
            state_dir: PathBuf::from("s"),
            json: false,
        };
        let _ = format!("{:?}", a);
    }
}

//! Tests: Coverage for lock dispatch commands (part 4b, split from 4).

#![allow(unused_imports)]
use super::commands::*;
use super::dispatch_lock::*;
use super::test_fixtures::*;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cov_dispatch_lock_compress() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockCompress(LockCompressArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_rehash() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockRehash(LockRehashArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_verify_hmac() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockVerifyHmac(LockVerifyHmacArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_archive() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockArchive(LockArchiveArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockSnapshot(LockSnapshotArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_repair() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockRepair(LockRepairArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_verify_chain() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockVerifyChain(LockVerifyChainArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_verify_schema() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockVerifySchema(LockVerifySchemaArgs {
            state_dir: state,
            json: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_cov_dispatch_lock_compact_all() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let result = dispatch_lock_cmd(Commands::LockCompactAll(LockCompactAllArgs {
            state_dir: state,
            yes: false,
            json: false,
        }));
        assert!(result.is_ok());
    }
}

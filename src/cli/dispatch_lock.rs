//! Lock command dispatch — routes lock sub-commands to handlers.

use std::path::Path;
use super::commands::*;
use super::helpers::*;
use super::helpers_state::*;
use super::lock_core::*;
use super::lock_repair::*;
use super::lock_lifecycle::*;
use super::lock_audit::*;
use super::lock_ops::*;
use super::lock_merge::*;
use super::lock_security::*;
use super::workspace::*;


/// Dispatch lock-related commands.
pub(crate) fn dispatch_lock_cmd(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::Lock(LockArgs { file, state_dir, env_file, workspace, verify, json }) => {
            let sd = resolve_state_dir(&state_dir, workspace.as_deref());
            cmd_lock(&file, &sd, env_file.as_deref(), workspace.as_deref(), verify, json)
        }
        Commands::LockPrune(LockPruneArgs { file, state_dir, yes }) => cmd_lock_prune(&file, &state_dir, yes),
        Commands::LockInfo(LockInfoArgs { state_dir, json }) => cmd_lock_info(&state_dir, json),
        Commands::LockCompact(LockCompactArgs { state_dir, yes, json }) => cmd_lock_compact(&state_dir, yes, json),
        Commands::LockGc(LockGcArgs { file, state_dir, yes, json }) => cmd_lock_gc(&file, &state_dir, yes, json),
        Commands::LockExport(LockExportArgs { state_dir, format: fmt, machine }) => {
            cmd_lock_export(&state_dir, &fmt, machine.as_deref())
        }
        Commands::LockVerify(LockVerifyArgs { state_dir, json }) => cmd_lock_verify(&state_dir, json),
        Commands::LockDiff(LockDiffArgs { from, to, json }) => cmd_lock_diff(&from, &to, json),
        Commands::LockMerge(LockMergeArgs { from, to, output, json }) => cmd_lock_merge(&from, &to, &output, json),
        Commands::LockRebase(LockRebaseArgs { from, file, output, json }) => cmd_lock_rebase(&from, &file, &output, json),
        Commands::LockSign(LockSignArgs { state_dir, key, json }) => cmd_lock_sign(&state_dir, &key, json),
        Commands::LockVerifySig(LockVerifySigArgs { state_dir, key, json }) => cmd_lock_verify_sig(&state_dir, &key, json),
        Commands::LockCompactAll(LockCompactAllArgs { state_dir, yes, json }) => cmd_lock_compact_all(&state_dir, yes, json),
        Commands::LockAuditTrail(LockAuditTrailArgs { state_dir, machine, json }) => {
            cmd_lock_audit_trail(&state_dir, machine.as_deref(), json)
        }
        Commands::LockRotateKeys(LockRotateKeysArgs { state_dir, old_key, new_key, json }) => {
            cmd_lock_rotate_keys(&state_dir, &old_key, &new_key, json)
        }
        Commands::LockBackup(LockBackupArgs { state_dir, json }) => cmd_lock_backup(&state_dir, json),
        Commands::LockVerifyChain(LockVerifyChainArgs { state_dir, json }) => cmd_lock_verify_chain(&state_dir, json),
        Commands::LockStats(LockStatsArgs { state_dir, json }) => cmd_lock_stats(&state_dir, json),
        Commands::LockAudit(LockAuditArgs { state_dir, json }) => cmd_lock_audit(&state_dir, json),
        Commands::LockCompress(LockCompressArgs { state_dir, json }) => cmd_lock_compress(&state_dir, json),
        Commands::LockDefrag(LockDefragArgs { state_dir, json }) => cmd_lock_defrag(&state_dir, json),
        Commands::LockNormalize(LockNormalizeArgs { state_dir, json }) => cmd_lock_normalize(&state_dir, json),
        Commands::LockValidate(LockValidateArgs { state_dir, json }) => cmd_lock_validate(&state_dir, json),
        Commands::LockVerifyHmac(LockVerifyHmacArgs { state_dir, json }) => cmd_lock_verify_hmac(&state_dir, json),
        Commands::LockArchive(LockArchiveArgs { state_dir, json }) => cmd_lock_archive(&state_dir, json),
        Commands::LockSnapshot(LockSnapshotArgs { state_dir, json }) => cmd_lock_snapshot(&state_dir, json),
        Commands::LockRepair(LockRepairArgs { state_dir, json }) => cmd_lock_repair(&state_dir, json),
        Commands::LockHistory(LockHistoryArgs { state_dir, json, limit }) => cmd_lock_history(&state_dir, json, limit),
        Commands::LockIntegrity(LockIntegrityArgs { state_dir, json }) => cmd_lock_integrity(&state_dir, json),
        Commands::LockRehash(LockRehashArgs { state_dir, json }) => cmd_lock_rehash(&state_dir, json),
        Commands::LockRestore(LockRestoreArgs { state_dir, name, json }) => {
            cmd_lock_restore(&state_dir, name.as_deref(), json)
        }
        Commands::LockVerifySchema(LockVerifySchemaArgs { state_dir, json }) => cmd_lock_verify_schema(&state_dir, json),
        Commands::LockTag(LockTagArgs { state_dir, name, value, json }) => {
            cmd_lock_tag(&state_dir, &name, &value, json)
        }
        Commands::LockMigrate(LockMigrateArgs { state_dir, from_version, json }) => {
            cmd_lock_migrate(&state_dir, &from_version, json)
        }
        _ => unreachable!(),
    }
}

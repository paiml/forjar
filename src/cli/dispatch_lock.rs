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
        Commands::Lock { file, state_dir, env_file, workspace, verify, json } => {
            let sd = resolve_state_dir(&state_dir, workspace.as_deref());
            cmd_lock(&file, &sd, env_file.as_deref(), workspace.as_deref(), verify, json)
        }
        Commands::LockPrune { file, state_dir, yes } => cmd_lock_prune(&file, &state_dir, yes),
        Commands::LockInfo { state_dir, json } => cmd_lock_info(&state_dir, json),
        Commands::LockCompact { state_dir, yes, json } => cmd_lock_compact(&state_dir, yes, json),
        Commands::LockGc { file, state_dir, yes, json } => cmd_lock_gc(&file, &state_dir, yes, json),
        Commands::LockExport { state_dir, format: fmt, machine } => {
            cmd_lock_export(&state_dir, &fmt, machine.as_deref())
        }
        Commands::LockVerify { state_dir, json } => cmd_lock_verify(&state_dir, json),
        Commands::LockDiff { from, to, json } => cmd_lock_diff(&from, &to, json),
        Commands::LockMerge { from, to, output, json } => cmd_lock_merge(&from, &to, &output, json),
        Commands::LockRebase { from, file, output, json } => cmd_lock_rebase(&from, &file, &output, json),
        Commands::LockSign { state_dir, key, json } => cmd_lock_sign(&state_dir, &key, json),
        Commands::LockVerifySig { state_dir, key, json } => cmd_lock_verify_sig(&state_dir, &key, json),
        Commands::LockCompactAll { state_dir, yes, json } => cmd_lock_compact_all(&state_dir, yes, json),
        Commands::LockAuditTrail { state_dir, machine, json } => {
            cmd_lock_audit_trail(&state_dir, machine.as_deref(), json)
        }
        Commands::LockRotateKeys { state_dir, old_key, new_key, json } => {
            cmd_lock_rotate_keys(&state_dir, &old_key, &new_key, json)
        }
        Commands::LockBackup { state_dir, json } => cmd_lock_backup(&state_dir, json),
        Commands::LockVerifyChain { state_dir, json } => cmd_lock_verify_chain(&state_dir, json),
        Commands::LockStats { state_dir, json } => cmd_lock_stats(&state_dir, json),
        Commands::LockAudit { state_dir, json } => cmd_lock_audit(&state_dir, json),
        Commands::LockCompress { state_dir, json } => cmd_lock_compress(&state_dir, json),
        Commands::LockDefrag { state_dir, json } => cmd_lock_defrag(&state_dir, json),
        Commands::LockNormalize { state_dir, json } => cmd_lock_normalize(&state_dir, json),
        Commands::LockValidate { state_dir, json } => cmd_lock_validate(&state_dir, json),
        Commands::LockVerifyHmac { state_dir, json } => cmd_lock_verify_hmac(&state_dir, json),
        Commands::LockArchive { state_dir, json } => cmd_lock_archive(&state_dir, json),
        Commands::LockSnapshot { state_dir, json } => cmd_lock_snapshot(&state_dir, json),
        Commands::LockRepair { state_dir, json } => cmd_lock_repair(&state_dir, json),
        Commands::LockHistory { state_dir, json, limit } => cmd_lock_history(&state_dir, json, limit),
        Commands::LockIntegrity { state_dir, json } => cmd_lock_integrity(&state_dir, json),
        Commands::LockRehash { state_dir, json } => cmd_lock_rehash(&state_dir, json),
        Commands::LockRestore { state_dir, name, json } => {
            cmd_lock_restore(&state_dir, name.as_deref(), json)
        }
        Commands::LockVerifySchema { state_dir, json } => cmd_lock_verify_schema(&state_dir, json),
        Commands::LockTag { state_dir, name, value, json } => {
            cmd_lock_tag(&state_dir, &name, &value, json)
        }
        Commands::LockMigrate { state_dir, from_version, json } => {
            cmd_lock_migrate(&state_dir, &from_version, json)
        }
        _ => unreachable!(),
    }
}

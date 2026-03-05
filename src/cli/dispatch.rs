//! CLI dispatch — routes commands to handlers.

use super::commands::*;
use super::destroy::*;
use super::dispatch_apply::dispatch_apply_cmd;
use super::dispatch_graph::dispatch_graph_cmd;
use super::dispatch_lock::dispatch_lock_cmd;
use super::dispatch_misc::dispatch_misc_cmd;
use super::dispatch_status::dispatch_status_cmd;
use super::dispatch_store::dispatch_store_cmd;
use super::dispatch_validate::dispatch_validate;
use super::drift::*;
use super::helpers::*;
use super::init::*;
use super::plan::*;
use super::workspace::resolve_state_dir;
use std::sync::atomic::Ordering;

/// Dispatch a CLI command.
pub fn dispatch(cmd: Commands, verbose: u8, no_color: bool) -> Result<(), String> {
    let verbose = verbose > 0;
    NO_COLOR.store(no_color, Ordering::Relaxed);
    match cmd {
        Commands::Init(InitArgs { path }) => cmd_init(&path),
        Commands::Validate(args) => dispatch_validate(args),
        Commands::Plan(PlanArgs {
            file,
            machine,
            resource,
            tag,
            group: _group,
            state_dir,
            json,
            output_dir,
            env_file,
            workspace,
            no_diff,
            target,
            cost,
            what_if,
            out,
            why,
        }) => {
            let sd = resolve_state_dir(&state_dir, workspace.as_deref());
            cmd_plan(
                &file,
                &sd,
                machine.as_deref(),
                resource.as_deref(),
                tag.as_deref(),
                json,
                verbose,
                output_dir.as_deref(),
                env_file.as_deref(),
                workspace.as_deref(),
                no_diff,
                target.as_deref(),
                cost,
                &what_if,
                out.as_deref(),
                why,
            )
        }
        cmd @ Commands::Apply(..) => dispatch_apply_cmd(cmd, verbose),
        Commands::Drift(DriftArgs {
            file,
            machine,
            state_dir,
            tripwire,
            alert_cmd,
            auto_remediate,
            dry_run,
            json,
            env_file,
            workspace,
        }) => {
            let sd = resolve_state_dir(&state_dir, workspace.as_deref());
            cmd_drift(
                &file,
                &sd,
                machine.as_deref(),
                tripwire,
                alert_cmd.as_deref(),
                auto_remediate,
                dry_run,
                json,
                verbose,
                env_file.as_deref(),
            )
        }
        Commands::Destroy(DestroyArgs {
            file,
            machine,
            yes,
            state_dir,
        }) => cmd_destroy(&file, &state_dir, machine.as_deref(), yes, verbose),
        cmd @ Commands::Status(..) => dispatch_status_cmd(cmd),
        cmd @ Commands::Graph(..) => dispatch_graph_cmd(cmd),
        // Delegate all lock commands
        cmd @ Commands::Lock(..)
        | cmd @ Commands::LockPrune(..)
        | cmd @ Commands::LockInfo(..)
        | cmd @ Commands::LockCompact(..)
        | cmd @ Commands::LockGc(..)
        | cmd @ Commands::LockExport(..)
        | cmd @ Commands::LockVerify(..)
        | cmd @ Commands::LockDiff(..)
        | cmd @ Commands::LockMerge(..)
        | cmd @ Commands::LockRebase(..)
        | cmd @ Commands::LockSign(..)
        | cmd @ Commands::LockVerifySig(..)
        | cmd @ Commands::LockCompactAll(..)
        | cmd @ Commands::LockAuditTrail(..)
        | cmd @ Commands::LockRotateKeys(..)
        | cmd @ Commands::LockBackup(..)
        | cmd @ Commands::LockVerifyChain(..)
        | cmd @ Commands::LockStats(..)
        | cmd @ Commands::LockAudit(..)
        | cmd @ Commands::LockCompress(..)
        | cmd @ Commands::LockDefrag(..)
        | cmd @ Commands::LockNormalize(..)
        | cmd @ Commands::LockValidate(..)
        | cmd @ Commands::LockVerifyHmac(..)
        | cmd @ Commands::LockArchive(..)
        | cmd @ Commands::LockSnapshot(..)
        | cmd @ Commands::LockRepair(..)
        | cmd @ Commands::LockHistory(..)
        | cmd @ Commands::LockIntegrity(..)
        | cmd @ Commands::LockRehash(..)
        | cmd @ Commands::LockRestore(..)
        | cmd @ Commands::LockVerifySchema(..)
        | cmd @ Commands::LockTag(..)
        | cmd @ Commands::LockMigrate(..) => dispatch_lock_cmd(cmd),
        // Store-related commands (pin, cache, store, archive, convert)
        cmd @ Commands::Pin(..)
        | cmd @ Commands::Cache(..)
        | cmd @ Commands::Store(..)
        | cmd @ Commands::Archive(..)
        | cmd @ Commands::Convert(..)
        | cmd @ Commands::StoreImport(..) => dispatch_store_cmd(cmd),
        // All remaining commands
        cmd => dispatch_misc_cmd(cmd, verbose),
    }
}

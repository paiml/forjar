//! CLI dispatch — routes commands to handlers.

use std::sync::atomic::Ordering;
use super::commands::*;
use super::helpers::*;
use super::helpers_state::*;
use super::plan::*;
use super::drift::*;
use super::history::*;
use super::doctor::*;
use super::import_cmd::*;
use super::lint::*;
use super::workspace::*;
use super::secrets::*;
use super::snapshot::*;
use super::init::*;
use super::show::*;
use super::check::*;
use super::diff_cmd::*;
use super::destroy::*;
use super::observe::*;
use super::fleet_ops::*;
use super::fleet_reporting::*;
use super::infra::*;
use super::dispatch_apply::dispatch_apply_cmd;
use super::dispatch_status::dispatch_status_cmd;
use super::dispatch_graph::dispatch_graph_cmd;
use super::dispatch_validate::dispatch_validate;
use super::dispatch_lock::dispatch_lock_cmd;


/// Dispatch a CLI command.
pub fn dispatch(cmd: Commands, verbose: bool, no_color: bool) -> Result<(), String> {
    NO_COLOR.store(no_color, Ordering::Relaxed);
    match cmd {
        Commands::Init { path } => cmd_init(&path),
        Commands::Validate {
            file, strict, json, dry_expand,
            schema_version: _schema_version, exhaustive, policy_file,
            check_connectivity, check_templates, strict_deps, check_secrets,
            check_idempotency, check_drift_coverage, check_cycles_deep,
            check_naming, check_overlaps, check_limits, check_complexity,
            check_security, check_deprecation, check_drift_risk, check_compliance,
            check_portability, check_resource_limits, check_unused,
            check_dependencies, check_permissions, check_idempotency_deep,
            check_machine_reachability, check_circular_refs,
            check_naming_conventions, check_owner_consistency,
            check_path_conflicts, check_service_deps, check_template_vars,
            check_mode_consistency, check_group_consistency, check_mount_points,
        } => dispatch_validate(
            &file, strict, json, dry_expand, exhaustive, policy_file.as_deref(),
            check_connectivity, check_templates, strict_deps, check_secrets,
            check_idempotency, check_drift_coverage, check_cycles_deep,
            check_naming, check_overlaps, check_limits, check_complexity,
            check_security, check_deprecation, check_drift_risk,
            check_compliance.as_deref(), check_portability, check_resource_limits,
            check_unused, check_dependencies, check_permissions,
            check_idempotency_deep, check_machine_reachability,
            check_circular_refs, check_naming_conventions,
            check_owner_consistency, check_path_conflicts, check_service_deps,
            check_template_vars, check_mode_consistency, check_group_consistency,
            check_mount_points,
        ),
        Commands::Plan {
            file, machine, resource, tag, group: _group,
            state_dir, json, output_dir, env_file, workspace,
            no_diff, target, cost, what_if,
        } => {
            let sd = resolve_state_dir(&state_dir, workspace.as_deref());
            cmd_plan(
                &file, &sd, machine.as_deref(), resource.as_deref(),
                tag.as_deref(), json, verbose, output_dir.as_deref(),
                env_file.as_deref(), workspace.as_deref(), no_diff,
                target.as_deref(), cost, &what_if,
            )
        }
        cmd @ Commands::Apply { .. } => dispatch_apply_cmd(cmd, verbose),
        Commands::Drift {
            file, machine, state_dir, tripwire, alert_cmd,
            auto_remediate, dry_run, json, env_file, workspace,
        } => {
            let sd = resolve_state_dir(&state_dir, workspace.as_deref());
            cmd_drift(
                &file, &sd, machine.as_deref(), tripwire, alert_cmd.as_deref(),
                auto_remediate, dry_run, json, verbose, env_file.as_deref(),
            )
        }
        Commands::Destroy { file, machine, yes, state_dir } => {
            cmd_destroy(&file, &state_dir, machine.as_deref(), yes, verbose)
        }
        cmd @ Commands::Status { .. } => dispatch_status_cmd(cmd),
        cmd @ Commands::Graph { .. } => dispatch_graph_cmd(cmd),
        // Delegate all lock commands
        cmd @ Commands::Lock { .. }
        | cmd @ Commands::LockPrune { .. }
        | cmd @ Commands::LockInfo { .. }
        | cmd @ Commands::LockCompact { .. }
        | cmd @ Commands::LockGc { .. }
        | cmd @ Commands::LockExport { .. }
        | cmd @ Commands::LockVerify { .. }
        | cmd @ Commands::LockDiff { .. }
        | cmd @ Commands::LockMerge { .. }
        | cmd @ Commands::LockRebase { .. }
        | cmd @ Commands::LockSign { .. }
        | cmd @ Commands::LockVerifySig { .. }
        | cmd @ Commands::LockCompactAll { .. }
        | cmd @ Commands::LockAuditTrail { .. }
        | cmd @ Commands::LockRotateKeys { .. }
        | cmd @ Commands::LockBackup { .. }
        | cmd @ Commands::LockVerifyChain { .. }
        | cmd @ Commands::LockStats { .. }
        | cmd @ Commands::LockAudit { .. }
        | cmd @ Commands::LockCompress { .. }
        | cmd @ Commands::LockDefrag { .. }
        | cmd @ Commands::LockNormalize { .. }
        | cmd @ Commands::LockValidate { .. }
        | cmd @ Commands::LockVerifyHmac { .. }
        | cmd @ Commands::LockArchive { .. }
        | cmd @ Commands::LockSnapshot { .. }
        | cmd @ Commands::LockRepair { .. }
        | cmd @ Commands::LockHistory { .. }
        | cmd @ Commands::LockIntegrity { .. }
        | cmd @ Commands::LockRehash { .. }
        | cmd @ Commands::LockRestore { .. }
        | cmd @ Commands::LockVerifySchema { .. }
        | cmd @ Commands::LockTag { .. }
        | cmd @ Commands::LockMigrate { .. } => dispatch_lock_cmd(cmd),
        // Remaining commands
        Commands::History {
            state_dir, machine, limit, json, since, resource,
        } => {
            if let Some(ref res) = resource {
                return cmd_history_resource(&state_dir, res, limit, json);
            }
            cmd_history(&state_dir, machine.as_deref(), limit, json, since.as_deref())
        }
        Commands::Show { file, resource, json } => cmd_show(&file, resource.as_deref(), json),
        Commands::Import { addr, user, name, output, scan } => {
            cmd_import(&addr, &user, name.as_deref(), &output, &scan, verbose)
        }
        Commands::Diff { from, to, machine, resource, json } => {
            cmd_diff(&from, &to, machine.as_deref(), resource.as_deref(), json)
        }
        Commands::Check { file, machine, resource, tag, json } => {
            cmd_check(&file, machine.as_deref(), resource.as_deref(), tag.as_deref(), json, verbose)
        }
        Commands::Fmt { file, check } => cmd_fmt(&file, check),
        Commands::Lint { file, json, strict, fix, rules: _rules } => cmd_lint(&file, json, strict, fix),
        Commands::Rollback { file, revision, machine, dry_run, state_dir } => {
            cmd_rollback(&file, &state_dir, revision, machine.as_deref(), dry_run, verbose)
        }
        Commands::Anomaly { state_dir, machine, min_events, json } => {
            cmd_anomaly(&state_dir, machine.as_deref(), min_events, json)
        }
        Commands::Trace { state_dir, machine, json } => cmd_trace(&state_dir, machine.as_deref(), json),
        Commands::Migrate { file, output } => cmd_migrate(&file, output.as_deref()),
        Commands::Mcp { schema } => {
            if schema { cmd_mcp_schema() } else { cmd_mcp() }
        }
        Commands::Bench { iterations, json } => cmd_bench(iterations, json),
        Commands::StateList { state_dir, machine, json } => {
            cmd_state_list(&state_dir, machine.as_deref(), json)
        }
        Commands::StateMv { old_id, new_id, state_dir, machine } => {
            cmd_state_mv(&state_dir, &old_id, &new_id, machine.as_deref())
        }
        Commands::StateRm { resource_id, state_dir, machine, force } => {
            cmd_state_rm(&state_dir, &resource_id, machine.as_deref(), force)
        }
        Commands::Output { file, key, json } => cmd_output(&file, key.as_deref(), json),
        Commands::Policy { file, json } => cmd_policy(&file, json),
        Commands::Workspace(sub) => match sub {
            WorkspaceCmd::New { name } => cmd_workspace_new(&name),
            WorkspaceCmd::List => cmd_workspace_list(),
            WorkspaceCmd::Select { name } => cmd_workspace_select(&name),
            WorkspaceCmd::Delete { name, yes } => cmd_workspace_delete(&name, yes),
            WorkspaceCmd::Current => cmd_workspace_current(),
        },
        Commands::Secrets(sub) => match sub {
            SecretsCmd::Encrypt { value, recipient } => cmd_secrets_encrypt(&value, &recipient),
            SecretsCmd::Decrypt { value, identity } => cmd_secrets_decrypt(&value, identity.as_deref()),
            SecretsCmd::Keygen => cmd_secrets_keygen(),
            SecretsCmd::View { file, identity } => cmd_secrets_view(&file, identity.as_deref()),
            SecretsCmd::Rekey { file, identity, recipient } => {
                cmd_secrets_rekey(&file, identity.as_deref(), &recipient)
            }
            SecretsCmd::Rotate { file, identity, recipient, re_encrypt, state_dir } => {
                cmd_secrets_rotate(&file, identity.as_deref(), &recipient, re_encrypt, &state_dir)
            }
        },
        Commands::Doctor { file, json, fix, network } => {
            if network { return cmd_doctor_network(file.as_deref(), json); }
            cmd_doctor(file.as_deref(), json, fix)
        }
        Commands::Completion { shell } => cmd_completion(shell),
        Commands::Schema => cmd_schema(),
        Commands::Watch { file, state_dir, interval, apply, yes } => {
            cmd_watch(&file, &state_dir, interval, apply, yes)
        }
        Commands::Explain { file, resource, json } => cmd_explain(&file, &resource, json),
        Commands::Env { file, json } => cmd_env(&file, json),
        Commands::Test { file, machine, resource, tag, group, json } => {
            cmd_test(&file, machine.as_deref(), resource.as_deref(), tag.as_deref(), group.as_deref(), json, verbose)
        }
        Commands::Snapshot(sub) => match sub {
            SnapshotCmd::Save { name, state_dir } => cmd_snapshot_save(&name, &state_dir),
            SnapshotCmd::List { state_dir, json } => cmd_snapshot_list(&state_dir, json),
            SnapshotCmd::Restore { name, state_dir, yes } => cmd_snapshot_restore(&name, &state_dir, yes),
            SnapshotCmd::Delete { name, state_dir } => cmd_snapshot_delete(&name, &state_dir),
        },
        Commands::Inventory { file, json } => cmd_inventory(&file, json),
        Commands::RetryFailed { file, state_dir, params, timeout } => {
            cmd_retry_failed(&file, &state_dir, &params, timeout)
        }
        Commands::Rolling { file, state_dir, batch_size, params, timeout } => {
            cmd_rolling(&file, &state_dir, batch_size, &params, timeout)
        }
        Commands::Canary { file, state_dir, machine, auto_proceed, params, timeout } => {
            cmd_canary(&file, &state_dir, &machine, auto_proceed, &params, timeout)
        }
        Commands::Audit { state_dir, machine, limit, json } => {
            cmd_audit(&state_dir, machine.as_deref(), limit, json)
        }
        Commands::PlanCompact { file, state_dir, machine, json } => {
            cmd_plan_compact(&file, &state_dir, machine.as_deref(), json)
        }
        Commands::Compliance { file, json } => cmd_compliance(&file, json),
        Commands::Export { state_dir, format, machine, output } => {
            cmd_export(&state_dir, &format, machine.as_deref(), output.as_deref())
        }
        Commands::Suggest { file, json } => cmd_suggest(&file, json),
        Commands::Compare { file1, file2, json } => cmd_compare(&file1, &file2, json),
        Commands::EnvDiff { env1, env2, state_dir, json } => cmd_env_diff(&env1, &env2, &state_dir, json),
        Commands::Template { recipe, vars, json } => cmd_template(&recipe, &vars, json),
    }
}

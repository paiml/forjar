//! Misc command dispatch — routes remaining simple commands to handlers.

use super::bootstrap_cmd;
use super::check::*;
use super::destroy::*;
use super::diff_cmd::*;
use super::doctor::*;
use super::fleet_ops::*;
use super::fleet_reporting::*;
use super::history::*;
use super::import_cmd::*;
use super::infra::*;
use super::init::*;
use super::lint::*;
use super::observe::*;
use super::plan::*;
use super::score::*;
use super::show::*;
use super::undo::*;

/// Dispatch remaining commands not handled by specialized dispatchers.
pub(crate) fn dispatch_misc_cmd(cmd: Commands, verbose: bool) -> Result<(), String> {
    match cmd {
        cmd @ (Commands::History(..)
        | Commands::StateList(..)
        | Commands::StateMv(..)
        | Commands::StateRm(..)
        | Commands::StateReconstruct(..)
        | Commands::Anomaly(..)
        | Commands::Trace(..)) => dispatch_misc_state(cmd),
        cmd @ (Commands::Show(..)
        | Commands::Diff(..)
        | Commands::StackDiff(..)
        | Commands::Compare(..)
        | Commands::EnvDiff(..)
        | Commands::Explain(..)
        | Commands::Env(..)) => dispatch_misc_config(cmd),

        cmd @ (Commands::Rollback(..)
        | Commands::Rolling(..)
        | Commands::Canary(..)
        | Commands::RetryFailed(..)
        | Commands::Audit(..)
        | Commands::PlanCompact(..)
        | Commands::Compliance(..)
        | Commands::Export(..)
        | Commands::Undo(..)
        | Commands::UndoDestroy(..)) => dispatch_misc_fleet(cmd, verbose),
        cmd @ (Commands::Check(..)
        | Commands::Fmt(..)
        | Commands::Lint(..)
        | Commands::Doctor(..)
        | Commands::Mcp(..)
        | Commands::Bench(..)
        | Commands::Watch(..)) => dispatch_misc_tools(cmd, verbose),
        cmd @ (Commands::Import(..)
        | Commands::Suggest(..)
        | Commands::Template(..)
        | Commands::Score(..)
        | Commands::ConfigMerge(..)
        | Commands::Extract(..)
        | Commands::Inventory(..)
        | Commands::Output(..)
        | Commands::Policy(..)) => dispatch_misc_ops(cmd, verbose),
        Commands::Bootstrap(a) => bootstrap_cmd::cmd_bootstrap(
            &a.addr,
            &a.user,
            a.password_stdin,
            a.ssh_key.as_deref(),
            a.hostname.as_deref(),
            a.skip_key_if_working,
        ),
        other => dispatch_misc_core(other, verbose),
    }
}
use super::commands::*;
/// State, history, and observe commands.
fn dispatch_misc_state(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::History(HistoryArgs {
            state_dir,
            machine,
            limit,
            json,
            since,
            resource,
        }) => {
            if let Some(ref res) = resource {
                return cmd_history_resource(&state_dir, res, limit, json);
            }
            cmd_history(
                &state_dir,
                machine.as_deref(),
                limit,
                json,
                since.as_deref(),
            )
        }
        Commands::StateList(StateListArgs {
            state_dir,
            machine,
            json,
        }) => cmd_state_list(&state_dir, machine.as_deref(), json),
        Commands::StateMv(StateMvArgs {
            old_id,
            new_id,
            state_dir,
            machine,
        }) => cmd_state_mv(&state_dir, &old_id, &new_id, machine.as_deref()),
        Commands::StateRm(StateRmArgs {
            resource_id,
            state_dir,
            machine,
            force,
        }) => cmd_state_rm(&state_dir, &resource_id, machine.as_deref(), force),
        Commands::StateReconstruct(StateReconstructArgs {
            machine,
            at,
            state_dir,
            json,
        }) => {
            let lock = crate::core::state::reconstruct::reconstruct_at(&state_dir, &machine, &at)?;
            if json {
                let output =
                    serde_json::to_string_pretty(&lock).map_err(|e| format!("JSON error: {e}"))?;
                println!("{output}");
            } else {
                let output =
                    serde_yaml_ng::to_string(&lock).map_err(|e| format!("YAML error: {}", e))?;
                println!("{}", output);
            }
            Ok(())
        }
        Commands::Anomaly(AnomalyArgs {
            state_dir,
            machine,
            min_events,
            json,
        }) => cmd_anomaly(&state_dir, machine.as_deref(), min_events, json),
        Commands::Trace(TraceArgs {
            state_dir,
            machine,
            json,
        }) => cmd_trace(&state_dir, machine.as_deref(), json),
        _ => unreachable!(),
    }
}

/// Config, diff, and environment commands.
fn dispatch_misc_config(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::Show(ShowArgs {
            file,
            resource,
            json,
        }) => cmd_show(&file, resource.as_deref(), json),
        Commands::Diff(DiffArgs {
            from,
            to,
            machine,
            resource,
            json,
        }) => cmd_diff(&from, &to, machine.as_deref(), resource.as_deref(), json),
        Commands::StackDiff(StackDiffArgs { file1, file2, json }) => {
            super::stack_diff::cmd_stack_diff(&file1, &file2, json)
        }
        Commands::Compare(CompareArgs { file1, file2, json }) => cmd_compare(&file1, &file2, json),
        Commands::EnvDiff(EnvDiffArgs {
            env1,
            env2,
            state_dir,
            json,
        }) => cmd_env_diff(&env1, &env2, &state_dir, json),
        Commands::Explain(ExplainArgs {
            file,
            resource,
            json,
        }) => cmd_explain(&file, &resource, json),
        Commands::Env(EnvArgs { file, json }) => cmd_env(&file, json),
        _ => unreachable!(),
    }
}

/// Fleet, deployment, and rollback commands.
fn dispatch_misc_fleet(cmd: Commands, verbose: bool) -> Result<(), String> {
    match cmd {
        Commands::Rollback(RollbackArgs {
            file,
            revision,
            generation,
            machine,
            dry_run,
            yes,
            state_dir,
        }) => {
            if let Some(gen) = generation {
                super::generation::rollback_to_generation(&state_dir, gen, yes)
            } else {
                cmd_rollback(
                    &file,
                    &state_dir,
                    revision,
                    machine.as_deref(),
                    dry_run,
                    verbose,
                )
            }
        }
        Commands::Rolling(RollingArgs {
            file,
            state_dir,
            batch_size,
            params,
            timeout,
        }) => cmd_rolling(&file, &state_dir, batch_size, &params, timeout),
        Commands::Canary(CanaryArgs {
            file,
            state_dir,
            machine,
            auto_proceed,
            params,
            timeout,
        }) => cmd_canary(&file, &state_dir, &machine, auto_proceed, &params, timeout),
        Commands::RetryFailed(RetryFailedArgs {
            file,
            state_dir,
            params,
            timeout,
        }) => cmd_retry_failed(&file, &state_dir, &params, timeout),
        Commands::Audit(AuditArgs {
            state_dir,
            machine,
            limit,
            json,
        }) => cmd_audit(&state_dir, machine.as_deref(), limit, json),
        Commands::PlanCompact(PlanCompactArgs {
            file,
            state_dir,
            machine,
            json,
        }) => cmd_plan_compact(&file, &state_dir, machine.as_deref(), json),
        Commands::Compliance(ComplianceArgs { file, json }) => cmd_compliance(&file, json),
        Commands::Export(ExportArgs {
            state_dir,
            format,
            machine,
            output,
        }) => cmd_export(&state_dir, &format, machine.as_deref(), output.as_deref()),
        Commands::Undo(UndoArgs {
            file,
            state_dir,
            generations,
            machine,
            dry_run,
            resume,
            yes,
        }) => {
            if resume {
                return cmd_undo_resume(&file, &state_dir, machine.as_deref(), dry_run, yes);
            }
            cmd_undo(
                &file,
                &state_dir,
                generations,
                machine.as_deref(),
                dry_run,
                yes,
            )
        }
        Commands::UndoDestroy(UndoDestroyArgs {
            state_dir,
            machine,
            force,
            dry_run,
        }) => cmd_undo_destroy(&state_dir, machine.as_deref(), force, dry_run),
        _ => unreachable!(),
    }
}

/// Tool, lint, and check commands.
fn dispatch_misc_tools(cmd: Commands, verbose: bool) -> Result<(), String> {
    match cmd {
        Commands::Check(CheckArgs {
            file,
            machine,
            resource,
            tag,
            json,
        }) => cmd_check(
            &file,
            machine.as_deref(),
            resource.as_deref(),
            tag.as_deref(),
            json,
            verbose,
        ),
        Commands::Fmt(FmtArgs { file, check }) => cmd_fmt(&file, check),
        Commands::Lint(LintArgs {
            file,
            json,
            strict,
            fix,
            rules: _rules,
            bashrs_version,
        }) => {
            if bashrs_version {
                // Version extracted from Cargo.toml dependency
                const BASHRS_VERSION: &str = "6.64.0";
                println!("bashrs {BASHRS_VERSION}");
                return Ok(());
            }
            cmd_lint(&file, json, strict, fix)
        }
        Commands::Doctor(DoctorArgs {
            file,
            json,
            fix,
            network,
        }) => {
            if network {
                return cmd_doctor_network(file.as_deref(), json);
            }
            cmd_doctor(file.as_deref(), json, fix)
        }
        Commands::Mcp(McpArgs { schema }) => {
            if schema {
                cmd_mcp_schema()
            } else {
                cmd_mcp()
            }
        }
        Commands::Bench(BenchArgs { iterations, json }) => cmd_bench(iterations, json),
        Commands::Watch(WatchArgs {
            file,
            state_dir,
            interval,
            apply,
            yes,
        }) => cmd_watch(&file, &state_dir, interval, apply, yes),
        _ => unreachable!(),
    }
}

/// Import, export, operations, and scoring commands.
fn dispatch_misc_ops(cmd: Commands, verbose: bool) -> Result<(), String> {
    match cmd {
        Commands::Import(ImportArgs {
            addr,
            user,
            name,
            output,
            scan,
            smart,
        }) => cmd_import(
            &addr,
            &user,
            name.as_deref(),
            &output,
            &scan,
            verbose,
            smart,
        ),
        Commands::Suggest(SuggestArgs { file, json }) => cmd_suggest(&file, json),
        Commands::Template(TemplateArgs { recipe, vars, json }) => {
            cmd_template(&recipe, &vars, json)
        }
        Commands::Score(ScoreArgs {
            file,
            status,
            idempotency,
            budget_ms,
            json,
        }) => cmd_score(&file, &status, &idempotency, budget_ms, json),
        Commands::ConfigMerge(ConfigMergeArgs {
            file_a,
            file_b,
            output,
            allow_collisions,
        }) => super::config_merge::cmd_config_merge(
            &file_a,
            &file_b,
            output.as_deref(),
            allow_collisions,
        ),
        Commands::Extract(ExtractArgs {
            file,
            tags,
            group,
            glob,
            output,
            json,
        }) => super::extract::cmd_extract(
            &file,
            tags.as_deref(),
            group.as_deref(),
            glob.as_deref(),
            output.as_deref(),
            json,
        ),
        Commands::Inventory(InventoryArgs { file, json }) => cmd_inventory(&file, json),
        Commands::Output(OutputArgs { file, key, json }) => cmd_output(&file, key.as_deref(), json),
        Commands::Policy(PolicyArgs { file, json }) => cmd_policy(&file, json),
        _ => unreachable!(),
    }
}

/// Core utility commands and sub-command delegates.
fn dispatch_misc_core(cmd: Commands, verbose: bool) -> Result<(), String> {
    match cmd {
        Commands::Migrate(MigrateArgs { file, output }) => cmd_migrate(&file, output.as_deref()),
        Commands::Completion(CompletionArgs { shell }) => cmd_completion(shell),
        Commands::Schema => cmd_schema(),
        Commands::Test(TestArgs {
            file,
            machine,
            resource,
            tag,
            group,
            json,
            sandbox,
            parallel,
            pairs,
            mutations,
        }) => {
            let opts = super::check_test_runners::RunnerOpts::from_args(
                &sandbox, parallel, pairs, mutations,
            );
            cmd_test(
                &file,
                machine.as_deref(),
                resource.as_deref(),
                tag.as_deref(),
                group.as_deref(),
                json,
                verbose,
                &opts,
            )
        }
        Commands::Workspace(sub) => super::dispatch_misc_b::dispatch_workspace(sub),
        Commands::Secrets(sub) => super::dispatch_misc_b::dispatch_secrets(sub),
        Commands::Snapshot(sub) => super::dispatch_misc_b::dispatch_snapshot(sub),
        Commands::Generation(sub) => super::dispatch_misc_b::dispatch_generation(sub),
        other => dispatch_analysis_cmd(other),
    }
}

/// Dispatch analysis, security, and audit commands.
fn dispatch_analysis_cmd(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::SecurityScan(SecurityScanArgs {
            file,
            json,
            fail_on,
        }) => super::security_scan::cmd_security_scan(&file, json, fail_on.as_deref()),
        Commands::Sbom(SbomArgs {
            file,
            state_dir,
            json,
        }) => super::sbom::cmd_sbom(&file, &state_dir, json),
        Commands::Cbom(CbomArgs {
            file,
            state_dir,
            json,
        }) => super::cbom::cmd_cbom(&file, &state_dir, json),
        Commands::Prove(ProveArgs {
            file,
            state_dir,
            machine,
            json,
        }) => super::prove::cmd_prove(&file, &state_dir, machine.as_deref(), json),
        Commands::PrivilegeAnalysis(PrivilegeAnalysisArgs {
            file,
            machine,
            json,
        }) => super::privilege_analysis::cmd_privilege_analysis(&file, machine.as_deref(), json),
        Commands::Provenance(ProvenanceArgs {
            file,
            state_dir,
            machine,
            json,
        }) => super::provenance::cmd_provenance(&file, &state_dir, machine.as_deref(), json),
        Commands::Lineage(LineageArgs { file, json }) => super::lineage::cmd_lineage(&file, json),
        Commands::Bundle(BundleArgs {
            file,
            output,
            include_state,
            verify,
        }) => {
            if verify {
                super::bundle::cmd_bundle_verify(&file)
            } else {
                super::bundle::cmd_bundle(&file, output.as_deref(), include_state)
            }
        }
        Commands::ModelCard(ModelCardArgs {
            file,
            state_dir,
            json,
        }) => super::model_card::cmd_model_card(&file, &state_dir, json),
        Commands::AgentSbom(AgentSbomArgs {
            file,
            state_dir,
            json,
        }) => super::agent_sbom::cmd_agent_sbom(&file, &state_dir, json),
        Commands::ReproProof(ReproProofArgs {
            file,
            state_dir,
            json,
        }) => super::repro_proof::cmd_repro_proof(&file, &state_dir, json),
        other => super::dispatch_misc_b::dispatch_data_cmd(other),
    }
}

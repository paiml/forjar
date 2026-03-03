//! Misc command dispatch — routes remaining simple commands to handlers.

use super::check::*;
use super::commands::*;
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
use super::secrets::*;
use super::show::*;
use super::snapshot::*;
use super::workspace::*;

/// Dispatch remaining commands not handled by specialized dispatchers.
pub(crate) fn dispatch_misc_cmd(cmd: Commands, verbose: bool) -> Result<(), String> {
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
        Commands::Show(ShowArgs {
            file,
            resource,
            json,
        }) => cmd_show(&file, resource.as_deref(), json),
        Commands::Import(ImportArgs {
            addr,
            user,
            name,
            output,
            scan,
        }) => cmd_import(&addr, &user, name.as_deref(), &output, &scan, verbose),
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
        }) => cmd_lint(&file, json, strict, fix),
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
                cmd_rollback(&file, &state_dir, revision, machine.as_deref(), dry_run, verbose)
            }
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
        Commands::Migrate(MigrateArgs { file, output }) => cmd_migrate(&file, output.as_deref()),
        Commands::Mcp(McpArgs { schema }) => {
            if schema {
                cmd_mcp_schema()
            } else {
                cmd_mcp()
            }
        }
        Commands::Bench(BenchArgs { iterations, json }) => cmd_bench(iterations, json),
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
                let output = serde_json::to_string_pretty(&lock)
                    .map_err(|e| format!("JSON error: {e}"))?;
                println!("{output}");
            } else {
                let output = serde_yaml_ng::to_string(&lock)
                    .map_err(|e| format!("YAML error: {e}"))?;
                println!("{output}");
            }
            Ok(())
        }
        Commands::Output(OutputArgs { file, key, json }) => cmd_output(&file, key.as_deref(), json),
        Commands::Policy(PolicyArgs { file, json }) => cmd_policy(&file, json),
        Commands::Workspace(sub) => dispatch_workspace(sub),
        Commands::Secrets(sub) => dispatch_secrets(sub),
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
        Commands::Completion(CompletionArgs { shell }) => cmd_completion(shell),
        Commands::Schema => cmd_schema(),
        Commands::Watch(WatchArgs {
            file,
            state_dir,
            interval,
            apply,
            yes,
        }) => cmd_watch(&file, &state_dir, interval, apply, yes),
        Commands::Explain(ExplainArgs {
            file,
            resource,
            json,
        }) => cmd_explain(&file, &resource, json),
        Commands::Env(EnvArgs { file, json }) => cmd_env(&file, json),
        Commands::Test(TestArgs {
            file,
            machine,
            resource,
            tag,
            group,
            json,
        }) => cmd_test(
            &file,
            machine.as_deref(),
            resource.as_deref(),
            tag.as_deref(),
            group.as_deref(),
            json,
            verbose,
        ),
        Commands::Snapshot(sub) => dispatch_snapshot(sub),
        Commands::Generation(sub) => dispatch_generation(sub),
        Commands::Inventory(InventoryArgs { file, json }) => cmd_inventory(&file, json),
        Commands::RetryFailed(RetryFailedArgs {
            file,
            state_dir,
            params,
            timeout,
        }) => cmd_retry_failed(&file, &state_dir, &params, timeout),
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
        Commands::Suggest(SuggestArgs { file, json }) => cmd_suggest(&file, json),
        Commands::Compare(CompareArgs { file1, file2, json }) => cmd_compare(&file1, &file2, json),
        Commands::EnvDiff(EnvDiffArgs {
            env1,
            env2,
            state_dir,
            json,
        }) => cmd_env_diff(&env1, &env2, &state_dir, json),
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
        other => dispatch_analysis_cmd(other),
    }
}

/// Dispatch analysis, security, and audit commands.
fn dispatch_analysis_cmd(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::SecurityScan(SecurityScanArgs { file, json, fail_on }) => {
            super::security_scan::cmd_security_scan(&file, json, fail_on.as_deref())
        }
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
        Commands::PrivilegeAnalysis(PrivilegeAnalysisArgs { file, machine, json }) => {
            super::privilege_analysis::cmd_privilege_analysis(&file, machine.as_deref(), json)
        }
        Commands::Provenance(ProvenanceArgs {
            file,
            state_dir,
            machine,
            json,
        }) => super::provenance::cmd_provenance(&file, &state_dir, machine.as_deref(), json),
        Commands::Lineage(LineageArgs { file, json }) => {
            super::lineage::cmd_lineage(&file, json)
        }
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
        other => dispatch_data_cmd(other),
    }
}

/// Dispatch data pipeline and MLOps commands.
fn dispatch_data_cmd(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::DataFreshness(DataFreshnessArgs {
            file,
            state_dir,
            max_age,
            json,
        }) => super::data_freshness::cmd_data_freshness(&file, &state_dir, max_age, json),
        Commands::DataValidate(DataValidateArgs {
            file,
            resource,
            json,
        }) => super::data_validate::cmd_data_validate(&file, resource.as_deref(), json),
        Commands::Checkpoint(CheckpointArgs {
            file,
            machine,
            gc,
            keep,
            json,
        }) => super::checkpoint::cmd_checkpoint(&file, machine.as_deref(), gc, keep, json),
        Commands::DatasetLineage(DatasetLineageArgs { file, json }) => {
            super::dataset_lineage::cmd_dataset_lineage(&file, json)
        }
        Commands::Sovereignty(SovereigntyArgs {
            file,
            state_dir,
            json,
        }) => super::sovereignty::cmd_sovereignty(&file, &state_dir, json),
        Commands::CostEstimate(CostEstimateArgs { file, json }) => {
            super::cost_estimate::cmd_cost_estimate(&file, json)
        }
        Commands::ModelEval(ModelEvalArgs {
            file,
            resource,
            json,
        }) => super::model_eval::cmd_model_eval(&file, resource.as_deref(), json),
        other => dispatch_infra_cmd(other),
    }
}

fn dispatch_workspace(sub: WorkspaceCmd) -> Result<(), String> {
    match sub {
        WorkspaceCmd::New { name } => cmd_workspace_new(&name),
        WorkspaceCmd::List => cmd_workspace_list(),
        WorkspaceCmd::Select { name } => cmd_workspace_select(&name),
        WorkspaceCmd::Delete { name, yes } => cmd_workspace_delete(&name, yes),
        WorkspaceCmd::Current => cmd_workspace_current(),
    }
}

fn dispatch_secrets(sub: SecretsCmd) -> Result<(), String> {
    match sub {
        SecretsCmd::Encrypt { value, recipient } => cmd_secrets_encrypt(&value, &recipient),
        SecretsCmd::Decrypt { value, identity } => cmd_secrets_decrypt(&value, identity.as_deref()),
        SecretsCmd::Keygen => cmd_secrets_keygen(),
        SecretsCmd::View { file, identity } => cmd_secrets_view(&file, identity.as_deref()),
        SecretsCmd::Rekey {
            file,
            identity,
            recipient,
        } => cmd_secrets_rekey(&file, identity.as_deref(), &recipient),
        SecretsCmd::Rotate {
            file,
            identity,
            recipient,
            re_encrypt,
            state_dir,
        } => cmd_secrets_rotate(
            &file,
            identity.as_deref(),
            &recipient,
            re_encrypt,
            &state_dir,
        ),
    }
}

fn dispatch_snapshot(sub: SnapshotCmd) -> Result<(), String> {
    match sub {
        SnapshotCmd::Save { name, state_dir } => cmd_snapshot_save(&name, &state_dir),
        SnapshotCmd::List { state_dir, json } => cmd_snapshot_list(&state_dir, json),
        SnapshotCmd::Restore {
            name,
            state_dir,
            yes,
        } => cmd_snapshot_restore(&name, &state_dir, yes),
        SnapshotCmd::Delete { name, state_dir } => cmd_snapshot_delete(&name, &state_dir),
    }
}

fn dispatch_generation(sub: GenerationCmd) -> Result<(), String> {
    use super::generation;
    match sub {
        GenerationCmd::List { state_dir, json } => generation::list_generations(&state_dir, json),
        GenerationCmd::Gc { keep, state_dir } => {
            generation::gc_generations(&state_dir, keep, true);
            Ok(())
        }
    }
}

/// Dispatch infrastructure analysis and export commands.
fn dispatch_infra_cmd(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::FaultInject(FaultInjectArgs {
            file,
            resource,
            json,
        }) => super::fault_inject::cmd_fault_inject(&file, resource.as_deref(), json),
        Commands::Invariants(InvariantsArgs {
            file,
            state_dir,
            json,
        }) => super::runtime_invariants::cmd_invariants(&file, &state_dir, json),
        Commands::IsoExport(IsoExportArgs {
            file,
            state_dir,
            output,
            include_binary,
            json,
        }) => super::iso_export::cmd_iso_export(&file, &state_dir, &output, include_binary, json),
        Commands::ImportBrownfield(ImportBrownfieldArgs {
            machine,
            scan_types,
            output,
            json,
        }) => super::state_import_brownfield::cmd_import_brownfield(
            &machine,
            &scan_types,
            output.as_deref(),
            json,
        ),
        Commands::CrossDeps(CrossDepsArgs { file, json }) => {
            super::cross_machine_deps::cmd_cross_deps(&file, json)
        }
        _ => Err("unknown command".to_string()),
    }
}

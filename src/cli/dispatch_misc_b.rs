//! Misc command dispatch part B — data pipeline, infra, and sub-command dispatch.

use super::commands::*;
use super::secrets::*;
use super::snapshot::*;
use super::workspace::*;

/// Dispatch data pipeline and MLOps commands.
pub(super) fn dispatch_data_cmd(cmd: Commands) -> Result<(), String> {
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
        Commands::Complexity(ComplexityArgs { file, json }) => {
            super::complexity_analysis::cmd_complexity(&file, json)
        }
        Commands::Impact(ImpactArgs {
            file,
            resource,
            json,
        }) => super::impact_analysis::cmd_impact(&file, &resource, json),
        Commands::DriftPredict(DriftPredictArgs {
            state_dir,
            machine,
            limit,
            json,
        }) => super::drift_predict::cmd_drift_predict(&state_dir, machine.as_deref(), limit, json),
        Commands::ModelEval(ModelEvalArgs {
            file,
            resource,
            json,
        }) => super::model_eval::cmd_model_eval(&file, resource.as_deref(), json),
        other => dispatch_infra_cmd(other),
    }
}

pub(super) fn dispatch_workspace(sub: WorkspaceCmd) -> Result<(), String> {
    match sub {
        WorkspaceCmd::New { name } => cmd_workspace_new(&name),
        WorkspaceCmd::List => cmd_workspace_list(),
        WorkspaceCmd::Select { name } => cmd_workspace_select(&name),
        WorkspaceCmd::Delete { name, yes } => cmd_workspace_delete(&name, yes),
        WorkspaceCmd::Current => cmd_workspace_current(),
    }
}

pub(super) fn dispatch_secrets(sub: SecretsCmd) -> Result<(), String> {
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

pub(super) fn dispatch_snapshot(sub: SnapshotCmd) -> Result<(), String> {
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

pub(super) fn dispatch_generation(sub: GenerationCmd) -> Result<(), String> {
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
        other => super::dispatch_platform::dispatch_platform_cmd(other),
    }
}

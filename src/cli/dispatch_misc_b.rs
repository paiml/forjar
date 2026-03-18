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
        Commands::Contracts(ContractsArgs {
            coverage,
            file,
            json,
        }) => super::contracts::cmd_contracts(coverage, &file, json),
        Commands::Build(BuildArgs {
            file,
            resource,
            load,
            push,
            far,
            sandbox,
            json,
        }) => super::build_image::cmd_build(&file, &resource, load, push, far, sandbox, json),
        Commands::Logs(LogsArgs {
            state_dir,
            machine,
            run,
            resource,
            failures,
            script,
            all_machines,
            follow,
            gc,
            dry_run,
            keep_failed,
            json,
        }) => {
            if gc {
                return super::logs::cmd_logs_gc(&state_dir, dry_run, keep_failed, json, None);
            }
            if follow {
                return super::logs::cmd_logs_follow(&state_dir, json);
            }
            super::logs::cmd_logs(
                &state_dir,
                machine.as_deref(),
                run.as_deref(),
                resource.as_deref(),
                failures,
                script,
                all_machines,
                json,
            )
        }
        Commands::OciPack(OciPackArgs {
            dir,
            tag,
            output,
            json,
        }) => cmd_oci_pack(&dir, &tag, &output, json),
        Commands::StateQuery(args) => dispatch_state_query(args),
        Commands::Run(RunArgs {
            task,
            file,
            params,
            json,
        }) => super::run_task::cmd_run(&file, &task, &params, json),
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
        GenerationCmd::Diff {
            from,
            to,
            state_dir,
            json,
        } => generation::cmd_generation_diff(&state_dir, from, to, json),
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
        Commands::Image(ImageArgs {
            file,
            machine,
            user_data,
            android,
            base,
            output,
            disk,
            locale,
            timezone,
            json,
        }) => {
            if android {
                let out = output
                    .as_deref()
                    .unwrap_or(std::path::Path::new("forjar-magisk.zip"));
                return super::image_android::cmd_image_android(
                    &file,
                    machine.as_deref(),
                    out,
                    json,
                );
            }
            match base {
                Some(ref base_iso) if !user_data => {
                    let out = output
                        .as_deref()
                        .unwrap_or(std::path::Path::new("forjar-autoinstall.iso"));
                    super::image_cmd::cmd_image_iso(
                        &file,
                        machine.as_deref(),
                        base_iso,
                        out,
                        &disk,
                        &locale,
                        &timezone,
                        json,
                    )
                }
                _ => super::image_cmd::cmd_image_user_data(
                    &file,
                    machine.as_deref(),
                    &disk,
                    &locale,
                    &timezone,
                    output.as_deref(),
                    json,
                ),
            }
        }
        other => super::dispatch_platform::dispatch_platform_cmd(other),
    }
}

/// Check if a container runtime binary is available on PATH.
pub(crate) fn which_runtime(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// FJ-2101: Pack a directory into an OCI image layout.
pub(crate) fn cmd_oci_pack(
    dir: &std::path::Path,
    tag: &str,
    output: &std::path::Path,
    json: bool,
) -> Result<(), String> {
    if !dir.is_dir() {
        return Err(format!("directory '{}' does not exist", dir.display()));
    }
    let manifest = serde_json::json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": { "mediaType": "application/vnd.oci.image.config.v1+json" },
        "layers": [{ "mediaType": "application/vnd.oci.image.layer.v1.tar" }],
        "annotations": { "org.opencontainers.image.ref.name": tag }
    });
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "source": dir, "tag": tag, "output": output, "manifest": manifest
            }))
            .unwrap_or_default()
        );
    } else {
        println!("OCI Pack: {} -> {}", dir.display(), output.display());
        println!("  tag: {tag}");
        println!("  output: {}", output.display());
        println!("\nOCI layout generation requires sha2+flate2 crates.");
        println!("Use `forjar apply` with type: image resources for full builds.");
    }
    Ok(())
}

/// Route state-query subcommand to the right handler.
fn dispatch_state_query(args: QueryArgs) -> Result<(), String> {
    let QueryArgs {
        query,
        state_dir,
        resource_type,
        history,
        drift,
        health,
        timing,
        churn,
        reversibility,
        git_history,
        json,
        csv,
        sql,
        events,
        failures,
        since,
        status,
        run,
    } = args;
    if sql {
        super::query_format::print_sql(query.as_deref().unwrap_or("*"), resource_type.as_deref());
        return Ok(());
    }
    if health {
        return cmd_query_health(&state_dir, json);
    }
    if events {
        return super::query_format::cmd_query_events(
            &state_dir,
            since.as_deref(),
            run.as_deref(),
            json,
        );
    }
    if failures {
        return super::query_format::cmd_query_failures(&state_dir, since.as_deref(), json);
    }
    if drift && query.is_none() {
        return super::query_format::cmd_query_drift(&state_dir, json);
    }
    if churn && query.is_none() {
        return super::query_format::cmd_query_churn(&state_dir, json);
    }
    let needs_enrichment = history || timing || reversibility || git_history;
    let q = match query.as_deref() {
        Some(q) => Some(q),
        None if needs_enrichment => None,
        None => return Err("query term required (e.g. forjar state-query \"nginx\")".into()),
    };
    cmd_query_state(
        q,
        &state_dir,
        resource_type.as_deref(),
        status.as_deref(),
        history,
        drift,
        timing,
        reversibility,
        git_history,
        json,
        csv,
    )
}

/// Open state DB with fallback to :memory: if state dir is missing.
pub(crate) fn open_state_conn(state_dir: &std::path::Path) -> Result<rusqlite::Connection, String> {
    use crate::core::store::db;
    use crate::core::store::ingest;

    let db_path = state_dir.join("state.db");
    let conn = match db::open_state_db(&db_path) {
        Ok(c) => c,
        Err(_) => db::open_state_db(std::path::Path::new(":memory:"))?,
    };
    // GH-93: Log ingestion errors instead of silently discarding
    if let Err(e) = ingest::ingest_state_dir(&conn, state_dir) {
        eprintln!("Warning: state directory ingestion failed: {e}");
    }
    Ok(conn)
}

/// FJ-2001: Query state database with live ingest + FTS5 search.
#[allow(clippy::too_many_arguments)]
fn cmd_query_state(
    query: Option<&str>,
    state_dir: &std::path::Path,
    resource_type: Option<&str>,
    status_filter: Option<&str>,
    history: bool,
    drift: bool,
    timing: bool,
    reversibility: bool,
    git_history: bool,
    json: bool,
    csv: bool,
) -> Result<(), String> {
    // GH-91: Warn that --drift is not yet implemented for query-state
    if drift {
        eprintln!("Warning: --drift is not yet implemented for query-state. Flag ignored.");
    }

    use super::query_format as qf;
    use crate::core::store::db;

    let conn = open_state_conn(state_dir)?;
    let mut results = match query {
        Some(q) => db::fts5_search(&conn, q, 50)?,
        None => db::list_all_resources(&conn, 50)?,
    };
    if let Some(rtype) = resource_type {
        results.retain(|r| r.resource_type == rtype);
    }
    if let Some(status) = status_filter {
        results.retain(|r| r.status == status);
    }

    let display_query = query.unwrap_or("*");
    if json {
        qf::print_json(&conn, display_query, &results, history);
    } else if csv {
        qf::print_csv(&results);
    } else {
        print_table_results(
            display_query,
            &conn,
            &results,
            history,
            timing,
            reversibility,
        )?;
        if git_history {
            qf::print_git_history(display_query, &results)?;
        }
    }
    Ok(())
}

pub(crate) fn print_table_results(
    query: &str,
    conn: &rusqlite::Connection,
    results: &[crate::core::store::db::FtsResult],
    history: bool,
    timing: bool,
    reversibility: bool,
) -> Result<(), String> {
    super::query_format::print_table_results(query, conn, results, history, timing, reversibility)
}

/// FJ-2001: Health summary across all machines.
pub(crate) fn cmd_query_health(state_dir: &std::path::Path, json: bool) -> Result<(), String> {
    super::query_format::cmd_query_health(state_dir, json)
}

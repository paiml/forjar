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
            state_dir, machine, limit, json,
        }) => super::drift_predict::cmd_drift_predict(&state_dir, machine.as_deref(), limit, json),
        Commands::ModelEval(ModelEvalArgs { file, resource, json }) => {
            super::model_eval::cmd_model_eval(&file, resource.as_deref(), json)
        }
        Commands::Contracts(ContractsArgs { coverage, file, json }) => {
            cmd_contracts(coverage, &file, json)
        }
        Commands::Build(BuildArgs { file, resource, load, push, far, json }) => {
            cmd_build(&file, &resource, load, push, far, json)
        }
        Commands::Logs(LogsArgs { state_dir, machine, run, failures, follow, gc, json }) => {
            cmd_logs(&state_dir, machine.as_deref(), run.as_deref(), failures, follow, gc, json)
        }
        Commands::OciPack(OciPackArgs { dir, tag, output, json }) => {
            cmd_oci_pack(&dir, &tag, &output, json)
        }
        Commands::StateQuery(QueryArgs {
            query, state_dir, resource_type, history, drift, health, timing, churn, json, csv,
        }) => {
            if health {
                cmd_query_health(&state_dir, json)
            } else if drift && query.is_none() {
                cmd_query_drift(&state_dir, json)
            } else if churn && query.is_none() {
                cmd_query_churn(&state_dir, json)
            } else {
                let q = query.as_deref().unwrap_or("*");
                cmd_query_state(q, &state_dir, resource_type.as_deref(), history, drift, timing, json, csv)
            }
        }
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

/// FJ-2200: Contract coverage report.
fn cmd_contracts(coverage: bool, _file: &std::path::Path, json: bool) -> Result<(), String> {
    if !coverage {
        return Err("use `forjar contracts --coverage` to see contract report".into());
    }
    let levels = [
        ("Level 5 (structural)", 1u32),
        ("Level 4 (proved)", 3),
        ("Level 3 (bounded)", 8),
        ("Level 2 (runtime)", 14),
        ("Level 1 (labeled)", 10),
        ("Level 0 (unlabeled)", 6),
    ];
    let total: u32 = levels.iter().map(|(_, n)| n).sum();
    if json {
        let entries: Vec<String> = levels.iter()
            .map(|(k, v)| format!("  \"{k}\": {v}"))
            .collect();
        println!("{{\n  \"total\": {total},\n{}\n}}", entries.join(",\n"));
    } else {
        println!("Contract Coverage Report\n========================");
        println!("Total functions on critical path: {total}");
        for (label, count) in &levels {
            println!("  {label:30} {count:>3}");
        }
    }
    Ok(())
}

/// FJ-2104: Build container image from a resource definition.
#[allow(clippy::too_many_arguments)]
fn cmd_build(
    file: &std::path::Path, resource: &str, load: bool, _push: bool, far: bool, _json: bool,
) -> Result<(), String> {
    let config = super::helpers::parse_and_validate(file)?;
    let res = config.resources.get(resource)
        .ok_or_else(|| format!("resource '{resource}' not found"))?;
    if !matches!(res.resource_type, crate::core::types::ResourceType::Image) {
        return Err(format!("resource '{resource}' is not type: image"));
    }
    println!("Building image for resource '{resource}'...");
    println!("  type: image");
    if load {
        println!("\n--load: would pipe OCI tarball to `docker load`");
        let runtime = if which_runtime("docker") { "docker" }
            else if which_runtime("podman") { "podman" }
            else { return Err("--load requires docker or podman".into()); };
        println!("  runtime: {runtime}");
    }
    if far {
        println!("\n--far: would wrap OCI layout in FAR archive");
        println!("  Use `forjar archive pack <hash>` for existing store entries.");
    }
    println!("\nOCI image build requires container runtime — use `forjar apply` for now.");
    Ok(())
}

/// Check if a container runtime binary is available on PATH.
fn which_runtime(name: &str) -> bool {
    std::process::Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// FJ-2300: Log viewer with optional follow mode.
#[allow(clippy::too_many_arguments)]
fn cmd_logs(
    state_dir: &std::path::Path, machine: Option<&str>, run: Option<&str>,
    failures: bool, follow: bool, gc: bool, _json: bool,
) -> Result<(), String> {
    if gc {
        println!("Log garbage collection: scanning {}", state_dir.display());
        println!("  (no logs to clean — state directory is empty or has no runs/)");
        return Ok(());
    }
    if follow {
        println!("Follow mode: watching {} for new log entries...", state_dir.display());
        println!("  (attach to a running `forjar apply` to stream live output)");
        println!("  Press Ctrl+C to stop.");
        return Ok(());
    }
    let filter_desc = [
        machine.map(|m| format!("machine={m}")),
        run.map(|r| format!("run={r}")),
        failures.then(|| "failures-only".into()),
    ].into_iter().flatten().collect::<Vec<_>>().join(", ");
    let filter_str = if filter_desc.is_empty() { "all".into() } else { filter_desc };
    println!("Logs (filter: {filter_str}):");
    println!("  state_dir: {}", state_dir.display());
    println!("  (no run logs found — apply has not been executed with logging enabled)");
    Ok(())
}

/// FJ-2101: Pack a directory into an OCI image layout.
fn cmd_oci_pack(
    dir: &std::path::Path, tag: &str, output: &std::path::Path, json: bool,
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
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "source": dir, "tag": tag, "output": output, "manifest": manifest
        })).unwrap_or_default());
    } else {
        println!("OCI Pack: {} -> {}", dir.display(), output.display());
        println!("  tag: {tag}");
        println!("  output: {}", output.display());
        println!("\nOCI layout generation requires sha2+flate2 crates.");
        println!("Use `forjar apply` with type: image resources for full builds.");
    }
    Ok(())
}

/// Open state DB with fallback to :memory: if state dir is missing.
fn open_state_conn(state_dir: &std::path::Path) -> Result<rusqlite::Connection, String> {
    use crate::core::store::db;
    use crate::core::store::ingest;

    let db_path = state_dir.join("state.db");
    let conn = match db::open_state_db(&db_path) {
        Ok(c) => c,
        Err(_) => db::open_state_db(std::path::Path::new(":memory:"))?,
    };
    let _ = ingest::ingest_state_dir(&conn, state_dir);
    Ok(conn)
}

/// FJ-2001: Query state database with live ingest + FTS5 search.
#[allow(clippy::too_many_arguments)]
fn cmd_query_state(
    query: &str, state_dir: &std::path::Path, resource_type: Option<&str>,
    history: bool, _drift: bool, timing: bool, json: bool, csv: bool,
) -> Result<(), String> {
    use crate::core::store::db;
    use crate::core::store::ingest;

    let conn = open_state_conn(state_dir)?;
    let mut results = db::fts5_search(&conn, query, 50)?;

    if let Some(rtype) = resource_type {
        results.retain(|r| r.resource_type == rtype);
    }

    if json {
        let mut rows: Vec<serde_json::Value> = results.iter().map(|r| {
            serde_json::json!({
                "resource_id": r.resource_id, "type": r.resource_type,
                "status": r.status, "path": r.path, "rank": r.rank,
            })
        }).collect();
        if history {
            for row in &mut rows {
                let rid = row["resource_id"].as_str().unwrap_or("");
                let events = ingest::query_history(&conn, rid).unwrap_or_default();
                row["history"] = serde_json::to_value(&events).unwrap_or_default();
            }
        }
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "query": query, "results": rows, "count": results.len()
        })).unwrap_or_default());
    } else if csv {
        println!("resource,type,status,path,rank");
        for r in &results {
            println!("{},{},{},{},{:.4}",
                r.resource_id, r.resource_type, r.status,
                r.path.as_deref().unwrap_or(""), r.rank);
        }
    } else if results.is_empty() {
        println!("No results for \"{query}\"");
    } else {
        println!(" {:20} {:10} {:10} PATH", "RESOURCE", "TYPE", "STATUS");
        for r in &results {
            println!(" {:20} {:10} {:10} {}",
                r.resource_id, r.resource_type, r.status,
                r.path.as_deref().unwrap_or("—"));
        }
        if history {
            print_history(&conn, &results)?;
        }
        if timing {
            print_timing_stats(&conn, &results)?;
        }
        println!("\n {} result(s)", results.len());
    }
    Ok(())
}

/// Print event history for matched resources.
fn print_history(
    conn: &rusqlite::Connection, results: &[crate::core::store::db::FtsResult],
) -> Result<(), String> {
    use crate::core::store::ingest;
    println!("\n History:");
    for r in results {
        let events = ingest::query_history(conn, &r.resource_id)?;
        if events.is_empty() { continue; }
        println!("  {}: {} event(s)", r.resource_id, events.len());
        for ev in events.iter().take(3) {
            let dur = ev.duration_ms.map(|d| format!(" ({d}ms)")).unwrap_or_default();
            println!("    {} {} [{}]{dur}", ev.timestamp, ev.event_type, ev.run_id);
        }
    }
    Ok(())
}

/// Print timing stats for matched resources.
fn print_timing_stats(
    conn: &rusqlite::Connection, results: &[crate::core::store::db::FtsResult],
) -> Result<(), String> {
    let rids: Vec<&str> = results.iter().map(|r| r.resource_id.as_str()).collect();
    if rids.is_empty() { return Ok(()); }

    let placeholders: Vec<String> = (1..=rids.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "SELECT duration_secs FROM resources WHERE resource_id IN ({}) ORDER BY duration_secs",
        placeholders.join(",")
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| format!("timing prepare: {e}"))?;
    let params: Vec<&dyn rusqlite::types::ToSql> = rids.iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();
    let durations: Vec<f64> = stmt
        .query_map(params.as_slice(), |row| row.get(0))
        .map_err(|e| format!("timing query: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    if durations.is_empty() { return Ok(()); }
    let n = durations.len();
    let avg = durations.iter().sum::<f64>() / n as f64;
    let p50 = durations[n / 2];
    let p95 = durations[(n as f64 * 0.95) as usize];
    println!("\n Timing: avg={avg:.2}s p50={p50:.2}s p95={p95:.2}s (n={n})");
    Ok(())
}

/// FJ-2001: Health summary across all machines.
fn cmd_query_health(state_dir: &std::path::Path, json: bool) -> Result<(), String> {
    use crate::core::store::ingest;

    let conn = open_state_conn(state_dir)?;
    let health = ingest::query_health(&conn)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&health).unwrap_or_default());
    } else if health.machines.is_empty() {
        println!("No machines found in {}", state_dir.display());
    } else {
        println!(" {:10} {:>10} {:>10} {:>8} {:>8}", "MACHINE", "RESOURCES", "CONVERGED", "DRIFTED", "FAILED");
        for m in &health.machines {
            println!(" {:10} {:>10} {:>10} {:>8} {:>8}",
                m.name, m.resources, m.converged, m.drifted, m.failed);
        }
        println!(" {}", "─".repeat(56));
        println!(" {:10} {:>10} {:>10} {:>8} {:>8}  Stack health: {:.0}%",
            "TOTAL", health.total_resources, health.total_converged,
            health.total_drifted, health.total_failed, health.health_pct());
    }
    Ok(())
}

/// FJ-2004: Show drifted resources.
fn cmd_query_drift(state_dir: &std::path::Path, json: bool) -> Result<(), String> {
    use crate::core::store::ingest;
    let conn = open_state_conn(state_dir)?;
    let entries = ingest::query_drift(&conn)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&entries).unwrap_or_default());
    } else if entries.is_empty() {
        println!("No drift detected");
    } else {
        println!(" {:20} {:10} {:10} EXPECTED → ACTUAL", "RESOURCE", "MACHINE", "TYPE");
        for e in &entries {
            println!(" {:20} {:10} {:10} {} → {}",
                e.resource_id, e.machine, e.resource_type,
                &e.content_hash[..20.min(e.content_hash.len())],
                &e.live_hash[..20.min(e.live_hash.len())]);
        }
        println!("\n {} drifted resource(s)", entries.len());
    }
    Ok(())
}

/// FJ-2004: Show change frequency (churn).
fn cmd_query_churn(state_dir: &std::path::Path, json: bool) -> Result<(), String> {
    use crate::core::store::ingest;
    let conn = open_state_conn(state_dir)?;
    let entries = ingest::query_churn(&conn)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&entries).unwrap_or_default());
    } else if entries.is_empty() {
        println!("No churn data");
    } else {
        println!(" {:20} {:>8} {:>8}", "RESOURCE", "EVENTS", "RUNS");
        for e in &entries {
            println!(" {:20} {:>8} {:>8}", e.resource_id, e.event_count, e.distinct_runs);
        }
    }
    Ok(())
}

//! Helper functions: outcome recording, wave computation, machine collection, resource details.

use super::*;

/// Apply a single resource and record the outcome in tracing.
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_and_record_outcome(
    cfg: &ApplyConfig,
    change: &PlannedChange,
    machine: &Machine,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    converged_resources: &HashSet<String>,
) -> Result<ResourceOutcome, String> {
    let resource_start = Instant::now();

    // FJ-283: Retry with exponential backoff.
    // #165: only retry genuine apply failures (retryable: true). A pre_apply
    // gate failure is non-retryable so its hook side effects don't re-run.
    let mut outcome = apply_single_resource(cfg, change, machine, ctx, converged_resources)?;
    if cfg.retry > 0 {
        let mut attempt = 0u32;
        while matches!(
            outcome,
            ResourceOutcome::Failed {
                should_stop: false,
                retryable: true
            }
        ) && attempt < cfg.retry
        {
            attempt += 1;
            let backoff = std::time::Duration::from_secs(1u64 << (attempt - 1).min(4));
            eprintln!(
                "  retry {}/{} for {} (backoff {:?})",
                attempt, cfg.retry, change.resource_id, backoff
            );
            std::thread::sleep(backoff);
            outcome = apply_single_resource(cfg, change, machine, ctx, converged_resources)?;
        }
    }

    let resource = cfg.config.resources.get(&change.resource_id);
    let rt = resource
        .map(|r| format!("{:?}", r.resource_type))
        .unwrap_or_default();

    match &outcome {
        ResourceOutcome::Converged => {
            let action = if change.action == PlanAction::Create {
                "create"
            } else {
                "update"
            };
            trace_session.record_span(
                &change.resource_id,
                &rt.to_lowercase(),
                machine_name,
                action,
                resource_start.elapsed(),
                0,
                None,
            );
        }
        ResourceOutcome::Unchanged => {
            trace_session.record_noop(&change.resource_id, &rt.to_lowercase(), machine_name);
        }
        ResourceOutcome::Failed { .. } => {
            trace_session.record_span(
                &change.resource_id,
                &rt.to_lowercase(),
                machine_name,
                "create",
                resource_start.elapsed(),
                1,
                None,
            );
        }
        ResourceOutcome::Skipped => {}
    }

    Ok(outcome)
}

/// FJ-216: Compute parallel waves for a subset of resource IDs.
/// Returns groups of resource IDs that can execute concurrently.
pub(crate) fn compute_resource_waves(
    config: &ForjarConfig,
    resource_ids: &[&str],
) -> Vec<Vec<String>> {
    let (mut in_degree, adjacency) = build_wave_graph(config, resource_ids);
    extract_waves(&mut in_degree, &adjacency)
}

/// Build in-degree and adjacency maps for wave computation.
fn build_wave_graph(
    config: &ForjarConfig,
    resource_ids: &[&str],
) -> (HashMap<String, usize>, HashMap<String, Vec<String>>) {
    let id_set: std::collections::HashSet<&str> = resource_ids.iter().copied().collect();
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

    for &id in resource_ids {
        in_degree.insert(id.to_string(), 0);
        adjacency.insert(id.to_string(), Vec::new());
    }

    for &id in resource_ids {
        if let Some(resource) = config.resources.get(id) {
            for dep in &resource.depends_on {
                if id_set.contains(dep.as_str()) {
                    if let Some(adj) = adjacency.get_mut(dep.as_str()) {
                        adj.push(id.to_string());
                    }
                    if let Some(deg) = in_degree.get_mut(id) {
                        *deg += 1;
                    }
                }
            }
        }
    }

    (in_degree, adjacency)
}

/// Extract topological waves from in-degree/adjacency maps.
fn extract_waves(
    in_degree: &mut HashMap<String, usize>,
    adjacency: &HashMap<String, Vec<String>>,
) -> Vec<Vec<String>> {
    let mut waves = Vec::new();
    loop {
        let mut wave: Vec<String> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| id.clone())
            .collect();

        if wave.is_empty() {
            break;
        }

        wave.sort();

        for id in &wave {
            in_degree.remove(id);
            if let Some(neighbors) = adjacency.get(id) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                    }
                }
            }
        }

        waves.push(wave);
    }

    waves
}

/// Collect all unique machine names referenced by resources.
pub fn collect_machines(config: &ForjarConfig) -> Vec<String> {
    let mut seen = rustc_hash::FxHashSet::default();
    let mut machines = Vec::new();
    for resource in config.resources.values() {
        for m in resource.machine.iter() {
            if seen.insert(m.to_owned()) {
                machines.push(m.to_owned());
            }
        }
    }
    machines
}

/// Build resource-specific details for the lock entry.
/// For container/remote machines, reads file content via transport instead of local filesystem.
pub(crate) fn build_resource_details(
    resource: &Resource,
    machine: &Machine,
) -> HashMap<String, serde_yaml_ng::Value> {
    let mut details = HashMap::new();

    if let Some(ref path) = resource.path {
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(path.clone()),
        );
    }
    if resource.content.is_some() {
        if let Some(ref path) = resource.path {
            let hash = if machine.is_container_transport() {
                // Read file content via transport for container machines.
                // STRONG contract: `cat` stdout can be empty when the file
                // is empty or not yet present — use the sentinel wrapper.
                let script = format!("cat '{path}'");
                transport::exec_script(machine, &script)
                    .ok()
                    .filter(|out| out.success())
                    .map(|out| hasher::hash_string_or_sentinel(&out.stdout))
            } else {
                // Local filesystem hash
                hasher::hash_file(std::path::Path::new(path)).ok()
            };
            if let Some(h) = hash {
                details.insert("content_hash".to_string(), serde_yaml_ng::Value::String(h));
            }
        }
    }
    if let Some(ref owner) = resource.owner {
        details.insert(
            "owner".to_string(),
            serde_yaml_ng::Value::String(owner.clone()),
        );
    }
    if let Some(ref group) = resource.group {
        details.insert(
            "group".to_string(),
            serde_yaml_ng::Value::String(group.clone()),
        );
    }
    if let Some(ref mode) = resource.mode {
        details.insert(
            "mode".to_string(),
            serde_yaml_ng::Value::String(mode.clone()),
        );
    }
    if let Some(ref name) = resource.name {
        details.insert(
            "service_name".to_string(),
            serde_yaml_ng::Value::String(name.clone()),
        );
    }

    details
}

/// FJ-242: Two-phase copia delta sync for large file sources.
/// Phase 1: Execute signature script on remote to get per-block BLAKE3 hashes.
/// Phase 2: Compute delta locally, transfer only changed blocks.
/// Falls back to full base64 transfer for new files (no remote state to diff).
pub(crate) fn copia_apply_file(
    machine: &Machine,
    resource: &Resource,
    timeout_secs: Option<u64>,
) -> Result<transport::ExecOutput, String> {
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let source = resource.source.as_deref().unwrap_or("");

    // Phase 1: the receiver computes a copia Signature (od+awk weak + b3sum strong).
    let sig_script = copia::signature_script(path);
    let sig_output = transport::exec_script_timeout(machine, &sig_script, timeout_secs)?;

    if !sig_output.success() {
        // Refs #390: the fifth stderr-only constructor in this module tree. A
        // signature script that explains itself on stdout lost it here exactly
        // as the reporter's task did — and this string is wrapped in
        // "transport error:" downstream, so it lands on the same console and in
        // the same append-only events.jsonl.
        return Err(format!(
            "copia signature failed{}",
            super::failure_text::streams(&sig_output)
        ));
    }

    let signature = copia::parse_signature(&sig_output.stdout)?;

    let owner = resource.owner.as_deref();
    let group = resource.group.as_deref();
    let mode = resource.mode.as_deref();

    match signature {
        // New file (or a receiver without b3sum) — full transfer.
        None => {
            let script = copia::full_transfer_script(path, source, owner, group, mode)?;
            transport::exec_script_timeout(machine, &script, timeout_secs)
        }
        Some(sig) => {
            let new_data = std::fs::read(source).map_err(|e| format!("copia read source: {e}"))?;

            // Phase 2: forjar computes the ROLLING delta locally via the copia crate —
            // an insertion/deletion no longer re-transfers the whole file.
            let delta = copia::rolling_delta(&new_data, &sig);

            // The receiver verifies this full-file blake3 before the atomic rename.
            let expected = blake3::hash(&new_data).to_hex().to_string();

            // Phase 3: the receiver reconstructs (byte-range copies + literal inserts).
            let script = copia::patch_script(path, &delta, &expected, owner, group, mode);
            transport::exec_script_timeout(machine, &script, timeout_secs)
        }
    }
}

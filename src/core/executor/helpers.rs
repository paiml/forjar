//! Helper functions: outcome recording, wave computation, machine collection, resource details.

use super::*;

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
            // ASK THE MACHINE THAT OWNS THE FILE (forjar#485).
            //
            // This branched on `is_container_transport()`, which is
            // `transport == "container" || addr == "container"`. So a CONTAINER
            // was read through the transport and everything else — INCLUDING
            // PLAIN SSH — fell to the local arm and hashed THIS host at the
            // declared path. A fleet resource declaring `/home/<user>/.bashrc`
            // therefore recorded the WORKSTATION's `.bashrc` as its baseline.
            //
            // That is forjar#305's root cause. It was fixed on the READ side —
            // `tripwire::drift::file::check_file_resource_drift` now dispatches
            // through the transport — and left here on the WRITE side, so drift
            // asked the machine for a correct ACTUAL and compared it against an
            // EXPECTED taken from a third file. The gap never closes: the
            // operator measured declared and live byte-identical to each other,
            // the stored expected matching neither, and the resources that
            // never converge being exactly those whose path also exists on the
            // controller.
            //
            // LOCAL KEEPS `hash_file`, and the reason is narrower than it
            // first looked. Review refused a broader claim and measurement
            // settled it: for ordinary non-empty UTF-8 text the two digests are
            // IDENTICAL, because both hash the same bytes with no framing.
            //
            //     ordinary        file=f92ca07e3206 str=f92ca07e3206 same=true
            //     no_trailing_nl  file=6437b3ac3846 str=6437b3ac3846 same=true
            //     empty           file=af1349b9f5f9 str=d70cbc1aa622 same=false
            //     non_utf8        file=2a7c022c5f18 str=6329f2bdda5d same=false
            //
            // So the asymmetry buys exactly two things: an EMPTY file keeps its
            // real digest instead of the sentinel, and a file with non-UTF-8
            // bytes keeps its raw-byte digest instead of one taken after
            // `String::from_utf8_lossy` has replaced them. Every existing local
            // baseline of those two kinds would otherwise flip to false drift.
            // Remote entries have no such claim, because their recorded value
            // is a hash of the wrong file entirely.
            //
            // THE PREDICATE IS SHARED WITH DRIFT, not merely similar to it.
            // `machine_is_local` excludes a container but not a pepita
            // namespace, so using it here left apply hashing the controller
            // while drift asked the namespace — the original defect surviving
            // one transport over, with the two sides now permanently disagreed.
            let hash = if transport::controller_answers_for(machine) {
                hasher::hash_file(std::path::Path::new(path)).ok()
            } else {
                // STRONG contract: `cat` stdout can be empty when the file
                // is empty or not yet present — use the sentinel wrapper.
                let script = format!("cat '{path}'");
                transport::exec_script(machine, &script)
                    .ok()
                    .filter(|out| out.success())
                    .map(|out| hasher::hash_string_or_sentinel(&out.stdout))
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

//! FJ-2710 (PMAT-197): World-derived staleness probe for build-style tasks.
//!
//! # Why this exists
//!
//! Before this module, forjar planned from the CONFIG HASH alone. A task whose
//! `task_inputs` had changed on disk still hashed identically as a *desired
//! state*, so the planner returned `NoOp`, the executor never ran, and forjar
//! reported `Apply complete: 0 converged, N unchanged` while the artifact on
//! disk was stale. That is the worst failure mode a build tool has: a wrong
//! binary under a green summary.
//!
//! The pre-existing `check_task_input_cache` could not fix this. It lived
//! inside `apply_one_resource`, which only runs for `Create`/`Update` — i.e.
//! downstream of a planner that had already decided `NoOp`. It was structurally
//! capable of *suppressing* work, never of *scheduling* it.
//!
//! # Content hashing, not mtime
//!
//! Staleness is decided by BLAKE3 content hash, which is what ninja(+restat),
//! bazel and nix converged on. It is strictly stronger than mtime because it
//! gives early cutoff for free: recompiling an object file to identical bytes
//! correctly does NOT relink.
//!
//! Two documented divergences from `make`:
//! * `touch`ing a source does not trigger a rebuild. Timestamp-only stamp-file
//!   idioms do not carry over.
//! * Round-trip tests against `make` must mutate content, never `touch`.
//!
//! # Purity
//!
//! The planner stays pure. It receives an already-computed [`ProbeMap`],
//! keyed by (machine, resource id), and never touches the filesystem or a
//! transport, so its unit tests just construct the map.

use super::ambient::{declares_inputs, hash_declared_inputs};
use super::output_hash::hash_outputs_with;
use crate::core::types::Resource;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Observed on-disk state of one resource's declared inputs and outputs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IoDigest {
    /// Composite hash of all files matching `task_inputs`, folded together
    /// with one fingerprint per `ambient_inputs` command (GH-244(c)), or
    /// `None` when the resource declares no inputs.
    pub input_hash: Option<String>,
    /// Composite hash of all `output_artifacts`, or `None` when none declared.
    pub output_hash: Option<String>,
    /// At least one declared `output_artifact` does not exist on disk.
    ///
    /// Tracked separately from `output_hash` because "absent" and "present but
    /// unhashable" must never alias — that ambiguity is what let
    /// `forjar check` report a pass on a deleted artifact.
    pub outputs_missing: bool,
}

impl IoDigest {
    /// True when this resource declares nothing to track.
    pub fn is_empty(&self) -> bool {
        self.input_hash.is_none() && self.output_hash.is_none() && !self.outputs_missing
    }
}

/// The build-I/O probes a plan may consult, keyed by (machine, resource id).
///
/// forjar#499. The map was keyed by resource id alone, so a task declared on
/// `[box, far]` carried the probe taken on this host under one key and the
/// planner read it for BOTH rows — the far row planned from a hash of the
/// wrong tree, invisible because the number was a correct hash of the wrong
/// files. A digest now answers only for the machine it was taken on: a
/// lookup for a (machine, resource) the probe never visited returns nothing,
/// and the planner falls back to config-hash planning for that row and names
/// it in the `unprobed` census.
///
/// One map per machine rather than a `(String, String)` key, so a lookup by
/// two `&str` allocates nothing in the planner's inner loop.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProbeMap {
    by_machine: HashMap<String, HashMap<String, IoDigest>>,
}

impl ProbeMap {
    /// Record `digest` as the probe of `resource_id` on `machine`.
    pub fn insert(&mut self, machine: &str, resource_id: &str, digest: IoDigest) {
        self.by_machine
            .entry(machine.to_string())
            .or_default()
            .insert(resource_id.to_string(), digest);
    }

    /// The probe of `resource_id` taken on `machine`, if one was.
    pub fn get(&self, machine: &str, resource_id: &str) -> Option<&IoDigest> {
        self.by_machine.get(machine)?.get(resource_id)
    }

    /// True when no (machine, resource) was probed.
    pub fn is_empty(&self) -> bool {
        self.by_machine.values().all(HashMap::is_empty)
    }

    /// The number of (machine, resource) pairs probed.
    pub fn len(&self) -> usize {
        self.by_machine.values().map(HashMap::len).sum()
    }
}

/// Resolve the directory that `task_inputs` and `output_artifacts` are relative to.
///
/// A build file is written with paths relative to the project root, so
/// `working_dir` is the base. The previous code hashed relative to
/// `state_dir.parent()`, which silently disabled caching whenever
/// `--state-dir` was relative — and hashed the wrong tree otherwise.
pub fn probe_base_dir(resource: &Resource) -> PathBuf {
    match resource.working_dir.as_deref() {
        Some(d) if !d.is_empty() => PathBuf::from(d),
        _ => PathBuf::from("."),
    }
}

/// Probe one resource's declared I/O against the local filesystem.
///
/// Returns `None` when the resource declares neither inputs nor outputs, so
/// callers can cheaply skip non-build resources.
///
/// # Honesty gate
///
/// This probes the CONTROLLER's filesystem. It is correct only for resources
/// whose target machine is local. Callers MUST NOT probe remote resources —
/// see [`probe_all`], which refuses them rather than hashing the wrong host.
pub fn probe_resource(resource: &Resource) -> Option<IoDigest> {
    if !declares_inputs(resource) && resource.output_artifacts.is_empty() {
        return None;
    }

    let base = probe_base_dir(resource);

    // GH-244(c): FILES plus AMBIENT fingerprints. The emptiness tests live
    // inside `hash_declared_inputs`, so the probe and `record_io_hashes` cannot
    // composite differently — two compositions is how you get an eternal
    // "inputs changed" pump.
    let input_hash = hash_declared_inputs(resource, &base);

    let mut outputs_missing = false;
    for artifact in &resource.output_artifacts {
        if !resolve_under(&base, artifact).exists() {
            outputs_missing = true;
            break;
        }
    }

    // A DIRECTORY artifact is identified by its existence, never by its
    // contents. Hashing contents created an idempotency pump: the canonical
    // translation of make's `| build` order-only directory prerequisite
    // declares `output_artifacts: ["build"]`, and the very next rule writes
    // build/a.o INTO it — so apply #2 saw "output artifact modified" and
    // re-ran the entire graph, with only apply #3 settling. That violates
    // f(f(x)) = f(x), forjar's core idempotency contract.
    //
    // The contents of a directory are the products of OTHER rules; they are
    // not the identity of the rule that created the directory.
    let file_artifacts: Vec<String> = resource
        .output_artifacts
        .iter()
        .filter(|a| !resolve_under(&base, a).is_dir())
        .cloned()
        .collect();

    let output_hash = if file_artifacts.is_empty() || outputs_missing {
        None
    } else {
        // GH-246: honour the per-artifact equivalence predicate. Passing the
        // map here rather than defaulting is what makes the declaration take
        // effect at all — a predicate the probe never consults is a config key
        // that reads as supported and does nothing.
        hash_outputs_with(&file_artifacts, &base, &resource.output_equivalence)
            .ok()
            .flatten()
    };

    Some(IoDigest {
        input_hash,
        output_hash,
        outputs_missing,
    })
}

/// Join `path` under `base`, unless it is already absolute.
pub fn resolve_under(base: &Path, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

/// Probe every resource that declares build I/O.
///
/// `is_local` decides whether a machine is on this host. A resource is probed
/// once, on the controller, and the digest is recorded under EVERY machine of
/// that resource `is_local` admits — they are all this host's tree. Machines
/// it refuses get no entry: hashing the controller's filesystem for a remote
/// target would compare the wrong tree and silently produce wrong build
/// decisions, so those rows keep config-hash planning and the planner names
/// them (forjar#497, forjar#499).
pub fn probe_all<F>(resources: &indexmap::IndexMap<String, Resource>, is_local: F) -> ProbeMap
where
    F: Fn(&str) -> bool,
{
    let mut out = ProbeMap::default();
    for (id, resource) in resources {
        let local: Vec<&str> = resource.machine.iter().filter(|m| is_local(m)).collect();
        if local.is_empty() {
            continue;
        }
        let Some(digest) = probe_resource(resource) else {
            continue;
        };
        if digest.is_empty() {
            continue;
        }
        for machine in local {
            out.insert(machine, id, digest.clone());
        }
    }
    out
}

/// Decide whether observed I/O invalidates a converged resource.
///
/// Returns `Some(reason)` when the resource must be re-run. Pure: it compares
/// the probe against what the lock recorded at the last successful apply.
///
/// Order matters. A missing output is checked first because it is the most
/// definite signal and the most user-visible: `rm build/demo` must rebuild,
/// and previously reported `unchanged`.
pub fn staleness_reason(
    probe: &IoDigest,
    stored_input_hash: Option<&str>,
    stored_output_hash: Option<&str>,
) -> Option<String> {
    if probe.outputs_missing {
        return Some("output artifact missing".to_string());
    }

    if let Some(current) = probe.input_hash.as_deref() {
        match stored_input_hash {
            // No recorded hash: the resource converged before it declared
            // inputs, or under an older forjar. Re-run once to establish a
            // baseline rather than assuming it is current.
            None => return Some("no recorded input hash".to_string()),
            Some(stored) if stored != current => {
                return Some("inputs changed".to_string());
            }
            _ => {}
        }
    }

    if let Some(current) = probe.output_hash.as_deref() {
        if let Some(stored) = stored_output_hash {
            if stored != current {
                return Some("output artifact modified".to_string());
            }
        }
    }

    None
}

/// Record the observed input/output hashes of a just-applied resource.
///
/// Called by the executor after a successful apply so the NEXT plan has a
/// baseline to compare the probe against.
///
/// Three fixes are baked in versus the code this replaces:
/// * the base directory is `working_dir`, not `state_dir.parent()`. A build
///   declares paths relative to its project root; hashing against the state
///   directory made every relative input hash as absent, silently disabling
///   caching whenever `--state-dir` was relative.
/// * it no longer requires `cache: true`. Recording is what makes correctness
///   possible (rebuild when inputs change), so it is not opt-in; `cache`
///   remains the switch for SKIPPING work, not for tracking it.
/// * forjar#501: it records NOTHING for a machine this host does not answer
///   for. The hashes are taken on the controller; under a remote machine's
///   lock they were a correct hash of the wrong tree, and the cache reader
///   would skip a remote run from them. An absent `input_hash` reads as
///   "no recorded input hash" in [`staleness_reason`] — re-run once — never
///   as clean.
pub fn record_io_hashes(
    resource: &Resource,
    machine: &crate::core::types::Machine,
    details: &mut std::collections::HashMap<String, serde_yaml_ng::Value>,
) {
    if !declares_inputs(resource) && resource.output_artifacts.is_empty() {
        return;
    }
    if !probe_answers_for(machine) {
        return;
    }
    let base = probe_base_dir(resource);

    if let Some(hash) = hash_declared_inputs(resource, &base) {
        details.insert("input_hash".to_string(), serde_yaml_ng::Value::String(hash));
    }
    if !resource.output_artifacts.is_empty() {
        if let Ok(Some(hash)) = hash_outputs_with(
            &resource.output_artifacts,
            &base,
            &resource.output_equivalence,
        ) {
            details.insert(
                "output_hash".to_string(),
                serde_yaml_ng::Value::String(hash),
            );
        }
    }
}

/// Build the probe map for a whole config.
///
/// The single place that decides what "observed state" means, so the read
/// paths (`plan`, `check`, `drift`, `observe`) and the write path (`apply`)
/// can never disagree about it.
///
/// v1.11.0 shipped without this: `planner::plan` forwarded an EMPTY map, so
/// after `rm build/demo` the tool reported `Plan: 0 to change, 3 unchanged`,
/// `Check: 3 pass, 0 fail`, and `No drift detected` — and then `apply`
/// rebuilt. A planner that cannot predict its own apply is worse than one
/// that is merely conservative.
///
/// Resources MUST be resolved first: `working_dir` is routinely
/// `{{params.proj}}`, and probing the raw form makes every artifact look
/// missing.
pub fn probe_config(config: &crate::core::types::ForjarConfig) -> ProbeMap {
    let resolved = crate::core::resolver::resolve_all(
        &config.resources,
        &config.params,
        &config.machines,
        &config.secrets,
    );
    probe_all(&resolved, |m| probe_covers(config, m))
}

/// Does this host's build-I/O probe cover `machine`?
///
/// ONE definition, forjar#497. `probe_config`, the executor's pre-plan probe
/// and the planner's unprobed census all ask this, so what the planner
/// reports as "not measured" is exactly the set the probe skipped — two
/// inlined copies of the predicate is how the census and the probe drift
/// apart and a resource gets reported as both.
///
/// forjar#495: NOT `machine_is_local`, which admits a pepita namespace. The
/// probe hashes declared inputs and outputs on the CONTROLLER; for a
/// namespaced machine those files live inside the namespace, so measuring
/// here answers about the wrong host.
pub fn probe_covers(config: &crate::core::types::ForjarConfig, machine: &str) -> bool {
    config.machines.get(machine).is_some_and(probe_answers_for)
}

/// Does a hash taken on THIS host answer for `machine`?
///
/// forjar#501. The one place this module names the transport predicate
/// (forjar#485's single definition of "this host's filesystem is that
/// machine's"): [`probe_covers`] asks it for a machine NAME through the
/// config; the executor's I/O writer ([`record_io_hashes`]) and its cache
/// reader ask it for the `Machine` they hold. A digest of this host's tree is
/// a fact about this host's machines and no other — recording it under a
/// remote machine's lock, or reading it back to skip a remote run, is a
/// correct hash of the wrong tree.
pub fn probe_answers_for(machine: &crate::core::types::Machine) -> bool {
    crate::transport::controller_answers_for(machine)
}

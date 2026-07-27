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
//! The planner stays pure. It receives an already-computed `HashMap<String,
//! IoDigest>` and never touches the filesystem or a transport, so its unit
//! tests just construct the map.

use super::io_tracking::{hash_inputs, hash_outputs_in};
use crate::core::types::Resource;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Observed on-disk state of one resource's declared inputs and outputs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IoDigest {
    /// Composite hash of all files matching `task_inputs`, or `None` when the
    /// resource declares no inputs.
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
    if resource.task_inputs.is_empty() && resource.output_artifacts.is_empty() {
        return None;
    }

    let base = probe_base_dir(resource);

    let input_hash = if resource.task_inputs.is_empty() {
        None
    } else {
        hash_inputs(&resource.task_inputs, &base).ok().flatten()
    };

    let mut outputs_missing = false;
    for artifact in &resource.output_artifacts {
        if !resolve_under(&base, artifact).exists() {
            outputs_missing = true;
            break;
        }
    }

    let output_hash = if resource.output_artifacts.is_empty() || outputs_missing {
        None
    } else {
        hash_outputs_in(&resource.output_artifacts, &base)
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
/// `is_local` decides whether a resource's machine is on this host. Resources
/// on remote machines are SKIPPED rather than probed: hashing the controller's
/// filesystem for a remote target would compare the wrong tree and silently
/// produce wrong build decisions. Skipping preserves today's behaviour for
/// them (config-hash planning) instead of inventing a wrong answer.
pub fn probe_all<F>(
    resources: &indexmap::IndexMap<String, Resource>,
    is_local: F,
) -> HashMap<String, IoDigest>
where
    F: Fn(&str) -> bool,
{
    let mut out = HashMap::new();
    for (id, resource) in resources {
        if !resource.machine.iter().any(&is_local) {
            continue;
        }
        if let Some(d) = probe_resource(resource) {
            if !d.is_empty() {
                out.insert(id.clone(), d);
            }
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
/// Two fixes are baked in versus the code this replaces:
/// * the base directory is `working_dir`, not `state_dir.parent()`. A build
///   declares paths relative to its project root; hashing against the state
///   directory made every relative input hash as absent, silently disabling
///   caching whenever `--state-dir` was relative.
/// * it no longer requires `cache: true`. Recording is what makes correctness
///   possible (rebuild when inputs change), so it is not opt-in; `cache`
///   remains the switch for SKIPPING work, not for tracking it.
pub fn record_io_hashes(
    resource: &Resource,
    details: &mut std::collections::HashMap<String, serde_yaml_ng::Value>,
) {
    if resource.task_inputs.is_empty() && resource.output_artifacts.is_empty() {
        return;
    }
    let base = probe_base_dir(resource);

    if !resource.task_inputs.is_empty() {
        if let Ok(Some(hash)) = hash_inputs(&resource.task_inputs, &base) {
            details.insert("input_hash".to_string(), serde_yaml_ng::Value::String(hash));
        }
    }
    if !resource.output_artifacts.is_empty() {
        if let Ok(Some(hash)) = hash_outputs_in(&resource.output_artifacts, &base) {
            details.insert(
                "output_hash".to_string(),
                serde_yaml_ng::Value::String(hash),
            );
        }
    }
}

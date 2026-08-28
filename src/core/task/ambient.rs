//! GH-244(c): ambient inputs — a declarable input kind for the things no glob
//! can name.
//!
//! # Why this exists
//!
//! `staleness_reason` decides entirely from the DECLARED set, and until now the
//! only declarable inputs were PATHS. `hash_inputs` glob-expands
//! `task_inputs` and hashes the files it matches; there is no environment
//! component, no tool-version component, no ambient component. So a task that
//! reads something outside the project tree yields `staleness_reason() == None`
//! after that thing changes, and `plan`, `check`, `drift` and `apply` all
//! report a clean converged stack over a stale artifact.
//!
//! The motivating case (#244) is a rasterizer that calls
//! `fontdb.load_system_fonts()` at renderer construction. That is an ambient
//! host input to every render, it cannot be enumerated as a glob in any honest
//! way, and it changes without anybody noticing: `apt install fonts-*`, a base
//! image bump, a new CI runner AMI. The consequence is not "we rebuild too
//! often" — it is that every subsequent frame renders with different glyph
//! metrics while forjar reports `Check: N pass, 0 fail`.
//!
//! # What this does NOT do
//!
//! This is still a DECLARATION, so it detects nothing nobody thought of. It
//! covers what you name; `forjar verify --check-declared-inputs` covers
//! unnamed reads of files inside the project tree; nothing on offer proves the
//! declaration complete.
//!
//! # Cost, stated rather than cached
//!
//! One subprocess per ambient input per probe, on every plan/check/drift/apply.
//! A cached fingerprint is a fingerprint that lies, so there is no cache.

use super::io_tracking::hash_inputs;
use super::probe::probe_base_dir;
use crate::core::types::Resource;
use crate::tripwire::hasher;
use std::path::Path;
use std::process::Command;

/// True when the resource declares anything the input hash can be built from.
///
/// The probe and the executor's cache-skip both gate on this. Keying either on
/// `task_inputs` alone would leave an ambient-only resource unprobed — the
/// fingerprint would be computed and then never consulted.
pub fn declares_inputs(resource: &Resource) -> bool {
    !resource.task_inputs.is_empty() || !resource.ambient_inputs.is_empty()
}

/// The input hash for a resource: declared FILES, plus declared AMBIENT
/// fingerprints.
///
/// `file_base` is the directory `task_inputs` globs are relative to. The
/// ambient commands run instead with cwd = [`probe_base_dir`], derived from the
/// RESOURCE — the executor passes `state_dir.parent()` where the probe passes
/// `working_dir`, and an ambient component that depended on which caller asked
/// would report "inputs changed" on every plan forever.
///
/// With no `ambient_inputs` the result is byte-identical to `hash_inputs`, so
/// upgrading forjar does not invalidate a single existing lock.
pub fn hash_declared_inputs(resource: &Resource, file_base: &Path) -> Option<String> {
    let files = hash_inputs(&resource.task_inputs, file_base).ok().flatten();
    if resource.ambient_inputs.is_empty() {
        return files;
    }

    let cwd = probe_base_dir(resource);
    let mut components: Vec<String> = vec![files.unwrap_or_default()];
    for command in &resource.ambient_inputs {
        components.push(ambient_component(command, &cwd));
    }
    let refs: Vec<&str> = components.iter().map(String::as_str).collect();
    Some(hasher::composite_hash(&refs))
}

/// One ambient input's contribution to the composite: the command text, then
/// the digest of what it printed.
///
/// A FAILING command is folded in as a failure marker rather than dropped.
/// Dropping it collapses the input hash back to the file-only value, which
/// reports clean over a stale artifact — the exact bug this feature exists to
/// close, reintroduced the moment the fingerprint breaks. That diverges from
/// `output_hash`'s hard-error precedent on purpose: `probe_resource` swallows
/// `Err` with `.ok().flatten()`, so an error here would become silence.
///
/// stderr is deliberately NOT hashed. It routinely carries a pid or a
/// timestamp, and folding that in would make every plan report "inputs
/// changed" — an idempotency pump dressed as vigilance.
fn ambient_component(command: &str, cwd: &Path) -> String {
    let run = Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .output();
    match run {
        Ok(out) if out.status.success() => {
            let digest = hasher::hash_string(&String::from_utf8_lossy(&out.stdout));
            format!("{command}\0{digest}")
        }
        Ok(out) => format!("{command}\0failed:{}", out.status.code().unwrap_or(-1)),
        Err(e) => format!("{command}\0unspawnable:{:?}", e.kind()),
    }
}

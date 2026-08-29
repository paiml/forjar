//! GH-376: the config body that produced a generation, and how `undo` replays it.
//!
//! THE DEFECT. `cmd_undo` rolled the lock back to the target generation and
//! then called `cmd_apply` on the CURRENT config, which immediately re-converged
//! the host to the current config's declared state. Three applies of v1 → v2 →
//! v3 followed by `undo --yes` exited 0, printed "1 converged", and left the
//! file holding `v3`. Nothing was undone.
//!
//! It could not have worked. A generation recorded only a BLAKE3 `config_hash`;
//! the body was never stored, and `git_ref` — the only other config trace — is
//! printed and never resolved, and would miss uncommitted edits anyway. Undo
//! had no way to know what the target generation's desired state WAS, so it
//! used the one config it had.
//!
//! THE FIX. A successful apply now records the generation AFTER it converges,
//! so generation N holds both the lock apply N produced and, beside it, the
//! exact bytes of the config that produced it. `undo` re-applies THAT.
//!
//! WHY THE REPLAY FILE LANDS BESIDE THE ORIGINAL CONFIG. A config's `includes:`
//! entries, `content_file:` sources and relative resource paths all resolve
//! against the directory holding the config. Replaying the recorded body from a
//! scratch directory would silently resolve them somewhere else — the same
//! class of defect as #377, one layer down. So the body is staged as a hidden
//! sibling of the config the operator named, used, and deleted.

use crate::core::types::GenerationMeta;
use std::path::{Path, PathBuf};

/// The config body that produced a generation, stored inside it.
///
/// Dot-prefixed so every generation reader that walks machine subdirectories
/// (`load_gen_locks`, `load_generation_locks`) skips it for free.
pub(super) const APPLIED_CONFIG: &str = ".applied-config.yaml";

/// Record the config that produced this generation: its BLAKE3 hash into the
/// metadata, its BODY beside the locks.
///
/// A body that cannot be written is a warning, not a failed apply — the host
/// has already converged. It costs the operator `undo` for that generation,
/// which `cmd_undo` then refuses explicitly rather than silently mis-applying.
pub(super) fn record_config(
    meta: &mut GenerationMeta,
    gen_path: &Path,
    config: &crate::core::types::ForjarConfig,
) {
    // THE EXPANDED CONFIG, not the file the operator named.
    //
    // Recording the raw bytes captured only the top-level document, so two
    // classes of desired state escaped and `undo` silently converged the host
    // FORWARDS for both (#376):
    //
    //   includes:  the bodies were re-read LIVE at replay time, so resources
    //              declared in an included file were never reverted.
    //   -p params: overrides are merged into the config at apply time and were
    //              never written down, so replay re-resolved every
    //              `{{params.*}}` to its DEFAULT — landing the host on bytes no
    //              generation ever held.
    //
    // Both are already resolved in the value we are handed: includes are merged
    // during parse, and `apply_param_overrides` runs before this. Serialising
    // it captures them for free.
    let mut snapshot = config.clone();
    // `includes` survives serialisation, and a replay would MERGE THOSE FILES
    // AGAIN from disk — reintroducing exactly the staleness this fixes, since
    // their resources are already inlined here. Clear it: this document is the
    // merged result and must replay as a closed unit.
    snapshot.includes.clear();
    let Ok(body) = serde_yaml_ng::to_string(&snapshot) else {
        eprintln!("warning: cannot serialise the applied config for the generation");
        return;
    };
    // Hash what is actually replayed, not the file on disk — they are
    // deliberately different documents now.
    let hash = blake3::hash(body.as_bytes()).to_hex().to_string();
    meta.config_hash = Some(format!("blake3:{hash}"));
    if let Err(e) = write_private(&gen_path.join(APPLIED_CONFIG), &body) {
        eprintln!("warning: cannot record the applied config in the generation: {e}");
    }
}

/// Write the recorded config body readable only by its owner.
///
/// The body is a full copy of the operator's config. A config kept at 0600
/// because it carries secrets would otherwise land in the state dir at the
/// default umask (0664) — one plaintext copy per generation. Pristine forjar
/// stored only a hash, so this file is new exposure and must not widen it.
fn write_private(path: &Path, body: &str) -> std::io::Result<()> {
    std::fs::write(path, body)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// The recorded config body for a generation, if one was ever written.
pub(super) fn load_snapshot(gen_path: &Path) -> Option<String> {
    std::fs::read_to_string(gen_path.join(APPLIED_CONFIG)).ok()
}

/// Refuse, naming the remedy, when the target generation predates the snapshot.
///
/// The old behaviour — fall back to the current config — IS #376, so it is
/// deleted rather than kept as a default. State written by forjar ≤ 1.22.0 has
/// no body, and the `config_hash` it does have cannot be resolved back to one.
pub(super) fn no_snapshot_error(target: u32, gen_path: &Path) -> String {
    format!(
        "generation {target} records no config, so undo cannot know what state to \
         return the host to ({} is missing). It was written by forjar 1.22.0 or \
         earlier, which stored only a hash of the config, never its body. \
         Undo will not fall back to the CURRENT config — doing that is what made \
         `undo` re-converge the host forwards and undo nothing. \
         Run `forjar apply` once with this version to record a replayable \
         generation, then undo becomes available from that point on",
        gen_path.join(APPLIED_CONFIG).display(),
    )
}

/// A recorded config body staged as a hidden sibling of the config the operator
/// named, so relative includes and file sources resolve exactly as they did
/// when it was applied. Removed on drop, including on the error paths.
pub(super) struct ReplayConfig {
    path: PathBuf,
}

impl ReplayConfig {
    /// Stage generation `gen`'s recorded body next to `original`.
    pub(super) fn stage(original: &Path, gen: u32, body: &str) -> Result<Self, String> {
        let dir = match original.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        };
        let path = dir.join(format!(".forjar-undo-gen{gen}-{}.yaml", std::process::id()));
        std::fs::write(&path, body).map_err(|e| {
            format!(
                "cannot stage generation {gen}'s recorded config at {} ({e}) — undo replays it \
                 from beside {} so the config's relative includes and file sources resolve the \
                 way they did when it was applied; make that directory writable and retry",
                path.display(),
                original.display(),
            )
        })?;
        Ok(Self { path })
    }

    /// The staged file, for handing to `cmd_apply`.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ReplayConfig {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Resources whose desired bytes live OUTSIDE the recorded config, and which
/// `undo` therefore cannot faithfully replay.
///
/// A generation records the config that produced it, not the tree around it. A
/// `file` resource with `source:`/`content_file:` points at a path whose
/// CURRENT contents are read at apply time, so replaying an old config against
/// a newer payload converges the host FORWARDS: measured on a three-apply
/// P1→P2→P3 stack, `undo` exited 0, printed "1 converged", and left the host on
/// P3 while stamping the lock with generation 1's hash — after which `drift`
/// reported clean, so the corruption was self-consistent and invisible.
///
/// Jidoka: a machine that cannot do the job correctly stops and signals rather
/// than producing a defective part. Undo refuses, names the resources, and
/// leaves the host untouched.
pub(super) fn unreplayable_resources(config: &crate::core::types::ForjarConfig) -> Vec<String> {
    config
        .resources
        .iter()
        .filter(|(_, r)| r.source.is_some())
        .map(|(id, _)| format!("{id} (source)"))
        .collect()
}

/// The refusal for a generation whose replay would read files the generation
/// never captured.
pub(super) fn unreplayable_error(target: u32, offenders: &[String]) -> String {
    format!(
        "refusing to undo to generation {target}: {} resource(s) take their content from a file \
         outside the config, and the generation recorded the config but not that file: {}. \
         Replaying would read whatever those paths hold NOW, converging the host forward and \
         stamping the lock with generation {target}'s hash — `drift` would then report clean over \
         the wrong bytes. Nothing has been changed. Restore the payload file(s) to the contents \
         they had at generation {target} and re-run, or set the content inline with `content:` \
         so the generation captures it.",
        offenders.len(),
        offenders.join(", ")
    )
}

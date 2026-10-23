//! Shared CLI helpers: color re-exports, parsing, state utilities.

use crate::core::{parser, types};
use std::path::Path;

// Re-export color system from colors.rs for backward compatibility.
// All callers that `use super::helpers::*` continue to work unchanged.
#[allow(unused_imports)]
pub(crate) use super::colors::{bold, color_enabled, dim, green, red, yellow, NO_COLOR};

/// Parse, validate, expand recipes AND resolve `data:` sources (forjar#660).
///
/// THE loader for every command that reads `--file`. `{{data.x}}` only resolves
/// after `resolve_data_sources` has run, and a reader that loaded the file by
/// hand skipped it (show, explain, check, output, apply --check, ...). Resolving
/// by default makes the correct path the short one; a failing `data:` source is
/// an `Err` here, never a config with the reference left in it.
pub(crate) fn parse_and_validate(file: &Path) -> Result<types::ForjarConfig, String> {
    crate::core::resolver::load_resolved_config(file)
}

/// The resolved manifest as a generic YAML document, for the readers that walk
/// `resources:` as a mapping (store pin / convert / purity). It goes through the
/// resolving loader like every other reader (forjar#660); it is never a raw parse
/// of the file.
pub(crate) fn load_resolved_value(
    file: &Path,
) -> Result<serde_yaml_ng::Value, crate::core::error::ForjarError> {
    let config = crate::core::resolver::load_resolved_config_lenient(file)?;
    serde_yaml_ng::to_value(&config).map_err(|e| {
        crate::core::error::ForjarError::validation(format!("serialize {}: {e}", file.display()))
    })
}

/// Parse, validate and expand recipes WITHOUT resolving `data:` sources.
///
/// Only for a command that must not execute `data:` commands or that has to
/// tell a parse failure apart from a resolve failure. Every caller is listed,
/// with its reason, in `tests/falsification_660_every_reader_resolves_data.rs`
/// (`UNRESOLVED_LOADER_ALLOWLIST`); a new caller fails that test.
pub(crate) fn parse_and_validate_unresolved(file: &Path) -> Result<types::ForjarConfig, String> {
    parser::parse_and_validate(file)
}

/// Refuse to report on a state directory that is not there.
///
/// [`discover_machines`] swallows the read error and answers "no machines", so
/// every command built on it used to treat a wrong `--state-dir`, a wiped
/// state, or a run before the first apply as an empty-but-healthy state and
/// exit 0 — `lock-validate` printing "All 0 lock files are valid" at a path
/// that does not exist. `lock-verify` and `lock-info` always rejected it; this
/// is that check, shared, so the rest can match them.
pub(crate) fn require_state_dir(state_dir: &Path) -> Result<(), String> {
    if state_dir.exists() {
        Ok(())
    } else {
        Err(format!(
            "state directory not found: {}",
            state_dir.display()
        ))
    }
}

/// FJ-1270 / CB-2010: BLAKE3 sidecar verification failures for a state directory,
/// as `(machine, reason)` pairs. Empty means every lock file still hashes to the
/// `.b3` sidecar written beside it by `save_lock`.
///
/// This is the measurement the `lock-verify` / `lock-integrity` / `lock-validate` /
/// `lock-audit` commands exist to make. Every one of them used to skip it and print
/// a green result from the lock's own self-reported hash *fields* — which a tamperer
/// controls — so a rewritten body, a deleted sidecar and a deleted lock all passed.
///
/// Unlike [`crate::core::state::integrity::has_errors`] (used by `apply`, which
/// tolerates a missing sidecar so pre-FJ-1270 state dirs still converge), this
/// counts a missing sidecar as a failure: a command whose whole job is integrity has
/// no instrument without it, and an absent verifier is a NO-GO, never a pass.
pub(crate) fn sidecar_failures(state_dir: &Path) -> Vec<(String, String)> {
    use crate::core::state::integrity;
    integrity::verify_state_integrity(state_dir)
        .iter()
        .filter_map(|r| {
            let reason = integrity::failure_reason(r)?;
            Some((sidecar_machine_name(state_dir, r), reason))
        })
        .collect()
}

/// Machine a sidecar result belongs to — the lock's parent directory name, or
/// `<global>` for `state_dir/forjar.lock.yaml`.
fn sidecar_machine_name(
    state_dir: &Path,
    result: &crate::core::state::integrity::IntegrityResult,
) -> String {
    crate::core::state::integrity::result_path(result)
        .and_then(|p| p.parent())
        .filter(|parent| *parent != state_dir)
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "<global>".to_string())
}

/// Discover machine names from a state directory by listing subdirectories that contain state.lock.yaml.
pub(crate) fn discover_machines(state_dir: &Path) -> Vec<String> {
    let mut machines = Vec::new();
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if entry.path().join("state.lock.yaml").exists() {
                    machines.push(name);
                }
            }
        }
    }
    machines.sort();
    machines
}

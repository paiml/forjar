//! Shared CLI helpers: color re-exports, parsing, state utilities.

use crate::core::{parser, types};
use std::path::Path;

// Re-export color system from colors.rs for backward compatibility.
// All callers that `use super::helpers::*` continue to work unchanged.
#[allow(unused_imports)]
pub(crate) use super::colors::{bold, color_enabled, dim, green, red, yellow, NO_COLOR};

/// Parse, validate, and expand recipes in a forjar config file.
pub(crate) fn parse_and_validate(file: &Path) -> Result<types::ForjarConfig, String> {
    parser::parse_and_validate(file)
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

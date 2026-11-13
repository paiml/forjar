//! forjar#661: which machines, and whose entries, `lock-prune -f <config>` may touch.
//!
//! `lock-prune` used to walk every machine directory under `--state-dir` and call
//! every lock key the ONE `-f` config lacks "stale". In a state dir shared by two
//! stacks that deletes the other stack's entries, and on a machine the config
//! never declared. Lock entries do not record their stack (that is #561), so the
//! guard here is the one the stamps allow: only machines the config declares, and
//! never a machine another stack's stamp also lists.

use crate::core::types;
use std::path::Path;

/// Machine directories under `state_dir` that `config` declares, sorted.
pub(super) fn declared_machine_dirs(
    config: &types::ForjarConfig,
    state_dir: &Path,
) -> Result<Vec<String>, String> {
    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("cannot read state dir: {e}"))?;
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| config.machines.contains_key(n))
        .collect();
    names.sort();
    Ok(names)
}

/// The other stack that owns `machine` per the global lock's stamps, if any.
fn other_owner(
    global: Option<&types::GlobalLock>,
    own_stack: &str,
    machine: &str,
) -> Option<String> {
    global?
        .stacks
        .iter()
        .find(|(name, stamp)| {
            name.as_str() != own_stack && stamp.machines.iter().any(|m| m == machine)
        })
        .map(|(name, _)| name.clone())
}

/// Refuse to prune `stale` entries from a machine that another stack also owns.
pub(super) fn refuse_if_owned_by_other(
    global: Option<&types::GlobalLock>,
    own_stack: &str,
    machine: &str,
    stale: &[String],
) -> Result<(), String> {
    if stale.is_empty() {
        return Ok(());
    }
    match other_owner(global, own_stack, machine) {
        None => Ok(()),
        Some(other) => Err(format!(
            "refusing to prune machine '{machine}': stack '{other}' also owns it in this state dir, \
             and lock entries do not record their stack, so {} (not in stack '{own_stack}') may be \
             '{other}''s. Give each stack its own --state-dir (#561 will key locks by machine and stack).",
            stale
                .iter()
                .map(|s| format!("'{s}'"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

//! State loading and machine discovery helpers.

use crate::core::{state, types};
use std::path::Path;

/// Load lock files for machines referenced in the config.
pub(crate) fn load_machine_locks(
    config: &types::ForjarConfig,
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Result<std::collections::HashMap<String, types::StateLock>, String> {
    let mut locks = std::collections::HashMap::new();
    if !state_dir.exists() {
        return Ok(locks);
    }
    for machine_name in config.machines.keys() {
        if let Some(filter) = machine_filter {
            if machine_name != filter {
                continue;
            }
        }
        if let Some(lock) = state::load_lock(state_dir, machine_name)? {
            locks.insert(machine_name.clone(), lock);
        }
    }
    Ok(locks)
}

/// List machine names from state directory subdirectories.
pub(crate) fn list_state_machines(state_dir: &Path) -> Result<Vec<String>, String> {
    let mut machines = Vec::new();
    let entries =
        std::fs::read_dir(state_dir).map_err(|e| format!("cannot read state dir: {e}"))?;
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden dirs and non-machine dirs
            if !name.starts_with('.') {
                machines.push(name);
            }
        }
    }
    machines.sort();
    Ok(machines)
}

// ============================================================================
// FJ-212: state-mv — rename a resource in state
// ============================================================================

/// Load all machine locks for planning (used by watch).
pub(crate) fn load_all_locks(
    state_dir: &Path,
    config: &types::ForjarConfig,
) -> std::collections::HashMap<String, types::StateLock> {
    let mut locks = std::collections::HashMap::new();
    for machine_name in config.machines.keys() {
        if let Ok(Some(lock)) = state::load_lock(state_dir, machine_name) {
            locks.insert(machine_name.clone(), lock);
        }
    }
    // Also check for "localhost" resources
    if config
        .resources
        .values()
        .any(|r| matches!(&r.machine, types::MachineTarget::Single(m) if m == "local" || m == "localhost"))
    {
        if let Ok(Some(lock)) = state::load_lock(state_dir, "local") {
            locks.insert("local".to_string(), lock);
        }
    }
    locks
}

#[allow(clippy::too_many_arguments)]
/// FJ-285: Collect a resource and its transitive dependencies.
pub(crate) fn collect_transitive_deps(
    config: &types::ForjarConfig,
    target: &str,
) -> Result<std::collections::HashSet<String>, String> {
    if !config.resources.contains_key(target) {
        return Err(format!("resource '{target}' not found"));
    }
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![target.to_string()];
    while let Some(id) = stack.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        if let Some(r) = config.resources.get(&id) {
            for dep in &r.depends_on {
                stack.push(dep.clone());
            }
        }
    }
    Ok(visited)
}

/// Simple glob matching — supports `*` wildcard at start/end/both.
pub(crate) fn simple_glob_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    let starts_with_star = pattern.starts_with('*');
    let ends_with_star = pattern.ends_with('*');
    let core = pattern.trim_matches('*');

    match (starts_with_star, ends_with_star) {
        (true, true) => text.contains(core),
        (true, false) => text.ends_with(core),
        (false, true) => text.starts_with(core),
        (false, false) => text == pattern,
    }
}

/// Load lock files from a generation directory, optionally filtered by machine.
pub(super) fn load_generation_locks(
    gen_dir: &std::path::Path,
    machine_filter: Option<&str>,
) -> std::collections::HashMap<String, crate::core::types::StateLock> {
    let mut locks = std::collections::HashMap::new();
    let Ok(entries) = std::fs::read_dir(gen_dir) else {
        return locks;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if let Some(filter) = machine_filter {
            if name != filter {
                continue;
            }
        }
        let lock_path = entry.path().join("state.lock.yaml");
        if let Ok(content) = std::fs::read_to_string(&lock_path) {
            if let Ok(lock) = serde_yaml_ng::from_str::<crate::core::types::StateLock>(&content) {
                locks.insert(name, lock);
            }
        }
    }
    locks
}

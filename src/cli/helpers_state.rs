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

/// FJ-1388: Get the current generation number before apply starts.
pub(super) fn pre_apply_generation(state_dir: &std::path::Path) -> Option<u32> {
    let gen_dir = state_dir.join("generations");
    super::generation::current_generation(&gen_dir)
}

/// FJ-1388: Rollback to pre-apply generation on failure.
///
/// PMAT-174: the multi-stack refusal (PMAT-162) leaves here as an `Err`, not as
/// `warning: generation rollback failed`. A rollback in a dir several stacks
/// share is whole-dir, so it would revert the neighbours; that is a refusal the
/// run must exit on, and it used to be one line of stderr under a failed apply
/// that exited on the resources instead.
///
/// `apply` no longer reaches it — `apply_preflight::rollback_on_failure_gate`
/// asks the same question before the first byte is written, which is where an
/// operator can still act on the answer. This copy stays because it is what any
/// future caller of the rollback path inherits, and because a guard that only
/// exists at the entrance is one refactor away from being missed.
///
/// Every OTHER rollback failure keeps its warning: the run is already failing
/// on its resources, and a missing generation dir is not a reason to replace
/// that diagnosis with a different one.
pub(super) fn maybe_rollback_generation(
    rollback_on_failure: bool,
    state_dir: &std::path::Path,
    pre_apply_gen: Option<u32>,
    verbose: bool,
) -> Result<(), String> {
    if !rollback_on_failure {
        return Ok(());
    }
    let Some(gen) = pre_apply_gen else {
        return Ok(());
    };
    super::generation::restore::refuse_multi_stack_restore(state_dir, Some(gen))?;
    eprintln!("rollback: restoring state to generation {gen}");
    if let Err(e) = super::generation::rollback_to_generation(state_dir, gen, true) {
        eprintln!("warning: generation rollback failed: {e}");
    } else if verbose {
        eprintln!("rollback: restored to generation {gen}");
    }
    Ok(())
}

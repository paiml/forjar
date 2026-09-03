//! forjar#449: `undo` must DESTROY what the target generation does not hold.
//!
//! `compute_undo_diff` has always announced such resources as "will be
//! destroyed", but the replay that follows is an `apply`, and apply never
//! removes anything. So `apply a` → `apply a,b` → `undo` printed the promise
//! and left `b` on the host, and — the review's poisoned rollback — undo onto
//! the generation a `destroy` recorded left the destroyed resources standing.
//!
//! The destroy runs BEFORE `rollback_to_generation` because it needs the live
//! locks (pre-hash for the destroy log) and the CURRENT config's definitions of
//! the resources; the target generation, by construction, no longer declares
//! them.

use std::collections::HashMap;
use std::path::Path;

use crate::core::types::{ForjarConfig, StateLock};

/// Resource ids present in any live lock but absent from the target
/// generation's lock for the same machine (or whose machine has no lock at all
/// in the target). Sorted, so the staged config is deterministic.
pub(super) fn absent_from_target(
    current_locks: &HashMap<String, StateLock>,
    target_locks: &HashMap<String, StateLock>,
) -> Vec<String> {
    let mut gone: Vec<String> = current_locks
        .iter()
        .flat_map(|(machine, cl)| {
            let tl = target_locks.get(machine);
            cl.resources
                .keys()
                .filter(move |rid| tl.is_none_or(|t| !t.resources.contains_key(*rid)))
                .cloned()
        })
        .collect();
    gone.sort();
    gone.dedup();
    gone
}

/// The current config narrowed to `gone`: same machines, params and secrets,
/// only those resources, with `depends_on` edges into kept resources dropped
/// (the resolver rejects dangling edges, and a kept resource is not destroyed).
pub(super) fn narrowed_config(current: &ForjarConfig, gone: &[String]) -> ForjarConfig {
    let mut cfg = current.clone();
    cfg.resources.retain(|id, _| gone.contains(id));
    for r in cfg.resources.values_mut() {
        r.depends_on.retain(|d| gone.contains(d));
    }
    cfg
}

/// What an undo needs in order to destroy: where the live config is, the
/// state dir, the current generation number (the staged file is named after
/// it), the current config and both lock sets, and the machine filter.
pub(super) struct UndoPrune<'a> {
    pub file: &'a Path,
    pub state_dir: &'a Path,
    pub current: u32,
    pub current_config: &'a ForjarConfig,
    pub current_locks: &'a HashMap<String, StateLock>,
    pub target_locks: &'a HashMap<String, StateLock>,
    pub machine_filter: Option<&'a str>,
}

/// Destroy every resource the live state holds that the target generation
/// does not, using the current config's definitions. A no-op when the target
/// holds everything. Generation recording is paused: this is half of an undo,
/// not a new generation.
pub(super) fn destroy_absent_from_target(p: &UndoPrune<'_>) -> Result<(), String> {
    let gone = absent_from_target(p.current_locks, p.target_locks);
    if gone.is_empty() {
        return Ok(());
    }
    let cfg = narrowed_config(p.current_config, &gone);
    let body = serde_yaml_ng::to_string(&cfg)
        .map_err(|e| format!("cannot stage the resources to destroy: {e}"))?;
    // Staged under the CURRENT generation's number so it never collides with
    // the target generation's replay file, which is already staged.
    let staged = super::undo_replay::ReplayConfig::stage(p.file, p.current, &body)?;
    println!(
        "\nDestroying {} resource(s) the target generation does not hold: {}",
        gone.len(),
        gone.join(", ")
    );
    let _paused = super::apply_snapshot::PauseGenerationRecording::new();
    super::destroy::cmd_destroy(staged.path(), p.state_dir, p.machine_filter, true, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{ResourceLock, ResourceStatus, ResourceType};

    fn lock(ids: &[&str]) -> StateLock {
        let mut l = crate::core::state::new_lock("m", "h");
        for id in ids {
            l.resources.insert(
                (*id).to_string(),
                ResourceLock {
                    resource_type: ResourceType::File,
                    status: ResourceStatus::Unknown,
                    applied_at: None,
                    duration_seconds: None,
                    hash: String::new(),
                    observed: None,
                    details: std::collections::HashMap::new(),
                },
            );
        }
        l
    }

    #[test]
    fn absent_is_the_live_minus_target_per_machine() {
        let mut cur = HashMap::new();
        cur.insert("m".to_string(), lock(&["a", "b"]));
        cur.insert("n".to_string(), lock(&["c"]));
        let mut tgt = HashMap::new();
        tgt.insert("m".to_string(), lock(&["a"]));
        assert_eq!(absent_from_target(&cur, &tgt), vec!["b", "c"]);
        assert!(absent_from_target(&tgt, &cur).is_empty());
    }

    #[test]
    fn narrowed_config_keeps_only_gone_and_drops_edges_into_kept() {
        let yaml = concat!(
            "version: \"1.0\"\nname: x\n",
            "machines: { m: { hostname: h, addr: 127.0.0.1 } }\n",
            "resources:\n",
            "  a: { type: file, machine: m, path: /tmp/a }\n",
            "  b: { type: file, machine: m, path: /tmp/b, depends_on: [a] }\n"
        );
        let cfg: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let n = narrowed_config(&cfg, &["b".to_string()]);
        assert_eq!(n.resources.keys().collect::<Vec<_>>(), vec!["b"]);
        assert!(n.resources["b"].depends_on.is_empty());
        assert_eq!(n.machines.len(), 1);
    }

    #[test]
    fn nothing_absent_is_a_noop_without_touching_the_host() {
        let cur = HashMap::from([("m".to_string(), lock(&["a"]))]);
        let cfg = ForjarConfig::default();
        let p = UndoPrune {
            file: Path::new("/nonexistent/forjar.yaml"),
            state_dir: Path::new("/nonexistent/state"),
            current: 1,
            current_config: &cfg,
            current_locks: &cur,
            target_locks: &cur,
            machine_filter: None,
        };
        assert_eq!(destroy_absent_from_target(&p), Ok(()));
    }
}

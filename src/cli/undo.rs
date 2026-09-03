use super::apply::*;
use super::helpers::*;
use super::helpers_state::load_generation_locks;
use crate::core::types;
use std::path::Path;

// Re-export cmd_undo_destroy from undo_helpers so callers using `undo::*` still find it.
pub(crate) use super::undo_helpers::cmd_undo_destroy;

// The undo-progress ledger lives in its own module; re-exported so the
// `undo::*` call sites and the existing tests keep their paths.
pub(super) use super::undo_progress::{
    init_undo_progress, mark_undo_progress_final, read_undo_progress, write_undo_progress,
};

/// Print generation metadata summary.
fn print_undo_meta(meta: &types::GenerationMeta) {
    println!("  target created: {}", meta.created_at);
    println!("  target action: {}", meta.action);
    if let Some(ref gr) = meta.git_ref {
        println!("  target git ref: {gr}");
    }
}

/// Compute resource diff for a single machine between current and target locks.
pub(super) fn diff_machine_locks(
    machine: &str,
    current_lock: Option<&types::StateLock>,
    target_lock: &types::StateLock,
) -> Vec<String> {
    let mut changes = Vec::new();
    for (rid, rl) in &target_lock.resources {
        match current_lock.and_then(|l| l.resources.get(rid)) {
            None => changes.push(format!("  + {rid} ({machine}): will be created")),
            Some(crl) if crl.hash != rl.hash => {
                changes.push(format!("  ~ {rid} ({machine}): will be updated"))
            }
            _ => {}
        }
    }
    if let Some(cl) = current_lock {
        for rid in cl
            .resources
            .keys()
            .filter(|r| !target_lock.resources.contains_key(*r))
        {
            changes.push(format!("  - {rid} ({machine}): will be destroyed"));
        }
    }
    changes
}

/// Every resource a machine currently holds, listed for destruction.
///
/// Reached when the TARGET generation has no lock for a machine that the live
/// state does. Returning to that generation means the machine holds nothing.
fn destroyed_machine_changes(machine: &str, current_lock: &types::StateLock) -> Vec<String> {
    current_lock
        .resources
        .keys()
        .map(|rid| format!("  - {rid} ({machine}): will be destroyed"))
        .collect()
}

/// Compute resource diff between current locks and target generation locks.
///
/// GH-376: iterates the UNION of machines. It used to iterate `target_locks`
/// alone, so a machine present in the live state but absent from the target
/// generation was never visited and its `-` destroy branch never ran. Its whole
/// resource set silently vanished from the diff, `changes` came back empty, and
/// `cmd_undo` returned Ok having done nothing — the loudest possible change
/// reported as no change at all.
pub(super) fn compute_undo_diff(
    current_locks: &std::collections::HashMap<String, types::StateLock>,
    target_locks: &std::collections::HashMap<String, types::StateLock>,
) -> Vec<String> {
    let machines: std::collections::BTreeSet<&str> = target_locks
        .keys()
        .chain(current_locks.keys())
        .map(String::as_str)
        .collect();
    machines
        .into_iter()
        .flat_map(|machine| match target_locks.get(machine) {
            Some(target_lock) => {
                diff_machine_locks(machine, current_locks.get(machine), target_lock)
            }
            None => current_locks
                .get(machine)
                .map(|cl| destroyed_machine_changes(machine, cl))
                .unwrap_or_default(),
        })
        .collect()
}

/// FJ-2003: Pre-flight SSH connectivity check for multi-machine undo.
///
/// Verifies all target machines are reachable before making any changes.
/// Returns Err if any machine is unreachable (fail fast).
pub(super) fn preflight_ssh_check(
    config: &types::ForjarConfig,
    machine_filter: Option<&str>,
) -> Result<(), String> {
    let machines: Vec<(&String, &types::Machine)> = config
        .machines
        .iter()
        .filter(|(name, _)| machine_filter.is_none_or(|f| name.as_str() == f))
        .collect();

    let mut unreachable = Vec::new();
    for (name, machine) in &machines {
        let is_local = machine.addr == "localhost"
            || machine.addr == "127.0.0.1"
            || machine.transport.as_deref() == Some("local");
        if is_local || machine.is_container_transport() {
            println!("  ✓ {name}: local/container (skip SSH)");
            continue;
        }
        let host = &machine.addr;
        let status = std::process::Command::new("ssh")
            .args([
                "-o",
                "ConnectTimeout=5",
                "-o",
                "BatchMode=yes",
                host,
                "true",
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        match status {
            Ok(s) if s.success() => println!("  ✓ {name}: {host} reachable"),
            _ => {
                eprintln!("  ✗ {name}: {host} unreachable");
                unreachable.push(name.as_str());
            }
        }
    }
    if unreachable.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "pre-flight failed: {} machine(s) unreachable: {}",
            unreachable.len(),
            unreachable.join(", ")
        ))
    }
}

/// Announce the generation transition and, when the target generation carries
/// readable metadata, summarise it.
fn print_undo_header(target_gen_dir: &Path, current: u32, target: u32) {
    let meta_content =
        std::fs::read_to_string(target_gen_dir.join(".generation.yaml")).unwrap_or_default();
    println!("Undo: generation {current} → {target}");
    if let Ok(meta) = types::GenerationMeta::from_yaml(&meta_content) {
        print_undo_meta(&meta);
    }
}

/// Explain why there is nothing to undo, naming the remedy that works for
/// THIS config.
///
/// `policy.snapshot_generations` is what makes `apply` record a generation
/// (`apply.rs::maybe_auto_snapshot` returns early without it). Unset, no
/// number of applies will ever produce one — so the old blanket advice to
/// "run `forjar apply` first" was, on a default config, the one instruction
/// that provably could not succeed.
fn nothing_recorded_error(config: &types::ForjarConfig, file: &Path, gen_dir: &Path) -> String {
    let dir = gen_dir.display();
    if config.policy.snapshot_generations.is_some_and(|n| n > 0) {
        // Generations are enabled, so applying really is the remedy.
        return format!("no generations found in {dir} — run `forjar apply` first");
    }
    let cfg = file.display();
    format!(
        "no generations found in {dir} — undo needs generation snapshots and {cfg} \
         does not enable them: without `policy.snapshot_generations`, `forjar apply` \
         records no generation, so re-applying can never make undo work. \
         Add `policy.snapshot_generations: 10` to {cfg}, then apply; \
         undo becomes available once a second generation is recorded"
    )
}

/// An empty diff has two very different causes, and returning Ok for both is
/// how `undo` reported success for a request it had not fulfilled.
///
/// A target generation that recorded no machine state cannot be described,
/// verified, or converged to — undo does not know what the host should hold, so
/// it says so. A target whose locks match the live locks genuinely IS the state
/// asked for, and that is a real success.
fn no_changes_outcome(target_gen_dir: &Path, current: u32, target: u32) -> Result<(), String> {
    if load_generation_locks(target_gen_dir, None).is_empty() {
        return Err(format!(
            "generation {target} records no machine state, so undo cannot tell what the \
             host should hold — it will not report success for a revert it did not \
             perform. Inspect it with `forjar generation diff --from {current} --to \
             {target}`, or pick a generation that has one from `forjar generation list`"
        ));
    }
    println!("\nAlready at generation {target}: every declared resource already matches it.");
    Ok(())
}

/// Load and validate the config that generation `target` recorded, staged beside
/// the operator's own config so its relative paths resolve unchanged.
fn stage_target_config(
    file: &Path,
    target: u32,
    body: &str,
) -> Result<(super::undo_replay::ReplayConfig, types::ForjarConfig), String> {
    let replay = super::undo_replay::ReplayConfig::stage(file, target, body)?;
    let config = parse_and_validate(replay.path())
        .map_err(|e| format!("generation {target}'s recorded config does not parse: {e}"))?;
    // GH-376: stop before the rollback rather than replay a config whose bytes
    // live in files the generation never captured. `replay` drops here, so the
    // staged file is cleaned up and the host is untouched.
    let offenders = super::undo_replay::unreplayable_resources(&config);
    if !offenders.is_empty() {
        return Err(super::undo_replay::unreplayable_error(target, &offenders));
    }
    Ok((replay, config))
}

/// FJ-2003 / GH-376: Active undo — return the host to a previous generation by
/// re-applying THAT generation's recorded config.
///
/// Re-applying the CURRENT config here (what this did through 1.22.0) converged
/// the host forwards to the state undo had just rolled the lock away from, so
/// `undo` exited 0, printed "1 converged", and changed nothing. `force` is on
/// during that re-apply, so the count is not evidence either way.
pub(crate) fn cmd_undo(
    file: &Path,
    state_dir: &Path,
    generations: u32,
    machine_filter: Option<&str>,
    dry_run: bool,
    yes: bool,
) -> Result<(), String> {
    // Parsed first: the refusal below has to know whether THIS config enables
    // generations at all before it can name a remedy that works.
    let current_config = parse_and_validate(file)?;
    // GH-377: before the generation arithmetic, whose "run `forjar apply` first"
    // advice would, followed against a foreign state dir, apply the wrong stack
    // for real. Above the dry run too — a diff read from another stack's
    // generations is not a preview of anything.
    super::state_identity::check_state_dir_owner("undo", &current_config, file, state_dir)?;

    let gen_dir = state_dir.join("generations");
    let current = super::generation::current_generation(&gen_dir)
        .ok_or_else(|| nothing_recorded_error(&current_config, file, &gen_dir))?;

    if current < generations {
        return Err(format!(
            "cannot undo {generations} generation(s): generation {current} is current, \
             so only {current} earlier generation(s) exist"
        ));
    }
    let target = current - generations;

    let target_gen_dir = gen_dir.join(target.to_string());
    if !target_gen_dir.exists() {
        return Err(format!("generation {target} does not exist"));
    }
    // GH-376: refuse BEFORE anything is printed or moved. Without the recorded
    // body there is no honest undo, and the fallback that used to fill the gap
    // is the defect itself.
    let target_body = super::undo_replay::load_snapshot(&target_gen_dir)
        .ok_or_else(|| super::undo_replay::no_snapshot_error(target, &target_gen_dir))?;

    print_undo_header(&target_gen_dir, current, target);

    let current_locks =
        super::helpers_state::load_machine_locks(&current_config, state_dir, machine_filter)
            .unwrap_or_default();
    let target_locks = load_generation_locks(&target_gen_dir, machine_filter);
    let changes = compute_undo_diff(&current_locks, &target_locks);

    if changes.is_empty() {
        return no_changes_outcome(&target_gen_dir, current, target);
    }
    println!("\nChanges ({} resource(s)):", changes.len());
    for c in &changes {
        println!("{c}");
    }

    if dry_run {
        println!("\nDry run: {} change(s) would be applied.", changes.len());
        return Ok(());
    }
    if !yes {
        return Err("undo requires --yes to confirm".to_string());
    }

    let (replay, target_config) = stage_target_config(file, target, &target_body)?;

    // Phase 1: Pre-flight SSH check against the machines of the config that is
    // about to be applied — the target generation's, not the current one's.
    println!("\nPre-flight check:");
    preflight_ssh_check(&target_config, machine_filter)?;

    // forjar#449: what the target does not hold is destroyed, with the current
    // config's definitions, before the lock is rolled away from under them.
    super::undo_prune::destroy_absent_from_target(&super::undo_prune::UndoPrune {
        file,
        state_dir,
        current,
        current_config: &current_config,
        current_locks: &current_locks,
        target_locks: &target_locks,
        machine_filter,
    })?;

    super::generation::rollback_to_generation(state_dir, target, true)?;

    // GH-376: written AFTER the rollback. `restore_generation_to_state` deletes
    // `state_dir/<machine>/` wholesale, so progress written before it was gone
    // by the time anything could fail and `undo --resume` could never find one.
    let progress = init_undo_progress(current, target, &changes);
    let affected: Vec<String> = target_locks
        .keys()
        .chain(current_locks.keys())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    for machine in &affected {
        write_undo_progress(state_dir, machine, &progress);
    }

    println!("\nRe-applying generation {target}'s recorded config to converge the host...");
    let result = replay_generation(replay.path(), state_dir, machine_filter);

    // Mark progress completed or partial
    mark_undo_progress_final(state_dir, affected.iter(), result.is_ok());
    result
}

/// Apply a generation's recorded config so the host converges to that
/// generation. `force` is on and confirmation is pre-granted; every other knob
/// is the `cmd_apply` default.
///
/// Generation recording is paused for the duration: `rollback_to_generation`
/// has already pointed `current` at the target, and appending a new generation
/// here would push `current` past the state the host is in, making a second
/// `undo` target what the first had just restored.
fn replay_generation(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
) -> Result<(), String> {
    let _paused = super::apply_snapshot::PauseGenerationRecording::new();
    cmd_apply(
        file,
        state_dir,
        machine_filter,
        None,
        None,
        None,
        true,
        false,
        false,
        &[],
        false,
        None,
        false,
        false,
        None,
        None,
        false,
        false,
        None,
        false,
        false,
        0,
        true,
        false,
        None,
        false,
        None,
        None,
        None,
        false,
        None,
        false,
        None,  // telemetry_endpoint
        false, // refresh
        None,  // force_tag
        &[],
    )
}

/// The generation a partial undo was heading for and the machines whose ledgers
/// say so, announcing each as it goes. Empty when nothing here needs resuming.
fn partial_undo_target(state_dir: &Path, machines: &[String]) -> (Option<u32>, Vec<String>) {
    let mut target = None;
    let mut resuming = Vec::new();
    for machine in machines {
        let Some(p) = read_undo_progress(state_dir, machine) else {
            continue;
        };
        if !p.needs_resume() {
            continue;
        }
        let pending = p.pending_count();
        let failed = p.failed_count();
        let done = p.completed_count();
        println!(
            "Resume {machine}: gen {} → {} ({done} done, {failed} failed, {pending} pending)",
            p.generation_from, p.generation_to
        );
        target = Some(p.generation_to);
        resuming.push(machine.clone());
    }
    (target, resuming)
}

/// FJ-2003 / GH-376: Resume a partial undo from undo-progress.yaml.
///
/// This replays the generation the interrupted undo was heading for, read from
/// the ledger. It used to re-apply the CURRENT config — the same defect as
/// `cmd_undo`, one flag away, and the one path that would have quietly undone
/// the fix for an operator whose first undo failed halfway.
pub(crate) fn cmd_undo_resume(
    file: &Path,
    state_dir: &Path,
    machine_filter: Option<&str>,
    dry_run: bool,
    yes: bool,
) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    // GH-377: `--resume` reads `state_dir/<machine>/undo-progress.yaml` for the
    // machines named in the CWD config, and machine names collide constantly
    // (`local`, `web`, `prod`). Unguarded it is the identical bypass.
    super::state_identity::check_state_dir_owner("undo --resume", &config, file, state_dir)?;
    let machines: Vec<String> = config
        .machines
        .keys()
        .filter(|&m| machine_filter.is_none_or(|f| m == f))
        .cloned()
        .collect();

    let (target, resuming) = partial_undo_target(state_dir, &machines);
    let Some(target) = target else {
        return Err("no partial undo found — nothing to resume".to_string());
    };
    if dry_run {
        println!("\nDry run: would resume partial undo.");
        return Ok(());
    }
    if !yes {
        return Err("undo --resume requires --yes to confirm".to_string());
    }

    let target_gen_dir = state_dir.join("generations").join(target.to_string());
    let body = super::undo_replay::load_snapshot(&target_gen_dir)
        .ok_or_else(|| super::undo_replay::no_snapshot_error(target, &target_gen_dir))?;
    let (replay, _) = stage_target_config(file, target, &body)?;

    println!("\nRe-applying generation {target}'s recorded config to complete the undo...");
    let result = replay_generation(replay.path(), state_dir, machine_filter);
    // Without this the ledger stays Partial after a successful resume and
    // `--resume` would find the same undo for ever. It was unreachable before
    // GH-376 only because the ledger never survived the rollback at all.
    mark_undo_progress_final(state_dir, resuming.iter(), result.is_ok());
    result
}

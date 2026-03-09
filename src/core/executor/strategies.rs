//! Machine-level execution strategies: sequential, parallel, rolling.

use super::*;

/// Sequential machine apply (default).
pub(crate) fn apply_machines_sequential(
    cfg: &ApplyConfig,
    target_machines: &[&String],
    localhost_machine: &Machine,
    plan: &ExecutionPlan,
    locks: &mut HashMap<String, StateLock>,
) -> Result<Vec<ApplyResult>, String> {
    let mut results = Vec::with_capacity(target_machines.len());
    for machine_name in target_machines {
        let machine = match cfg.config.machines.get(*machine_name) {
            Some(m) => m,
            None if *machine_name == "localhost" => localhost_machine,
            None => continue,
        };
        let result = apply_machine(cfg, machine_name, machine, plan, locks)?;
        results.push(result);
    }
    Ok(results)
}

/// Parallel machine apply (FJ-034) — uses std::thread::scope for zero-copy sharing.
pub(crate) fn apply_machines_parallel(
    cfg: &ApplyConfig,
    target_machines: &[&String],
    localhost_machine: &Machine,
    plan: &ExecutionPlan,
    locks: &mut HashMap<String, StateLock>,
) -> Result<Vec<ApplyResult>, String> {
    // Extract per-machine locks so each thread can take its own
    let lock_mutex = Mutex::new(std::mem::take(locks));
    let results_mutex: Mutex<Vec<Result<ApplyResult, String>>> = Mutex::new(Vec::new());

    std::thread::scope(|s| {
        for machine_name in target_machines {
            let machine = match cfg.config.machines.get(*machine_name) {
                Some(m) => m,
                None if *machine_name == "localhost" => localhost_machine,
                None => continue,
            };

            // Take this machine's lock out of the shared map
            let machine_lock = lock_mutex
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(machine_name.as_str());

            // Borrow the mutexes; move only per-thread owned data
            let lock_ref = &lock_mutex;
            let results_ref = &results_mutex;

            s.spawn(move || {
                let mut single_lock_map = HashMap::new();
                if let Some(l) = machine_lock {
                    single_lock_map.insert(machine_name.to_string(), l);
                }
                let result = apply_machine(cfg, machine_name, machine, plan, &mut single_lock_map);

                // Put the lock back
                if let Some((k, v)) = single_lock_map.into_iter().next() {
                    lock_ref
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(k, v);
                }

                results_ref
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(result);
            });
        }
    });

    // Restore locks
    *locks = lock_mutex.into_inner().unwrap_or_else(|e| e.into_inner());

    // Collect results, returning first error if any
    let mut all_results = Vec::new();
    for result in results_mutex
        .into_inner()
        .unwrap_or_else(|e| e.into_inner())
    {
        all_results.push(result?);
    }
    Ok(all_results)
}

/// FJ-222: Rolling deploy — apply machines in batches of `batch_size`.
/// Within each batch, machines run in parallel if `parallel_machines` is true.
/// After each batch, checks `max_fail_percentage` and aborts if exceeded.
pub(crate) fn apply_machines_rolling(
    cfg: &ApplyConfig,
    target_machines: &[&String],
    localhost_machine: &Machine,
    plan: &ExecutionPlan,
    locks: &mut HashMap<String, StateLock>,
    batch_size: usize,
) -> Result<Vec<ApplyResult>, String> {
    let mut all_results = Vec::new();
    let total_machines = target_machines.len();

    for batch in target_machines.chunks(batch_size) {
        let batch_results = if cfg.config.policy.parallel_machines && batch.len() > 1 {
            apply_machines_parallel(cfg, batch, localhost_machine, plan, locks)?
        } else {
            apply_machines_sequential(cfg, batch, localhost_machine, plan, locks)?
        };
        all_results.extend(batch_results);

        // FJ-222: Check max_fail_percentage after each batch
        if let Some(max_pct) = cfg.config.policy.max_fail_percentage {
            let failed = all_results
                .iter()
                .filter(|r| r.resources_failed > 0)
                .count();
            let pct = (failed as f64 / total_machines as f64 * 100.0) as u8;
            if pct > max_pct {
                return Err(format!(
                    "rolling deploy aborted: {pct}% failure rate exceeds max_fail_percentage {max_pct}%"
                ));
            }
        }
    }

    Ok(all_results)
}

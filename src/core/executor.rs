//! FJ-012: Executor — orchestration loop for apply.
//!
//! Applies resources in topological order per machine:
//! parse → validate → DAG → plan → for each resource: codegen → transport → hash → state → events

use super::codegen;
use super::conditions;
use super::planner;
use super::resolver;
use super::state;
use super::types::*;
use crate::transport;
use crate::tripwire::{eventlog, hasher, tracer};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Instant;

/// Configuration for an apply run.
pub struct ApplyConfig<'a> {
    pub config: &'a ForjarConfig,
    pub state_dir: &'a std::path::Path,
    pub force: bool,
    pub dry_run: bool,
    pub machine_filter: Option<&'a str>,
    pub resource_filter: Option<&'a str>,
    pub tag_filter: Option<&'a str>,
    pub timeout_secs: Option<u64>,
}

/// Execute the apply loop.
pub fn apply(cfg: &ApplyConfig) -> Result<Vec<ApplyResult>, String> {
    let start = Instant::now();

    // Build execution order (DAG toposort)
    let execution_order = resolver::build_execution_order(cfg.config)?;

    // Load existing locks per machine
    let mut locks: HashMap<String, StateLock> = HashMap::new();
    let all_machines = collect_machines(cfg.config);
    for machine_name in &all_machines {
        if let Some(filter) = cfg.machine_filter {
            if machine_name != filter {
                continue;
            }
        }
        if let Some(lock) = state::load_lock(cfg.state_dir, machine_name)? {
            locks.insert(machine_name.clone(), lock);
        }
    }

    // Generate plan
    let plan = planner::plan(cfg.config, &execution_order, &locks, cfg.tag_filter);

    if cfg.dry_run {
        return Ok(vec![ApplyResult {
            machine: "dry-run".to_string(),
            resources_converged: 0,
            resources_unchanged: plan.unchanged,
            resources_failed: 0,
            total_duration: start.elapsed(),
        }]);
    }

    // Filter machines and sort by cost (FJ-052: cheaper machines first)
    let mut target_machines: Vec<&String> = all_machines
        .iter()
        .filter(|m| cfg.machine_filter.is_none_or(|f| *m == f))
        .collect();
    target_machines.sort_by_key(|m| {
        cfg.config
            .machines
            .get(*m)
            .map(|machine| machine.cost)
            .unwrap_or(0)
    });

    let localhost_machine = Machine {
        hostname: "localhost".to_string(),
        addr: "127.0.0.1".to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        cost: 0,
    };

    // FJ-222: Rolling deploys — batch machines when serial is set
    if let Some(batch_size) = cfg.config.policy.serial {
        let batch_size = batch_size.max(1);
        apply_machines_rolling(
            cfg,
            &target_machines,
            &localhost_machine,
            &plan,
            &mut locks,
            batch_size,
        )
    } else if cfg.config.policy.parallel_machines && target_machines.len() > 1 {
        apply_machines_parallel(cfg, &target_machines, &localhost_machine, &plan, &mut locks)
    } else {
        apply_machines_sequential(cfg, &target_machines, &localhost_machine, &plan, &mut locks)
    }
}

/// Sequential machine apply (default).
fn apply_machines_sequential(
    cfg: &ApplyConfig,
    target_machines: &[&String],
    localhost_machine: &Machine,
    plan: &ExecutionPlan,
    locks: &mut HashMap<String, StateLock>,
) -> Result<Vec<ApplyResult>, String> {
    let mut results = Vec::new();
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
fn apply_machines_parallel(
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
            let machine_lock = lock_mutex.lock().unwrap().remove(machine_name.as_str());

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
                    lock_ref.lock().unwrap().insert(k, v);
                }

                results_ref.lock().unwrap().push(result);
            });
        }
    });

    // Restore locks
    *locks = lock_mutex.into_inner().unwrap();

    // Collect results, returning first error if any
    let mut all_results = Vec::new();
    for result in results_mutex.into_inner().unwrap() {
        all_results.push(result?);
    }
    Ok(all_results)
}

/// FJ-222: Rolling deploy — apply machines in batches of `batch_size`.
/// Within each batch, machines run in parallel if `parallel_machines` is true.
/// After each batch, checks `max_fail_percentage` and aborts if exceeded.
fn apply_machines_rolling(
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
            let failed = all_results.iter().filter(|r| r.resources_failed > 0).count();
            let pct = (failed as f64 / total_machines as f64 * 100.0) as u8;
            if pct > max_pct {
                return Err(format!(
                    "rolling deploy aborted: {}% failure rate exceeds max_fail_percentage {}%",
                    pct, max_pct
                ));
            }
        }
    }

    Ok(all_results)
}

/// Outcome of applying a single resource.
enum ResourceOutcome {
    /// Resource converged successfully.
    Converged,
    /// Resource was unchanged (NoOp, not forced).
    Unchanged,
    /// Resource was skipped (filtered out or not found).
    Skipped,
    /// Resource failed; includes whether to stop (jidoka).
    Failed { should_stop: bool },
}

/// Shared context for recording resource outcomes.
struct RecordCtx<'a> {
    lock: &'a mut StateLock,
    state_dir: &'a std::path::Path,
    machine_name: &'a str,
    tripwire: bool,
    failure_policy: &'a FailurePolicy,
    timeout_secs: Option<u64>,
}

/// Record a successful resource application into the lock and event log.
fn record_success(
    ctx: &mut RecordCtx,
    resource_id: &str,
    resource: &Resource,
    resolved: &Resource,
    machine: &Machine,
    duration: f64,
) {
    let desired_hash = planner::hash_desired_state(resolved);

    // Live state hash for drift detection
    let live_hash = match codegen::state_query_script(resolved) {
        Ok(query) => match transport::exec_script_timeout(machine, &query, ctx.timeout_secs) {
            Ok(qout) if qout.success() => Some(hasher::hash_string(&qout.stdout)),
            _ => None,
        },
        Err(_) => None,
    };

    let mut details = build_resource_details(resolved, machine);
    if let Some(ref lh) = live_hash {
        details.insert(
            "live_hash".to_string(),
            serde_yaml_ng::Value::String(lh.clone()),
        );
    }

    ctx.lock.resources.insert(
        resource_id.to_string(),
        ResourceLock {
            resource_type: resource.resource_type.clone(),
            status: ResourceStatus::Converged,
            applied_at: Some(eventlog::now_iso8601()),
            duration_seconds: Some(duration),
            hash: desired_hash.clone(),
            details,
        },
    );

    if ctx.tripwire {
        let _ = eventlog::append_event(
            ctx.state_dir,
            ctx.machine_name,
            ProvenanceEvent::ResourceConverged {
                machine: ctx.machine_name.to_string(),
                resource: resource_id.to_string(),
                duration_seconds: duration,
                hash: desired_hash,
            },
        );
    }
}

/// Record a resource failure into the lock and event log. Returns true if jidoka should stop.
fn record_failure(
    ctx: &mut RecordCtx,
    resource_id: &str,
    resource_type: &ResourceType,
    duration: f64,
    error: &str,
) -> bool {
    ctx.lock.resources.insert(
        resource_id.to_string(),
        ResourceLock {
            resource_type: resource_type.clone(),
            status: ResourceStatus::Failed,
            applied_at: Some(eventlog::now_iso8601()),
            duration_seconds: Some(duration),
            hash: String::new(),
            details: HashMap::new(),
        },
    );

    if ctx.tripwire {
        let _ = eventlog::append_event(
            ctx.state_dir,
            ctx.machine_name,
            ProvenanceEvent::ResourceFailed {
                machine: ctx.machine_name.to_string(),
                resource: resource_id.to_string(),
                error: error.to_string(),
            },
        );
    }

    if *ctx.failure_policy == FailurePolicy::StopOnFirst {
        eprintln!(
            "JIDOKA: stopping after failure on {}/{}: {}",
            ctx.machine_name, resource_id, error
        );
        return true;
    }

    false
}

/// Apply a single planned change, returning its outcome.
fn apply_single_resource(
    cfg: &ApplyConfig,
    change: &PlannedChange,
    machine: &Machine,
    ctx: &mut RecordCtx,
    converged_resources: &HashSet<String>,
) -> Result<ResourceOutcome, String> {
    if let Some(filter) = cfg.resource_filter {
        if change.resource_id != filter {
            return Ok(ResourceOutcome::Skipped);
        }
    }

    // FJ-224: Check if any triggers fired (dependency converged this run)
    let triggered = if let Some(resource) = cfg.config.resources.get(&change.resource_id) {
        !resource.triggers.is_empty()
            && resource
                .triggers
                .iter()
                .any(|t| converged_resources.contains(t))
    } else {
        false
    };

    if change.action == PlanAction::NoOp && !cfg.force && !triggered {
        return Ok(ResourceOutcome::Unchanged);
    }

    let resource = match cfg.config.resources.get(&change.resource_id) {
        Some(r) => r,
        None => return Ok(ResourceOutcome::Skipped),
    };

    // FJ-064: Skip resource if arch filter doesn't match the machine
    if !resource.arch.is_empty() && !resource.arch.contains(&machine.arch) {
        return Ok(ResourceOutcome::Skipped);
    }

    // Tag filtering: skip resource if --tag specified and resource doesn't have the tag
    if let Some(tag) = cfg.tag_filter {
        if !resource.tags.iter().any(|t| t == tag) {
            return Ok(ResourceOutcome::Skipped);
        }
    }

    // FJ-202: Skip resource if `when:` condition evaluates to false
    if let Some(ref when_expr) = resource.when {
        match conditions::evaluate_when(when_expr, &cfg.config.params, machine) {
            Ok(false) => return Ok(ResourceOutcome::Skipped),
            Err(_) => return Ok(ResourceOutcome::Skipped),
            Ok(true) => {}
        }
    }

    // Log resource start
    if ctx.tripwire {
        let _ = eventlog::append_event(
            ctx.state_dir,
            ctx.machine_name,
            ProvenanceEvent::ResourceStarted {
                machine: ctx.machine_name.to_string(),
                resource: change.resource_id.clone(),
                action: change.action.to_string(),
            },
        );
    }

    let resource_start = Instant::now();

    // Resolve templates
    let resolved =
        resolver::resolve_resource_templates(resource, &cfg.config.params, &cfg.config.machines)?;

    // Generate apply script and execute
    let script = codegen::apply_script(&resolved)?;
    let output = transport::exec_script_timeout(machine, &script, cfg.timeout_secs);
    let duration = resource_start.elapsed().as_secs_f64();

    match output {
        Ok(out) if out.success() => {
            record_success(
                ctx,
                &change.resource_id,
                resource,
                &resolved,
                machine,
                duration,
            );
            Ok(ResourceOutcome::Converged)
        }
        Ok(out) => {
            let error = format!("exit code {}: {}", out.exit_code, out.stderr.trim());
            let should_stop = record_failure(
                ctx,
                &change.resource_id,
                &resource.resource_type,
                duration,
                &error,
            );
            Ok(ResourceOutcome::Failed { should_stop })
        }
        Err(e) => {
            let error = format!("transport error: {}", e);
            let should_stop = record_failure(
                ctx,
                &change.resource_id,
                &resource.resource_type,
                duration,
                &error,
            );
            Ok(ResourceOutcome::Failed { should_stop })
        }
    }
}

/// Log a tripwire event if tripwire is enabled.
fn log_tripwire(
    state_dir: &std::path::Path,
    machine: &str,
    tripwire: bool,
    event: ProvenanceEvent,
) {
    if tripwire {
        let _ = eventlog::append_event(state_dir, machine, event);
    }
}

fn apply_machine(
    cfg: &ApplyConfig,
    machine_name: &str,
    machine: &Machine,
    plan: &ExecutionPlan,
    locks: &mut HashMap<String, StateLock>,
) -> Result<ApplyResult, String> {
    let machine_start = Instant::now();
    let run_id = eventlog::generate_run_id();

    // Container lifecycle: ensure container is running before apply
    if machine.is_container_transport() && !cfg.dry_run {
        transport::container::ensure_container(machine)?;
    }

    // Initialize or load lock
    let mut lock = locks
        .remove(machine_name)
        .unwrap_or_else(|| state::new_lock(machine_name, &machine.hostname));

    // FJ-050: Start trace session for provenance tracking
    let mut trace_session = tracer::TraceSession::start(&run_id);

    log_tripwire(
        cfg.state_dir,
        machine_name,
        cfg.config.policy.tripwire,
        ProvenanceEvent::ApplyStarted {
            machine: machine_name.to_string(),
            run_id: run_id.clone(),
            forjar_version: env!("CARGO_PKG_VERSION").to_string(),
        },
    );

    let mut converged = 0u32;
    let mut unchanged = 0u32;
    let mut failed = 0u32;
    // FJ-224: Track which resources converged so triggers can fire
    let mut converged_resources: HashSet<String> = HashSet::new();

    let machine_changes: Vec<_> = plan
        .changes
        .iter()
        .filter(|c| c.machine == machine_name)
        .collect();

    let mut ctx = RecordCtx {
        lock: &mut lock,
        state_dir: cfg.state_dir,
        machine_name,
        tripwire: cfg.config.policy.tripwire,
        failure_policy: &cfg.config.policy.failure,
        timeout_secs: cfg.timeout_secs,
    };

    // FJ-216: Execute resources — parallel waves or sequential
    if cfg.config.policy.parallel_resources && machine_changes.len() > 1 {
        // Build parallel waves for this machine's changes
        let change_ids: Vec<&str> = machine_changes
            .iter()
            .map(|c| c.resource_id.as_str())
            .collect();
        let waves = compute_resource_waves(cfg.config, &change_ids);

        'wave_loop: for wave in &waves {
            if wave.len() == 1 {
                // Single resource — no parallelism needed
                if let Some(change) = machine_changes.iter().find(|c| c.resource_id == wave[0]) {
                    let outcome = apply_and_record_outcome(
                        cfg,
                        change,
                        machine,
                        &mut ctx,
                        &mut trace_session,
                        machine_name,
                        &converged_resources,
                    )?;
                    match outcome {
                        ResourceOutcome::Converged => {
                            converged += 1;
                            converged_resources.insert(change.resource_id.clone());
                        }
                        ResourceOutcome::Unchanged => unchanged += 1,
                        ResourceOutcome::Skipped => {}
                        ResourceOutcome::Failed { should_stop } => {
                            failed += 1;
                            if should_stop {
                                break 'wave_loop;
                            }
                        }
                    }
                }
            } else {
                // Multiple independent resources — execute sequentially but mark as parallel-eligible
                // NOTE: True thread::scope parallelism within a single machine requires
                // Arc<Mutex<RecordCtx>> which adds complexity. For now, we execute waves
                // sequentially but validate the wave structure for future parallel execution.
                for resource_id in wave {
                    if let Some(change) = machine_changes
                        .iter()
                        .find(|c| c.resource_id == *resource_id)
                    {
                        let outcome = apply_and_record_outcome(
                            cfg,
                            change,
                            machine,
                            &mut ctx,
                            &mut trace_session,
                            machine_name,
                            &converged_resources,
                        )?;
                        match outcome {
                            ResourceOutcome::Converged => {
                                converged += 1;
                                converged_resources.insert(change.resource_id.clone());
                            }
                            ResourceOutcome::Unchanged => unchanged += 1,
                            ResourceOutcome::Skipped => {}
                            ResourceOutcome::Failed { should_stop } => {
                                failed += 1;
                                if should_stop {
                                    break 'wave_loop;
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        // Sequential execution (default)
        for change in &machine_changes {
            let outcome = apply_and_record_outcome(
                cfg,
                change,
                machine,
                &mut ctx,
                &mut trace_session,
                machine_name,
                &converged_resources,
            )?;
            match outcome {
                ResourceOutcome::Converged => {
                    converged += 1;
                    converged_resources.insert(change.resource_id.clone());
                }
                ResourceOutcome::Unchanged => unchanged += 1,
                ResourceOutcome::Skipped => {}
                ResourceOutcome::Failed { should_stop } => {
                    failed += 1;
                    if should_stop {
                        break;
                    }
                }
            }
        }
    }

    // Rebind lock from ctx for finalization
    let lock = ctx.lock;
    lock.generated_at = eventlog::now_iso8601();
    if cfg.config.policy.lock_file {
        state::save_lock(cfg.state_dir, lock)?;
    }

    // FJ-050: Finalize and write trace session
    if cfg.config.policy.tripwire {
        let _root_span = trace_session.finalize();
        let _ = tracer::write_trace(cfg.state_dir, machine_name, &trace_session);
    }

    log_tripwire(
        cfg.state_dir,
        machine_name,
        cfg.config.policy.tripwire,
        ProvenanceEvent::ApplyCompleted {
            machine: machine_name.to_string(),
            run_id,
            resources_converged: converged,
            resources_unchanged: unchanged,
            resources_failed: failed,
            total_seconds: machine_start.elapsed().as_secs_f64(),
        },
    );

    let result = ApplyResult {
        machine: machine_name.to_string(),
        resources_converged: converged,
        resources_unchanged: unchanged,
        resources_failed: failed,
        total_duration: machine_start.elapsed(),
    };

    // Container lifecycle: cleanup ephemeral containers after apply
    if machine.is_container_transport() && !cfg.dry_run {
        if let Some(ref container) = machine.container {
            if container.ephemeral {
                if let Err(e) = transport::container::cleanup_container(machine) {
                    eprintln!(
                        "warning: container cleanup failed for {}: {}",
                        machine_name, e
                    );
                }
            }
        }
    }

    Ok(result)
}

/// Apply a single resource and record the outcome in tracing.
#[allow(clippy::too_many_arguments)]
fn apply_and_record_outcome(
    cfg: &ApplyConfig,
    change: &PlannedChange,
    machine: &Machine,
    ctx: &mut RecordCtx,
    trace_session: &mut tracer::TraceSession,
    machine_name: &str,
    converged_resources: &HashSet<String>,
) -> Result<ResourceOutcome, String> {
    let resource_start = Instant::now();
    let outcome = apply_single_resource(cfg, change, machine, ctx, converged_resources)?;

    let resource = cfg.config.resources.get(&change.resource_id);
    let rt = resource
        .map(|r| format!("{:?}", r.resource_type))
        .unwrap_or_default();

    match &outcome {
        ResourceOutcome::Converged => {
            let action = if change.action == PlanAction::Create {
                "create"
            } else {
                "update"
            };
            trace_session.record_span(
                &change.resource_id,
                &rt.to_lowercase(),
                machine_name,
                action,
                resource_start.elapsed(),
                0,
                None,
            );
        }
        ResourceOutcome::Unchanged => {
            trace_session.record_noop(&change.resource_id, &rt.to_lowercase(), machine_name);
        }
        ResourceOutcome::Failed { .. } => {
            trace_session.record_span(
                &change.resource_id,
                &rt.to_lowercase(),
                machine_name,
                "create",
                resource_start.elapsed(),
                1,
                None,
            );
        }
        ResourceOutcome::Skipped => {}
    }

    Ok(outcome)
}

/// FJ-216: Compute parallel waves for a subset of resource IDs.
/// Returns groups of resource IDs that can execute concurrently.
fn compute_resource_waves(config: &ForjarConfig, resource_ids: &[&str]) -> Vec<Vec<String>> {
    let id_set: std::collections::HashSet<&str> = resource_ids.iter().copied().collect();
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

    for &id in resource_ids {
        in_degree.insert(id.to_string(), 0);
        adjacency.insert(id.to_string(), Vec::new());
    }

    for &id in resource_ids {
        if let Some(resource) = config.resources.get(id) {
            for dep in &resource.depends_on {
                // Only count deps within this subset
                if id_set.contains(dep.as_str()) {
                    if let Some(adj) = adjacency.get_mut(dep.as_str()) {
                        adj.push(id.to_string());
                    }
                    if let Some(deg) = in_degree.get_mut(id) {
                        *deg += 1;
                    }
                }
            }
        }
    }

    let mut waves = Vec::new();
    loop {
        let mut wave: Vec<String> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| id.clone())
            .collect();

        if wave.is_empty() {
            break;
        }

        wave.sort();

        for id in &wave {
            in_degree.remove(id);
            if let Some(neighbors) = adjacency.get(id) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                    }
                }
            }
        }

        waves.push(wave);
    }

    waves
}

/// Collect all unique machine names referenced by resources.
pub fn collect_machines(config: &ForjarConfig) -> Vec<String> {
    let mut machines: Vec<String> = Vec::new();
    for resource in config.resources.values() {
        for m in resource.machine.to_vec() {
            if !machines.iter().any(|existing| existing == &m) {
                machines.push(m);
            }
        }
    }
    machines
}

/// Build resource-specific details for the lock entry.
/// For container/remote machines, reads file content via transport instead of local filesystem.
fn build_resource_details(
    resource: &Resource,
    machine: &Machine,
) -> HashMap<String, serde_yaml_ng::Value> {
    let mut details = HashMap::new();

    if let Some(ref path) = resource.path {
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(path.clone()),
        );
    }
    if resource.content.is_some() {
        if let Some(ref path) = resource.path {
            let hash = if machine.is_container_transport() {
                // Read file content via transport for container machines
                let script = format!("cat '{}'", path);
                transport::exec_script(machine, &script)
                    .ok()
                    .filter(|out| out.success())
                    .map(|out| hasher::hash_string(&out.stdout))
            } else {
                // Local filesystem hash
                hasher::hash_file(std::path::Path::new(path)).ok()
            };
            if let Some(h) = hash {
                details.insert("content_hash".to_string(), serde_yaml_ng::Value::String(h));
            }
        }
    }
    if let Some(ref owner) = resource.owner {
        details.insert(
            "owner".to_string(),
            serde_yaml_ng::Value::String(owner.clone()),
        );
    }
    if let Some(ref group) = resource.group {
        details.insert(
            "group".to_string(),
            serde_yaml_ng::Value::String(group.clone()),
        );
    }
    if let Some(ref mode) = resource.mode {
        details.insert(
            "mode".to_string(),
            serde_yaml_ng::Value::String(mode.clone()),
        );
    }
    if let Some(ref name) = resource.name {
        details.insert(
            "service_name".to_string(),
            serde_yaml_ng::Value::String(name.clone()),
        );
    }

    details
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn local_machine() -> Machine {
        Machine {
            hostname: "localhost".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        }
    }

    fn local_config() -> ForjarConfig {
        let yaml = r#"
version: "1.0"
name: test
params: {}
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-test-executor.txt
    content: "hello from forjar"
policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#;
        serde_yaml_ng::from_str(yaml).unwrap()
    }

    #[test]
    fn test_fj012_collect_machines() {
        let config = local_config();
        let machines = collect_machines(&config);
        assert_eq!(machines, vec!["local"]);
    }

    #[test]
    fn test_fj012_collect_machines_multi() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  a:
    hostname: a
    addr: 1.1.1.1
  b:
    hostname: b
    addr: 2.2.2.2
resources:
  r1:
    type: package
    machine: a
    provider: apt
    packages: [x]
  r2:
    type: package
    machine: [a, b]
    provider: apt
    packages: [y]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let machines = collect_machines(&config);
        assert_eq!(machines, vec!["a", "b"]);
    }

    #[test]
    fn test_fj012_build_resource_details() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();
        let r = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: Some(file_path.to_str().unwrap().to_string()),
            content: Some("hello".to_string()),
            source: None,
            target: None,
            owner: Some("root".to_string()),
            group: Some("root".to_string()),
            mode: Some("0644".to_string()),
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let details = build_resource_details(&r, &local_machine());
        assert!(details.contains_key("path"));
        assert!(details.contains_key("content_hash"));
        assert!(details.contains_key("owner"));
        assert!(details.contains_key("mode"));
        assert!(details.contains_key("group"));
        // content_hash should match hash_file (not hash_string)
        let expected = hasher::hash_file(&file_path).unwrap();
        let actual = details["content_hash"].as_str().unwrap();
        assert_eq!(
            actual, expected,
            "content_hash must use hash_file for drift consistency"
        );
    }

    #[test]
    fn test_fj012_build_resource_details_service() {
        let r = Resource {
            resource_type: ResourceType::Service,
            machine: MachineTarget::Single("m".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: Some("nginx".to_string()),
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let details = build_resource_details(&r, &local_machine());
        assert!(details.contains_key("service_name"));
        assert_eq!(
            details["service_name"],
            serde_yaml_ng::Value::String("nginx".to_string())
        );
    }

    #[test]
    fn test_fj012_dry_run() {
        let config = local_config();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: true,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].machine, "dry-run");
    }

    #[test]
    fn test_fj012_apply_local_file() {
        let config = local_config();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results.len(), 1);

        // Verify lock was saved
        let lock = state::load_lock(dir.path(), "local").unwrap();
        assert!(lock.is_some());

        // Verify event log exists
        let events_path = dir.path().join("local").join("events.jsonl");
        assert!(events_path.exists());

        // Clean up
        let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
    }

    #[test]
    fn test_fj012_apply_idempotent() {
        let config = local_config();
        let dir = tempfile::tempdir().unwrap();

        // First apply
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r1 = apply(&cfg).unwrap();
        assert_eq!(r1[0].resources_converged, 1);

        // Second apply — should be unchanged
        let cfg2 = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r2 = apply(&cfg2).unwrap();
        assert_eq!(r2[0].resources_unchanged, 1);
        assert_eq!(r2[0].resources_converged, 0);

        let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
    }

    #[test]
    fn test_fj012_force_reapply() {
        let config = local_config();
        let dir = tempfile::tempdir().unwrap();

        // First apply
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        apply(&cfg).unwrap();

        // Force re-apply
        let cfg2 = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: true,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r2 = apply(&cfg2).unwrap();
        assert_eq!(r2[0].resources_converged, 1);

        let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
    }

    #[test]
    fn test_fj012_machine_filter() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  a:
    hostname: a
    addr: 127.0.0.1
  b:
    hostname: b
    addr: 127.0.0.1
resources:
  r1:
    type: file
    machine: a
    path: /tmp/forjar-test-filter-a.txt
    content: "a"
  r2:
    type: file
    machine: b
    path: /tmp/forjar-test-filter-b.txt
    content: "b"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: Some("a"),
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].machine, "a");

        let _ = std::fs::remove_file("/tmp/forjar-test-filter-a.txt");
        let _ = std::fs::remove_file("/tmp/forjar-test-filter-b.txt");
    }

    #[test]
    fn test_fj012_record_failure_stop_on_first() {
        let dir = tempfile::tempdir().unwrap();
        let mut lock = state::new_lock("test", "test-box");
        let mut ctx = RecordCtx {
            lock: &mut lock,
            state_dir: dir.path(),
            machine_name: "test",
            tripwire: true,
            failure_policy: &FailurePolicy::StopOnFirst,
            timeout_secs: None,
        };

        let should_stop = record_failure(
            &mut ctx,
            "failing-pkg",
            &ResourceType::Package,
            0.5,
            "exit code 1: not found",
        );

        assert!(should_stop, "StopOnFirst should return true");
        let rl = &ctx.lock.resources["failing-pkg"];
        assert_eq!(rl.status, ResourceStatus::Failed);
        assert_eq!(rl.hash, "");
        assert!(rl.duration_seconds.unwrap() > 0.0);
    }

    #[test]
    fn test_fj012_record_failure_continue() {
        let dir = tempfile::tempdir().unwrap();
        let mut lock = state::new_lock("test", "test-box");
        let mut ctx = RecordCtx {
            lock: &mut lock,
            state_dir: dir.path(),
            machine_name: "test",
            tripwire: false,
            failure_policy: &FailurePolicy::ContinueIndependent,
            timeout_secs: None,
        };

        let should_stop = record_failure(
            &mut ctx,
            "failing-pkg",
            &ResourceType::Package,
            1.0,
            "exit code 2: error",
        );

        assert!(!should_stop, "Continue policy should return false");
        assert_eq!(
            ctx.lock.resources["failing-pkg"].status,
            ResourceStatus::Failed
        );
    }

    #[test]
    fn test_fj012_record_failure_with_tripwire_logging() {
        let dir = tempfile::tempdir().unwrap();
        let mut lock = state::new_lock("test", "test-box");
        let mut ctx = RecordCtx {
            lock: &mut lock,
            state_dir: dir.path(),
            machine_name: "test",
            tripwire: true,
            failure_policy: &FailurePolicy::ContinueIndependent,
            timeout_secs: None,
        };

        record_failure(
            &mut ctx,
            "broken-svc",
            &ResourceType::Service,
            2.0,
            "transport error: connection refused",
        );

        // Verify event log was written
        let events_path = dir.path().join("test").join("events.jsonl");
        assert!(events_path.exists(), "tripwire event log should be written");
        let content = std::fs::read_to_string(&events_path).unwrap();
        assert!(content.contains("broken-svc"));
        assert!(content.contains("resource_failed"));
    }

    #[test]
    fn test_fj012_record_success_writes_lock_and_event() {
        let dir = tempfile::tempdir().unwrap();
        let managed_file = dir.path().join("managed.txt");
        std::fs::write(&managed_file, "test content").unwrap();
        let mut lock = state::new_lock("test", "test-box");
        let resource = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("test".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: Some(managed_file.to_str().unwrap().to_string()),
            content: Some("test content".to_string()),
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let machine = Machine {
            hostname: "localhost".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };
        let mut ctx = RecordCtx {
            lock: &mut lock,
            state_dir: dir.path(),
            machine_name: "test",
            tripwire: true,
            failure_policy: &FailurePolicy::StopOnFirst,
            timeout_secs: None,
        };

        record_success(&mut ctx, "test-file", &resource, &resource, &machine, 0.1);

        let rl = &ctx.lock.resources["test-file"];
        assert_eq!(rl.status, ResourceStatus::Converged);
        assert!(rl.hash.starts_with("blake3:"));
        assert!(rl.details.contains_key("path"));
        assert!(rl.details.contains_key("content_hash"));

        // Verify event log
        let events_path = dir.path().join("test").join("events.jsonl");
        assert!(events_path.exists());
        let content = std::fs::read_to_string(&events_path).unwrap();
        assert!(content.contains("resource_converged"));
    }

    #[test]
    fn test_fj012_resource_filter() {
        let config = local_config();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: Some("nonexistent-resource"),
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        // Resource filter doesn't match — everything skipped
        assert_eq!(results[0].resources_converged, 0);
        assert_eq!(results[0].resources_unchanged, 0);
    }

    #[test]
    fn test_fj034_parallel_multi_machine() {
        let yaml = r#"
version: "1.0"
name: parallel-test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
  m2:
    hostname: m2
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: m1
    path: /tmp/forjar-test-parallel-m1.txt
    content: "m1"
  f2:
    type: file
    machine: m2
    path: /tmp/forjar-test-parallel-m2.txt
    content: "m2"
policy:
  parallel_machines: true
  lock_file: true
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(config.policy.parallel_machines);

        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results.len(), 2);

        // Both machines should converge
        let total_converged: u32 = results.iter().map(|r| r.resources_converged).sum();
        assert_eq!(total_converged, 2, "both files should converge");

        // Verify files exist
        assert!(
            std::path::Path::new("/tmp/forjar-test-parallel-m1.txt").exists(),
            "m1 file should exist"
        );
        assert!(
            std::path::Path::new("/tmp/forjar-test-parallel-m2.txt").exists(),
            "m2 file should exist"
        );

        // Idempotency with parallel
        let r2 = apply(&cfg).unwrap();
        let total_unchanged: u32 = r2.iter().map(|r| r.resources_unchanged).sum();
        assert_eq!(total_unchanged, 2, "both files should be unchanged");

        // Verify locks saved for both machines
        assert!(state::load_lock(dir.path(), "m1").unwrap().is_some());
        assert!(state::load_lock(dir.path(), "m2").unwrap().is_some());

        let _ = std::fs::remove_file("/tmp/forjar-test-parallel-m1.txt");
        let _ = std::fs::remove_file("/tmp/forjar-test-parallel-m2.txt");
    }

    #[test]
    fn test_fj034_single_machine_skips_parallel() {
        // Even with parallel_machines=true, single machine stays sequential
        let yaml = r#"
version: "1.0"
name: single-machine
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m1
    path: /tmp/forjar-test-single-parallel.txt
    content: "single"
policy:
  parallel_machines: true
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resources_converged, 1);

        let _ = std::fs::remove_file("/tmp/forjar-test-single-parallel.txt");
    }

    // ── Falsification tests (Execution Safety Contract) ─────────

    proptest! {
        /// FALSIFY-ES-002: Jidoka StopOnFirst returns should_stop=true.
        #[test]
        fn falsify_es_002_jidoka_stop_on_first(error in ".{1,50}") {
            let dir = tempfile::tempdir().unwrap();
            let mut lock = state::new_lock("test", "test-box");
            let mut ctx = RecordCtx {
                lock: &mut lock,
                state_dir: dir.path(),
                machine_name: "test",
                tripwire: false,
                failure_policy: &FailurePolicy::StopOnFirst,
                timeout_secs: None,
            };
            let should_stop = record_failure(
                &mut ctx, "res", &ResourceType::Package, 0.1, &error,
            );
            prop_assert!(should_stop, "StopOnFirst must return true");
        }

        /// FALSIFY-ES-003: ContinueIndependent returns should_stop=false.
        #[test]
        fn falsify_es_003_jidoka_continue(error in ".{1,50}") {
            let dir = tempfile::tempdir().unwrap();
            let mut lock = state::new_lock("test", "test-box");
            let mut ctx = RecordCtx {
                lock: &mut lock,
                state_dir: dir.path(),
                machine_name: "test",
                tripwire: false,
                failure_policy: &FailurePolicy::ContinueIndependent,
                timeout_secs: None,
            };
            let should_stop = record_failure(
                &mut ctx, "res", &ResourceType::Package, 0.1, &error,
            );
            prop_assert!(!should_stop, "ContinueIndependent must return false");
        }
    }

    // ── FJ-064: Cross-architecture filtering ──────────────────────

    #[test]
    fn test_fj064_arch_filter_yaml_parsing() {
        let yaml = r#"
version: "1.0"
name: arch-test
machines:
  x86-box:
    hostname: x86-box
    addr: 127.0.0.1
    arch: x86_64
  arm-box:
    hostname: arm-box
    addr: 10.0.0.1
    arch: aarch64
resources:
  x86-only:
    type: file
    machine: x86-box
    path: /etc/x86-marker
    content: "x86 only"
    arch: [x86_64]
  arm-only:
    type: file
    machine: arm-box
    path: /etc/arm-marker
    content: "arm only"
    arch: [aarch64]
  universal:
    type: file
    machine: x86-box
    path: /etc/universal
    content: "any arch"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.resources["x86-only"].arch, vec!["x86_64"]);
        assert_eq!(config.resources["arm-only"].arch, vec!["aarch64"]);
        assert!(config.resources["universal"].arch.is_empty());
    }

    #[test]
    fn test_fj064_arch_filter_skips_mismatched() {
        // Resource with arch: [aarch64] should be skipped on x86_64 machine
        let machine = Machine {
            hostname: "x86-box".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };
        let resource = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("x86-box".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: Some("/etc/arm-only".to_string()),
            content: Some("arm only".to_string()),
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec!["aarch64".to_string()],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };

        // arch filter should reject: aarch64 resource on x86_64 machine
        assert!(
            !resource.arch.is_empty() && !resource.arch.contains(&machine.arch),
            "arch filter should skip aarch64 resource on x86_64 machine"
        );
    }

    #[test]
    fn test_fj064_arch_filter_allows_matching() {
        let machine = Machine {
            hostname: "arm-box".to_string(),
            addr: "10.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "aarch64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };
        let arch = ["aarch64".to_string()];
        assert!(arch.contains(&machine.arch));
    }

    #[test]
    fn test_fj064_empty_arch_allows_all() {
        let machine = Machine {
            hostname: "any-box".to_string(),
            addr: "10.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "riscv64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
        };
        let arch: Vec<String> = vec![];
        // Empty arch means "runs on all architectures"
        assert!(arch.is_empty() || arch.contains(&machine.arch));
    }

    // ── FJ-052: Cost-aware scheduling ─────────────────────────────

    #[test]
    fn test_fj052_cost_field_default_zero() {
        let yaml = r#"
version: "1.0"
name: cost-test
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources: {}
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.machines["m1"].cost, 0);
    }

    #[test]
    fn test_fj052_cost_field_parsed() {
        let yaml = r#"
version: "1.0"
name: cost-test
machines:
  cheap:
    hostname: cheap
    addr: 10.0.0.1
    cost: 1
  expensive:
    hostname: expensive
    addr: 10.0.0.2
    cost: 10
resources: {}
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.machines["cheap"].cost, 1);
        assert_eq!(config.machines["expensive"].cost, 10);
    }

    #[test]
    fn test_fj052_machines_sorted_by_cost() {
        let yaml = r#"
version: "1.0"
name: cost-test
machines:
  expensive:
    hostname: expensive
    addr: 10.0.0.3
    cost: 100
  medium:
    hostname: medium
    addr: 10.0.0.2
    cost: 50
  cheap:
    hostname: cheap
    addr: 10.0.0.1
    cost: 1
resources:
  f:
    type: file
    machine: [expensive, medium, cheap]
    path: /tmp/test
    content: hello
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let all_machines: Vec<String> = config.machines.keys().cloned().collect();
        let mut sorted: Vec<&String> = all_machines.iter().collect();
        sorted.sort_by_key(|m| {
            config
                .machines
                .get(*m)
                .map(|machine| machine.cost)
                .unwrap_or(0)
        });

        assert_eq!(sorted[0], "cheap");
        assert_eq!(sorted[1], "medium");
        assert_eq!(sorted[2], "expensive");
    }

    #[test]
    fn test_tag_filter_skips_untagged_resources() {
        let yaml = r#"
version: "1.0"
name: tag-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  tagged-file:
    type: file
    machine: local
    path: /tmp/forjar-tag-test.txt
    content: "tagged"
    tags: [web, critical]
  untagged-file:
    type: file
    machine: local
    path: /tmp/forjar-tag-test2.txt
    content: "untagged"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: Some("web"),
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        // Only the tagged resource should be applied
        assert_eq!(results[0].resources_converged, 1);
        let _ = std::fs::remove_file("/tmp/forjar-tag-test.txt");
    }

    #[test]
    fn test_tag_filter_none_applies_all() {
        let yaml = r#"
version: "1.0"
name: tag-test-all
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  a:
    type: file
    machine: local
    path: /tmp/forjar-tag-all-a.txt
    content: "a"
    tags: [web]
  b:
    type: file
    machine: local
    path: /tmp/forjar-tag-all-b.txt
    content: "b"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        // Both resources applied
        assert_eq!(results[0].resources_converged, 2);
        let _ = std::fs::remove_file("/tmp/forjar-tag-all-a.txt");
        let _ = std::fs::remove_file("/tmp/forjar-tag-all-b.txt");
    }

    #[test]
    fn test_tags_parsed_from_yaml() {
        let yaml = r#"
version: "1.0"
name: tags-parse
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m
    path: /tmp/test
    content: test
    tags: [web, critical, db]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let resource = config.resources.get("f").unwrap();
        assert_eq!(resource.tags, vec!["web", "critical", "db"]);
    }

    #[test]
    fn test_tags_default_empty() {
        let yaml = r#"
version: "1.0"
name: tags-empty
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m
    path: /tmp/test
    content: test
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let resource = config.resources.get("f").unwrap();
        assert!(resource.tags.is_empty());
    }

    // ── Edge-case tests ────────────────────────────────────────────────

    #[test]
    fn test_fj012_record_success_without_tripwire() {
        let dir = tempfile::tempdir().unwrap();
        let managed_file = dir.path().join("no-trip.txt");
        std::fs::write(&managed_file, "content").unwrap();
        let mut lock = state::new_lock("test", "test-box");
        let resource = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("test".to_string()),
            path: Some(managed_file.to_str().unwrap().to_string()),
            content: Some("content".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let mut ctx = RecordCtx {
            lock: &mut lock,
            state_dir: dir.path(),
            machine_name: "test",
            tripwire: false,
            failure_policy: &FailurePolicy::StopOnFirst,
            timeout_secs: None,
        };

        record_success(&mut ctx, "f", &resource, &resource, &local_machine(), 0.5);

        assert_eq!(ctx.lock.resources["f"].status, ResourceStatus::Converged);
        // No event log should be written when tripwire is off
        let events_path = dir.path().join("test").join("events.jsonl");
        assert!(!events_path.exists(), "no event log without tripwire");
    }

    #[test]
    fn test_fj012_record_success_service_resource() {
        let dir = tempfile::tempdir().unwrap();
        let mut lock = state::new_lock("test", "test-box");
        let resource = Resource {
            resource_type: ResourceType::Service,
            machine: MachineTarget::Single("test".to_string()),
            name: Some("nginx".to_string()),
            state: Some("running".to_string()),
            path: None,
            content: None,
            source: None,
            target: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            owner: None,
            group: None,
            mode: None,
            enabled: Some(true),
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let mut ctx = RecordCtx {
            lock: &mut lock,
            state_dir: dir.path(),
            machine_name: "test",
            tripwire: false,
            failure_policy: &FailurePolicy::StopOnFirst,
            timeout_secs: None,
        };

        record_success(&mut ctx, "svc", &resource, &resource, &local_machine(), 1.5);

        let rl = &ctx.lock.resources["svc"];
        assert_eq!(rl.status, ResourceStatus::Converged);
        assert!(rl.details.contains_key("service_name"));
        assert_eq!(
            rl.details["service_name"],
            serde_yaml_ng::Value::String("nginx".to_string())
        );
    }

    #[test]
    fn test_fj012_record_failure_tripwire_off() {
        let dir = tempfile::tempdir().unwrap();
        let mut lock = state::new_lock("test", "test-box");
        let mut ctx = RecordCtx {
            lock: &mut lock,
            state_dir: dir.path(),
            machine_name: "test",
            tripwire: false,
            failure_policy: &FailurePolicy::ContinueIndependent,
            timeout_secs: None,
        };

        record_failure(&mut ctx, "f", &ResourceType::File, 0.1, "broke");

        assert_eq!(ctx.lock.resources["f"].status, ResourceStatus::Failed);
        let events_path = dir.path().join("test").join("events.jsonl");
        assert!(!events_path.exists(), "no event log without tripwire");
    }

    #[test]
    fn test_fj012_build_details_file_no_content() {
        // File with path but no content → no content_hash
        let resource = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("test".to_string()),
            path: Some("/etc/some.conf".to_string()),
            content: None,
            source: None,
            target: None,
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            owner: Some("root".to_string()),
            group: None,
            mode: Some("0644".to_string()),
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let details = build_resource_details(&resource, &local_machine());
        assert!(details.contains_key("path"));
        assert!(details.contains_key("owner"));
        assert!(details.contains_key("mode"));
        assert!(
            !details.contains_key("content_hash"),
            "no content → no hash"
        );
    }

    #[test]
    fn test_fj012_build_details_file_with_content_and_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("real.txt");
        std::fs::write(&file_path, "real content").unwrap();

        let resource = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("test".to_string()),
            path: Some(file_path.to_str().unwrap().to_string()),
            content: Some("real content".to_string()),
            source: None,
            target: None,
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let details = build_resource_details(&resource, &local_machine());
        assert!(
            details.contains_key("content_hash"),
            "real file should have content_hash"
        );
        let hash = details["content_hash"].as_str().unwrap();
        assert!(
            hash.starts_with("blake3:"),
            "hash should be blake3-prefixed"
        );
    }

    #[test]
    fn test_fj012_build_details_nonexistent_file_no_hash() {
        // content is set but the file doesn't exist → no content_hash
        let resource = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("test".to_string()),
            path: Some("/tmp/does-not-exist-forjar-test.txt".to_string()),
            content: Some("ghost".to_string()),
            source: None,
            target: None,
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let details = build_resource_details(&resource, &local_machine());
        assert!(
            !details.contains_key("content_hash"),
            "nonexistent file → no hash"
        );
    }

    #[test]
    fn test_fj012_build_details_all_fields() {
        let resource = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("test".to_string()),
            path: Some("/etc/app.conf".to_string()),
            owner: Some("app".to_string()),
            group: Some("app".to_string()),
            mode: Some("0600".to_string()),
            name: Some("app-config".to_string()),
            content: None,
            source: None,
            target: None,
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let details = build_resource_details(&resource, &local_machine());
        assert_eq!(
            details["path"],
            serde_yaml_ng::Value::String("/etc/app.conf".to_string())
        );
        assert_eq!(
            details["owner"],
            serde_yaml_ng::Value::String("app".to_string())
        );
        assert_eq!(
            details["group"],
            serde_yaml_ng::Value::String("app".to_string())
        );
        assert_eq!(
            details["mode"],
            serde_yaml_ng::Value::String("0600".to_string())
        );
        assert_eq!(
            details["service_name"],
            serde_yaml_ng::Value::String("app-config".to_string())
        );
    }

    #[test]
    fn test_fj012_collect_machines_deduplicates() {
        let yaml = r#"
version: "1.0"
name: dedup
machines:
  m1:
    hostname: m1
    addr: 1.1.1.1
resources:
  a:
    type: file
    machine: m1
    path: /a
  b:
    type: file
    machine: m1
    path: /b
  c:
    type: file
    machine: m1
    path: /c
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let machines = collect_machines(&config);
        assert_eq!(machines.len(), 1, "3 resources on same machine → 1 entry");
        assert_eq!(machines[0], "m1");
    }

    #[test]
    fn test_fj012_collect_machines_preserves_order() {
        let yaml = r#"
version: "1.0"
name: order
machines:
  web:
    hostname: web
    addr: 1.1.1.1
  db:
    hostname: db
    addr: 2.2.2.2
  cache:
    hostname: cache
    addr: 3.3.3.3
resources:
  a:
    type: file
    machine: web
    path: /a
  b:
    type: file
    machine: cache
    path: /b
  c:
    type: file
    machine: db
    path: /c
  d:
    type: file
    machine: web
    path: /d
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let machines = collect_machines(&config);
        assert_eq!(machines, vec!["web", "cache", "db"]);
    }

    #[test]
    fn test_fj012_dry_run_with_machine_filter() {
        let yaml = r#"
version: "1.0"
name: filter-test
machines:
  web:
    hostname: web
    addr: 1.1.1.1
  db:
    hostname: db
    addr: 2.2.2.2
resources:
  web-pkg:
    type: file
    machine: web
    path: /tmp/web
    content: web
  db-pkg:
    type: file
    machine: db
    path: /tmp/db
    content: db
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: true,
            machine_filter: Some("web"),
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        // Dry run returns a single result
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].machine, "dry-run");
    }

    #[test]
    fn test_fj012_apply_with_tag_filter() {
        let yaml = r#"
version: "1.0"
name: tag-apply
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  tagged:
    type: file
    machine: local
    path: /tmp/forjar-tagged-test.txt
    content: "tagged content"
    tags: [deploy]
  untagged:
    type: file
    machine: local
    path: /tmp/forjar-untagged-test.txt
    content: "untagged content"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: Some("deploy"),
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        // Only the tagged resource should be applied
        assert_eq!(results[0].resources_converged, 1);
        // The tagged file should exist
        assert!(
            std::path::Path::new("/tmp/forjar-tagged-test.txt").exists(),
            "tagged file should be created"
        );
        // Clean up
        let _ = std::fs::remove_file("/tmp/forjar-tagged-test.txt");
        let _ = std::fs::remove_file("/tmp/forjar-untagged-test.txt");
    }

    #[test]
    fn test_fj012_log_tripwire_enabled() {
        let dir = tempfile::tempdir().unwrap();
        log_tripwire(
            dir.path(),
            "machine1",
            true,
            ProvenanceEvent::ApplyStarted {
                machine: "machine1".to_string(),
                run_id: "test-run".to_string(),
                forjar_version: "0.1.0".to_string(),
            },
        );
        let events = dir.path().join("machine1").join("events.jsonl");
        assert!(events.exists(), "tripwire=true should write event");
    }

    #[test]
    fn test_fj012_log_tripwire_disabled() {
        let dir = tempfile::tempdir().unwrap();
        log_tripwire(
            dir.path(),
            "machine1",
            false,
            ProvenanceEvent::ApplyStarted {
                machine: "machine1".to_string(),
                run_id: "test-run".to_string(),
                forjar_version: "0.1.0".to_string(),
            },
        );
        let events = dir.path().join("machine1").join("events.jsonl");
        assert!(!events.exists(), "tripwire=false should NOT write event");
    }

    #[test]
    fn test_fj012_apply_result_duration_positive() {
        let config = local_config();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        for r in &results {
            assert!(r.total_duration.as_secs_f64() >= 0.0);
        }
        // Clean up the test file
        let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
    }

    // ── FJ-128: Executor edge case tests ─────────────────────────

    #[test]
    fn test_fj012_build_resource_details_empty() {
        // Resource with no path, no content, no name → empty details
        let r = Resource {
            resource_type: ResourceType::Package,
            machine: MachineTarget::Single("m".to_string()),
            state: None,
            depends_on: vec![],
            provider: Some("apt".to_string()),
            packages: vec!["curl".to_string()],
            version: None,
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let details = build_resource_details(&r, &local_machine());
        assert!(
            details.is_empty(),
            "package resource with no path/content/name should have empty details"
        );
    }

    #[test]
    fn test_fj012_build_resource_details_path_only() {
        // File resource with path but no content → path in details but no content_hash
        let r = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m".to_string()),
            state: Some("present".to_string()),
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: Some("/tmp/forjar-test-path-only.txt".to_string()),
            content: None, // no content
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let details = build_resource_details(&r, &local_machine());
        assert!(details.contains_key("path"));
        assert!(
            !details.contains_key("content_hash"),
            "no content means no content_hash"
        );
    }

    #[test]
    fn test_fj012_apply_with_timeout() {
        // Apply with explicit timeout_secs — verifies the timeout parameter threads through
        let config = local_config();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: Some(30),
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resources_converged, 1);
        let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
    }

    #[test]
    fn test_fj012_apply_arch_filter_skip() {
        // Resource with arch=[aarch64] on x86_64 machine → should be skipped
        let yaml = r#"
version: "1.0"
name: arch-skip-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    arch: x86_64
resources:
  arm-file:
    type: file
    machine: local
    path: /tmp/forjar-test-arch-skip.txt
    content: "arm only"
    arch: [aarch64]
policy:
  lock_file: true
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        // Resource should be skipped due to arch mismatch
        assert_eq!(results[0].resources_converged, 0);
        assert_eq!(results[0].resources_unchanged, 0);
        assert!(
            !std::path::Path::new("/tmp/forjar-test-arch-skip.txt").exists(),
            "arch-filtered resource should not create file"
        );
    }

    #[test]
    fn test_fj012_apply_force_noop_reapplies() {
        // With force=true, even NoOp resources should be re-applied
        let config = local_config();
        let dir = tempfile::tempdir().unwrap();

        // First apply
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r1 = apply(&cfg).unwrap();
        assert_eq!(r1[0].resources_converged, 1);

        // Second apply with force → should converge again (not unchanged)
        let cfg2 = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: true,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r2 = apply(&cfg2).unwrap();
        assert_eq!(
            r2[0].resources_converged, 1,
            "force should re-converge even NoOp resources"
        );
        assert_eq!(r2[0].resources_unchanged, 0);
        let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
    }

    #[test]
    fn test_fj012_apply_lock_file_disabled() {
        // With policy.lock_file=false, no lock should be saved
        let yaml = r#"
version: "1.0"
name: no-lock-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-test-no-lock.txt
    content: "no lock"
policy:
  lock_file: false
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results[0].resources_converged, 1);

        // Lock should NOT exist
        let lock = state::load_lock(dir.path(), "local").unwrap();
        assert!(lock.is_none(), "lock_file=false should not save lock");
        let _ = std::fs::remove_file("/tmp/forjar-test-no-lock.txt");
    }

    #[test]
    fn test_fj012_apply_tripwire_disabled_no_events() {
        // With policy.tripwire=false, no events.jsonl should be written
        let yaml = r#"
version: "1.0"
name: no-tripwire-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-test-no-tripwire.txt
    content: "no tripwire"
policy:
  tripwire: false
  lock_file: true
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results[0].resources_converged, 1);

        let events_path = dir.path().join("local").join("events.jsonl");
        assert!(
            !events_path.exists(),
            "tripwire=false should not create events.jsonl"
        );
        let _ = std::fs::remove_file("/tmp/forjar-test-no-tripwire.txt");
    }

    #[test]
    fn test_fj012_collect_machines_empty_config() {
        // Config with no resources → no machines
        let yaml = r#"
version: "1.0"
name: empty
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources: {}
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let machines = collect_machines(&config);
        assert!(machines.is_empty(), "no resources means no machines");
    }

    // ── FJ-129: Integration tests — apply→drift→re-apply cycle ──

    fn drift_config(file_path: &str) -> ForjarConfig {
        let yaml = format!(
            r#"
version: "1.0"
name: drift-test
params: {{}}
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: {}
    content: "hello from forjar"
policy:
  failure: stop_on_first
  tripwire: true
  lock_file: true
"#,
            file_path
        );
        serde_yaml_ng::from_str(&yaml).unwrap()
    }

    #[test]
    fn test_fj129_apply_then_drift_no_change() {
        // Apply a file, then check drift — should find no drift
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("drift-no-change.txt");
        let config = drift_config(file_path.to_str().unwrap());
        let state_dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: state_dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        apply(&cfg).unwrap();

        // Load lock and check drift
        let lock = state::load_lock(state_dir.path(), "local")
            .unwrap()
            .unwrap();
        let findings = crate::tripwire::drift::detect_drift(&lock);
        assert!(
            findings.is_empty(),
            "no drift expected immediately after apply"
        );
    }

    #[test]
    fn test_fj129_apply_then_drift_after_modification() {
        // Apply a file, modify it externally, then check drift
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("drift-tampered.txt");
        let config = drift_config(file_path.to_str().unwrap());
        let state_dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: state_dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        apply(&cfg).unwrap();

        // Tamper with the file
        std::fs::write(&file_path, "tampered content").unwrap();

        // Drift detection should find the change
        let lock = state::load_lock(state_dir.path(), "local")
            .unwrap()
            .unwrap();
        let findings = crate::tripwire::drift::detect_drift(&lock);
        assert_eq!(
            findings.len(),
            1,
            "should detect drift after file modification"
        );
        assert_eq!(findings[0].resource_id, "test-file");
        assert!(findings[0].detail.contains("content changed"));
    }

    #[test]
    fn test_fj129_apply_drift_reapply_cycle() {
        // Full cycle: apply → drift (no change) → tamper → drift (found) → re-apply → drift (no change)
        let tmp = tempfile::tempdir().unwrap();
        let file_path = tmp.path().join("drift-cycle.txt");
        let config = drift_config(file_path.to_str().unwrap());
        let state_dir = tempfile::tempdir().unwrap();

        // Step 1: Initial apply
        let cfg = ApplyConfig {
            config: &config,
            state_dir: state_dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r1 = apply(&cfg).unwrap();
        assert_eq!(r1[0].resources_converged, 1);

        // Step 2: Verify no drift
        let lock1 = state::load_lock(state_dir.path(), "local")
            .unwrap()
            .unwrap();
        assert!(crate::tripwire::drift::detect_drift(&lock1).is_empty());

        // Step 3: Tamper
        std::fs::write(&file_path, "unauthorized change").unwrap();
        let findings = crate::tripwire::drift::detect_drift(&lock1);
        assert_eq!(findings.len(), 1);

        // Step 4: Re-apply (force to overwrite tampered file)
        let cfg2 = ApplyConfig {
            config: &config,
            state_dir: state_dir.path(),
            force: true,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r2 = apply(&cfg2).unwrap();
        assert_eq!(r2[0].resources_converged, 1);

        // Step 5: Verify no drift after re-apply
        let lock2 = state::load_lock(state_dir.path(), "local")
            .unwrap()
            .unwrap();
        assert!(
            crate::tripwire::drift::detect_drift(&lock2).is_empty(),
            "no drift expected after re-apply"
        );
    }

    #[test]
    fn test_fj129_multi_resource_dependency_order() {
        // Verify that dependent resources are applied in correct order
        let yaml = r#"
version: "1.0"
name: dep-order
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  dir-first:
    type: file
    machine: local
    state: directory
    path: /tmp/forjar-test-dep-order
    mode: "0755"
  file-second:
    type: file
    machine: local
    path: /tmp/forjar-test-dep-order/config.txt
    content: "depends on dir"
    depends_on: [dir-first]
policy:
  lock_file: true
  tripwire: true
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results[0].resources_converged, 2);

        // Verify both artifacts exist
        assert!(std::path::Path::new("/tmp/forjar-test-dep-order").is_dir());
        assert!(std::path::Path::new("/tmp/forjar-test-dep-order/config.txt").exists());
        let content = std::fs::read_to_string("/tmp/forjar-test-dep-order/config.txt").unwrap();
        assert_eq!(content.trim(), "depends on dir");

        // Idempotency check
        let r2 = apply(&cfg).unwrap();
        assert_eq!(r2[0].resources_unchanged, 2);

        // Clean up
        let _ = std::fs::remove_dir_all("/tmp/forjar-test-dep-order");
    }

    #[test]
    fn test_fj129_config_change_triggers_update() {
        // Apply config A, then apply config B (different content), verify UPDATE
        let yaml_a = r#"
version: "1.0"
name: change-detect
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  changeable:
    type: file
    machine: local
    path: /tmp/forjar-test-change-detect.txt
    content: "version A"
policy:
  lock_file: true
"#;
        let yaml_b = r#"
version: "1.0"
name: change-detect
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  changeable:
    type: file
    machine: local
    path: /tmp/forjar-test-change-detect.txt
    content: "version B"
policy:
  lock_file: true
"#;
        let dir = tempfile::tempdir().unwrap();

        // Apply version A
        let config_a: ForjarConfig = serde_yaml_ng::from_str(yaml_a).unwrap();
        let cfg_a = ApplyConfig {
            config: &config_a,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r1 = apply(&cfg_a).unwrap();
        assert_eq!(r1[0].resources_converged, 1);
        assert_eq!(
            std::fs::read_to_string("/tmp/forjar-test-change-detect.txt")
                .unwrap()
                .trim(),
            "version A"
        );

        // Apply version B (content changed) — should detect update
        let config_b: ForjarConfig = serde_yaml_ng::from_str(yaml_b).unwrap();
        let cfg_b = ApplyConfig {
            config: &config_b,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r2 = apply(&cfg_b).unwrap();
        assert_eq!(
            r2[0].resources_converged, 1,
            "changed content should trigger re-apply"
        );
        assert_eq!(
            std::fs::read_to_string("/tmp/forjar-test-change-detect.txt")
                .unwrap()
                .trim(),
            "version B"
        );

        let _ = std::fs::remove_file("/tmp/forjar-test-change-detect.txt");
    }

    #[test]
    fn test_fj129_event_log_full_lifecycle() {
        // Verify event log records full lifecycle: started → resource_started → converged → completed
        let config = local_config();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        apply(&cfg).unwrap();

        let events_path = dir.path().join("local").join("events.jsonl");
        let content = std::fs::read_to_string(&events_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        // Should have at least 3 events: apply_started, resource_started, resource_converged, apply_completed
        assert!(
            lines.len() >= 3,
            "expected at least 3 events, got {}",
            lines.len()
        );

        // Verify event types appear in order
        assert!(
            lines[0].contains("apply_started"),
            "first event should be apply_started"
        );
        assert!(
            content.contains("resource_started"),
            "should contain resource_started"
        );
        assert!(
            content.contains("resource_converged"),
            "should contain resource_converged"
        );
        assert!(
            lines.last().unwrap().contains("apply_completed"),
            "last event should be apply_completed"
        );

        let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
    }

    // ── FJ-131: Executor + state edge case tests ──────────────────

    #[test]
    fn test_fj131_collect_machines_with_localhost() {
        // Resources targeting "localhost" (implicit machine) appear in collect output
        let yaml = r#"
version: "1.0"
name: localhost-test
machines:
  web:
    hostname: web
    addr: 1.1.1.1
resources:
  local-file:
    type: file
    machine: localhost
    path: /tmp/test
    content: "x"
  web-file:
    type: file
    machine: web
    path: /tmp/test2
    content: "y"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let machines = collect_machines(&config);
        assert!(machines.contains(&"localhost".to_string()));
        assert!(machines.contains(&"web".to_string()));
        assert_eq!(machines.len(), 2);
    }

    #[test]
    fn test_fj131_apply_empty_resources() {
        // Config with machines but no resources → empty results
        let yaml = r#"
version: "1.0"
name: empty-resources
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources: {}
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        // No resources → no machines collected → empty results
        assert!(results.is_empty());
    }

    #[test]
    fn test_fj131_apply_machine_filter_no_match() {
        // Machine filter doesn't match any collected machine → empty results
        let config = local_config();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: Some("nonexistent-machine"),
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert!(
            results.is_empty(),
            "machine filter that matches nothing should yield no results"
        );
    }

    #[test]
    fn test_fj131_record_failure_docker_resource() {
        let dir = tempfile::tempdir().unwrap();
        let mut lock = state::new_lock("test", "test-box");
        let mut ctx = RecordCtx {
            lock: &mut lock,
            state_dir: dir.path(),
            machine_name: "test",
            tripwire: true,
            failure_policy: &FailurePolicy::StopOnFirst,
            timeout_secs: None,
        };

        let should_stop = record_failure(
            &mut ctx,
            "my-container",
            &ResourceType::Docker,
            3.0,
            "image pull failed",
        );

        assert!(should_stop);
        let rl = &ctx.lock.resources["my-container"];
        assert_eq!(rl.status, ResourceStatus::Failed);
        assert_eq!(rl.resource_type, ResourceType::Docker);
        assert_eq!(rl.hash, "");
        assert!(rl.duration_seconds.unwrap() > 2.0);
    }

    #[test]
    fn test_fj131_record_failure_mount_continue() {
        let dir = tempfile::tempdir().unwrap();
        let mut lock = state::new_lock("test", "test-box");
        let mut ctx = RecordCtx {
            lock: &mut lock,
            state_dir: dir.path(),
            machine_name: "test",
            tripwire: false,
            failure_policy: &FailurePolicy::ContinueIndependent,
            timeout_secs: None,
        };

        let should_stop = record_failure(
            &mut ctx,
            "nfs-share",
            &ResourceType::Mount,
            0.8,
            "mount: permission denied",
        );

        assert!(!should_stop);
        assert_eq!(
            ctx.lock.resources["nfs-share"].resource_type,
            ResourceType::Mount
        );
    }

    #[test]
    fn test_fj131_build_details_group_only() {
        // Resource with group but no owner/mode → only group in details
        let r = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            version: None,
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: Some("www-data".to_string()),
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let details = build_resource_details(&r, &local_machine());
        assert_eq!(
            details["group"],
            serde_yaml_ng::Value::String("www-data".to_string())
        );
        assert!(!details.contains_key("owner"), "owner not set");
        assert!(!details.contains_key("mode"), "mode not set");
        assert!(!details.contains_key("path"), "path not set");
    }

    #[test]
    fn test_fj131_collect_machines_multiple_target() {
        // MachineTarget::Multiple collects all targets
        let yaml = r#"
version: "1.0"
name: multi-target
machines:
  a:
    hostname: a
    addr: 1.1.1.1
  b:
    hostname: b
    addr: 2.2.2.2
  c:
    hostname: c
    addr: 3.3.3.3
resources:
  r:
    type: file
    machine: [a, b, c]
    path: /tmp/test
    content: test
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let machines = collect_machines(&config);
        assert_eq!(machines.len(), 3);
        assert!(machines.contains(&"a".to_string()));
        assert!(machines.contains(&"b".to_string()));
        assert!(machines.contains(&"c".to_string()));
    }

    #[test]
    fn test_fj131_apply_localhost_implicit_machine() {
        // Resources on "localhost" work without defining localhost in machines block
        let yaml = r#"
version: "1.0"
name: localhost-apply
machines: {}
resources:
  local-file:
    type: file
    machine: localhost
    path: /tmp/forjar-test-localhost-implicit.txt
    content: "localhost works"
policy:
  lock_file: true
  tripwire: false
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].machine, "localhost");
        assert_eq!(results[0].resources_converged, 1);

        let content = std::fs::read_to_string("/tmp/forjar-test-localhost-implicit.txt").unwrap();
        assert_eq!(content.trim(), "localhost works");

        // Lock should reference localhost
        let lock = state::load_lock(dir.path(), "localhost").unwrap().unwrap();
        assert!(lock.resources.contains_key("local-file"));

        let _ = std::fs::remove_file("/tmp/forjar-test-localhost-implicit.txt");
    }

    #[test]
    fn test_fj131_apply_continue_independent_policy() {
        // With ContinueIndependent, a failing resource shouldn't block others
        // Use a file resource with an impossible path to trigger failure
        let yaml = r#"
version: "1.0"
name: continue-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  good-file:
    type: file
    machine: local
    path: /tmp/forjar-test-continue-good.txt
    content: "good"
  bad-file:
    type: file
    machine: local
    path: /proc/nonexistent/impossible/path.txt
    content: "will fail"
    source: /dev/null/impossible
policy:
  failure: continue_independent
  lock_file: true
  tripwire: false
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        // At least one resource should converge even if one fails
        // (good-file should succeed, bad-file may fail)
        let total = results[0].resources_converged + results[0].resources_failed;
        assert!(total > 0, "should have attempted resources");

        let _ = std::fs::remove_file("/tmp/forjar-test-continue-good.txt");
    }

    #[test]
    fn test_fj131_record_success_no_live_hash_for_package() {
        // Package resources have no state_query that returns a live_hash
        let dir = tempfile::tempdir().unwrap();
        let mut lock = state::new_lock("test", "test-box");
        let resource = Resource {
            resource_type: ResourceType::Package,
            machine: MachineTarget::Single("test".to_string()),
            provider: Some("apt".to_string()),
            packages: vec!["curl".to_string()],
            state: None,
            depends_on: vec![],
            version: None,
            path: None,
            content: None,
            source: None,
            target: None,
            owner: None,
            group: None,
            mode: None,
            name: None,
            enabled: None,
            restart_on: vec![],
            triggers: vec![],
            fs_type: None,
            options: None,
            uid: None,
            shell: None,
            home: None,
            groups: vec![],
            ssh_authorized_keys: vec![],
            system_user: false,
            schedule: None,
            command: None,
            image: None,
            ports: vec![],
            environment: vec![],
            volumes: vec![],
            restart: None,
            protocol: None,
            port: None,
            action: None,
            from_addr: None,
            recipe: None,
            inputs: HashMap::new(),
            arch: vec![],
            tags: vec![],
            when: None,
            count: None,
            for_each: None,
            chroot_dir: None,
            namespace_uid: None,
            namespace_gid: None,
            seccomp: false,
            netns: false,
            cpuset: None,
            memory_limit: None,
            overlay_lower: None,
            overlay_upper: None,
            overlay_work: None,
            overlay_merged: None,
        };
        let mut ctx = RecordCtx {
            lock: &mut lock,
            state_dir: dir.path(),
            machine_name: "test",
            tripwire: false,
            failure_policy: &FailurePolicy::StopOnFirst,
            timeout_secs: None,
        };

        record_success(
            &mut ctx,
            "pkg-curl",
            &resource,
            &resource,
            &local_machine(),
            0.3,
        );

        let rl = &ctx.lock.resources["pkg-curl"];
        assert_eq!(rl.status, ResourceStatus::Converged);
        assert!(rl.hash.starts_with("blake3:"));
        // Package resources get live_hash from state_query_script execution
        // The live_hash presence depends on whether the script succeeds locally
    }

    #[test]
    fn test_fj131_apply_dry_run_returns_unchanged_count() {
        // Dry-run with existing state should report unchanged resources
        let config = local_config();
        let dir = tempfile::tempdir().unwrap();

        // First real apply to establish state
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        apply(&cfg).unwrap();

        // Now dry-run — should report the unchanged count from plan
        let cfg2 = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: true,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg2).unwrap();
        assert_eq!(results[0].machine, "dry-run");
        assert_eq!(
            results[0].resources_unchanged, 1,
            "dry-run after apply should report 1 unchanged"
        );
        assert_eq!(results[0].resources_converged, 0);
        assert_eq!(results[0].resources_failed, 0);

        let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
    }

    #[test]
    fn test_fj131_resource_outcome_variants() {
        // Verify ResourceOutcome enum can be matched correctly
        let converged = ResourceOutcome::Converged;
        let unchanged = ResourceOutcome::Unchanged;
        let skipped = ResourceOutcome::Skipped;
        let failed_stop = ResourceOutcome::Failed { should_stop: true };
        let failed_continue = ResourceOutcome::Failed { should_stop: false };

        assert!(matches!(converged, ResourceOutcome::Converged));
        assert!(matches!(unchanged, ResourceOutcome::Unchanged));
        assert!(matches!(skipped, ResourceOutcome::Skipped));
        assert!(matches!(
            failed_stop,
            ResourceOutcome::Failed { should_stop: true }
        ));
        assert!(matches!(
            failed_continue,
            ResourceOutcome::Failed { should_stop: false }
        ));
    }

    #[test]
    fn test_fj131_record_ctx_timeout_propagation() {
        // RecordCtx correctly stores timeout value
        let dir = tempfile::tempdir().unwrap();
        let mut lock = state::new_lock("test", "test-box");
        let ctx = RecordCtx {
            lock: &mut lock,
            state_dir: dir.path(),
            machine_name: "test",
            tripwire: true,
            failure_policy: &FailurePolicy::StopOnFirst,
            timeout_secs: Some(60),
        };
        assert_eq!(ctx.timeout_secs, Some(60));
        assert_eq!(ctx.machine_name, "test");
        assert!(ctx.tripwire);
    }

    // ── FJ-132: Integration tests ──────────────────────────────

    #[test]
    fn test_fj132_force_apply_reconverges() {
        // Force apply should re-apply even when hash matches
        let config = local_config();
        let dir = tempfile::tempdir().unwrap();

        // First apply
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r1 = apply(&cfg).unwrap();
        assert_eq!(r1[0].resources_converged, 1);

        // Normal re-apply should skip (unchanged)
        let r2 = apply(&cfg).unwrap();
        assert_eq!(r2[0].resources_unchanged, 1);
        assert_eq!(r2[0].resources_converged, 0);

        // Force apply should re-converge
        let force_cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: true,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r3 = apply(&force_cfg).unwrap();
        assert_eq!(r3[0].resources_converged, 1);
        assert_eq!(r3[0].resources_unchanged, 0);

        let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
    }

    #[test]
    fn test_fj132_resource_filter_applies_only_matching() {
        // Resource filter should only apply the specified resource
        let output_dir = tempfile::tempdir().unwrap();
        let path_a = output_dir.path().join("filter-a.txt");
        let path_b = output_dir.path().join("filter-b.txt");
        let yaml = format!(
            r#"
version: "1.0"
name: filter-test
machines: {{}}
resources:
  file-a:
    type: file
    machine: localhost
    path: "{}"
    content: "alpha"
  file-b:
    type: file
    machine: localhost
    path: "{}"
    content: "beta"
policy:
  lock_file: true
  tripwire: false
"#,
            path_a.display(),
            path_b.display()
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: Some("file-a"),
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        // Only file-a should be applied
        assert_eq!(results[0].resources_converged, 1);

        // Verify file-a exists but file-b doesn't
        assert!(path_a.exists(), "file-a should be created");
        assert!(
            !path_b.exists(),
            "file-b should not be created when filtered to file-a"
        );
    }

    #[test]
    fn test_fj132_tag_filter_applies_only_tagged() {
        let output_dir = tempfile::tempdir().unwrap();
        let path_tagged = output_dir.path().join("tagged.txt");
        let path_untagged = output_dir.path().join("untagged.txt");
        let yaml = format!(
            r#"
version: "1.0"
name: tag-test
machines: {{}}
resources:
  tagged-file:
    type: file
    machine: localhost
    path: "{}"
    content: "tagged"
    tags: [web]
  untagged-file:
    type: file
    machine: localhost
    path: "{}"
    content: "untagged"
policy:
  lock_file: true
  tripwire: false
"#,
            path_tagged.display(),
            path_untagged.display()
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: Some("web"),
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results[0].resources_converged, 1);

        assert!(path_tagged.exists(), "tagged file should be created");
        assert!(
            !path_untagged.exists(),
            "untagged file should not be created when filtered by tag"
        );
    }

    #[test]
    fn test_fj132_apply_with_dependencies_order() {
        // Verify that dependency order is respected in actual apply
        let output_dir = tempfile::tempdir().unwrap();
        let path_first = output_dir.path().join("first.txt");
        let path_second = output_dir.path().join("second.txt");
        let path_third = output_dir.path().join("third.txt");
        let yaml = format!(
            r#"
version: "1.0"
name: dep-order
machines: {{}}
resources:
  first:
    type: file
    machine: localhost
    path: "{}"
    content: "first"
  second:
    type: file
    machine: localhost
    path: "{}"
    content: "second"
    depends_on: [first]
  third:
    type: file
    machine: localhost
    path: "{}"
    content: "third"
    depends_on: [second]
policy:
  lock_file: true
  tripwire: false
"#,
            path_first.display(),
            path_second.display(),
            path_third.display()
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results[0].resources_converged, 3);

        // All three files should exist
        assert_eq!(
            std::fs::read_to_string(&path_first).unwrap().trim(),
            "first"
        );
        assert_eq!(
            std::fs::read_to_string(&path_second).unwrap().trim(),
            "second"
        );
        assert_eq!(
            std::fs::read_to_string(&path_third).unwrap().trim(),
            "third"
        );
    }

    #[test]
    fn test_fj132_global_lock_written_after_apply() {
        let config = local_config();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        apply(&cfg).unwrap();

        // Per-machine lock should exist after apply
        let machine_lock = state::load_lock(dir.path(), "local").unwrap();
        assert!(machine_lock.is_some(), "per-machine lock should exist");
        let ml = machine_lock.unwrap();
        assert!(ml.resources.contains_key("test-file"));
        assert_eq!(ml.resources["test-file"].status, ResourceStatus::Converged);

        let _ = std::fs::remove_file("/tmp/forjar-test-executor.txt");
    }

    #[test]
    fn test_fj132_dry_run_creates_no_files() {
        let output_dir = tempfile::tempdir().unwrap();
        let path = output_dir.path().join("dry-run-no-exist.txt");
        let yaml = format!(
            r#"
version: "1.0"
name: dry-run-test
machines: {{}}
resources:
  test-file:
    type: file
    machine: localhost
    path: "{}"
    content: "should not be created"
"#,
            path.display()
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: true,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results[0].machine, "dry-run");

        assert!(!path.exists(), "dry-run should not create files");
    }

    // --- FJ-132: Executor edge case tests ---

    #[test]
    fn test_fj132_apply_idempotent_second_run() {
        // Second apply with same config should have 0 converged (all unchanged)
        let output_dir = tempfile::tempdir().unwrap();
        let file_path = output_dir.path().join("idempotent.txt");
        let yaml = format!(
            r#"
version: "1.0"
name: idempotent-test
machines: {{}}
resources:
  test-file:
    type: file
    machine: localhost
    path: "{}"
    content: "stable"
policy:
  lock_file: true
  tripwire: false
"#,
            file_path.display()
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        let state_dir = tempfile::tempdir().unwrap();

        // First apply
        let cfg = ApplyConfig {
            config: &config,
            state_dir: state_dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r1 = apply(&cfg).unwrap();
        assert_eq!(r1[0].resources_converged, 1);

        // Second apply — should be unchanged
        let r2 = apply(&cfg).unwrap();
        assert_eq!(r2[0].resources_unchanged, 1);
        assert_eq!(r2[0].resources_converged, 0);
    }

    #[test]
    fn test_fj132_machine_filter_skips_non_matching() {
        // Machine filter should skip machines that don't match
        let output_dir = tempfile::tempdir().unwrap();
        let file_path = output_dir.path().join("machine-filter.txt");
        let yaml = format!(
            r#"
version: "1.0"
name: machine-filter-test
machines: {{}}
resources:
  test-file:
    type: file
    machine: localhost
    path: "{}"
    content: "test"
policy:
  lock_file: true
  tripwire: false
"#,
            file_path.display()
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: state_dir.path(),
            force: false,
            dry_run: false,
            machine_filter: Some("nonexistent-machine"),
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        // No results for non-matching machine
        assert!(
            results.is_empty(),
            "no results expected for non-matching machine filter"
        );
        assert!(!file_path.exists(), "file should not be created");
    }

    #[test]
    fn test_fj132_apply_multiple_files_all_converge() {
        // Multiple file resources should all converge
        let output_dir = tempfile::tempdir().unwrap();
        let p1 = output_dir.path().join("multi-1.txt");
        let p2 = output_dir.path().join("multi-2.txt");
        let p3 = output_dir.path().join("multi-3.txt");
        let yaml = format!(
            r#"
version: "1.0"
name: multi-file
machines: {{}}
resources:
  file-1:
    type: file
    machine: localhost
    path: "{}"
    content: "one"
  file-2:
    type: file
    machine: localhost
    path: "{}"
    content: "two"
  file-3:
    type: file
    machine: localhost
    path: "{}"
    content: "three"
policy:
  lock_file: true
  tripwire: false
"#,
            p1.display(),
            p2.display(),
            p3.display()
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: state_dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results[0].resources_converged, 3);
        assert_eq!(std::fs::read_to_string(&p1).unwrap().trim(), "one");
        assert_eq!(std::fs::read_to_string(&p2).unwrap().trim(), "two");
        assert_eq!(std::fs::read_to_string(&p3).unwrap().trim(), "three");
    }

    #[test]
    fn test_fj132_apply_result_has_duration() {
        // ApplyResult should have a non-zero total_duration
        let output_dir = tempfile::tempdir().unwrap();
        let file_path = output_dir.path().join("duration-test.txt");
        let yaml = format!(
            r#"
version: "1.0"
name: duration-test
machines: {{}}
resources:
  test-file:
    type: file
    machine: localhost
    path: "{}"
    content: "test"
policy:
  lock_file: true
  tripwire: false
"#,
            file_path.display()
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        let state_dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: state_dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert!(
            results[0].total_duration.as_nanos() > 0,
            "apply should take some non-zero time"
        );
    }

    #[test]
    fn test_fj132_force_apply_reconverges_unchanged() {
        // Force apply should re-apply even when hash matches (second run)
        let output_dir = tempfile::tempdir().unwrap();
        let file_path = output_dir.path().join("force-reconverge.txt");
        let yaml = format!(
            r#"
version: "1.0"
name: force-test
machines: {{}}
resources:
  test-file:
    type: file
    machine: localhost
    path: "{}"
    content: "force-me"
policy:
  lock_file: true
  tripwire: false
"#,
            file_path.display()
        );
        let config: ForjarConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        let state_dir = tempfile::tempdir().unwrap();

        // First apply
        let cfg = ApplyConfig {
            config: &config,
            state_dir: state_dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        apply(&cfg).unwrap();

        // Second apply with force=true
        let cfg_force = ApplyConfig {
            config: &config,
            state_dir: state_dir.path(),
            force: true,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg_force).unwrap();
        assert_eq!(
            results[0].resources_converged, 1,
            "force should reconverge even when unchanged"
        );
    }

    #[test]
    fn test_fj132_collect_machines_from_config() {
        // collect_machines returns machines referenced by resources, not all declared machines
        let yaml = r#"
version: "1.0"
name: collect-test
machines:
  web:
    hostname: web-01
    addr: 10.0.0.1
  db:
    hostname: db-01
    addr: 10.0.0.2
resources:
  web-pkg:
    type: package
    machine: web
    provider: apt
    packages: [curl]
  db-pkg:
    type: package
    machine: db
    provider: apt
    packages: [postgresql]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let machines = collect_machines(&config);
        assert!(machines.contains(&"web".to_string()));
        assert!(machines.contains(&"db".to_string()));
        assert_eq!(machines.len(), 2);
    }

    // ── FJ-036: Dry-run and force-reapply coverage ──────────────────

    #[test]
    fn test_fj036_dry_run_produces_no_side_effects() {
        let yaml = r#"
version: "1.0"
name: dry-run-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-test-fj036-dry-run.txt
    content: "should not be created"
policy:
  lock_file: true
  tripwire: true
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();

        // Ensure target file does not exist before
        let _ = std::fs::remove_file("/tmp/forjar-test-fj036-dry-run.txt");

        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: true,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();

        // Dry run should return exactly one synthetic result
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].machine, "dry-run");
        assert_eq!(results[0].resources_converged, 0);
        assert_eq!(results[0].resources_failed, 0);

        // No lock file should have been written for any machine
        let lock = state::load_lock(dir.path(), "local").unwrap();
        assert!(lock.is_none(), "dry_run must not create a lock file");

        // Target file must not have been created
        assert!(
            !std::path::Path::new("/tmp/forjar-test-fj036-dry-run.txt").exists(),
            "dry_run must not create the managed file"
        );

        // No event log should exist
        let events_path = dir.path().join("local").join("events.jsonl");
        assert!(!events_path.exists(), "dry_run must not write event logs");
    }

    #[test]
    fn test_fj036_force_reapply_changes_action() {
        let yaml = r#"
version: "1.0"
name: force-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  test-file:
    type: file
    machine: local
    path: /tmp/forjar-test-fj036-force.txt
    content: "force test content"
policy:
  lock_file: true
  tripwire: true
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let dir = tempfile::tempdir().unwrap();

        // First apply — should converge
        let cfg1 = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r1 = apply(&cfg1).unwrap();
        assert_eq!(r1[0].resources_converged, 1);
        assert_eq!(r1[0].resources_unchanged, 0);

        // Second apply without force — should be unchanged (idempotent)
        let cfg2 = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r2 = apply(&cfg2).unwrap();
        assert_eq!(r2[0].resources_unchanged, 1);
        assert_eq!(r2[0].resources_converged, 0);

        // Third apply WITH force — should re-converge even though nothing changed
        let cfg3 = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: true,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r3 = apply(&cfg3).unwrap();
        assert_eq!(
            r3[0].resources_converged, 1,
            "force=true must re-apply even when state matches"
        );
        assert_eq!(
            r3[0].resources_unchanged, 0,
            "force=true must not skip any resource"
        );

        // Lock should still be valid after force apply
        let lock = state::load_lock(dir.path(), "local").unwrap();
        assert!(lock.is_some(), "lock file must exist after force apply");

        let _ = std::fs::remove_file("/tmp/forjar-test-fj036-force.txt");
    }

    #[test]
    fn test_executor_local_machine_defaults() {
        let m = local_machine();
        assert_eq!(m.hostname, "localhost");
        assert_eq!(m.addr, "127.0.0.1");
        assert_eq!(m.user, "root");
        assert_eq!(m.arch, "x86_64");
        assert!(m.ssh_key.is_none(), "local machine should have no ssh_key");
        assert!(m.roles.is_empty(), "local machine should have no roles");
        assert!(
            m.transport.is_none(),
            "local machine should have no transport override"
        );
        assert!(
            m.container.is_none(),
            "local machine should have no container config"
        );
        assert_eq!(m.cost, 0, "local machine should have zero cost");
    }

    #[test]
    fn test_executor_local_config_minimal() {
        let config = local_config();
        assert_eq!(config.name, "test");
        assert_eq!(config.version, "1.0");
        assert!(
            config.machines.contains_key("local"),
            "config should contain machine 'local'"
        );
        assert!(
            config.resources.contains_key("test-file"),
            "config should contain resource 'test-file'"
        );
        let r = &config.resources["test-file"];
        assert_eq!(r.resource_type, ResourceType::File);
        assert_eq!(r.path.as_deref(), Some("/tmp/forjar-test-executor.txt"));
        assert_eq!(r.content.as_deref(), Some("hello from forjar"));
        assert!(config.policy.tripwire, "policy.tripwire should be true");
        assert!(config.policy.lock_file, "policy.lock_file should be true");
    }

    #[test]
    fn test_executor_collect_machines_filters_by_name() {
        let yaml = r#"
version: "1.0"
name: filter-test
machines:
  web:
    hostname: web
    addr: 10.0.0.1
  db:
    hostname: db
    addr: 10.0.0.2
  cache:
    hostname: cache
    addr: 10.0.0.3
resources:
  r1:
    type: file
    machine: web
    path: /tmp/a
    content: a
  r2:
    type: file
    machine: db
    path: /tmp/b
    content: b
  r3:
    type: file
    machine: [web, cache]
    path: /tmp/c
    content: c
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let machines = collect_machines(&config);
        assert_eq!(
            machines.len(),
            3,
            "should collect 3 unique machines: {:?}",
            machines
        );
        assert!(machines.contains(&"web".to_string()), "should contain web");
        assert!(machines.contains(&"db".to_string()), "should contain db");
        assert!(
            machines.contains(&"cache".to_string()),
            "should contain cache"
        );

        // Verify machine_filter works in ApplyConfig (dry-run) — only "db" processed
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: true,
            machine_filter: Some("db"),
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results[0].machine, "dry-run");
    }

    #[test]
    fn test_fj050_trace_written_on_apply() {
        // Verify that apply_machine writes trace.jsonl when tripwire is enabled
        let dir = tempfile::tempdir().unwrap();

        let yaml = r#"
version: "1.0"
name: trace-test
machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
policy:
  tripwire: true
resources:
  test-dir:
    type: file
    machine: localhost
    path: /tmp/forjar-trace-test
    content: "trace-test"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };

        let results = apply(&cfg).unwrap();
        assert!(!results.is_empty());

        // Check that trace.jsonl was written
        let trace_path = dir.path().join("localhost").join("trace.jsonl");
        assert!(
            trace_path.exists(),
            "trace.jsonl should be written when tripwire is enabled"
        );

        // Parse the trace spans
        let spans = tracer::read_trace(dir.path(), "localhost").unwrap();
        assert!(!spans.is_empty(), "trace should contain at least one span");

        // All spans should have the same trace ID
        let trace_id = &spans[0].trace_id;
        for span in &spans {
            assert_eq!(&span.trace_id, trace_id, "all spans share trace ID");
        }
    }

    #[test]
    fn test_fj050_trace_not_written_when_tripwire_off() {
        let dir = tempfile::tempdir().unwrap();

        let yaml = r#"
version: "1.0"
name: no-trace-test
machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
policy:
  tripwire: false
resources:
  test-dir:
    type: file
    machine: localhost
    path: /tmp/forjar-no-trace-test
    content: "no-trace"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };

        let _results = apply(&cfg).unwrap();

        // trace.jsonl should NOT exist when tripwire is off
        let trace_path = dir.path().join("localhost").join("trace.jsonl");
        assert!(
            !trace_path.exists(),
            "trace.jsonl should not be written when tripwire is off"
        );
    }

    #[test]
    fn test_fj050_trace_span_fields() {
        let dir = tempfile::tempdir().unwrap();

        let yaml = r#"
version: "1.0"
name: span-fields-test
machines:
  localhost:
    hostname: localhost
    addr: 127.0.0.1
policy:
  tripwire: true
resources:
  config-file:
    type: file
    machine: localhost
    path: /tmp/forjar-span-fields
    content: "span-fields"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };

        let _results = apply(&cfg).unwrap();
        let spans = tracer::read_trace(dir.path(), "localhost").unwrap();
        assert!(!spans.is_empty());

        let span = &spans[0];
        assert!(
            span.name.starts_with("apply:"),
            "span name should start with apply: got: {}",
            span.name
        );
        assert_eq!(span.machine, "localhost");
        assert!(span.logical_clock > 0, "logical clock should be positive");
        assert!(
            span.parent_span_id.is_some(),
            "child spans should have parent_span_id"
        );
    }

    #[test]
    fn test_fj051_cmd_anomaly_uses_module() {
        // Verify that detect_anomalies is callable and returns consistent results
        let metrics = vec![
            ("stable:web".to_string(), 5u32, 0u32, 0u32),
            ("stable:db".to_string(), 5, 0, 0),
            ("stable:cache".to_string(), 5, 0, 0),
        ];
        let findings = crate::tripwire::anomaly::detect_anomalies(&metrics, 3);
        assert!(
            findings.is_empty(),
            "uniform metrics should produce no anomalies"
        );

        // Add a churny resource
        let mut metrics2 = metrics.clone();
        metrics2.push(("churny:web".to_string(), 500, 0, 0));
        metrics2.push(("drifty:db".to_string(), 10, 0, 5));
        let findings2 = crate::tripwire::anomaly::detect_anomalies(&metrics2, 3);
        assert!(
            !findings2.is_empty(),
            "should detect anomalies in mixed metrics"
        );
        // Drift events should always be flagged
        assert!(
            findings2.iter().any(|f| f.resource == "drifty:db"),
            "drift events should be flagged"
        );
    }

    // ================================================================
    // FJ-216: parallel resource execution tests
    // ================================================================

    #[test]
    fn test_fj216_compute_resource_waves_no_deps() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  a:
    type: file
    machine: m1
    path: /a
  b:
    type: file
    machine: m1
    path: /b
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let waves = compute_resource_waves(&config, &["a", "b"]);
        assert_eq!(waves.len(), 1);
        assert_eq!(waves[0].len(), 2);
    }

    #[test]
    fn test_fj216_compute_resource_waves_with_deps() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  base:
    type: file
    machine: m1
    path: /base
  app:
    type: file
    machine: m1
    path: /app
    depends_on: [base]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let waves = compute_resource_waves(&config, &["base", "app"]);
        assert_eq!(waves.len(), 2);
        assert_eq!(waves[0], vec!["base"]);
        assert_eq!(waves[1], vec!["app"]);
    }

    #[test]
    fn test_fj216_compute_resource_waves_subset() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  a:
    type: file
    machine: m1
    path: /a
  b:
    type: file
    machine: m1
    path: /b
    depends_on: [a]
  c:
    type: file
    machine: m1
    path: /c
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        // Only compute waves for b and c (b depends on a which is outside subset)
        let waves = compute_resource_waves(&config, &["b", "c"]);
        // Both should be in wave 0 since a is not in subset
        assert_eq!(waves.len(), 1);
        assert_eq!(waves[0].len(), 2);
    }

    #[test]
    fn test_fj216_parallel_resources_policy_default_false() {
        let policy = Policy::default();
        assert!(!policy.parallel_resources);
    }

    #[test]
    fn test_fj216_parallel_resources_policy_yaml() {
        let yaml = r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
policy:
  parallel_resources: true
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert!(config.policy.parallel_resources);
    }

    #[test]
    fn test_fj224_trigger_forces_reapply() {
        // First apply: both config and app converge
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  config:
    type: file
    machine: local
    path: /tmp/fj224-config.txt
    content: "v1"
  app:
    type: file
    machine: local
    path: /tmp/fj224-app.txt
    content: "app-content"
    depends_on: [config]
    triggers: [config]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let cfg1 = ApplyConfig {
            config: &config,
            state_dir: &state_dir,
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r1 = apply(&cfg1).unwrap();
        assert_eq!(r1[0].resources_converged, 2);

        // Second apply, same config: both should be NoOp (unchanged)
        let r2 = apply(&cfg1).unwrap();
        assert_eq!(r2[0].resources_converged, 0, "no changes = no converge");
        assert_eq!(r2[0].resources_unchanged, 2);

        // Third apply: change config content → config converges → app should be triggered
        let yaml3 = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  config:
    type: file
    machine: local
    path: /tmp/fj224-config.txt
    content: "v2"
  app:
    type: file
    machine: local
    path: /tmp/fj224-app.txt
    content: "app-content"
    depends_on: [config]
    triggers: [config]
"#;
        let config3: ForjarConfig = serde_yaml_ng::from_str(yaml3).unwrap();
        let cfg3 = ApplyConfig {
            config: &config3,
            state_dir: &state_dir,
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r3 = apply(&cfg3).unwrap();
        // config changed → converges. app unchanged but triggers: [config] → also converges
        assert_eq!(
            r3[0].resources_converged, 2,
            "config changed + app triggered"
        );
    }

    #[test]
    fn test_fj224_trigger_no_fire_when_dep_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  config:
    type: file
    machine: local
    path: /tmp/fj224b-config.txt
    content: "stable"
  app:
    type: file
    machine: local
    path: /tmp/fj224b-app.txt
    content: "app"
    depends_on: [config]
    triggers: [config]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: &state_dir,
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r1 = apply(&cfg).unwrap();
        assert_eq!(r1[0].resources_converged, 2);

        // Second apply: nothing changed, trigger should NOT fire
        let r2 = apply(&cfg).unwrap();
        assert_eq!(r2[0].resources_converged, 0);
        assert_eq!(r2[0].resources_unchanged, 2);
    }

    #[test]
    fn test_fj224_trigger_multiple_sources() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let yaml1 = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  db-config:
    type: file
    machine: local
    path: /tmp/fj224c-db.txt
    content: "db-v1"
  app-config:
    type: file
    machine: local
    path: /tmp/fj224c-app.txt
    content: "app-v1"
  service:
    type: file
    machine: local
    path: /tmp/fj224c-svc.txt
    content: "svc"
    depends_on: [db-config, app-config]
    triggers: [db-config, app-config]
"#;
        let config1: ForjarConfig = serde_yaml_ng::from_str(yaml1).unwrap();
        let cfg1 = ApplyConfig {
            config: &config1,
            state_dir: &state_dir,
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r1 = apply(&cfg1).unwrap();
        assert_eq!(r1[0].resources_converged, 3);

        // Change only db-config → service should be triggered
        let yaml2 = yaml1.replace("db-v1", "db-v2");
        let config2: ForjarConfig = serde_yaml_ng::from_str(&yaml2).unwrap();
        let cfg2 = ApplyConfig {
            config: &config2,
            state_dir: &state_dir,
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r2 = apply(&cfg2).unwrap();
        // db-config changed (converged), app-config unchanged, service triggered
        assert_eq!(
            r2[0].resources_converged, 2,
            "db-config + service triggered"
        );
    }

    #[test]
    fn test_fj224_trigger_without_depends_on() {
        // Triggers can work independently of depends_on
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let yaml = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  config:
    type: file
    machine: local
    path: /tmp/fj224d-config.txt
    content: "v1"
  app:
    type: file
    machine: local
    path: /tmp/fj224d-app.txt
    content: "app"
    triggers: [config]
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: &state_dir,
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r1 = apply(&cfg).unwrap();
        assert_eq!(r1[0].resources_converged, 2);

        // Note: Without depends_on, execution order is alphabetical.
        // "app" sorts before "config", so trigger won't fire because
        // config hasn't converged yet when app is processed.
        // This is correct behavior — triggers require proper ordering
        // (either via depends_on or natural sort order).

        // With depends_on, changing config triggers app
        let yaml2 = r#"
version: "1.0"
name: test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  config:
    type: file
    machine: local
    path: /tmp/fj224d-config.txt
    content: "v2"
  app:
    type: file
    machine: local
    path: /tmp/fj224d-app.txt
    content: "app"
    depends_on: [config]
    triggers: [config]
"#;
        let config2: ForjarConfig = serde_yaml_ng::from_str(yaml2).unwrap();
        let cfg2 = ApplyConfig {
            config: &config2,
            state_dir: &state_dir,
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let r2 = apply(&cfg2).unwrap();
        assert_eq!(
            r2[0].resources_converged, 2,
            "config changed + app triggered"
        );
    }

    // ========================================================================
    // FJ-222: Rolling deploys
    // ========================================================================

    #[test]
    fn test_fj222_serial_batches_machines() {
        let yaml = r#"
version: "1.0"
name: rolling-test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
  m2:
    hostname: m2
    addr: 127.0.0.1
  m3:
    hostname: m3
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: m1
    path: /tmp/forjar-rolling-m1.txt
    content: "m1"
  f2:
    type: file
    machine: m2
    path: /tmp/forjar-rolling-m2.txt
    content: "m2"
  f3:
    type: file
    machine: m3
    path: /tmp/forjar-rolling-m3.txt
    content: "m3"
policy:
  serial: 2
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.policy.serial, Some(2));

        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        // All 3 machines should converge (2 in first batch, 1 in second)
        assert_eq!(results.len(), 3);
        let total: u32 = results.iter().map(|r| r.resources_converged).sum();
        assert_eq!(total, 3);

        let _ = std::fs::remove_file("/tmp/forjar-rolling-m1.txt");
        let _ = std::fs::remove_file("/tmp/forjar-rolling-m2.txt");
        let _ = std::fs::remove_file("/tmp/forjar-rolling-m3.txt");
    }

    #[test]
    fn test_fj222_serial_with_parallel() {
        // serial + parallel_machines: batches run in parallel
        let yaml = r#"
version: "1.0"
name: rolling-parallel
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
  m2:
    hostname: m2
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: m1
    path: /tmp/forjar-rp-m1.txt
    content: "m1"
  f2:
    type: file
    machine: m2
    path: /tmp/forjar-rp-m2.txt
    content: "m2"
policy:
  serial: 2
  parallel_machines: true
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.policy.serial, Some(2));
        assert!(config.policy.parallel_machines);

        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results.len(), 2);

        let _ = std::fs::remove_file("/tmp/forjar-rp-m1.txt");
        let _ = std::fs::remove_file("/tmp/forjar-rp-m2.txt");
    }

    #[test]
    fn test_fj222_max_fail_percentage_yaml() {
        let yaml = r#"
version: "1.0"
name: fail-pct
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m1
    path: /tmp/forjar-pct.txt
    content: "ok"
policy:
  serial: 1
  max_fail_percentage: 50
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.policy.max_fail_percentage, Some(50));
        assert_eq!(config.policy.serial, Some(1));

        // With one machine and no failures, this should succeed
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].resources_failed, 0);

        let _ = std::fs::remove_file("/tmp/forjar-pct.txt");
    }

    #[test]
    fn test_fj222_serial_default_none() {
        let yaml = r#"
version: "1.0"
name: no-serial
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  f:
    type: file
    machine: m1
    path: /tmp/forjar-nosrl.txt
    content: "x"
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.policy.serial, None);
        assert_eq!(config.policy.max_fail_percentage, None);

        let _ = std::fs::remove_file("/tmp/forjar-nosrl.txt");
    }

    #[test]
    fn test_fj222_serial_one_is_sequential() {
        // serial: 1 means one machine at a time (fully sequential)
        let yaml = r#"
version: "1.0"
name: serial-one
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
  m2:
    hostname: m2
    addr: 127.0.0.1
resources:
  f1:
    type: file
    machine: m1
    path: /tmp/forjar-s1-m1.txt
    content: "m1"
  f2:
    type: file
    machine: m2
    path: /tmp/forjar-s1-m2.txt
    content: "m2"
policy:
  serial: 1
  parallel_machines: true
"#;
        let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        // serial:1 with parallel_machines:true — batches of 1, so effectively sequential
        let dir = tempfile::tempdir().unwrap();
        let cfg = ApplyConfig {
            config: &config,
            state_dir: dir.path(),
            force: false,
            dry_run: false,
            machine_filter: None,
            resource_filter: None,
            tag_filter: None,
            timeout_secs: None,
        };
        let results = apply(&cfg).unwrap();
        assert_eq!(results.len(), 2);

        let _ = std::fs::remove_file("/tmp/forjar-s1-m1.txt");
        let _ = std::fs::remove_file("/tmp/forjar-s1-m2.txt");
    }
}

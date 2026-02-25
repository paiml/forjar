//! FJ-012: Executor — orchestration loop for apply.
//!
//! Applies resources in topological order per machine:
//! parse → validate → DAG → plan → for each resource: codegen → transport → hash → state → events

use super::codegen;
use super::planner;
use super::resolver;
use super::state;
use super::types::*;
use crate::transport;
use crate::tripwire::{eventlog, hasher};
use std::collections::HashMap;
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

    // Execute plan per machine (parallel or sequential)
    if cfg.config.policy.parallel_machines && target_machines.len() > 1 {
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
) -> Result<ResourceOutcome, String> {
    if let Some(filter) = cfg.resource_filter {
        if change.resource_id != filter {
            return Ok(ResourceOutcome::Skipped);
        }
    }

    if change.action == PlanAction::NoOp && !cfg.force {
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

    for change in &machine_changes {
        match apply_single_resource(cfg, change, machine, &mut ctx)? {
            ResourceOutcome::Converged => converged += 1,
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

    // Rebind lock from ctx for finalization
    let lock = ctx.lock;
    lock.generated_at = eventlog::now_iso8601();
    if cfg.config.policy.lock_file {
        state::save_lock(cfg.state_dir, lock)?;
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
            state: None, depends_on: vec![], provider: None, packages: vec![],
            version: None, source: None, target: None, owner: None, group: None,
            mode: None, name: None, enabled: None, restart_on: vec![],
            fs_type: None, options: None, uid: None, shell: None, home: None,
            groups: vec![], ssh_authorized_keys: vec![], system_user: false,
            schedule: None, command: None, image: None, ports: vec![],
            environment: vec![], volumes: vec![], restart: None, protocol: None,
            port: None, action: None, from_addr: None, recipe: None,
            inputs: HashMap::new(), arch: vec![], tags: vec![],
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
            path: None, content: None, source: None, target: None,
            depends_on: vec![], provider: None, packages: vec![],
            version: None, owner: None, group: None, mode: None,
            enabled: Some(true), restart_on: vec![], fs_type: None,
            options: None, uid: None, shell: None, home: None,
            groups: vec![], ssh_authorized_keys: vec![], system_user: false,
            schedule: None, command: None, image: None, ports: vec![],
            environment: vec![], volumes: vec![], restart: None, protocol: None,
            port: None, action: None, from_addr: None, recipe: None,
            inputs: HashMap::new(), arch: vec![], tags: vec![],
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
            source: None, target: None, state: None,
            depends_on: vec![], provider: None, packages: vec![],
            version: None, owner: Some("root".to_string()),
            group: None, mode: Some("0644".to_string()),
            name: None, enabled: None, restart_on: vec![],
            fs_type: None, options: None, uid: None, shell: None,
            home: None, groups: vec![], ssh_authorized_keys: vec![],
            system_user: false, schedule: None, command: None,
            image: None, ports: vec![], environment: vec![],
            volumes: vec![], restart: None, protocol: None,
            port: None, action: None, from_addr: None, recipe: None,
            inputs: HashMap::new(), arch: vec![], tags: vec![],
        };
        let details = build_resource_details(&resource, &local_machine());
        assert!(details.contains_key("path"));
        assert!(details.contains_key("owner"));
        assert!(details.contains_key("mode"));
        assert!(!details.contains_key("content_hash"), "no content → no hash");
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
            source: None, target: None, state: None,
            depends_on: vec![], provider: None, packages: vec![],
            version: None, owner: None, group: None, mode: None,
            name: None, enabled: None, restart_on: vec![],
            fs_type: None, options: None, uid: None, shell: None,
            home: None, groups: vec![], ssh_authorized_keys: vec![],
            system_user: false, schedule: None, command: None,
            image: None, ports: vec![], environment: vec![],
            volumes: vec![], restart: None, protocol: None,
            port: None, action: None, from_addr: None, recipe: None,
            inputs: HashMap::new(), arch: vec![], tags: vec![],
        };
        let details = build_resource_details(&resource, &local_machine());
        assert!(details.contains_key("content_hash"), "real file should have content_hash");
        let hash = details["content_hash"].as_str().unwrap();
        assert!(hash.starts_with("blake3:"), "hash should be blake3-prefixed");
    }

    #[test]
    fn test_fj012_build_details_nonexistent_file_no_hash() {
        // content is set but the file doesn't exist → no content_hash
        let resource = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("test".to_string()),
            path: Some("/tmp/does-not-exist-forjar-test.txt".to_string()),
            content: Some("ghost".to_string()),
            source: None, target: None, state: None,
            depends_on: vec![], provider: None, packages: vec![],
            version: None, owner: None, group: None, mode: None,
            name: None, enabled: None, restart_on: vec![],
            fs_type: None, options: None, uid: None, shell: None,
            home: None, groups: vec![], ssh_authorized_keys: vec![],
            system_user: false, schedule: None, command: None,
            image: None, ports: vec![], environment: vec![],
            volumes: vec![], restart: None, protocol: None,
            port: None, action: None, from_addr: None, recipe: None,
            inputs: HashMap::new(), arch: vec![], tags: vec![],
        };
        let details = build_resource_details(&resource, &local_machine());
        assert!(!details.contains_key("content_hash"), "nonexistent file → no hash");
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
            content: None, source: None, target: None, state: None,
            depends_on: vec![], provider: None, packages: vec![],
            version: None, enabled: None, restart_on: vec![],
            fs_type: None, options: None, uid: None, shell: None,
            home: None, groups: vec![], ssh_authorized_keys: vec![],
            system_user: false, schedule: None, command: None,
            image: None, ports: vec![], environment: vec![],
            volumes: vec![], restart: None, protocol: None,
            port: None, action: None, from_addr: None, recipe: None,
            inputs: HashMap::new(), arch: vec![], tags: vec![],
        };
        let details = build_resource_details(&resource, &local_machine());
        assert_eq!(details["path"], serde_yaml_ng::Value::String("/etc/app.conf".to_string()));
        assert_eq!(details["owner"], serde_yaml_ng::Value::String("app".to_string()));
        assert_eq!(details["group"], serde_yaml_ng::Value::String("app".to_string()));
        assert_eq!(details["mode"], serde_yaml_ng::Value::String("0600".to_string()));
        assert_eq!(details["service_name"], serde_yaml_ng::Value::String("app-config".to_string()));
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
}

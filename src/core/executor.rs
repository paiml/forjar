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
use std::time::Instant;

/// Configuration for an apply run.
pub struct ApplyConfig<'a> {
    pub config: &'a ForjarConfig,
    pub state_dir: &'a std::path::Path,
    pub force: bool,
    pub dry_run: bool,
    pub machine_filter: Option<&'a str>,
    pub resource_filter: Option<&'a str>,
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
    let plan = planner::plan(cfg.config, &execution_order, &locks);

    if cfg.dry_run {
        return Ok(vec![ApplyResult {
            machine: "dry-run".to_string(),
            resources_converged: 0,
            resources_unchanged: plan.unchanged,
            resources_failed: 0,
            total_duration: start.elapsed(),
        }]);
    }

    // Execute plan per machine
    let mut results = Vec::new();

    for machine_name in &all_machines {
        if let Some(filter) = cfg.machine_filter {
            if machine_name != filter {
                continue;
            }
        }

        let machine = match cfg.config.machines.get(machine_name) {
            Some(m) => m,
            None => {
                if machine_name == "localhost" {
                    &Machine {
                        hostname: "localhost".to_string(),
                        addr: "127.0.0.1".to_string(),
                        user: "root".to_string(),
                        arch: "x86_64".to_string(),
                        ssh_key: None,
                        roles: vec![],
                    }
                } else {
                    continue;
                }
            }
        };

        let result = apply_machine(cfg, machine_name, machine, &plan, &mut locks)?;
        results.push(result);
    }

    Ok(results)
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
        Ok(query) => match transport::exec_script(machine, &query) {
            Ok(qout) if qout.success() => Some(hasher::hash_string(&qout.stdout)),
            _ => None,
        },
        Err(_) => None,
    };

    let mut details = build_resource_details(resolved);
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
    let output = transport::exec_script(machine, &script);
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

    Ok(ApplyResult {
        machine: machine_name.to_string(),
        resources_converged: converged,
        resources_unchanged: unchanged,
        resources_failed: failed,
        total_duration: machine_start.elapsed(),
    })
}

/// Collect all unique machine names referenced by resources.
fn collect_machines(config: &ForjarConfig) -> Vec<String> {
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
fn build_resource_details(resource: &Resource) -> HashMap<String, serde_yaml_ng::Value> {
    let mut details = HashMap::new();

    if let Some(ref path) = resource.path {
        details.insert(
            "path".to_string(),
            serde_yaml_ng::Value::String(path.clone()),
        );
    }
    if let Some(ref content) = resource.content {
        let hash = hasher::hash_string(content);
        details.insert(
            "content_hash".to_string(),
            serde_yaml_ng::Value::String(hash),
        );
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
        let r = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("m".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            path: Some("/etc/test".to_string()),
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
        };
        let details = build_resource_details(&r);
        assert!(details.contains_key("path"));
        assert!(details.contains_key("content_hash"));
        assert!(details.contains_key("owner"));
        assert!(details.contains_key("mode"));
        assert!(details.contains_key("group"));
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
        };
        let details = build_resource_details(&r);
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
        let mut lock = state::new_lock("test", "test-box");
        let resource = Resource {
            resource_type: ResourceType::File,
            machine: MachineTarget::Single("test".to_string()),
            state: None,
            depends_on: vec![],
            provider: None,
            packages: vec![],
            path: Some("/tmp/forjar-record-success-test.txt".to_string()),
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
        };
        let machine = Machine {
            hostname: "localhost".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
        };
        let mut ctx = RecordCtx {
            lock: &mut lock,
            state_dir: dir.path(),
            machine_name: "test",
            tripwire: true,
            failure_policy: &FailurePolicy::StopOnFirst,
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
        };
        let results = apply(&cfg).unwrap();
        // Resource filter doesn't match — everything skipped
        assert_eq!(results[0].resources_converged, 0);
        assert_eq!(results[0].resources_unchanged, 0);
    }
}

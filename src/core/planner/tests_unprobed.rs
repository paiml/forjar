//! forjar#497: the planner names what it did not probe.
//!
//! The binary-driven cases live in
//! `tests/falsification_planner_names_what_it_did_not_probe.rs`; these pin
//! the census at the planner boundary, where the probe map is a parameter and
//! every branch of the rule can be reached without a transport.

use super::hashing::hash_desired_state;
use super::plan;
use crate::core::task::{probe_covers, probe_resource};
use crate::core::types::*;
use std::collections::HashMap;

/// RFC 5737 TEST-NET-3: never routable, never this host.
const REMOTE: &str = "203.0.113.7";

/// `box` is this host; `far` is not. `build` declares inputs and artifacts;
/// `note` declares nothing the probe would measure.
fn config(build_machine: &str, working_dir: &str) -> ForjarConfig {
    let yaml = format!(
        "version: \"1.0\"\nname: t\nmachines:\n  box:\n    hostname: box\n    addr: 127.0.0.1\n\
         \x20 far:\n    hostname: far\n    addr: {REMOTE}\nresources:\n  build:\n    type: task\n\
         \x20   machine: {build_machine}\n    command: \"true\"\n    working_dir: {working_dir}\n\
         \x20   task_inputs: [src.txt]\n    output_artifacts: [out.txt]\n  note:\n    type: file\n\
         \x20   machine: far\n    path: /tmp/forjar-497-note.txt\n    content: \"x\"\n"
    );
    serde_yaml_ng::from_str(&yaml).expect("fixture config")
}

fn order() -> Vec<String> {
    vec!["build".to_string(), "note".to_string()]
}

/// One converged lock per machine, every resource at its desired hash,
/// with `details` from `details_for`.
fn converged_locks(
    config: &ForjarConfig,
    details_for: impl Fn(&str) -> HashMap<String, serde_yaml_ng::Value>,
) -> HashMap<String, StateLock> {
    let mut locks = HashMap::new();
    for machine in config.machines.keys() {
        let mut resources = indexmap::IndexMap::new();
        for (id, resource) in &config.resources {
            if !resource.machine.iter().any(|m| m == machine) {
                continue;
            }
            resources.insert(
                id.clone(),
                ResourceLock {
                    resource_type: resource.resource_type.clone(),
                    status: ResourceStatus::Converged,
                    applied_at: None,
                    duration_seconds: None,
                    hash: hash_desired_state(resource),
                    observed: None,
                    details: details_for(id),
                },
            );
        }
        locks.insert(
            machine.clone(),
            StateLock {
                schema: "1.0".to_string(),
                machine: machine.clone(),
                hostname: machine.clone(),
                generated_at: "2026-01-01T00:00:00Z".to_string(),
                generator: "forjar".to_string(),
                blake3_version: "1.8".to_string(),
                resources,
            },
        );
    }
    locks
}

/// A real working directory holding the declared input and artifact, so a
/// probe of `build` on this host succeeds and matches the lock.
struct WorkDir(std::path::PathBuf);

impl WorkDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-497-unit-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("workdir");
        std::fs::write(dir.join("src.txt"), "v1\n").expect("input");
        std::fs::write(dir.join("out.txt"), "v1\n").expect("artifact");
        Self(dir)
    }

    fn path(&self) -> String {
        self.0.display().to_string()
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The lock details a converged apply would have recorded for `build`.
fn recorded_io(config: &ForjarConfig) -> HashMap<String, serde_yaml_ng::Value> {
    let digest = probe_resource(&config.resources["build"]).expect("build declares I/O");
    let mut details = HashMap::new();
    if let Some(h) = digest.input_hash {
        details.insert("input_hash".to_string(), serde_yaml_ng::Value::String(h));
    }
    if let Some(h) = digest.output_hash {
        details.insert("output_hash".to_string(), serde_yaml_ng::Value::String(h));
    }
    details
}

fn action_of(plan: &ExecutionPlan, id: &str, machine: &str) -> PlanAction {
    plan.changes
        .iter()
        .find(|c| c.resource_id == id && c.machine == machine)
        .unwrap_or_else(|| panic!("no change for {id}@{machine} in {plan:#?}"))
        .action
        .clone()
}

/// THE DEFECT, at the boundary. A converged task on a machine this host does
/// not answer for plans `NoOp` — unchanged — and is now NAMED, with the
/// machine and a reason. The file resource beside it declares nothing the
/// probe would have measured and is not.
#[test]
fn a_converged_task_on_a_machine_this_host_does_not_answer_for_is_named() {
    let config = config("far", "/nonexistent/forjar-497");
    let locks = converged_locks(&config, |_| HashMap::new());

    let plan = plan(&config, &order(), &locks, None);

    assert_eq!(action_of(&plan, "build", "far"), PlanAction::NoOp);
    assert_eq!(plan.unprobed.len(), 1, "{:#?}", plan.unprobed);
    let entry = &plan.unprobed[0];
    assert_eq!(entry.resource_id, "build");
    assert_eq!(entry.machine, "far");
    assert!(
        entry.reason.contains("far") && entry.reason.contains("not measured"),
        "the reason names the machine and says the I/O was not measured: {}",
        entry.reason
    );
}

/// The census changes NOTHING about the action. A plan that rebuilt every
/// unprobed resource would rebuild every remote task on every apply.
#[test]
fn the_census_never_changes_the_action() {
    let config = config("far", "/nonexistent/forjar-497");
    let locks = converged_locks(&config, |_| HashMap::new());

    let plan = plan(&config, &order(), &locks, None);

    assert_eq!((plan.to_create, plan.to_update, plan.to_destroy), (0, 0, 0));
    assert_eq!(plan.unchanged, 2);
    assert!(!plan.unprobed.is_empty(), "…and it still says so");
}

/// A resource that is going to run is not a silent gap.
#[test]
fn a_task_that_will_run_anyway_is_not_named() {
    let config = config("far", "/nonexistent/forjar-497");
    let mut locks = converged_locks(&config, |_| HashMap::new());
    locks.get_mut("far").expect("far lock").resources["build"].hash = "stale".to_string();

    let plan = plan(&config, &order(), &locks, None);

    assert_eq!(action_of(&plan, "build", "far"), PlanAction::Update);
    assert!(plan.unprobed.is_empty(), "{:#?}", plan.unprobed);
}

/// The suppression at zero: a task on THIS host is probed, so there is
/// nothing to disclose.
#[test]
fn a_probed_task_is_not_named() {
    let wd = WorkDir::new("probed");
    let config = config("box", &wd.path());
    let io = recorded_io(&config);
    let locks = converged_locks(&config, |id| {
        if id == "build" {
            io.clone()
        } else {
            HashMap::new()
        }
    });

    let plan = plan(&config, &order(), &locks, None);

    assert_eq!(
        action_of(&plan, "build", "box"),
        PlanAction::NoOp,
        "precondition: probed and unchanged"
    );
    assert!(plan.unprobed.is_empty(), "{:#?}", plan.unprobed);
}

/// The probe map is keyed by resource id alone, so a resource on both a
/// local and a remote machine carries the local probe's answer under its id.
/// That answer says nothing about the remote tree: the remote row is named,
/// the local row is not, and neither action moves.
#[test]
fn a_local_probe_says_nothing_about_a_remote_machine() {
    let wd = WorkDir::new("mixed");
    let config = config("[box, far]", &wd.path());
    let io = recorded_io(&config);
    let locks = converged_locks(&config, |id| {
        if id == "build" {
            io.clone()
        } else {
            HashMap::new()
        }
    });

    let plan = plan(&config, &order(), &locks, None);

    assert_eq!(action_of(&plan, "build", "box"), PlanAction::NoOp);
    assert_eq!(action_of(&plan, "build", "far"), PlanAction::NoOp);
    let named: Vec<(&str, &str)> = plan
        .unprobed
        .iter()
        .map(|u| (u.resource_id.as_str(), u.machine.as_str()))
        .collect();
    assert_eq!(named, vec![("build", "far")], "{:#?}", plan.unprobed);
}

/// A probe the caller COULD have taken and did not is disclosed with the
/// other reason — an empty map from an older caller is not "nothing stale".
#[test]
fn an_empty_probe_map_on_a_local_machine_is_disclosed_too() {
    let wd = WorkDir::new("emptymap");
    let config = config("box", &wd.path());
    let locks = converged_locks(&config, |_| HashMap::new());

    let plan = super::plan_with_probes(&config, &order(), &locks, None, &HashMap::new());

    assert_eq!(action_of(&plan, "build", "box"), PlanAction::NoOp);
    assert_eq!(plan.unprobed.len(), 1, "{:#?}", plan.unprobed);
    assert!(
        plan.unprobed[0].reason.contains("no probe was taken"),
        "{}",
        plan.unprobed[0].reason
    );
}

/// `probe_covers` is the predicate the probe, the executor and the census all
/// ask, so it must answer exactly as the probe skips.
#[test]
fn probe_covers_answers_for_this_host_only() {
    let config = config("far", "/nonexistent/forjar-497");
    assert!(probe_covers(&config, "box"), "loopback is this host");
    assert!(!probe_covers(&config, "far"), "TEST-NET is not");
    assert!(
        !probe_covers(&config, "absent"),
        "an undeclared machine is not"
    );
}

/// ONE definition. The predicate is inlined nowhere the census must agree
/// with: the executor's pre-plan probe and the census call `probe_covers`,
/// and the transport predicate it wraps is named in `probe.rs` alone.
#[test]
fn the_probe_coverage_predicate_has_one_definition() {
    const PROBE: &str = include_str!("../task/probe.rs");
    const EXECUTOR: &str = include_str!("../executor/mod.rs");
    const CENSUS: &str = include_str!("unprobed.rs");
    let raw = "crate::transport::controller_answers_for";

    assert_eq!(
        PROBE.matches(raw).count(),
        1,
        "probe.rs wraps the transport predicate exactly once"
    );
    assert!(
        EXECUTOR.contains("probe_covers("),
        "the executor asks probe_covers"
    );
    assert!(
        CENSUS.contains("probe_covers("),
        "the census asks probe_covers"
    );
    assert!(
        !EXECUTOR.contains(raw) && !CENSUS.contains(raw),
        "a second inlined copy is how the probe and the census drift apart"
    );
}

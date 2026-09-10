//! forjar#501: the FJ-2701 input cache reads the base the writer used, and
//! answers only for a machine this host answers for.
//!
//! `check_task_input_cache` hashed relative to the state directory's parent
//! while `record_io_hashes` hashed relative to `working_dir`; `hash_inputs`
//! folds the expanded path into the hash, so the two never agreed and the
//! cache was dead. And it read the lock's `input_hash` — a hash of THIS
//! host's tree — for any machine, so once the base agreed, a `cache: true`
//! task on a remote machine would be skipped from files that were never there.

use super::*;
use crate::core::state;

fn machine(addr: &str) -> Machine {
    serde_yaml_ng::from_str(&format!("hostname: far\naddr: {addr}")).expect("machine")
}

/// `working_dir` is `proj/`; the state directory is `state/` beside it, so
/// the state directory's parent is NOT the working directory — the base
/// mismatch the reader had.
fn fixture() -> (tempfile::TempDir, Resource) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("proj")).expect("proj");
    std::fs::create_dir_all(dir.path().join("state")).expect("state");
    std::fs::write(dir.path().join("proj/src.txt"), "v1").expect("input");
    let resource = Resource {
        resource_type: ResourceType::Task,
        task_inputs: vec!["src.txt".to_string()],
        working_dir: Some(dir.path().join("proj").display().to_string()),
        cache: true,
        ..Default::default()
    };
    (dir, resource)
}

/// The lock as a successful apply on `writer` leaves it.
fn converged_lock(resource: &Resource, writer: &Machine) -> StateLock {
    let mut details = HashMap::new();
    crate::core::task::probe::record_io_hashes(resource, writer, &mut details);
    let mut lock = state::new_lock("far", "far");
    lock.resources.insert(
        "build".to_string(),
        ResourceLock {
            resource_type: ResourceType::Task,
            status: ResourceStatus::Converged,
            applied_at: None,
            duration_seconds: None,
            hash: String::new(),
            observed: None,
            details,
        },
    );
    lock
}

#[test]
fn the_cache_reads_the_base_the_writer_used() {
    let (dir, resource) = fixture();
    let here = machine("127.0.0.1");
    let mut lock = converged_lock(&resource, &here);
    assert!(
        lock.resources["build"].details.contains_key("input_hash"),
        "precondition: the writer recorded a hash for this host"
    );
    let state_dir = dir.path().join("state");
    let ctx = RecordCtx {
        lock: &mut lock,
        state_dir: &state_dir,
        machine_name: "far",
        tripwire: false,
        failure_policy: &FailurePolicy::StopOnFirst,
        timeout_secs: None,
    };

    assert!(
        check_task_input_cache("build", &resource, &here, &ctx).is_some(),
        "unchanged inputs must hit: the reader hashed a different base than the writer"
    );

    std::fs::write(dir.path().join("proj/src.txt"), "v2").expect("mutate");
    assert!(
        check_task_input_cache("build", &resource, &here, &ctx).is_none(),
        "changed inputs must miss"
    );
}

#[test]
fn the_cache_has_no_answer_for_a_machine_this_host_cannot_read() {
    let (dir, resource) = fixture();
    let here = machine("127.0.0.1");
    // The lock as an older forjar wrote it: this host's hash under far's row.
    let mut lock = converged_lock(&resource, &here);
    let state_dir = dir.path().join("state");
    let ctx = RecordCtx {
        lock: &mut lock,
        state_dir: &state_dir,
        machine_name: "far",
        tripwire: false,
        failure_policy: &FailurePolicy::StopOnFirst,
        timeout_secs: None,
    };

    let far = machine("203.0.113.7");
    assert!(
        check_task_input_cache("build", &resource, &far, &ctx).is_none(),
        "a remote cache: true task was skipped from a hash of this host's files"
    );
}

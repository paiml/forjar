//! Tests: GH-244(c) ambient inputs.
//!
//! The load-bearing property is not "an ambient change is detected" but that
//! the probe and the lock composite the SAME way. A probe that folds the
//! ambient value in while `record_io_hashes` records the file-only hash reports
//! "inputs changed" on every plan forever, which is worse than the bug.

use super::ambient::{declares_inputs, hash_declared_inputs};
use super::io_tracking::hash_inputs;
use crate::core::types::{Resource, ResourceType};
use std::path::Path;

fn task(inputs: &[&str], ambient: &[&str], working_dir: Option<&str>) -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        task_inputs: inputs.iter().map(|s| (*s).to_string()).collect(),
        ambient_inputs: ambient.iter().map(|s| (*s).to_string()).collect(),
        working_dir: working_dir.map(str::to_string),
        ..Default::default()
    }
}

#[test]
fn a_resource_declaring_only_ambient_inputs_declares_inputs() {
    assert!(!declares_inputs(&task(&[], &[], None)));
    assert!(declares_inputs(&task(&["src/a.c"], &[], None)));
    assert!(declares_inputs(&task(&[], &["echo v1"], None)));
}

#[test]
fn no_ambient_inputs_is_byte_identical_to_the_file_only_hash() {
    // Back-compat: upgrading forjar must not invalidate every existing lock.
    let dir = std::env::temp_dir().join(format!("forjar-amb-id-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/a.c"), "int main(){}").unwrap();

    let r = task(&["src/a.c"], &[], dir.to_str());
    assert_eq!(
        hash_declared_inputs(&r, &dir),
        hash_inputs(&r.task_inputs, &dir).unwrap()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_ambient_command_changes_the_hash_when_its_stdout_changes() {
    let base = Path::new(".");
    let a = hash_declared_inputs(&task(&[], &["echo v1"], Some(".")), base);
    let b = hash_declared_inputs(&task(&[], &["echo v2"], Some(".")), base);
    assert!(
        a.is_some(),
        "an ambient-only resource still has an input hash"
    );
    assert_ne!(a, b, "the ambient fingerprint is not in the hash");
}

#[test]
fn the_same_ambient_command_settles() {
    // f(f(x)) = f(x): the fingerprint must not be a source of churn.
    let base = Path::new(".");
    let r = task(&[], &["echo stable"], Some("."));
    assert_eq!(
        hash_declared_inputs(&r, base),
        hash_declared_inputs(&r, base)
    );
}

#[test]
fn a_failing_ambient_command_is_not_silently_dropped() {
    // Dropping it collapses the hash back to the file-only value, which is
    // "report clean over a stale artifact" — the bug this feature closes.
    let base = Path::new(".");
    let failing = hash_declared_inputs(&task(&[], &["exit 7"], Some(".")), base);
    assert!(failing.is_some());
    assert_ne!(
        failing,
        hash_declared_inputs(&task(&[], &[], Some(".")), base),
        "a broken fingerprint collapsed to the no-ambient hash"
    );
    assert_ne!(
        failing,
        hash_declared_inputs(&task(&[], &["exit 9"], Some(".")), base),
        "two different failures hash the same"
    );
}

#[test]
fn stderr_is_not_hashed() {
    // stderr routinely carries a pid or a timestamp; folding it in would report
    // "inputs changed" on every plan.
    let base = Path::new(".");
    let a = hash_declared_inputs(&task(&[], &["echo same; echo $$ >&2"], Some(".")), base);
    let b = hash_declared_inputs(&task(&[], &["echo same; echo $$ >&2"], Some(".")), base);
    assert_eq!(a, b);
}

#[test]
fn ambient_commands_run_in_the_resources_working_dir() {
    // Derived from the RESOURCE, not from the caller's base_dir: the executor
    // passes state_dir.parent() where the probe passes working_dir, and an
    // ambient component that differed between them would pump forever.
    let dir = std::env::temp_dir().join(format!("forjar-amb-cwd-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("fonts.txt"), "v1").unwrap();

    let r = task(&[], &["cat fonts.txt"], dir.to_str());
    let from_probe = hash_declared_inputs(&r, &dir);
    let from_executor = hash_declared_inputs(&r, Path::new("."));
    assert_eq!(
        from_probe, from_executor,
        "the ambient component depended on which caller asked"
    );

    std::fs::write(dir.join("fonts.txt"), "v2").unwrap();
    assert_ne!(from_probe, hash_declared_inputs(&r, &dir));
    let _ = std::fs::remove_dir_all(&dir);
}

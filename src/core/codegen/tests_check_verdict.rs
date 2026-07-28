//! FJ-2720 (PMAT-199): a check script must report its verdict in its EXIT CODE.
//!
//! # The defect these tests pin
//!
//! Every generator emitted its verdict as a stdout marker in the shape
//! `<test> && echo 'exists:x' || echo 'missing:x'`. That pipeline always exits
//! 0 — an `if/else` where both branches are `echo` cannot fail. Meanwhile
//! `cli::check::run_single_check` decides pass/fail purely on `out.success()`,
//! i.e. the exit code, and NOTHING anywhere parsed the markers.
//!
//! The two halves never met, so `forjar check` reported `pass` for every
//! resource unconditionally. Verified on the published 1.11.1 binary against a
//! config that had never been applied, in an empty directory:
//!
//! ```text
//!   ok never-applied-file (local)
//!   ok never-applied-task (local)
//!   Check: 2 pass, 0 fail, 0 skip     exit=0
//! ```
//!
//! `forjar apply --check` shares the path, so its documented "exit 2 = changes
//! needed" (FJ-226) was unreachable too.
//!
//! # Why the test runs the script
//!
//! Asserting on the generated TEXT is what let this survive: the text was
//! always plausible. These tests EXECUTE the script against a real filesystem
//! and assert on the exit status, because the exit status is the entire
//! contract. A generator can only pass by actually being right.

use super::*;
use crate::core::types::{Resource, ResourceType};

/// Run a generated check script the way the transport does and return its code.
fn run(script: &str) -> i32 {
    let out = std::process::Command::new("sh")
        .arg("-c")
        .arg(script)
        .output()
        .expect("sh is available");
    out.status.code().unwrap_or(-1)
}

fn tmp(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("forjar-ckv-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn absent_file_check_exits_nonzero() {
    let d = tmp("file");
    let mut r = Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    };
    r.path = Some(d.join("definitely-absent.txt").display().to_string());
    r.state = Some("file".to_string());

    let script = check_script(&r).expect("file has a check script");
    assert_ne!(
        run(&script),
        0,
        "a file that does not exist must FAIL its check, not pass:\n{script}"
    );

    // And the converged case must still pass, or the fix is just "always fail".
    std::fs::write(d.join("definitely-absent.txt"), "x").unwrap();
    assert_eq!(
        run(&script),
        0,
        "a file that exists must PASS its check:\n{script}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn absent_directory_check_exits_nonzero() {
    let d = tmp("dir");
    let mut r = Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    };
    r.path = Some(d.join("no-such-dir").display().to_string());
    r.state = Some("directory".to_string());

    let script = check_script(&r).unwrap();
    assert_ne!(run(&script), 0, "missing directory must fail:\n{script}");

    std::fs::create_dir_all(d.join("no-such-dir")).unwrap();
    assert_eq!(run(&script), 0, "present directory must pass:\n{script}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn file_state_absent_is_inverted_correctly() {
    // `state: absent` converges when the path is GONE. A naive "any negative
    // marker fails" rule would invert this, so it is pinned explicitly.
    let d = tmp("absent");
    let target = d.join("should-not-exist");
    let mut r = Resource {
        resource_type: ResourceType::File,
        ..Default::default()
    };
    r.path = Some(target.display().to_string());
    r.state = Some("absent".to_string());

    let script = check_script(&r).unwrap();
    assert_eq!(
        run(&script),
        0,
        "state:absent with the path already gone is CONVERGED:\n{script}"
    );

    std::fs::write(&target, "oops").unwrap();
    assert_ne!(
        run(&script),
        0,
        "state:absent with the path present must FAIL:\n{script}"
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn task_with_missing_output_artifact_exits_nonzero() {
    // The exact shape that reported `ok link` after `rm build/demo`.
    let d = tmp("task");
    let r = Resource {
        resource_type: ResourceType::Task,
        output_artifacts: vec![d.join("demo").display().to_string()],
        ..Default::default()
    };

    let script = check_script(&r).unwrap();
    assert_ne!(
        run(&script),
        0,
        "a task whose output artifact was deleted must FAIL:\n{script}"
    );

    std::fs::write(d.join("demo"), "bin").unwrap();
    assert_eq!(run(&script), 0, "artifact present must pass:\n{script}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn task_completion_check_drives_the_exit_code() {
    let mut r = Resource {
        resource_type: ResourceType::Task,
        ..Default::default()
    };

    r.completion_check = Some("false".to_string());
    assert_ne!(
        run(&check_script(&r).unwrap()),
        0,
        "a failing completion_check must fail the resource check"
    );

    r.completion_check = Some("true".to_string());
    assert_eq!(
        run(&check_script(&r).unwrap()),
        0,
        "a passing completion_check must pass"
    );
}

#[test]
fn absent_user_check_exits_nonzero() {
    let mut r = Resource {
        resource_type: ResourceType::User,
        ..Default::default()
    };
    r.name = Some("forjar-no-such-user-ckv".to_string());
    assert_ne!(
        run(&check_script(&r).unwrap()),
        0,
        "a user that does not exist must fail its check"
    );

    // A user that certainly does exist on any POSIX host.
    r.name = Some("root".to_string());
    assert_eq!(
        run(&check_script(&r).unwrap()),
        0,
        "root exists, so its check must pass"
    );
}

#[test]
fn absent_docker_container_check_exits_nonzero() {
    // Passes whether or not docker is installed: with no docker binary the
    // script must still not claim the container is present.
    let mut r = Resource {
        resource_type: ResourceType::Docker,
        ..Default::default()
    };
    r.name = Some("forjar-no-such-container-ckv".to_string());
    r.image = Some("scratch".to_string());
    assert_ne!(
        run(&check_script(&r).unwrap()),
        0,
        "a container that does not exist must fail its check"
    );
}

#[test]
fn a_task_declaring_nothing_is_not_reported_as_converged() {
    // With no artifacts and no completion_check there is no evidence of
    // convergence. Claiming `pass` here is the unconditional-success bug in
    // miniature, so the honest answer is "not converged".
    let r = Resource {
        resource_type: ResourceType::Task,
        command: Some("echo hi".to_string()),
        ..Default::default()
    };
    assert_ne!(
        run(&check_script(&r).unwrap()),
        0,
        "a task with no completion evidence must not report converged"
    );
}

/// COMPLETENESS: no resource type may generate a check script that cannot fail.
///
/// This is the test that would have caught the original defect. A generator
/// added later inherits it automatically, so the class cannot silently return.
#[test]
fn no_resource_type_generates_an_unfailable_check_script() {
    let d = tmp("completeness");
    let absent = d.join("absent-everything");

    // Each type configured to reference something that certainly does NOT
    // exist. Types whose check is inherently unavailable on a dev host (no
    // systemd, no docker) must still fail rather than pass.
    let cases: Vec<(String, Resource)> = ALL_CHECKABLE_TYPES
        .iter()
        .map(|ty| {
            let mut r = Resource {
                resource_type: ty.clone(),
                ..Default::default()
            };
            r.name = Some("forjar-absent-ckv".to_string());
            r.path = Some(absent.display().to_string());
            r.state = Some("file".to_string());
            r.image = Some("forjar/absent:ckv".to_string());
            r.schedule = Some("0 * * * *".to_string());
            r.command = Some("/bin/true".to_string());
            r.port = Some("65534".to_string());
            r.packages = vec!["forjar-absent-pkg-ckv".to_string()];
            r.output_artifacts = vec![absent.display().to_string()];
            (ty.to_string(), r)
        })
        .collect();

    let mut unfailable = Vec::new();
    for (label, r) in &cases {
        let Ok(script) = check_script(r) else {
            continue; // no check script for this type is a separate concern
        };
        if run(&script) == 0 {
            unfailable.push(format!("{label}: script cannot fail:\n{script}\n"));
        }
    }

    assert!(
        unfailable.is_empty(),
        "these check scripts report success for a resource that does not exist \
         — the exact defect that made `forjar check` a no-op:\n\n{}",
        unfailable.join("\n")
    );
    let _ = std::fs::remove_dir_all(&d);
}

/// GPU is checked separately by [`gpu_check_is_not_unconditional`].
///
/// The sweep above configures each resource to reference something that does
/// not exist. That premise does not hold for GPU: its check observes HARDWARE,
/// and on a machine with a driver installed (this repo is developed on one)
/// reporting `match` is the CORRECT answer, not an unconditional pass. Leaving
/// it in the sweep would make the suite assert something host-dependent — a
/// test that fails for the wrong reason teaches the wrong lesson.
#[test]
fn gpu_check_is_not_unconditional() {
    // Host-independent invariant: `present` and `absent` are complementary, so
    // on ANY host exactly one of them must fail. An unfailable script passes
    // both and is caught here whether or not a GPU exists.
    let mut present = Resource {
        resource_type: ResourceType::Gpu,
        ..Default::default()
    };
    present.name = Some("gpu0".to_string());
    present.state = Some("present".to_string());

    let mut absent = present.clone();
    absent.state = Some("absent".to_string());

    let p = run(&check_script(&present).unwrap());
    let a = run(&check_script(&absent).unwrap());
    assert_ne!(
        (p == 0),
        (a == 0),
        "state:present and state:absent must not agree — exactly one describes \
         this host. present exited {p}, absent exited {a}"
    );
}

/// Types whose `check_script` asserts a condition about the world.
///
/// `Recipe` is excluded: it is a config-inclusion directive with no runtime
/// state to observe. `Gpu` is excluded for the reason above.
const ALL_CHECKABLE_TYPES: &[ResourceType] = &[
    ResourceType::Package,
    ResourceType::File,
    ResourceType::Service,
    ResourceType::Mount,
    ResourceType::User,
    ResourceType::Docker,
    ResourceType::Pepita,
    ResourceType::Network,
    ResourceType::Cron,
    ResourceType::Model,
    ResourceType::Task,
    ResourceType::WasmBundle,
    ResourceType::Image,
    ResourceType::Build,
    ResourceType::GithubRelease,
    ResourceType::OverlayInterface,
];

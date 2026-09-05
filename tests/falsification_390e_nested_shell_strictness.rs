//! Refs #390-E: a nested shell must not silently discard strictness, stdin, or
//! the rest of its own script.
//!
//! THE FLAW THIS CLOSES.
//!
//! `timeout:` and `sudo: true` wrapped the user's command in
//! `bash <<'FORJAR_TIMEOUT'` / `sudo bash <<'FORJAR_SUDO'`. Three things follow
//! from that shape, and the first is silent corruption:
//!
//! 1. The nested shell does not inherit the outer `set -euo pipefail`, and a
//!    shell exits with the status of its LAST command. Measured on the PUBLISHED
//!    1.24.0 binary: a task with `timeout: 30`, a passing `completion_check`, and
//!    a command beginning with `false` reported `1 converged, 0 failed` and exit
//!    0 — while the identical config with the `timeout:` line deleted correctly
//!    failed with exit 1. Apply called a wrong result a success.
//!
//! 2. The nested bash's stdin IS the heredoc, so a command that reads stdin
//!    consumes the remainder of its own script. That is FJ-2732 — the defect
//!    `transport::stdin_isolation` was written to close — re-opened one layer in,
//!    below the wrapper that closes it.
//!
//! 3. A fixed delimiter collides. A command containing `FORJAR_TIMEOUT` on its
//!    own line closes the heredoc early and the remaining lines run in the OUTER
//!    shell. Reproduced against 1.24.0: `FORJAR_TIMEOUT: command not found`
//!    followed by the lines that were meant to be inside. For `sudo:` this is
//!    worse than untidy — it runs unprivileged the commands an author asked to
//!    run as root.
//!
//! WHAT THIS TEST MUST NOT BECOME. Asserting on the generated text alone would
//! pass against a script that still behaves wrongly, so the strictness and stdin
//! cases EXECUTE the generated script through a real shell and assert on what
//! happened to the filesystem.

use forjar::core::types::{MachineTarget, Resource, ResourceType};
use std::process::Command;

fn task(command: &str, timeout: Option<u64>) -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        machine: MachineTarget::Single("localhost".into()),
        command: Some(command.to_string()),
        timeout,
        ..Default::default()
    }
}

/// Run a generated script the way the transport does: stdin closed.
fn run(script: &str) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.sh");
    std::fs::write(&path, script).unwrap();
    Command::new("bash")
        .arg(&path)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("bash must run")
}

#[test]
fn a_failing_line_under_timeout_is_not_swallowed() {
    // THE REGRESSION, and the reason this is the highest-severity item from
    // #390: without the fix the script exits 0 and apply records Converged.
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("ran-after-failure");
    let cmd = format!("false\ntouch {}\n", marker.display());
    let script = forjar::core::codegen::apply_script(&task(&cmd, Some(30))).unwrap();

    let out = run(&script);
    assert!(
        !out.status.success(),
        "a failing line inside `timeout:` must fail the script.\n\
         Exit 0 here is apply reporting a wrong result as success.\n--- script ---\n{script}"
    );
    assert!(
        !marker.exists(),
        "the line after `false` RAN — the nested shell did not inherit `set -e`"
    );
}

#[test]
fn the_same_command_without_timeout_already_failed() {
    // THE CONTROL. Without it the test above could pass because of something
    // unrelated to `timeout:`. This is the behaviour `timeout:` diverged from.
    let script = forjar::core::codegen::apply_script(&task("false\ntrue\n", None)).unwrap();
    assert!(
        !run(&script).status.success(),
        "the un-wrapped path must already be strict; if this fails the premise is wrong"
    );
}

#[test]
fn a_stdin_reader_under_timeout_does_not_eat_its_own_script() {
    // FJ-2732, one layer in. `cat` consumed the rest of the heredoc, so the
    // second line never ran and the script still exited 0.
    let dir = tempfile::tempdir().unwrap();
    let eaten = dir.path().join("eaten");
    let second = dir.path().join("second");
    let cmd = format!("cat > {}\ntouch {}\n", eaten.display(), second.display());
    let script = forjar::core::codegen::apply_script(&task(&cmd, Some(30))).unwrap();

    run(&script);
    assert!(
        second.exists(),
        "the line after a stdin-reading command did not run — the nested shell's \
         stdin was its own script.\n--- script ---\n{script}"
    );
    assert!(
        std::fs::metadata(&eaten).map(|m| m.len()).unwrap_or(0) == 0,
        "the stdin-reader captured script text, which means it read the heredoc"
    );
}

#[test]
fn a_command_containing_the_delimiter_does_not_escape_the_heredoc() {
    // C8 delimiter collision. A bare `FORJAR_TIMEOUT` line closed the heredoc
    // and the rest executed in the OUTER shell.
    let script = forjar::core::codegen::apply_script(&task(
        "echo one\nFORJAR_TIMEOUT\necho two\n",
        Some(30),
    ))
    .unwrap();

    // The chosen delimiter must not appear in the body it is supposed to bound.
    let delim_line = script
        .lines()
        .find(|l| l.contains("3<<'"))
        .expect("a heredoc must be emitted for a timeout task");
    let delim = delim_line
        .split("3<<'")
        .nth(1)
        .and_then(|r| r.split('\'').next())
        .expect("delimiter must be parseable");
    assert_ne!(
        delim, "FORJAR_TIMEOUT",
        "the delimiter collides with the command body — the heredoc closes early"
    );
    assert!(
        script.contains(&format!("3<<'{delim}'")) && script.contains(&format!("\n{delim}\n")),
        "delimiter must open and close the heredoc"
    );
}

#[test]
fn the_sudo_wrapper_is_stdin_isolated_and_collision_free() {
    let mut r = task("echo hi\n", None);
    r.sudo = true;
    let script = forjar::core::codegen::apply_script(&r).unwrap();
    // PMAT-158: fd 3 was the first answer and sudo closed it before exec, so
    // the script now crosses the boundary as a private temp file.
    assert!(
        script.contains("cat >\"$forjar_sudo_script\" <<'")
            && script.contains("sudo bash \"$forjar_sudo_script\""),
        "sudo must pass the script as a file it wrote, leaving stdin free.\n--- script ---\n{script}"
    );

    let mut collide = task("echo hi\nFORJAR_SUDO\necho bye\n", None);
    collide.sudo = true;
    let s2 = forjar::core::codegen::apply_script(&collide).unwrap();
    assert!(
        !s2.contains("3<<'FORJAR_SUDO'"),
        "a script mentioning FORJAR_SUDO must not use it as its own delimiter — \
         for a sudo: resource that runs the remainder UNPRIVILEGED"
    );
}

#[test]
fn the_delimiter_is_deterministic() {
    // recipe-determinism-v1: the generated script is a pure function of the
    // declaration, so the collision-avoiding delimiter must be too.
    let r = task("echo x\nFORJAR_TIMEOUT\n", Some(5));
    let a = forjar::core::codegen::apply_script(&r).unwrap();
    let b = forjar::core::codegen::apply_script(&r).unwrap();
    assert_eq!(a, b, "delimiter selection must be deterministic");
}

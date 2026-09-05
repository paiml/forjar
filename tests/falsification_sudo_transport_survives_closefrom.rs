//! Refs PMAT-159: `sudo: true` ran nothing for a non-root user.
//!
//! THE FLAW THIS CLOSES.
//!
//! #390-E moved the sudo wrapper off stdin — `sudo bash <<'D'` let a
//! stdin-reading command eat the rest of its own script — onto descriptor 3:
//! `sudo bash /dev/fd/3 3<<'D'`. sudo closes every descriptor >= 3 before it
//! execs the command (`closefrom`, on by default), so the elevated bash opened
//! `/dev/fd/3` and found nothing:
//!
//! ```text
//!   bash: /dev/fd/3: No such file or directory        exit 127
//! ```
//!
//! Measured 2026-09-05 on lambda-labs against forjar 1.24.0. It is every
//! `sudo: true` resource on every host where forjar runs unprivileged — apply,
//! check and state_query alike, because all three share the wrapper. The repro
//! is two lines: `sudo bash /dev/fd/3 3<<<'echo hi'` fails and
//! `sudo bash -s <<<'echo hi'` prints `hi`. The unit tests in
//! `src/core/codegen/tests_sudo.rs` asserted the TEXT
//! `sudo bash /dev/fd/3 3<<'FORJAR_SUDO'` and were green throughout.
//!
//! WHAT THIS TEST MUST NOT BECOME: another text assertion. It EXECUTES the
//! generated wrapper under the host's real sudo, fed to bash the way every
//! transport feeds it (on stdin, inside the stdin-isolation brace group), and
//! asserts on the exit status, on stdout, and on the temp file the wrapper
//! must not leave behind. Reverting the emitter to the fd-3 form makes every
//! gated test here fail with exit 127.
//!
//! THIS IS THE LIVE-PRIVILEGE CONFIRMATION, NOT THE ALWAYS-ON FALSIFIER.
//! Every test below is gated on real sudo, so on a host without it this file
//! executes NOTHING and would have been green against the fd-3 emitter too.
//! The always-on falsifier is
//! `tests/falsification_sudo_transport_closefrom_emulated.rs`, which needs no
//! privilege: it reproduces sudo's closefrom with a fake `sudo` on `PATH` and
//! runs everywhere. This file adds what emulation cannot: the wrapper under
//! the host's REAL sudo, with a real uid 0 inside.
//!
//! HOST GATE, AND HOW IT FAILS CLOSED. The sudo branch is only taken when
//! uid != 0, and executing it needs a passwordless `sudo -n true`. Where either
//! is missing the test prints `SKIP: <test>: <capability>` and returns — the
//! convention `tests/container_transport.rs` uses for docker. A skip is an
//! absence of evidence, so a caller that needs the evidence sets
//! `FORJAR_REQUIRE_SUDO_TESTS=1` and the same missing capability PANICS instead,
//! naming the test and what was missing.

use forjar::core::codegen;
use forjar::core::types::{MachineTarget, Resource, ResourceType};
use forjar::transport::stdin_isolation::wrap_script_stdin_isolated;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Set to `1` to turn a skip into a failure. A gated test that prints SKIP
/// proves nothing; `scripts/quorum-gate.sh` executes a receipt's
/// `falsification.cargo_test_target` and reads green as verification, so a
/// caller that is relying on THIS file rather than the emulated one must be
/// able to demand that it actually ran.
const REQUIRE_ENV: &str = "FORJAR_REQUIRE_SUDO_TESTS";

/// Skip, or fail closed when the caller demanded the capability.
fn missing(test: &str, capability: &str) -> bool {
    if std::env::var(REQUIRE_ENV).as_deref() == Ok("1") {
        panic!(
            "{REQUIRE_ENV}=1 but {test} cannot run: {capability}. \
             This test is the live-privilege confirmation of the sudo \
             transport; the always-on falsifier is \
             tests/falsification_sudo_transport_closefrom_emulated.rs."
        );
    }
    eprintln!("SKIP: {test}: {capability}");
    false
}

/// Can this host take the wrapper's sudo branch and execute it?
fn sudo_branch_is_executable(test: &str) -> bool {
    let uid = Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    if uid == "0" {
        return missing(
            test,
            "running as root, so the wrapper's sudo branch is never taken",
        );
    }
    let ok = Command::new("sudo")
        .args(["-n", "true"])
        .stdin(Stdio::null())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        return missing(
            test,
            "`sudo -n true` fails on this host (no passwordless sudo)",
        );
    }
    true
}

/// Guard every sudo-executing test on the prerequisite.
macro_rules! require_sudo {
    ($test:literal) => {
        if !sudo_branch_is_executable($test) {
            return;
        }
    };
}

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Run a generated script the way the transports do: written to bash's STDIN,
/// inside the stdin-isolation brace group. `TMPDIR` is pointed at `tmpdir` so
/// the wrapper's temp file — if it makes one — lands where the test can look.
fn run_as_transport(script: &str, tmpdir: &Path) -> Run {
    let mut child = Command::new("bash")
        .env("TMPDIR", tmpdir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bash must spawn");
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(wrap_script_stdin_isolated(script).as_bytes())
        .expect("write script to bash's stdin");
    let out = child.wait_with_output().expect("bash must finish");
    Run {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn sudo_task(command: &str, timeout: Option<u64>) -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        machine: MachineTarget::Single("localhost".into()),
        command: Some(command.to_string()),
        timeout,
        sudo: true,
        ..Default::default()
    }
}

/// Names left in a directory the wrapper was told to use for its temp file.
fn leftovers(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .expect("read TMPDIR")
        .map(|e| {
            e.expect("dir entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[test]
fn the_sudo_wrapper_runs_the_script_as_root_and_removes_its_temp_file() {
    require_sudo!("the_sudo_wrapper_runs_the_script_as_root_and_removes_its_temp_file");
    let tmp = tempfile::tempdir().unwrap();
    // `$0` is the path bash was handed, so the script reports which file it
    // ran from — the "temp file is gone" assertion below is then about a file
    // that demonstrably existed, not vacuously about an empty directory.
    let script = codegen::apply_script(&sudo_task(
        "echo forjar-sudo-ran\nid -u\necho \"$0\"\n",
        None,
    ))
    .unwrap();

    let r = run_as_transport(&script, tmp.path());
    assert_eq!(
        r.code,
        Some(0),
        "the sudo wrapper did not run the script.\nstderr: {}\n--- script ---\n{script}",
        r.stderr
    );
    let lines: Vec<&str> = r.stdout.lines().collect();
    assert_eq!(
        lines.first().copied(),
        Some("forjar-sudo-ran"),
        "stdout: {:?}",
        r.stdout
    );
    assert_eq!(
        lines.get(1).copied(),
        Some("0"),
        "the script did not run as root.\nstdout: {:?}",
        r.stdout
    );
    let ran_from = Path::new(lines.get(2).copied().unwrap_or_default());
    assert!(
        ran_from.starts_with(tmp.path()),
        "the script must have been run from a private temp file under TMPDIR; \
         it ran from {}",
        ran_from.display()
    );
    assert!(
        !ran_from.exists(),
        "the wrapper left its temp file behind: {}",
        ran_from.display()
    );
    assert!(
        leftovers(tmp.path()).is_empty(),
        "TMPDIR is not empty after the run: {:?}",
        leftovers(tmp.path())
    );
}

#[test]
fn a_failing_script_propagates_its_exit_code_and_still_removes_the_temp_file() {
    require_sudo!("a_failing_script_propagates_its_exit_code_and_still_removes_the_temp_file");
    let tmp = tempfile::tempdir().unwrap();
    let script = codegen::apply_script(&sudo_task("exit 42\n", None)).unwrap();

    let r = run_as_transport(&script, tmp.path());
    assert_eq!(
        r.code,
        Some(42),
        "the inner script's exit code must reach the transport unchanged \
         (127 here is the fd-3 form: bash never opened the script).\nstderr: {}",
        r.stderr
    );
    assert!(
        leftovers(tmp.path()).is_empty(),
        "the temp file must be removed on the failure path too: {:?}",
        leftovers(tmp.path())
    );
}

#[test]
fn stdin_under_sudo_is_the_transports_not_the_script() {
    // The property fd 3 was chosen for (#390-E), kept: the elevated script's
    // stdin is whatever the transport left it — here the brace group's
    // /dev/null — so `cat` reads EOF instead of the rest of its own script.
    require_sudo!("stdin_under_sudo_is_the_transports_not_the_script");
    let tmp = tempfile::tempdir().unwrap();
    let script = codegen::apply_script(&sudo_task("cat\necho second-line-ran\n", None)).unwrap();

    let r = run_as_transport(&script, tmp.path());
    assert_eq!(r.code, Some(0), "stderr: {}", r.stderr);
    assert_eq!(
        r.stdout, "second-line-ran\n",
        "either `cat` printed script text (stdin was the script) or the line \
         after it never ran"
    );
}

#[test]
fn the_timeout_wrapper_still_runs_under_sudo() {
    // `timeout:` also passes its command on descriptor 3 (`timeout N bash
    // /dev/fd/3 3<<'D'`), and that form is NOT affected: the descriptor is
    // opened by the elevated bash itself, after sudo's closefrom, and
    // `timeout` closes nothing. This pins the combination so a future "fix
    // every /dev/fd/3" sweep cannot break a working emitter unnoticed.
    require_sudo!("the_timeout_wrapper_still_runs_under_sudo");
    let tmp = tempfile::tempdir().unwrap();
    let script = codegen::apply_script(&sudo_task("echo under-timeout\n", Some(30))).unwrap();

    let r = run_as_transport(&script, tmp.path());
    assert_eq!(
        r.code,
        Some(0),
        "stderr: {}\n--- script ---\n{script}",
        r.stderr
    );
    assert_eq!(r.stdout, "under-timeout\n");
    assert!(leftovers(tmp.path()).is_empty());
}

#[test]
fn the_check_script_crosses_the_same_boundary() {
    // #349 made check and state_query share the wrapper, so the defect was
    // theirs too: every `sudo: true` resource read `missing:` (exit 127) on
    // every check, whatever was on disk.
    require_sudo!("the_check_script_crosses_the_same_boundary");
    let tmp = tempfile::tempdir().unwrap();
    let probe_dir = tempfile::tempdir().unwrap();
    let probe = probe_dir.path().join("probe.txt");
    std::fs::write(&probe, "present").unwrap();
    let r = Resource {
        resource_type: ResourceType::File,
        path: Some(probe.to_string_lossy().into_owned()),
        state: Some("file".to_string()),
        sudo: true,
        ..Default::default()
    };
    let script = codegen::check_script(&r).unwrap();

    let run = run_as_transport(&script, tmp.path());
    assert_eq!(
        run.code,
        Some(0),
        "an existing file must check as converged.\nstderr: {}\n--- script ---\n{script}",
        run.stderr
    );
    assert!(
        run.stdout.contains("exists:file"),
        "the check's verdict marker is missing: {:?}",
        run.stdout
    );
    assert!(leftovers(tmp.path()).is_empty());
}

#[test]
fn the_same_task_without_sudo_already_ran() {
    // THE CONTROL: the failures above are the wrapper's, not the task's or
    // the transport's. Not gated — it takes no privileged branch.
    let tmp = tempfile::tempdir().unwrap();
    let mut r = sudo_task("echo plain\n", None);
    r.sudo = false;
    let script = codegen::apply_script(&r).unwrap();
    let run = run_as_transport(&script, tmp.path());
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert_eq!(run.stdout, "plain\n");
}

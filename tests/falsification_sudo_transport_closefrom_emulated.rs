//! Refs PMAT-159: `sudo: true` ran nothing for a non-root user — asserted on
//! EVERY host, with no privilege required.
//!
//! THE FLAW THIS CLOSES, AND THE SECOND ONE BEHIND IT.
//!
//! The first: #390-E moved the sudo wrapper off stdin onto descriptor 3,
//! `sudo bash /dev/fd/3 3<<'D'`. sudo closes every descriptor >= 3 before it
//! execs the command (`closefrom`, on by default), so the elevated bash opened
//! `/dev/fd/3`, found nothing, and exited 127 — apply, check and state_query
//! alike, since all three share the wrapper.
//!
//! ```text
//!   bash: /dev/fd/3: No such file or directory        exit 127
//! ```
//!
//! The second, and the reason this file exists next to
//! `falsification_sudo_transport_survives_closefrom.rs`: that test EXECUTES the
//! wrapper under the host's REAL sudo, and is therefore gated on `uid != 0` and
//! a passwordless `sudo -n true`. On a host with neither — a CI container
//! running as root is the normal case — it printed `SKIP` and returned, having
//! executed nothing, AND WOULD HAVE DONE THE SAME AGAINST THE BROKEN EMITTER.
//! `scripts/quorum-gate.sh` runs the receipt's `falsification.cargo_test_target`
//! and reads green as verification, so a vacuously green falsifier is precisely
//! the hole the gate exists to close.
//!
//! HOW THIS ONE NEEDS NO PRIVILEGE. It does not elevate; it reproduces the only
//! property of sudo the transport depends on. A fake `sudo` first on `PATH`
//! closes every descriptor >= 3 it inherited and then execs its arguments —
//! `closefrom`, and nothing else — and a fake `id` reports uid 1000 so the
//! emitted wrapper takes its non-root branch even when the test itself runs as
//! root. The generated script then goes to `bash` on stdin inside
//! `wrap_script_stdin_isolated`, exactly as every transport feeds it.
//!
//! THE FIXTURE PROBES ITSELF. `the_fake_sudo_really_emulates_closefrom` runs the
//! OLD fd-3 form through the same fake and requires it to fail with
//! `/dev/fd/3: No such file or directory` and exit 127. Without that, a fake
//! that quietly stopped closing descriptors would make every assertion below
//! pass for the wrong reason — the "0 violations over 0 files" shape.
//!
//! Measured 2026-09-05 on lambda-labs against forjar 1.24.0; the repro is two
//! lines, `sudo bash /dev/fd/3 3<<<'echo hi'` fails and `sudo bash -s <<<'echo
//! hi'` prints `hi`. The unit tests asserted the TEXT of the fd-3 form and were
//! green for the whole life of the defect.

use forjar::core::codegen;
use forjar::core::types::{MachineTarget, Resource, ResourceType};
use forjar::transport::stdin_isolation::wrap_script_stdin_isolated;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// A `sudo` that does the one thing that broke the fd-3 transport, and a
/// non-root `id` so the wrapper takes the branch under test.
///
/// The emulator must NOT redirect its own stderr: bash implements a
/// `2>/dev/null` on a compound command by stashing the real descriptor 2 on a
/// high descriptor, which this very loop would then close — swallowing the
/// `No such file or directory` this fixture exists to provoke. (Found by
/// running the old form through an earlier draft and getting exit 127 with an
/// empty stderr.)
const FAKE_SUDO: &str = r#"#!/usr/bin/env bash
# Emulate sudo's closefrom(3): close every inherited descriptor >= 3, then exec
# the command. 0, 1 and 2 are preserved, exactly as sudo preserves them. 255 is
# bash's own terminal descriptor and is not inherited from the caller.
for fd in /proc/self/fd/*; do
  n=${fd##*/}
  case "$n" in ''|*[!0-9]*) continue ;; esac
  [ "$n" -lt 3 ] && continue
  [ "$n" -eq 255 ] && continue
  eval "exec $n>&-" || true
done
exec "$@"
"#;

/// `id -u` must answer non-root, or the wrapper runs the root branch and the
/// sudo transport is never exercised at all (the CI-container case).
const FAKE_ID: &str = r#"#!/usr/bin/env bash
if [ "$1" = "-u" ]; then
  echo 1000
  exit 0
fi
for real in /usr/bin/id /bin/id; do
  [ -x "$real" ] && exec "$real" "$@"
done
echo "fake id: no real id(1) found" >&2
exit 127
"#;

/// Write the fake toolchain into `dir` and return `dir` for `PATH`.
fn fake_bin(dir: &Path) -> PathBuf {
    for (name, body) in [("sudo", FAKE_SUDO), ("id", FAKE_ID)] {
        let p = dir.join(name);
        std::fs::write(&p, body).expect("write fake");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755))
                .expect("chmod fake");
        }
    }
    dir.to_path_buf()
}

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Feed `script` to bash the way every transport does — on STDIN, inside the
/// stdin-isolation brace group — with the fake toolchain first on `PATH` and
/// `TMPDIR` pointed at `tmpdir` so the wrapper's temp file lands where the test
/// can look for it.
fn run_as_transport(script: &str, bin: &Path, tmpdir: &Path) -> Run {
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut child = Command::new("bash")
        .env("PATH", &path)
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

/// Names left in the directory the wrapper was told to use for its temp file.
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

/// A fresh (fake-bin, TMPDIR) pair. Both are `TempDir`s and must outlive the
/// run, so they are returned rather than dropped at the end of a helper.
fn fixture() -> (tempfile::TempDir, tempfile::TempDir) {
    let bin = tempfile::tempdir().unwrap();
    fake_bin(bin.path());
    (bin, tempfile::tempdir().unwrap())
}

// ---------------------------------------------------------------------------
// The instrument, probed with an input it must reject.
// ---------------------------------------------------------------------------

#[test]
fn the_fake_sudo_really_emulates_closefrom() {
    let (bin, tmp) = fixture();
    // The #390-E form, verbatim. If the fake still passes descriptor 3 through,
    // every assertion in this file would pass against the broken emitter too.
    let r = run_as_transport(
        "sudo bash /dev/fd/3 3<<<'echo hi'\n",
        bin.path(),
        tmp.path(),
    );
    assert_eq!(
        r.code,
        Some(127),
        "the fake sudo is not closing descriptor 3, so this whole file proves \
         nothing.\nstdout: {:?}\nstderr: {:?}",
        r.stdout,
        r.stderr
    );
    assert!(
        r.stderr.contains("/dev/fd/3: No such file or directory"),
        "the fake sudo must reproduce sudo's own failure text.\nstderr: {:?}",
        r.stderr
    );
}

#[test]
fn the_fake_sudo_passes_stdout_stderr_and_status_through() {
    // The control for every assertion below: a failure there is the WRAPPER's,
    // not the emulator eating output or inventing a status.
    let (bin, tmp) = fixture();
    let r = run_as_transport(
        "sudo bash -c 'echo OUT; echo ERR >&2; exit 7'\n",
        bin.path(),
        tmp.path(),
    );
    assert_eq!(r.code, Some(7), "stderr: {}", r.stderr);
    assert_eq!(r.stdout, "OUT\n");
    assert!(r.stderr.contains("ERR"), "stderr: {:?}", r.stderr);
}

#[test]
fn the_fake_id_forces_the_non_root_branch() {
    // Without this the wrapper's `if [ "$(id -u)" -eq 0 ]` takes the ROOT
    // branch in any CI container running as root, and the sudo transport —
    // the entire subject of this file — is never executed.
    let (bin, tmp) = fixture();
    let r = run_as_transport("echo \"$(id -u)\"\n", bin.path(), tmp.path());
    assert_eq!(r.code, Some(0), "stderr: {}", r.stderr);
    assert_eq!(r.stdout, "1000\n", "stdout: {:?}", r.stdout);
}

// ---------------------------------------------------------------------------
// The transport, through the emulator.
// ---------------------------------------------------------------------------

#[test]
fn the_sudo_wrapper_runs_the_script_under_closefrom_and_removes_its_temp_file() {
    let (bin, tmp) = fixture();
    // `$0` is the path bash was handed, so the script reports which file it ran
    // from — the "temp file is gone" assertion is then about a file that
    // demonstrably existed, not vacuously about an empty directory.
    let script =
        codegen::apply_script(&sudo_task("echo forjar-sudo-ran\necho \"$0\"\n", None)).unwrap();

    let r = run_as_transport(&script, bin.path(), tmp.path());
    assert_eq!(
        r.code,
        Some(0),
        "the sudo wrapper did not run the script under closefrom \
         (127 is the fd-3 form: bash never opened the script).\nstderr: {}\n\
         --- script ---\n{script}",
        r.stderr
    );
    let lines: Vec<&str> = r.stdout.lines().collect();
    assert_eq!(
        lines.first().copied(),
        Some("forjar-sudo-ran"),
        "stdout: {:?}",
        r.stdout
    );
    let ran_from = Path::new(lines.get(1).copied().unwrap_or_default());
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
fn a_failing_script_propagates_its_exit_code_through_the_wrapper() {
    let (bin, tmp) = fixture();
    let script = codegen::apply_script(&sudo_task("exit 42\n", None)).unwrap();

    let r = run_as_transport(&script, bin.path(), tmp.path());
    assert_eq!(
        r.code,
        Some(42),
        "the inner script's exit code must reach the transport unchanged \
         (127 here is the fd-3 form; 0 would mean the trap or the `fi` ate the \
         status).\nstderr: {}",
        r.stderr
    );
    assert!(
        leftovers(tmp.path()).is_empty(),
        "the temp file must be removed on the failure path too: {:?}",
        leftovers(tmp.path())
    );
}

#[test]
fn stdin_under_the_sudo_wrapper_is_the_transports_not_the_script() {
    // The property descriptor 3 was chosen for (#390-E), kept by the temp file:
    // the elevated script's stdin is whatever the transport left it — here the
    // brace group's /dev/null — so `cat` reads EOF instead of the rest of its
    // own script.
    let (bin, tmp) = fixture();
    let script = codegen::apply_script(&sudo_task("cat\necho second-line-ran\n", None)).unwrap();

    let r = run_as_transport(&script, bin.path(), tmp.path());
    assert_eq!(r.code, Some(0), "stderr: {}", r.stderr);
    assert_eq!(
        r.stdout, "second-line-ran\n",
        "either `cat` printed script text (stdin was the script) or the line \
         after it never ran"
    );
    assert!(leftovers(tmp.path()).is_empty());
}

#[test]
fn the_timeout_wrapper_still_runs_under_closefrom() {
    // `timeout:` also passes its command on descriptor 3 (`timeout N bash
    // /dev/fd/3 3<<'D'`), and that form is NOT affected: the descriptor is
    // opened by the ELEVATED bash itself, after sudo's closefrom, and `timeout`
    // closes nothing. This pins the combination so a future "fix every
    // /dev/fd/3" sweep cannot break a working emitter unnoticed.
    let (bin, tmp) = fixture();
    let script = codegen::apply_script(&sudo_task("echo under-timeout\n", Some(30))).unwrap();

    let r = run_as_transport(&script, bin.path(), tmp.path());
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
    // theirs too: every `sudo: true` resource read as diverged (exit 127) on
    // every check, whatever was on disk.
    let (bin, tmp) = fixture();
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

    let run = run_as_transport(&script, bin.path(), tmp.path());
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
fn the_state_query_crosses_the_same_boundary() {
    // The half that writes `live_hash`/`observed` (#349). Under the fd-3 form
    // it produced nothing, and a state query that answers nothing is recorded
    // as "could not observe" — which is not drift, so nothing ever moved.
    let (bin, tmp) = fixture();
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
    let script = codegen::state_query_script(&r).unwrap();

    let run = run_as_transport(&script, bin.path(), tmp.path());
    assert_eq!(
        run.code,
        Some(0),
        "stderr: {}\n--- script ---\n{script}",
        run.stderr
    );
    assert!(
        !run.stdout.trim().is_empty(),
        "the state query answered nothing, so drift can never fire"
    );
    assert!(leftovers(tmp.path()).is_empty());
}

#[test]
fn the_same_task_without_sudo_already_ran() {
    // THE CONTROL: a failure above is the WRAPPER's, not the task's or the
    // transport's. Takes no privileged branch and uses no fake.
    let (bin, tmp) = fixture();
    let mut r = sudo_task("echo plain\n", None);
    r.sudo = false;
    let script = codegen::apply_script(&r).unwrap();
    let run = run_as_transport(&script, bin.path(), tmp.path());
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert_eq!(run.stdout, "plain\n");
}

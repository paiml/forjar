//! PMAT-560 (paiml/forjar#560): a `service` is not converged while it
//! executes something other than what was declared.
//!
//! MEASURED, yoga, 2026-09-15. `github-runner-ephemeral.service` is declared
//! in paiml/infra `machines/yoga/forjar-ephemeral.yaml`; the declared unit
//! file's `ExecStart` names `run-ephemeral-docker.sh`. The unit systemd had
//! loaded executed `/opt/github-runner-ephemeral/run-ephemeral-docker-v3.sh`
//! — root-owned, mtime 2026-09-10 17:28, a 95-minute hand iteration (v1 → v2
//! → v3) the repo never saw — and every check reported the service
//! converged. The `service` resource asked the host two questions,
//! `is-active` and `is-enabled`, and both were true of a unit running code
//! nobody had reviewed. "Declared" at the unit boundary said nothing about
//! what executes.
//!
//! The layers are unit → `ExecStart` → script bytes. These tests EXECUTE the
//! emitted shell against a fake host whose `systemctl show` answers for the
//! loaded unit, so the check is judged on its exit code, exactly as
//! `cli::check` judges it, and never on the text it was generated from.

use forjar::core::types::{ExecParity, MachineTarget, Resource, ResourceType};
use forjar::resources::service::{apply_script, check_script, state_query_script};
use sha2::{Digest, Sha256};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const UNIT: &str = "github-runner-ephemeral";
const DECLARED_BYTES: &str = "#!/bin/sh\necho v1\n";
const V3_BYTES: &str = "#!/bin/sh\necho v3 # hand-iterated on the box\n";

fn sha256_hex(bytes: &str) -> String {
    format!("{:x}", Sha256::digest(bytes.as_bytes()))
}

/// A fake host: `systemctl` answers `is-active`/`is-enabled` with success and
/// reports `FJ_LIVE_EXEC` as the loaded unit's `ExecStart` program, in the
/// exact shape systemd 249 prints for `show -p ExecStart --value`. An empty
/// `FJ_LIVE_EXEC` prints an empty line, which is what systemctl prints for a
/// unit it has not loaded.
struct FakeHost {
    dir: tempfile::TempDir,
    bin: PathBuf,
}

impl FakeHost {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        let systemctl = r#"#!/bin/sh
case "$1" in
  is-active|is-enabled) exit 0 ;;
  show)
    if [ -z "${FJ_LIVE_EXEC:-}" ]; then echo ""; exit 0; fi
    echo "{ path=${FJ_LIVE_EXEC} ; argv[]=${FJ_LIVE_EXEC} ; ignore_errors=no ; start_time=[n/a] ; stop_time=[n/a] ; pid=0 ; code=(null) ; status=0/0 }"
    ;;
  *) exit 0 ;;
esac
"#;
        let p = bin.join("systemctl");
        fs::write(&p, systemctl).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        Self { dir, bin }
    }

    fn script(&self, name: &str, bytes: &str) -> String {
        let p = self.dir.path().join(name);
        fs::write(&p, bytes).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
        p.to_str().unwrap().to_string()
    }

    fn run(&self, script: &str, live_exec: &str) -> Output {
        Command::new("bash")
            .arg("-c")
            .arg(script)
            .env("PATH", format!("{}:/usr/bin:/bin", self.bin.display()))
            .env("FJ_LIVE_EXEC", live_exec)
            .output()
            .expect("bash must run")
    }
}

fn service(exec_start: Option<&str>, exec_sha256: Option<&str>) -> Resource {
    Resource {
        resource_type: ResourceType::Service,
        machine: MachineTarget::Single("yoga".into()),
        name: Some(UNIT.into()),
        state: Some("running".into()),
        enabled: Some(true),
        exec: ExecParity {
            exec_start: exec_start.map(str::to_string),
            exec_sha256: exec_sha256.map(str::to_string),
        },
        ..Default::default()
    }
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn code(o: &Output) -> i32 {
    o.status.code().unwrap_or(-1)
}

#[test]
fn a_unit_executing_a_different_path_is_not_converged() {
    // THE REGRESSION. Declared: run.sh. Loaded: run-v3.sh. Both active,
    // both enabled — the two questions the check used to ask.
    let host = FakeHost::new();
    let declared = host.script("run.sh", DECLARED_BYTES);
    let v3 = host.script("run-v3.sh", V3_BYTES);
    let r = service(Some(&declared), Some(&sha256_hex(DECLARED_BYTES)));

    let out = host.run(&check_script(&r), &v3);
    assert_ne!(
        code(&out),
        0,
        "check exited 0 over a unit executing {v3} while {declared} was declared.\n\
         stdout:\n{}\nscript:\n{}",
        stdout(&out),
        check_script(&r)
    );
    let s = stdout(&out);
    assert!(
        s.contains("exec_start_diverged:") && s.contains(&v3),
        "the divergence marker must name the LIVE program so the operator sees \
         what actually runs:\n{s}"
    );
}

#[test]
fn a_unit_executing_the_declared_path_with_different_bytes_is_not_converged() {
    // Path parity is not enough: the same path can hold different bytes than
    // the ones that were reviewed.
    let host = FakeHost::new();
    let declared = host.script("run.sh", V3_BYTES);
    let r = service(Some(&declared), Some(&sha256_hex(DECLARED_BYTES)));

    let out = host.run(&check_script(&r), &declared);
    assert_ne!(code(&out), 0, "stdout:\n{}", stdout(&out));
    let s = stdout(&out);
    assert!(
        s.contains("exec_sha256_diverged:"),
        "must report the digest divergence:\n{s}"
    );
    assert!(
        !s.contains("exec_start_diverged:"),
        "the path DID match; reporting it divergent would misdirect the operator:\n{s}"
    );
}

#[test]
fn the_digest_is_taken_over_the_live_program_not_the_declared_path() {
    // Declared path holds the declared bytes, so hashing the DECLARED path
    // would confirm the file forjar wrote and say nothing about the unit.
    // The loaded unit runs v3, whose bytes differ.
    let host = FakeHost::new();
    let declared = host.script("run.sh", DECLARED_BYTES);
    let v3 = host.script("run-v3.sh", V3_BYTES);
    let r = service(Some(&declared), Some(&sha256_hex(DECLARED_BYTES)));

    let out = host.run(&check_script(&r), &v3);
    let s = stdout(&out);
    assert!(
        s.contains("exec_sha256_diverged:") && s.contains(&sha256_hex(V3_BYTES)),
        "the digest must be of the LIVE program ({}), not of the declared path:\n{s}",
        sha256_hex(V3_BYTES)
    );
}

#[test]
fn a_unit_executing_the_declared_bytes_at_the_declared_path_is_converged() {
    // THE CONTROL. Without it, "always diverged" discharges every other test.
    let host = FakeHost::new();
    let declared = host.script("run.sh", DECLARED_BYTES);
    let r = service(Some(&declared), Some(&sha256_hex(DECLARED_BYTES)));

    let out = host.run(&check_script(&r), &declared);
    assert_eq!(code(&out), 0, "stdout:\n{}", stdout(&out));
    let s = stdout(&out);
    assert!(
        s.contains("exec_start:") && s.contains("exec_sha256:"),
        "{s}"
    );
    assert!(!s.contains("diverged"), "{s}");
}

#[test]
fn a_digest_alone_is_checked_against_whatever_the_unit_runs() {
    // `exec_sha256` without `exec_start`: the operator pins the bytes and
    // lets the path float. The bytes are still read from the loaded program.
    let host = FakeHost::new();
    let v3 = host.script("run-v3.sh", V3_BYTES);
    let ok = service(None, Some(&sha256_hex(V3_BYTES)));
    assert_eq!(code(&host.run(&check_script(&ok), &v3)), 0);
    let wrong = service(None, Some(&sha256_hex(DECLARED_BYTES)));
    assert_ne!(code(&host.run(&check_script(&wrong), &v3)), 0);
}

#[test]
fn an_unloaded_unit_is_not_converged() {
    // `systemctl show -p ExecStart --value` prints an empty line and exits 0
    // for a unit it has not loaded. Absence of a program is not parity.
    let host = FakeHost::new();
    let declared = host.script("run.sh", DECLARED_BYTES);
    let r = service(Some(&declared), None);
    let out = host.run(&check_script(&r), "");
    assert_ne!(code(&out), 0, "stdout:\n{}", stdout(&out));
}

#[test]
fn a_unit_with_no_exec_declaration_keeps_the_presence_check() {
    // Every fleet service that declares neither field must be unaffected:
    // same verdict, same questions asked, same observed digest.
    let host = FakeHost::new();
    let v3 = host.script("run-v3.sh", V3_BYTES);
    let r = service(None, None);

    let check = check_script(&r);
    assert!(!check.contains("ExecStart"), "{check}");
    assert_eq!(code(&host.run(&check, &v3)), 0);

    let query = state_query_script(&r);
    assert!(!query.contains("ExecStart"), "{query}");
    let s = stdout(&host.run(&query, &v3));
    assert!(
        !s.contains("exec_"),
        "an undeclared service's digest must not move:\n{s}"
    );
}

#[test]
fn apply_refuses_to_report_converged_over_the_wrong_executable() {
    // `forjar apply` runs this script and reads its exit code as the verdict.
    // Starting and enabling a unit that runs the wrong program is not
    // convergence, and the unit file is another resource's to fix.
    let host = FakeHost::new();
    let declared = host.script("run.sh", DECLARED_BYTES);
    let v3 = host.script("run-v3.sh", V3_BYTES);
    let r = service(Some(&declared), Some(&sha256_hex(DECLARED_BYTES)));

    let out = host.run(&apply_script(&r), &v3);
    assert_ne!(
        code(&out),
        0,
        "stdout:\n{}\nstderr:\n{}",
        stdout(&out),
        String::from_utf8_lossy(&out.stderr)
    );

    let ok = host.run(&apply_script(&r), &declared);
    assert_eq!(
        code(&ok),
        0,
        "apply must still succeed when parity holds:\n{}",
        String::from_utf8_lossy(&ok.stderr)
    );
}

#[test]
fn the_state_query_reports_the_live_program_and_its_digest() {
    // The tripwire hashes this output. A swapped program after apply must
    // move the digest, so drift sees the swap even between checks.
    let host = FakeHost::new();
    let declared = host.script("run.sh", DECLARED_BYTES);
    let v3 = host.script("run-v3.sh", V3_BYTES);
    let r = service(Some(&declared), Some(&sha256_hex(DECLARED_BYTES)));
    let query = state_query_script(&r);

    let before = stdout(&host.run(&query, &declared));
    assert!(
        before.contains(&format!("exec_start={declared}")),
        "{before}"
    );
    assert!(
        before.contains(&format!("exec_sha256={}", sha256_hex(DECLARED_BYTES))),
        "{before}"
    );

    let after = stdout(&host.run(&query, &v3));
    assert!(after.contains(&format!("exec_start={v3}")), "{after}");
    assert!(
        after.contains(&format!("exec_sha256={}", sha256_hex(V3_BYTES))),
        "{after}"
    );
    assert_ne!(
        before, after,
        "the observation must move when the program does"
    );
}

#[test]
fn the_fake_host_matches_a_real_systemctl_line() {
    // The fixture's value is only as good as its fidelity. This is the exact
    // line systemd 249 printed for `systemctl show -p ExecStart --value
    // ssh.service` on the box these tests were written on.
    let real = "{ path=/usr/sbin/sshd ; argv[]=/usr/sbin/sshd -D $SSHD_OPTS ; ignore_errors=no ; start_time=[n/a] ; stop_time=[n/a] ; pid=0 ; code=(null) ; status=0/0 }";
    let host = FakeHost::new();
    let s = stdout(&host.run(
        "systemctl show -p ExecStart --value ssh.service",
        "/usr/sbin/sshd",
    ));
    let fake_prefix = s.split(" ; argv").next().unwrap();
    let real_prefix = real.split(" ; argv").next().unwrap();
    assert_eq!(fake_prefix, real_prefix);
    assert!(Path::new("/usr/bin/sha256sum").exists() || Path::new("/bin/sha256sum").exists());
}

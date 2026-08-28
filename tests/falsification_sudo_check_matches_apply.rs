//! `sudo: true` must govern the READ path too, not only the apply.
//!
//! THE FLAW THIS CLOSES.
//!
//! `sudo` is a property of the RESOURCE, but forjar treated it as a property of
//! the APPLY PHASE. `src/core/codegen/dispatch.rs` had one privilege resolver
//! and exactly one of its three sibling entry points called it: `apply_script`
//! wrapped the script in a `sudo bash` heredoc, while `check_script` and
//! `state_query_script` returned the handler's raw output.
//!
//! So the check answered a different question than the apply. The apply asked
//! "is there a file at P, as root?"; the check asked "is there a file at P, as
//! the invoking user?". Under a mode-0750 root-owned directory those have
//! permanently different answers, because DAC denies the TRAVERSAL, not the
//! file.
//!
//! Measured on paiml intel (#349): `/etc/audit/rules.d/50-cargo-bin.rules`
//! declared `sudo: true`, was written correctly, and was then probed with a
//! bare `test -f` that cannot enter `drwxr-x--- root root /etc/audit`. apply
//! exited 0, the bytes on disk were right, and forjar reported
//!
//!     apply exited 0 but the host does not report the declared state
//!     (check exit 1). missing:file
//!
//! forever — after which jidoka skipped every dependent, and the dependents
//! were the readback and the `augenrules --load` that arm kernel auditing. A
//! privilege bug in the READ path disabled a security control by refusing to
//! run the steps that enable it.
//!
//! `state_query_script` is the same defect with a quieter symptom: it recorded
//! the digest of the literal string `MISSING` as `observed`/`live_hash`.
//!
//! WHAT THIS TEST MUST NOT BECOME. A string match for the word "sudo" is
//! satisfiable by emitting it anywhere. The load-bearing cases here run the
//! generated script against a real escalator stub on `PATH` and assert the
//! stub was INVOKED — the check actually requested the declared privilege
//! context — while still asserting that the verdict and its stdout marker
//! survived the wrapping.

use forjar::core::codegen::{apply_script, check_script, state_query_script};
use forjar::core::types::{Resource, ResourceType};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// True when the test process is root, in which case the wrapper's `then`
/// branch runs and no escalator is ever invoked.
fn running_as_root() -> bool {
    let out = Command::new("id").arg("-u").output().expect("id -u");
    String::from_utf8_lossy(&out.stdout).trim() == "0"
}

fn tempdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "fj349-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create tempdir");
    dir
}

/// Install a `sudo` on PATH that records the fact it was asked to elevate and
/// then delegates. A real escalator, not a string.
fn escalator_stub(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).expect("create bin");
    let sudo = bin.join("sudo");
    fs::write(
        &sudo,
        "#!/bin/sh\nprintf 'invoked\\n' >> \"$FJ_SUDO_LOG\"\nexec \"$@\"\n",
    )
    .expect("write sudo stub");
    fs::set_permissions(&sudo, fs::Permissions::from_mode(0o755)).expect("chmod sudo stub");
    bin
}

fn run_with_stub(script: &str, dir: &Path, log: &Path) -> Output {
    let bin = escalator_stub(dir);
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    Command::new("sh")
        .arg("-c")
        .arg(script)
        .env("PATH", path)
        .env("FJ_SUDO_LOG", log)
        .output()
        .expect("run generated script")
}

fn sudo_file(path: &Path, sudo: bool) -> Resource {
    Resource {
        resource_type: ResourceType::File,
        path: Some(path.display().to_string()),
        state: Some("file".to_string()),
        sudo,
        ..Default::default()
    }
}

/// FALSIFY-349-A: the check asks for the privilege context the resource
/// declared — proved by a real escalator being invoked, not by a substring.
#[test]
fn the_check_for_a_sudo_resource_actually_asks_for_elevation() {
    let dir = tempdir("check");
    let target = dir.join("present.conf");
    fs::write(&target, "hello\n").expect("write target");
    let log = dir.join("sudo.log");

    let script = check_script(&sudo_file(&target, true)).expect("check_script");
    let out = run_with_stub(&script, &dir, &log);
    let stdout = String::from_utf8_lossy(&out.stdout);

    // The wrapping must not swallow the verdict or its marker.
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout={stdout}\nscript={script}"
    );
    assert!(
        stdout.contains("exists:file"),
        "the marker the operator reads was lost: {stdout}"
    );

    if running_as_root() {
        return; // root takes the `then` branch; nothing to escalate.
    }
    assert!(
        log.exists() && !fs::read_to_string(&log).unwrap().is_empty(),
        "the check never requested elevation — it ran in a different security \
         context than the apply, which is #349:\n{script}"
    );
}

/// FALSIFY-349-B: the state query is the half that writes `live_hash` /
/// `observed` into the lock, so it must be elevated too or drift compares the
/// digest of the string `MISSING` against itself.
#[test]
fn the_state_query_for_a_sudo_resource_asks_for_elevation() {
    let dir = tempdir("query");
    let target = dir.join("present.conf");
    fs::write(&target, "hello\n").expect("write target");
    let log = dir.join("sudo.log");

    let script = state_query_script(&sudo_file(&target, true)).expect("state_query_script");
    let out = run_with_stub(&script, &dir, &log);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("MISSING"),
        "the state query could not see a file it just read: {stdout}"
    );

    if running_as_root() {
        return;
    }
    assert!(
        log.exists() && !fs::read_to_string(&log).unwrap().is_empty(),
        "the state query never requested elevation:\n{script}"
    );
}

/// FALSIFY-349-C: the PREMISE, reproduced hermetically rather than quoted from
/// the issue. A file that provably exists reports `missing:file` when the probe
/// cannot traverse its parent. This is why A and B matter; it is green in both
/// directions.
#[test]
fn a_root_only_path_reports_missing_when_probed_unprivileged() {
    if running_as_root() {
        return; // root ignores DAC, so the premise is unobservable.
    }
    let dir = tempdir("premise");
    let inner = dir.join("inner");
    fs::create_dir_all(&inner).expect("create inner");
    let target = inner.join("f");
    fs::write(&target, "present\n").expect("write target");
    fs::set_permissions(&inner, fs::Permissions::from_mode(0o000)).expect("chmod 0000");

    let script = check_script(&sudo_file(&target, false)).expect("check_script");
    let out = Command::new("sh")
        .arg("-c")
        .arg(&script)
        .output()
        .expect("run check");

    // Restore before asserting so a failure does not leave an unreadable dir.
    fs::set_permissions(&inner, fs::Permissions::from_mode(0o755)).expect("restore mode");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "stdout={stdout}");
    assert!(
        stdout.contains("missing:file"),
        "expected the unprivileged probe to miss a file that exists: {stdout}"
    );
    assert!(target.exists(), "the file was there the whole time");
}

/// FALSIFY-349-D: the structural obligation, in the spirit of the dispatch
/// module's own symmetry argument — a type cannot be elevated on one path and
/// left plain on the others, because one resolver serves all three.
#[test]
fn every_dispatchable_type_elevates_all_three_or_none() {
    let all = [
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
        ResourceType::Gpu,
        ResourceType::Task,
        ResourceType::WasmBundle,
        ResourceType::Image,
        ResourceType::Build,
        ResourceType::GithubRelease,
        ResourceType::OverlayInterface,
        ResourceType::DiskBudget,
        ResourceType::BackupSync,
        ResourceType::NasArchive,
    ];

    for rt in all {
        let r = Resource {
            resource_type: rt.clone(),
            name: Some("thing".to_string()),
            path: Some("/etc/thing.conf".to_string()),
            sudo: true,
            ..Default::default()
        };
        if apply_script(&r).is_err() {
            continue; // not dispatchable with this fixture; symmetry is FALSIFY-CD-002's job.
        }
        assert!(
            check_script(&r).unwrap().contains("FORJAR_SUDO"),
            "{rt:?}: apply is elevated, check is not"
        );
        assert!(
            state_query_script(&r).unwrap().contains("FORJAR_SUDO"),
            "{rt:?}: apply is elevated, state_query is not"
        );
    }
}

/// FALSIFY-349-E: the over-wrap guard. Elevating unconditionally would demand a
/// working `sudo` for every check on every host, which is a different bug.
#[test]
fn an_unelevated_resource_is_left_alone() {
    let r = sudo_file(Path::new("/etc/thing.conf"), false);
    for (label, script) in [
        ("check", check_script(&r).unwrap()),
        ("apply", apply_script(&r).unwrap()),
        ("state_query", state_query_script(&r).unwrap()),
    ] {
        assert!(
            !script.contains("sudo bash"),
            "{label} elevated a resource that did not declare sudo:\n{script}"
        );
    }
}

//! The build resource must pull its artifact with the sovereign tool.
//!
//! forjar#290. `build` shelled out to `scp -o BatchMode=yes` to copy one
//! cross-compiled binary back from the build machine. That is squarely copia's
//! domain — `copia sync host:path dest` works today and requires NOTHING on the
//! remote, because it streams over `ssh host "cat …"` — so unlike rclone
//! (cloud) or curl (HTTP) there was no out-of-domain argument for it. It was
//! simply the tool reached for first.
//!
//! PR #291 made that visible by adding a `Justification::Debt` row to
//! `sync_tools()`, and a standing exception that nobody has to remove is how a
//! debt becomes permanent. So this file pins BOTH halves: the invocation is
//! copia, AND the ledger row is gone. Either edit alone fails one of the
//! partition tests in `src/resources/sync_tools.rs`.
//!
//! The third test EXECUTES the generated shell against stubbed binaries. A
//! `script.contains("copia")` assertion cannot tell the difference between
//! calling copia and calling scp with the word copia in a comment.

use forjar::core::types::{MachineTarget, Resource, ResourceType};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tmpdir(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "forjar-290-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&p).unwrap();
    p
}

fn stub(bin: &Path, name: &str, body: &str) {
    fs::create_dir_all(bin).unwrap();
    let p = bin.join(name);
    fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
}

fn build_resource(target: &str) -> Resource {
    Resource {
        resource_type: ResourceType::Build,
        machine: MachineTarget::Single("jetson".to_string()),
        build_machine: Some("intel".to_string()),
        command: Some("cargo build --release".to_string()),
        source: Some("/tmp/cross/release/apr".to_string()),
        target: Some(target.to_string()),
        ..Default::default()
    }
}

/// The generated script must name copia and must not name scp at all.
#[test]
fn the_build_resource_does_not_shell_out_to_scp() {
    let script =
        forjar::resources::build::apply_script(&build_resource("/home/user/.cargo/bin/apr"));
    assert!(
        !script.contains("scp"),
        "the build resource still shells out to scp:\n{script}"
    );
    assert!(
        script.contains("copia sync 'intel:/tmp/cross/release/apr' '/home/user/.cargo/bin/apr'"),
        "the artifact is not pulled with copia sync:\n{script}"
    );
}

/// The ledger row must go with the invocation. A `Debt` entry for a call site
/// that no longer exists reads as a live exception and quietly widens the
/// policy — which is what `the_partition_has_no_stale_entries` catches from the
/// other side.
#[test]
fn the_sovereignty_ledger_no_longer_carries_an_scp_exception() {
    assert!(
        forjar::resources::sync_tools::sync_tools()
            .iter()
            .all(|t| t.binary != "scp"),
        "the scp Debt row outlived the invocation it excused"
    );
}

/// THE DEFECT, executed: the transfer must go through copia, and scp must never
/// be reached even when it is sitting right there on PATH.
#[test]
fn the_transfer_runs_through_copia_and_never_through_scp() {
    let dir = tmpdir("exec");
    let bin = dir.join("bin");
    let sentinel = dir.join("scp-was-called");
    let deploy = dir.join("deploy").join("apr");

    // ssh consumes the Phase-1 heredoc from stdin and reports a clean build.
    stub(&bin, "ssh", "cat >/dev/null\nexit 0");
    // copia writes the artifact, non-executable, exactly as the real one does.
    stub(
        &bin,
        "copia",
        "[ \"$1\" = sync ] || exit 91\nprintf ARTIFACT > \"$3\"",
    );
    // scp is present and works — so reaching for it would SUCCEED. Only the
    // sentinel distinguishes "used the sovereign tool" from "used what was
    // there".
    stub(&bin, "scp", "touch \"$SENTINEL\"\nexit 97");

    let script = forjar::resources::build::apply_script(&build_resource(deploy.to_str().unwrap()));
    let out = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .env("SENTINEL", &sentinel)
        .output()
        .expect("run generated script");

    assert!(
        out.status.success(),
        "generated script failed: {}\n{script}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        fs::read_to_string(&deploy).expect("artifact not deployed"),
        "ARTIFACT",
        "the deployed bytes did not come from copia"
    );
    let mode = fs::metadata(&deploy).unwrap().permissions().mode();
    assert!(
        mode & 0o111 != 0,
        "copia writes non-executable; chmod +x must still run, got {mode:o}"
    );
    assert!(
        !sentinel.exists(),
        "scp was invoked — the build resource is still not sovereign"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// A machine without copia must refuse BEFORE it starts making directories.
/// Otherwise the operator sees a half-made destination and then a message about
/// a missing binary, in that order — the failure mode nas_archive's preflight
/// already exists to avoid.
#[test]
fn a_machine_without_copia_refuses_before_making_the_destination_directory() {
    let dir = tmpdir("preflight");
    let bin = dir.join("bin");
    let deploy = dir.join("deploy").join("nested").join("apr");

    stub(&bin, "ssh", "cat >/dev/null\nexit 0");
    // scp IS on this PATH and fails fast. The point is not that scp is
    // unavailable — it is that a build resource with no copia must refuse
    // rather than quietly fall back to whatever else can move bytes.
    stub(&bin, "scp", "exit 97");

    // A PATH with the shell's own utilities and no copia anywhere on it.
    let path = format!("{}:/usr/bin:/bin", bin.display());
    let probe = Command::new("bash")
        .arg("-c")
        .arg("command -v copia")
        .env("PATH", &path)
        .output()
        .expect("probe PATH");
    assert!(
        !probe.status.success(),
        "this test needs a PATH with no copia on it; found: {}",
        String::from_utf8_lossy(&probe.stdout)
    );

    let script = forjar::resources::build::apply_script(&build_resource(deploy.to_str().unwrap()));
    let out = Command::new("bash")
        .arg("-c")
        .arg(&script)
        .env("PATH", &path)
        .output()
        .expect("run generated script");

    assert!(
        !out.status.success(),
        "a machine without copia must refuse, not proceed:\n{script}"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("copia is not installed"),
        "the refusal must name the missing tool, got: {stderr}"
    );
    assert!(
        !dir.join("deploy").exists(),
        "the destination directory was created before the refusal"
    );

    let _ = fs::remove_dir_all(&dir);
}

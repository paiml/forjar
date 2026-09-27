//! Refs #642: a mount's ownership options are part of its declared state.
//!
//! THE FLAW THIS CLOSES. lambda-labs' NAS share was redeclared `gid=courses`
//! (997) while the kernel still showed `gid=1000`. The mount check compared only
//! the `findmnt` SOURCE — the right share with the wrong owner said converged, so
//! apply never remounted and the `course` user could not write.
//!
//! WHAT THIS TEST MUST NOT BECOME. Asserting that the script text mentions `gid`
//! would stay green if the comparison were a no-op. Every case here RUNS the
//! generated script under `sh`/`bash` against a fake `findmnt`/`umount`/`mount`
//! on PATH and asserts on exit codes and the calls the host received.
//! Reverting `options_condition` to the source-only check turns
//! `wrong_gid_is_drift` and `apply_remounts_on_ownership_drift` red.

use forjar::core::types::{Resource, ResourceType};
use forjar::resources::mount::{apply_script, check_script};

const SRC: &str = "//nas/media";
const TGT: &str = "/mnt/unas";
const LIVE_1000: &str =
    "rw,relatime,vers=3.1.1,uid=1000,forceuid,gid=1000,forcegid,file_mode=0664,dir_mode=0775,soft";
const LIVE_997: &str =
    "rw,relatime,vers=3.1.1,uid=1000,forceuid,gid=997,forcegid,file_mode=0664,dir_mode=0775,soft";
const DECL_997: &str =
    "rw,vers=3.1.1,credentials=/etc/c,uid=1000,gid=997,file_mode=0664,dir_mode=0775";

fn cifs(options: &str) -> Resource {
    Resource {
        resource_type: ResourceType::Mount,
        source: Some(SRC.to_string()),
        path: Some(TGT.to_string()),
        fs_type: Some("cifs".to_string()),
        options: Some(options.to_string()),
        ..Default::default()
    }
}

/// A PATH dir whose `findmnt` reports `SRC` mounted with `live` OPTIONS and whose
/// `umount` exits non-zero (busy) when `busy`; every call is logged to `calls`.
fn fake_host(live: &str, busy: bool) -> tempfile::TempDir {
    let d = tempfile::tempdir().expect("tempdir");
    let calls = d.path().join("calls");
    let rc = if busy { 32 } else { 0 };
    for (name, body) in [
        (
            "findmnt",
            format!("#!/bin/sh\ncase \"$*\" in *SOURCE*) echo '{SRC}' ;; *OPTIONS*) echo '{live}' ;; esac\n"),
        ),
        (
            "umount",
            format!("#!/bin/sh\necho \"umount $*\" >> '{}'\nexit {rc}\n", calls.display()),
        ),
        (
            "mount",
            format!("#!/bin/sh\necho \"mount $*\" >> '{}'\n", calls.display()),
        ),
        ("mountpoint", "#!/bin/sh\nexit 0\n".to_string()),
        ("mkdir", "#!/bin/sh\nexit 0\n".to_string()),
    ] {
        let p = d.path().join(name);
        std::fs::write(&p, body).expect("write fake");
        std::process::Command::new("chmod")
            .arg("+x")
            .arg(&p)
            .status()
            .expect("chmod");
    }
    d
}

fn run(shell: &str, script: &str, host: &tempfile::TempDir) -> i32 {
    let path = format!(
        "{}:{}",
        host.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    std::process::Command::new(shell)
        .arg("-c")
        .arg(script)
        .env("PATH", path)
        .output()
        .expect("spawn shell")
        .status
        .code()
        .unwrap_or(-1)
}

fn apply_on(host: &tempfile::TempDir, decl: &str) -> (i32, String) {
    let script = apply_script(&cifs(decl)).replace(
        "/etc/fstab",
        &host.path().join("fstab").display().to_string(),
    );
    let rc = run("bash", &script, host);
    let calls = std::fs::read_to_string(host.path().join("calls")).unwrap_or_default();
    (rc, calls)
}

#[test]
fn wrong_gid_is_drift() {
    let h = fake_host(LIVE_1000, false);
    assert_ne!(run("sh", &check_script(&cifs(DECL_997)), &h), 0);
    let h = fake_host(LIVE_997, false);
    assert_eq!(run("sh", &check_script(&cifs(DECL_997)), &h), 0);
}

#[test]
fn an_omitted_key_is_its_default_not_a_wildcard() {
    let h = fake_host("rw,nosuid,size=1024k", false);
    assert_ne!(
        run("sh", &check_script(&cifs("size=1024k,uid=1000")), &h),
        0
    );
    assert_ne!(run("sh", &check_script(&cifs("dir_mode=02775")), &h), 0);
}

#[test]
fn apply_remounts_on_ownership_drift() {
    let (rc, calls) = apply_on(&fake_host(LIVE_1000, false), DECL_997);
    assert_eq!(rc, 0, "{calls}");
    assert!(calls.contains("umount /mnt/unas"), "{calls}");
    assert!(calls.contains("gid=997"), "{calls}");
    assert!(!calls.contains("-l"), "never a lazy detach: {calls}");
}

#[test]
fn apply_refuses_to_detach_a_busy_mount() {
    let (rc, calls) = apply_on(&fake_host(LIVE_1000, true), DECL_997);
    assert_ne!(rc, 0, "busy must fail loudly: {calls}");
    assert!(!calls.contains("mount -t"), "{calls}");
}

#[test]
fn apply_leaves_a_converged_mount_alone() {
    let (rc, calls) = apply_on(&fake_host(LIVE_997, false), DECL_997);
    assert_eq!(rc, 0);
    assert!(!calls.contains("umount"), "{calls}");
}

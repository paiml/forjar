//! #642: a mount's ownership options are part of its declared state.
//!
//! These run the GENERATED scripts under `sh` against a fake `findmnt` on PATH, so
//! they measure what the host would be told, not the text of the script. The real
//! case: lambda-labs' NAS share declared `gid=997` while the kernel still showed
//! `gid=1000`, and check (source only) said converged.
use super::mount::*;
use super::tests_mount::make_mount_resource;
use crate::core::types::Resource;

const SRC: &str = "//nas/media";
const TGT: &str = "/mnt/unas";

fn cifs(options: &str) -> Resource {
    let mut r = make_mount_resource();
    r.source = Some(SRC.to_string());
    r.path = Some(TGT.to_string());
    r.fs_type = Some("cifs".to_string());
    r.options = Some(options.to_string());
    r
}

/// A PATH directory whose `findmnt` reports `SRC` mounted with `live` OPTIONS, and
/// whose `umount` fails (busy) when `busy` — recording every call to `calls`.
fn fake_host(live: &str, busy: bool) -> tempfile::TempDir {
    let d = tempfile::tempdir().expect("tempdir");
    let calls = d.path().join("calls");
    let findmnt = format!(
        "#!/bin/sh\ncase \"$*\" in *SOURCE*) echo '{SRC}' ;; *OPTIONS*) echo '{live}' ;; esac\n"
    );
    let rc = if busy { 32 } else { 0 };
    let umount = format!(
        "#!/bin/sh\necho \"umount $*\" >> '{}'\nexit {rc}\n",
        calls.display()
    );
    let mount = format!("#!/bin/sh\necho \"mount $*\" >> '{}'\n", calls.display());
    for (name, body) in [
        ("findmnt", findmnt),
        ("umount", umount),
        ("mount", mount),
        ("mountpoint", "#!/bin/sh\nexit 0\n".into()),
        ("mkdir", "#!/bin/sh\nexit 0\n".into()),
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

fn run(script: &str, host: &tempfile::TempDir) -> i32 {
    run_in("sh", script, host)
}

fn run_bash(script: &str, host: &tempfile::TempDir) -> i32 {
    run_in("bash", script, host)
}

fn run_in(shell: &str, script: &str, host: &tempfile::TempDir) -> i32 {
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
        .expect("sh")
        .status
        .code()
        .unwrap_or(-1)
}

fn calls(host: &tempfile::TempDir) -> String {
    std::fs::read_to_string(host.path().join("calls")).unwrap_or_default()
}

const LIVE_1000: &str =
    "rw,relatime,vers=3.1.1,uid=1000,forceuid,gid=1000,forcegid,file_mode=0664,dir_mode=0775,soft";
const LIVE_997: &str =
    "rw,relatime,vers=3.1.1,uid=1000,forceuid,gid=997,forcegid,file_mode=0664,dir_mode=0775,soft";
const DECL_997: &str = "rw,vers=3.1.1,credentials=/etc/c,uid=1000,gid=997,file_mode=0664,dir_mode=0775,noauto,x-systemd.automount";

#[test]
fn fj642_check_is_red_when_the_right_share_has_the_wrong_gid() {
    let h = fake_host(LIVE_1000, false);
    assert_ne!(
        run(&check_script(&cifs(DECL_997)), &h),
        0,
        "same source, gid 1000 live vs 997 declared must diverge"
    );
}

#[test]
fn fj642_check_is_green_when_every_declared_ownership_option_is_live() {
    let h = fake_host(LIVE_997, false);
    assert_eq!(run(&check_script(&cifs(DECL_997)), &h), 0);
}

#[test]
fn fj642_modes_compare_as_numbers_and_absent_keys_are_the_default() {
    // tmpfs echoes mode=755 for a declared 0755, and omits uid=0 entirely.
    let h = fake_host("rw,nosuid,size=1024k,mode=755", false);
    assert_eq!(
        run(&check_script(&cifs("size=1024k,mode=0755,uid=0")), &h),
        0
    );
    assert_ne!(
        run(&check_script(&cifs("size=1024k,mode=0700")), &h),
        0,
        "a different mode must diverge"
    );
}

#[test]
fn fj642_an_omitted_key_is_its_default_not_a_wildcard() {
    // tmpfs omitting uid= means uid 0; a declared uid=1000 is drift, not a match.
    let h = fake_host("rw,nosuid,size=1024k", false);
    assert_ne!(
        run(&check_script(&cifs("size=1024k,uid=1000")), &h),
        0,
        "omitted uid is 0, not whatever was declared"
    );
    assert_eq!(run(&check_script(&cifs("size=1024k,mode=1777")), &h), 0);
    // cifs always echoes dir_mode; a missing one never matches a declared value.
    assert_ne!(run(&check_script(&cifs("dir_mode=02775")), &h), 0);
}

#[test]
fn fj642_a_group_name_is_compared_by_its_number() {
    // root is gid 0 on every host this suite runs on.
    let h = fake_host("rw,gid=0", false);
    assert_eq!(run(&check_script(&cifs("gid=root")), &h), 0);
    let h = fake_host("rw,gid=1000", false);
    assert_ne!(run(&check_script(&cifs("gid=root")), &h), 0);
}

#[test]
fn fj642_no_ownership_option_declared_keeps_the_source_only_check() {
    let h = fake_host(LIVE_1000, false);
    assert_eq!(run(&check_script(&cifs("rw,noatime")), &h), 0);
}

#[test]
fn fj642_apply_remounts_on_ownership_drift_without_a_lazy_detach() {
    let h = fake_host(LIVE_1000, false);
    let script = apply_script(&cifs(DECL_997))
        .replace("/etc/fstab", &h.path().join("fstab").display().to_string());
    assert_eq!(run_bash(&script, &h), 0);
    let c = calls(&h);
    assert!(c.contains("umount /mnt/unas"), "{c}");
    assert!(
        c.contains("mount -t cifs -o rw,vers=3.1.1,credentials=/etc/c,uid=1000,gid=997"),
        "{c}"
    );
    assert!(!c.contains("-l"), "never a lazy detach: {c}");
}

#[test]
fn fj642_apply_refuses_to_detach_a_busy_mount() {
    let h = fake_host(LIVE_1000, true);
    let script = apply_script(&cifs(DECL_997))
        .replace("/etc/fstab", &h.path().join("fstab").display().to_string());
    assert_ne!(
        run_bash(&script, &h),
        0,
        "busy must fail loudly, not converge"
    );
    let c = calls(&h);
    assert!(!c.contains("mount -t"), "no mount over a busy path: {c}");
}

#[test]
fn fj642_apply_leaves_a_converged_mount_alone() {
    let h = fake_host(LIVE_997, false);
    let script = apply_script(&cifs(DECL_997))
        .replace("/etc/fstab", &h.path().join("fstab").display().to_string());
    assert_eq!(run_bash(&script, &h), 0);
    assert!(!calls(&h).contains("umount"), "{}", calls(&h));
}

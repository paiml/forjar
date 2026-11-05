//! `state: absent` must fail CLOSED when the crontab cannot be read.
//!
//! Found by the infra#2441 review. The absent script read the crontab with
//! `crontab -l 2>/dev/null || true` and piped the result through the block
//! filter into `crontab -`. A failed read (EPERM, a broken sudo, a missing
//! binary) therefore became an EMPTY string, the filter turned it into an empty
//! crontab, and the install overwrote the user's real crontab with nothing.
//!
//! Only one read failure is benign: `no crontab for <user>`, which means there
//! is nothing to remove. Every other failure must exit non-zero WITHOUT
//! installing. These tests EXECUTE the generated script against stub
//! `crontab`/`sudo`/`id` on a hermetic PATH; each stub asserts its argv.

use forjar::core::types::{MachineTarget, Resource, ResourceType};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const GUARDED: [&str; 7] = [
    "systemctl",
    "journalctl",
    "logger",
    "sudo",
    "docker",
    "crontab",
    "id",
];

fn tmpdir(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "forjar-cron-absent-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(p.join("bin")).unwrap();
    p
}

fn stub(dir: &Path, name: &str, body: &str) {
    let p = dir.join("bin").join(name);
    fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
}

/// How the stub `crontab -u root -l` behaves.
enum Read {
    /// Exit 0, print this crontab.
    Has(&'static str),
    /// Exit 1, stderr `no crontab for root` (the benign case).
    NoCrontab,
    /// Exit 1, any other stderr (the dangerous case).
    Fails(&'static str),
    /// Exit 1, `no crontab` in French unless the caller forces `LC_ALL=C`.
    LocalizedNone,
}

fn host(tag: &str, read: &Read) -> PathBuf {
    let dir = tmpdir(tag);
    let d = dir.display();
    if let Read::Has(text) = read {
        fs::write(dir.join("store"), text).unwrap();
    }
    let read_branch = match read {
        Read::Has(_) => format!("cat '{d}/store'; exit 0"),
        Read::NoCrontab => "echo 'no crontab for root' >&2; exit 1".to_string(),
        Read::Fails(msg) => format!("echo '{msg}' >&2; exit 1"),
        Read::LocalizedNone => "[ \"$LC_ALL\" = C ] && echo 'no crontab for root' >&2 || echo 'aucun crontab pour root' >&2; exit 1".to_string(),
    };
    // argv is asserted: only `-u root -l` and `-u root -` are legal.
    stub(
        &dir,
        "crontab",
        &format!(
            "[ \"$1\" = -u ] && [ \"$2\" = root ] || {{ echo \"bad argv: $*\" >> '{d}/argv-violations'; exit 99; }}\n\
             case \"$3\" in\n\
             -l) {read_branch} ;;\n\
             -) cat > '{d}/installed'; exit 0 ;;\n\
             *) echo \"bad argv: $*\" >> '{d}/argv-violations'; exit 99 ;;\n\
             esac"
        ),
    );
    stub(
        &dir,
        "id",
        "[ \"$1\" = \"-u\" ] && [ $# -eq 1 ] && { echo 0; exit 0; }\n\
         echo \"bad id argv: $*\" >&2; exit 99",
    );
    for n in ["systemctl", "journalctl", "logger", "sudo", "docker"] {
        stub(&dir, n, "echo \"unexpected stub call: $0 $*\" >&2; exit 98");
    }
    dir
}

fn run(script: &str, dir: &Path) -> Output {
    let bin = dir.join("bin");
    // Hermetic PATH: refuse (exit 3 semantics) unless every guarded tool
    // resolves inside the stub dir.
    let path = format!("{}:/usr/bin:/bin", bin.display());
    for t in GUARDED {
        let out = Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {t}"))
            .env("PATH", &path)
            .output()
            .unwrap();
        let got = String::from_utf8_lossy(&out.stdout).trim().to_string();
        assert!(
            got.starts_with(&bin.display().to_string()),
            "hermetic PATH violated (exit 3): {t} resolves to {got:?}"
        );
    }
    Command::new("bash")
        .arg("-c")
        .arg(script)
        .env("PATH", path)
        .env("LC_ALL", "fr_FR.UTF-8") // the script must force C itself
        .output()
        .expect("run generated script")
}

fn absent() -> Resource {
    Resource {
        resource_type: ResourceType::Cron,
        machine: MachineTarget::Single("m".to_string()),
        name: Some("backup".to_string()),
        state: Some("absent".to_string()),
        ..Default::default()
    }
}

fn script() -> String {
    forjar::core::codegen::apply_script(&absent()).expect("cron apply script")
}

fn installed(dir: &Path) -> Option<String> {
    fs::read_to_string(dir.join("installed")).ok()
}

fn no_argv_violations(dir: &Path) {
    assert!(
        !dir.join("argv-violations").exists(),
        "a stub saw illegal argv: {}",
        fs::read_to_string(dir.join("argv-violations")).unwrap_or_default()
    );
}

/// THE DEFECT. A read that fails for any reason except "no crontab" must not
/// reach the installer: the user's crontab stays untouched and the apply fails.
#[test]
fn a_failed_crontab_read_does_not_wipe_the_crontab() {
    let dir = host(
        "fail",
        &Read::Fails("crontab: must be privileged to use -u"),
    );
    let out = run(&script(), &dir);
    assert!(
        installed(&dir).is_none(),
        "a failed read installed a crontab: {:?}",
        installed(&dir)
    );
    assert_ne!(
        out.status.code(),
        Some(0),
        "a failed read must fail the apply, not report converged"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("must be privileged"),
        "the read error must be surfaced: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    no_argv_violations(&dir);
    let _ = fs::remove_dir_all(&dir);
}

/// The benign case: nothing is installed, so there is nothing to remove, and
/// nothing may be written.
#[test]
fn no_crontab_for_user_is_a_clean_noop() {
    let dir = host("none", &Read::NoCrontab);
    let out = run(&script(), &dir);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        installed(&dir).is_none(),
        "nothing to remove, nothing to install"
    );
    no_argv_violations(&dir);
    let _ = fs::remove_dir_all(&dir);
}

/// The working path still removes exactly this resource's block and keeps
/// every other line.
#[test]
fn a_readable_crontab_loses_only_its_own_block() {
    let dir = host(
        "ok",
        &Read::Has(
            "MAILTO=x\n# forjar:backup\n# forjar-cmd:backup\n0 1 * * * run-backup\n5 5 * * * keep-me\n",
        ),
    );
    let out = run(&script(), &dir);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        installed(&dir).as_deref(),
        Some("MAILTO=x\n5 5 * * * keep-me\n")
    );
    no_argv_violations(&dir);
    let _ = fs::remove_dir_all(&dir);
}

/// A readable crontab without our block is already converged: no write.
#[test]
fn a_crontab_without_the_block_is_not_rewritten() {
    let dir = host("clean", &Read::Has("5 5 * * * keep-me\n"));
    let out = run(&script(), &dir);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        installed(&dir).is_none(),
        "converged crontab must not be rewritten"
    );
    let _ = fs::remove_dir_all(&dir);
}

fn present() -> Resource {
    Resource {
        state: Some("present".to_string()),
        schedule: Some("0 2 * * *".to_string()),
        command: Some("run-backup".to_string()),
        ..absent()
    }
}

fn present_script() -> String {
    forjar::core::codegen::apply_script(&present()).expect("cron apply script")
}

fn check_script() -> String {
    forjar::core::codegen::check_script(&present()).expect("cron check script")
}

/// THE SAME DEFECT on `state: present`: `2>/dev/null | strip || true` turned a
/// failed read into an empty string, then `crontab -` wrote only this entry
/// over the user's real crontab, wiping every other line.
#[test]
fn present_with_a_failed_read_does_not_wipe_the_crontab() {
    let dir = host(
        "pfail",
        &Read::Fails("crontab: must be privileged to use -u"),
    );
    let out = run(&present_script(), &dir);
    assert!(
        installed(&dir).is_none(),
        "installed: {:?}",
        installed(&dir)
    );
    assert_ne!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("must be privileged"));
    no_argv_violations(&dir);
    let _ = fs::remove_dir_all(&dir);
}

/// No crontab yet: present must still install the entry (a first apply).
#[test]
fn present_with_no_crontab_installs_the_entry() {
    let dir = host("pnone", &Read::NoCrontab);
    let out = run(&present_script(), &dir);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let got = installed(&dir).expect("entry installed");
    assert!(
        got.contains("# forjar:backup") && got.contains("0 2 * * * run-backup"),
        "{got:?}"
    );
    no_argv_violations(&dir);
    let _ = fs::remove_dir_all(&dir);
}

/// A readable crontab keeps its other lines and gains the entry.
#[test]
fn present_keeps_the_other_lines() {
    let dir = host("pok", &Read::Has("MAILTO=x\n5 5 * * * keep-me\n"));
    let out = run(&present_script(), &dir);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let got = installed(&dir).expect("installed");
    assert!(
        got.contains("MAILTO=x") && got.contains("keep-me") && got.contains("run-backup"),
        "{got:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// check and apply must agree: an unreadable crontab is an error for check
/// too, not `missing:` (which would send apply to rewrite it).
#[test]
fn check_with_a_failed_read_is_an_error_not_missing() {
    let dir = host(
        "cfail",
        &Read::Fails("crontab: must be privileged to use -u"),
    );
    let out = run(&check_script(), &dir);
    let so = String::from_utf8_lossy(&out.stdout);
    assert!(
        !so.contains("missing:"),
        "failed read reported as missing: {so}"
    );
    assert_ne!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("must be privileged"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn check_with_no_crontab_is_missing() {
    let dir = host("cnone", &Read::NoCrontab);
    let out = run(&check_script(), &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("missing:backup"));
    assert_ne!(out.status.code(), Some(0));
    let _ = fs::remove_dir_all(&dir);
}

/// A localized `no crontab` message must still converge `absent`: the read
/// forces LC_ALL=C so the match is not English-by-accident.
#[test]
fn a_localized_no_crontab_message_still_converges_absent() {
    let dir = host("loc", &Read::LocalizedNone);
    let out = run(&script(), &dir);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(installed(&dir).is_none());
    let _ = fs::remove_dir_all(&dir);
}

/// A failed INSTALL (`crontab -`) must fail the apply.
#[test]
fn a_failed_install_fails_the_apply() {
    let dir = host(
        "ifail",
        &Read::Has("5 5 * * * keep-me\n# forjar:backup\n# forjar-cmd:backup\n0 1 * * * x\n"),
    );
    let d = dir.display();
    stub(
        &dir,
        "crontab",
        &format!("case \"$3\" in -l) cat '{d}/store';; -) cat >/dev/null; exit 1;; esac"),
    );
    let out = run(&script(), &dir);
    assert_ne!(out.status.code(), Some(0));
    let _ = fs::remove_dir_all(&dir);
}

fn state_query() -> String {
    forjar::core::codegen::state_query_script(&present()).expect("cron state query")
}

/// The observation hashed into the lock: an unreadable crontab is an error,
/// never `cron=MISSING`.
#[test]
fn state_query_with_a_failed_read_is_an_error_not_missing() {
    let dir = host(
        "qfail",
        &Read::Fails("crontab: must be privileged to use -u"),
    );
    let out = run(&state_query(), &dir);
    assert!(!String::from_utf8_lossy(&out.stdout).contains("MISSING"));
    assert_ne!(out.status.code(), Some(0));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn state_query_with_no_crontab_is_missing() {
    let dir = host("qnone", &Read::NoCrontab);
    let out = run(&state_query(), &dir);
    assert!(String::from_utf8_lossy(&out.stdout).contains("cron=MISSING:backup"));
    let _ = fs::remove_dir_all(&dir);
}

/// A filter that dies after a good read must not become an empty crontab.
#[test]
fn present_with_a_failing_filter_does_not_wipe_the_crontab() {
    let dir = host("pawk", &Read::Has("MAILTO=x\n5 5 * * * keep-me\n"));
    stub(&dir, "awk", "exit 2");
    let out = run(&present_script(), &dir);
    assert!(installed(&dir).is_none(), "installed {:?}", installed(&dir));
    assert_ne!(out.status.code(), Some(0));
    let _ = fs::remove_dir_all(&dir);
}

/// The non-root path: `LC_ALL=C sudo crontab` must actually execute.
#[test]
fn the_sudo_read_form_executes_for_a_non_root_caller() {
    let dir = host(
        "sudo",
        &Read::Has("5 5 * * * keep-me\n# forjar:backup\n# forjar-cmd:backup\n0 1 * * * x\n"),
    );
    stub(&dir, "id", "echo 1000");
    stub(&dir, "sudo", "[ \"$1\" = -n ] && shift\nexec \"$@\"");
    let out = run(&script(), &dir);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(installed(&dir).as_deref(), Some("5 5 * * * keep-me\n"));
    let _ = fs::remove_dir_all(&dir);
}

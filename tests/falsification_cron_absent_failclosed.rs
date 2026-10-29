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

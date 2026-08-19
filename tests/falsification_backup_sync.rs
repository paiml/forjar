//! FJ-037: falsification tests for the backup sync.
//!
//! These EXECUTE the generated shell with a stubbed `rclone` and assert on the
//! exit code and the status file. They exist because the failure this resource
//! replaces was invisible to every test that reads a script instead of running
//! it: the predecessor's `Backup complete` was emitted by code that was, on
//! inspection, perfectly reasonable.
//!
//! The stub lets a test dictate exactly what `rclone check` reports, so the
//! interesting cases — zero matches, partial coverage, a crashed check, an
//! unconfigured remote — are reachable deterministically.

use forjar::core::types::{BackupSpec, MachineTarget, Resource, ResourceType};
use forjar::resources::backup_sync;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn tmpdir(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "forjar-bkp-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&p).unwrap();
    p
}

/// How the stubbed `rclone check` should behave.
struct Stub {
    /// Remote names `listremotes` reports (empty = remote not configured).
    remotes: &'static str,
    /// Lines the `--combined` file receives; `None` = write nothing at all.
    combined: Option<&'static str>,
}

fn install_rclone_stub(bin: &Path, stub: &Stub) {
    fs::create_dir_all(bin).unwrap();
    let combined = match stub.combined {
        Some(c) => format!(
            "    # emit the caller's --combined file\n    \
             for a in \"$@\"; do\n      \
             if [ \"$prev\" = \"--combined\" ]; then printf '%b' {c:?} > \"$a\"; fi\n      \
             prev=\"$a\"\n    done\n"
        ),
        None => "    :\n".to_string(),
    };
    let script = format!(
        "#!/bin/sh\n\
         cmd=\"$1\"; shift\n\
         prev=\"\"\n\
         case \"$cmd\" in\n\
         \x20 listremotes) printf '%b' {remotes:?}; exit 0 ;;\n\
         \x20 about|lsd)   exit 0 ;;\n\
         \x20 sync)        exit 0 ;;\n\
         \x20 check)\n{combined}    exit 0 ;;\n\
         esac\n\
         exit 0\n",
        remotes = stub.remotes,
    );
    let p = bin.join("rclone");
    fs::write(&p, script).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
}

fn resource(root: &Path, src: &Path) -> Resource {
    Resource {
        resource_type: ResourceType::BackupSync,
        machine: MachineTarget::Single("test".into()),
        home: Some(root.to_string_lossy().into_owned()),
        backup: BackupSpec {
            remote: Some("gdrive:media".into()),
            remote_config: HashMap::new(),
            source: vec![src.to_string_lossy().into_owned()],
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Pull the sync body out of apply's heredoc.
fn sync_body(r: &Resource) -> String {
    const OPEN: &str = "<<'FORJAR_BACKUP_EOF'\n";
    const CLOSE: &str = "\nFORJAR_BACKUP_EOF";
    let apply = backup_sync::apply_script(r);
    let s = apply.find(OPEN).expect("sync heredoc open") + OPEN.len();
    let e = apply[s..].find(CLOSE).expect("sync heredoc close") + s;
    apply[s..e].to_string()
}

struct Run {
    code: i32,
    out: String,
    status: String,
}

fn run(r: &Resource, root: &Path, bin: &Path) -> Run {
    let script = root.join("sync.sh");
    fs::write(&script, sync_body(r)).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let status = root.join("status.json");
    let out = Command::new("/bin/sh")
        .arg(&script)
        .env(
            "PATH",
            format!(
                "{}:{}",
                bin.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("FORJAR_BACKUP_STATUS", &status)
        .output()
        .expect("run sync");
    Run {
        code: out.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&out.stdout).into_owned(),
        status: fs::read_to_string(&status).unwrap_or_default(),
    }
}

fn setup(tag: &str, stub: &Stub) -> (PathBuf, PathBuf, PathBuf) {
    let root = tmpdir(tag);
    let bin = root.join("bin");
    let src = root.join("media");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("a.mp4"), b"aaaa").unwrap();
    install_rclone_stub(&bin, stub);
    (root, bin, src)
}

#[test]
fn a_fully_matched_backup_is_verified_and_exits_zero() {
    let (root, bin, src) = setup(
        "ok",
        &Stub {
            remotes: "gdrive:\n",
            combined: Some("= a.mp4\n= b.mp4\n"),
        },
    );
    let r = run(&resource(&root, &src), &root, &bin);
    assert_eq!(r.code, 0, "verified backup must exit 0\n{}", r.out);
    assert!(r.status.contains(r#""health":"verified""#), "{}", r.status);
    assert!(r.status.contains(r#""coverage_pct":100"#), "{}", r.status);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn zero_matched_files_fails_even_though_rclone_exited_zero() {
    // THE regression. Every rclone invocation succeeds; nothing is in the
    // remote. The predecessor printed "Backup complete" here.
    //
    // `+` is rclone's character for "missing on the destination" — i.e. present
    // locally and NOT backed up. Using `-` here (only-in-remote) would make
    // this test pass against an implementation that cannot see a missing file.
    let (root, bin, src) = setup(
        "nomatch",
        &Stub {
            remotes: "gdrive:\n",
            combined: Some("+ a.mp4\n+ b.mp4\n+ c.mp4\n"),
        },
    );
    let r = run(&resource(&root, &src), &root, &bin);
    assert_eq!(
        r.code, 1,
        "a backup holding nothing must FAIL\n{}\n{}",
        r.out, r.status
    );
    assert!(
        r.status.contains(r#""health":"unverified""#),
        "{}",
        r.status
    );
    assert!(r.status.contains(r#""missing":3"#), "{}", r.status);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn an_empty_verification_is_a_broken_check_not_an_empty_backup() {
    // rclone check writes an empty --combined file: 0 files examined.
    let (root, bin, src) = setup(
        "empty",
        &Stub {
            remotes: "gdrive:\n",
            combined: Some(""),
        },
    );
    let r = run(&resource(&root, &src), &root, &bin);
    assert_eq!(r.code, 1, "0 examined files must fail\n{}", r.out);
    assert!(
        r.out.contains("broken check, not an empty backup"),
        "{}",
        r.out
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_crashed_check_that_writes_nothing_counts_as_an_error() {
    let (root, bin, src) = setup(
        "nofile",
        &Stub {
            remotes: "gdrive:\n",
            combined: None,
        },
    );
    let r = run(&resource(&root, &src), &root, &bin);
    assert_eq!(
        r.code, 1,
        "a check producing no output must fail\n{}",
        r.out
    );
    assert!(r.out.contains("produced NO output"), "{}", r.out);
    assert!(r.status.contains(r#""errors":1"#), "{}", r.status);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn coverage_below_the_threshold_fails() {
    // 2 of 4 = 50%, well under the default 99%.
    let (root, bin, src) = setup(
        "partial",
        &Stub {
            remotes: "gdrive:\n",
            combined: Some("= a.mp4\n= b.mp4\n+ c.mp4\n* d.mp4\n"),
        },
    );
    let r = run(&resource(&root, &src), &root, &bin);
    assert_eq!(r.code, 1, "partial coverage must fail\n{}", r.out);
    assert!(r.status.contains(r#""health":"partial""#), "{}", r.status);
    assert!(r.status.contains(r#""coverage_pct":50"#), "{}", r.status);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn an_unconfigured_remote_stops_the_run_before_any_sync() {
    // `listremotes` reports nothing. rclone would otherwise treat `gdrive:media`
    // as a LOCAL path — the mechanism behind the self-referential predecessor.
    let (root, bin, src) = setup(
        "noremote",
        &Stub {
            remotes: "",
            combined: Some("= a.mp4\n"),
        },
    );
    let r = run(&resource(&root, &src), &root, &bin);
    assert_eq!(r.code, 1, "unconfigured remote must abort\n{}", r.out);
    assert!(r.out.contains("is not configured"), "{}", r.out);
    // ...and it must abort BEFORE claiming to have synced anything.
    assert!(!r.out.contains("sync media: starting"), "{}", r.out);
    assert!(
        r.status.is_empty(),
        "no status may be written: {}",
        r.status
    );
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_missing_source_stops_the_run() {
    let (root, bin, src) = setup(
        "nosrc",
        &Stub {
            remotes: "gdrive:\n",
            combined: Some("= a.mp4\n"),
        },
    );
    fs::remove_dir_all(&src).unwrap();
    let r = run(&resource(&root, &src), &root, &bin);
    assert_eq!(r.code, 1, "missing source must abort\n{}", r.out);
    assert!(r.out.contains("does not exist"), "{}", r.out);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn a_missing_rclone_binary_stops_the_run() {
    let root = tmpdir("norclone");
    let bin = root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let src = root.join("media");
    fs::create_dir_all(&src).unwrap();

    // Hermetic PATH: symlink in exactly the utilities the preflight needs and
    // nothing else. Relying on "rclone happens not to be installed on this
    // host" is not a test — it broke the moment rclone was deployed here.
    for tool in [
        "sh", "mktemp", "grep", "find", "rmdir", "head", "awk", "sed", "cat", "du", "stat",
    ] {
        for dir in ["/usr/bin", "/bin", "/usr/local/bin"] {
            let p = Path::new(dir).join(tool);
            if p.exists() {
                let _ = std::os::unix::fs::symlink(&p, bin.join(tool));
                break;
            }
        }
    }
    assert!(
        !bin.join("rclone").exists(),
        "the hermetic bin must not contain rclone"
    );

    let script = root.join("sync.sh");
    fs::write(&script, sync_body(&resource(&root, &src))).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let out = Command::new("/bin/sh")
        .arg(&script)
        .env("PATH", bin.display().to_string())
        .env("FORJAR_BACKUP_STATUS", root.join("status.json"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("rclone is not installed"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn files_only_in_the_remote_are_stale_and_do_not_fail_the_backup() {
    // `-` = missing on the source = present only in the remote. That means the
    // local file was deleted, not that anything is unprotected. It must be
    // reported but must not drag coverage below threshold, or every legitimate
    // local deletion would show up as a backup failure until the next sync.
    let (root, bin, src) = setup(
        "stale",
        &Stub {
            remotes: "gdrive:\n",
            combined: Some("= a.mp4\n= b.mp4\n- old.mp4\n"),
        },
    );
    let r = run(&resource(&root, &src), &root, &bin);
    assert_eq!(
        r.code, 0,
        "stale remote files must not fail a fully-covered backup\n{}\n{}",
        r.out, r.status
    );
    assert!(r.status.contains(r#""health":"verified""#), "{}", r.status);
    assert!(r.status.contains(r#""coverage_pct":100"#), "{}", r.status);
    assert!(r.status.contains(r#""stale_in_remote":1"#), "{}", r.status);
    fs::remove_dir_all(&root).ok();
}

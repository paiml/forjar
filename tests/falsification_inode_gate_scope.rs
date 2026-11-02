//! forjar#847 fix round 3: the apply inode check measures only the machines
//! the run selects (-r/-t/-g), and validate names the machines it skips.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Fx {
    path: String,
    _d: tempfile::TempDir,
    cfg: PathBuf,
    state: PathBuf,
    p1: PathBuf,
    p2: PathBuf,
}

/// `cold` is this host (inodes fine); `hot` is a container whose fake runtime
/// reports 99% inode use. r1 lives on cold, r2 on hot.
fn fx() -> Fx {
    let d = tempfile::tempdir().unwrap();
    let bin = d.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    let rt = bin.join("docker");
    std::fs::write(
        &rt,
        "#!/bin/sh\ncase \"$1\" in inspect) echo true; exit 0;; esac\ncat >/dev/null\n\
         printf '%s\\n' 'Filesystem Type Inodes IUsed IFree IUse% Mounted on' 'overlay overlay 1000 990 10 99% /'\n",
    )
    .unwrap();
    std::fs::set_permissions(&rt, std::fs::Permissions::from_mode(0o755)).unwrap();
    let (p1, p2) = (d.path().join("one"), d.path().join("two"));
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: t\nmachines:\n  cold:\n    hostname: localhost\n    addr: 127.0.0.1\n  \
             hot:\n    hostname: hot\n    addr: container\n    transport: container\n    container:\n      \
             runtime: docker\n      image: x\n      ephemeral: false\n\
             resources:\n  r1:\n    type: file\n    machine: cold\n    path: {p1}\n    content: hi\n    \
             tags: [a]\n    resource_group: g1\n  r2:\n    type: file\n    machine: hot\n    path: {p2}\n    \
             content: hi\n    tags: [b]\n    resource_group: g2\n\
             inode_pressure:\n  threshold_pct: 100\n  mounts:\n    - path: /\n      threshold_pct: 98\n",
            p1 = p1.display(),
            p2 = p2.display()
        ),
    )
    .unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    Fx {
        path,
        state: d.path().join("state"),
        cfg,
        p1,
        p2,
        _d: d,
    }
}

fn forjar(f: &Fx, args: &[&str]) -> (bool, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .env("PATH", &f.path)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    (
        o.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
    )
}

fn apply(f: &Fx, extra: &[&str]) -> (bool, String) {
    let (c, s) = (f.cfg.to_str().unwrap(), f.state.to_str().unwrap());
    let mut a = vec!["apply", "-f", c, "--state-dir", s, "--yes"];
    a.extend_from_slice(extra);
    forjar(f, &a)
}

fn exists(p: &Path) -> bool {
    p.exists()
}

#[test]
fn control_an_unscoped_apply_is_refused_by_the_hot_machine() {
    let f = fx();
    let (ok, out) = apply(&f, &[]);
    assert!(!ok && out.contains("hot") && out.contains("99%"), "{out}");
    assert!(!exists(&f.p1), "refusal must precede every write");
}

#[test]
fn apply_scoped_to_a_resource_measures_only_its_machine() {
    let f = fx();
    let (ok, out) = apply(&f, &["-r", "r1"]);
    assert!(ok, "-r r1 was blocked by a machine it never touches: {out}");
    assert!(exists(&f.p1) && !exists(&f.p2));
}

#[test]
fn apply_scoped_by_tag_measures_only_its_machine() {
    let f = fx();
    let (ok, out) = apply(&f, &["-t", "a"]);
    assert!(ok, "-t a was blocked by a machine it never touches: {out}");
    assert!(exists(&f.p1) && !exists(&f.p2));
}

#[test]
fn apply_scoped_by_group_measures_only_its_machine() {
    let f = fx();
    let (ok, out) = apply(&f, &["-g", "g1"]);
    assert!(ok, "-g g1 was blocked by a machine it never touches: {out}");
    assert!(exists(&f.p1) && !exists(&f.p2));
}

#[test]
fn a_scope_that_reaches_the_hot_machine_is_still_refused() {
    let f = fx();
    let (ok, out) = apply(&f, &["-r", "r2"]);
    assert!(!ok && out.contains("hot"), "{out}");
}

#[test]
fn validate_says_not_measured_for_each_machine_it_skips() {
    let f = fx();
    let (ok, out) = forjar(&f, &["validate", "-f", f.cfg.to_str().unwrap()]);
    assert!(ok, "{out}");
    assert!(
        out.lines()
            .any(|l| l.contains("not measured") && l.contains("hot")),
        "validate skipped `hot` silently: {out}"
    );
}

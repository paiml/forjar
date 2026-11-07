//! forjar#893 — `scripts/install-lambda.sh --check` must refuse a dirty tree
//! and a HEAD that is neither the origin/main tip nor an exact tag, and accept
//! a clean origin/main tip and a clean exact tag. Runs against throwaway git
//! repos; installs nothing.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn git(dir: &Path, args: &[&str]) {
    let o = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("git");
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/install-lambda.sh")
}

fn check(repo: &Path) -> Output {
    Command::new("bash")
        .arg(script())
        .arg("--check")
        .env("INSTALL_LAMBDA_REPO", repo)
        .env("INSTALL_LAMBDA_NO_FETCH", "1")
        .output()
        .expect("run script")
}

/// repo with origin/main == HEAD, clean.
fn fixture(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("il-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let origin = base.join("origin.git");
    let work = base.join("work");
    std::fs::create_dir_all(&work).unwrap();
    git(&base, &["init", "-q", "--bare", "-b", "main", "origin.git"]);
    git(&work, &["init", "-q", "-b", "main"]);
    std::fs::write(work.join("f"), "1").unwrap();
    git(&work, &["add", "f"]);
    git(&work, &["commit", "-q", "-m", "one"]);
    git(
        &work,
        &["remote", "add", "origin", origin.to_str().unwrap()],
    );
    git(&work, &["push", "-q", "origin", "main"]);
    git(&work, &["fetch", "-q", "origin"]);
    work
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn accepts_clean_origin_main_tip() {
    let w = fixture("ok");
    let o = check(&w);
    assert!(o.status.success(), "{}", err(&o));
}

#[test]
fn refuses_dirty_tree() {
    let w = fixture("dirty");
    std::fs::write(w.join("f"), "2").unwrap();
    let o = check(&w);
    assert!(!o.status.success());
    assert!(err(&o).contains("dirty tree"), "{}", err(&o));
}

#[test]
fn refuses_untracked_file() {
    let w = fixture("untracked");
    std::fs::write(w.join("new"), "x").unwrap();
    let o = check(&w);
    assert!(!o.status.success());
    assert!(err(&o).contains("dirty tree"), "{}", err(&o));
}

#[test]
fn refuses_head_off_main_and_untagged() {
    let w = fixture("off");
    git(&w, &["checkout", "-q", "-b", "feat"]);
    std::fs::write(w.join("f"), "2").unwrap();
    git(&w, &["commit", "-q", "-am", "two"]);
    let o = check(&w);
    assert!(!o.status.success());
    assert!(
        err(&o).contains("neither origin/main tip nor an exact tag"),
        "{}",
        err(&o)
    );
}

#[test]
fn accepts_exact_tag_off_main() {
    let w = fixture("tag");
    git(&w, &["checkout", "-q", "-b", "rel"]);
    std::fs::write(w.join("f"), "2").unwrap();
    git(&w, &["commit", "-q", "-am", "two"]);
    git(&w, &["tag", "v9.9.9"]);
    let o = check(&w);
    assert!(o.status.success(), "{}", err(&o));
}

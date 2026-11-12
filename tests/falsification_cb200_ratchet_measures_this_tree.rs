//! PMAT-206: the CB-200 ratchet must grade THIS tree, not a cached one.
//!
//! # Why this test exists
//!
//! `scripts/cb200-ratchet.sh` compares `pmat comply`'s count of functions
//! below grade A against a recorded ceiling. `pmat comply` grades from its own
//! cache under `~/.cache/paiml-mcp-agent-toolkit/comply/index/<tree>-<hash>/`,
//! and `pmat query --rebuild-index` — the refresh the script always ran — does
//! not touch that cache. Measured on 2026-09-08: after `observe::classify` had
//! been turned from a 34-arm match into a table, the gate went on reporting
//! `classify [F] (complexity: 34)` at its old line, and the release count sat
//! three above the ceiling through three real reductions. Removing the cache
//! entry gave the true number, 651, at once.
//!
//! A gate quoting a tree that no longer exists is worse than no gate. The rule
//! this file pins: when the newest tracked source file is newer than the cache
//! directory, the script says so and removes it BEFORE comply runs; when the
//! cache is newer, it is left alone; and the ceiling comparison is untouched by
//! either branch. The `pmat` on PATH is a shim that records whether the cache
//! directory existed at the moment comply was invoked — the one fact that
//! distinguishes a measurement of this tree from a measurement of a stale one.
//!
//! Observed RED with the age rule reverted: the stale directory survives into
//! the comply run and the NOTE never prints.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
    std::fs::write(&p, body).expect("write");
}

fn git(root: &Path, args: &[&str]) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .status()
        .expect("git")
        .success();
    assert!(ok, "git {args:?} failed");
}

/// A `pmat` that answers `query` with nothing and `comply check --format json`
/// with one CB-200 line reporting `$CB200_COUNT` functions — and that writes,
/// at the moment comply is invoked, whether `$CB200_WATCH` still exists.
const PMAT_SHIM: &str = r#"#!/usr/bin/env bash
set -euo pipefail
case "${1:-}" in
  query) exit 0 ;;
  comply)
    if [ -d "$CB200_WATCH" ]; then echo present > "$CB200_SEEN"; else echo absent > "$CB200_SEEN"; fi
    # The newline INSIDE the JSON string must reach the parser as the two
    # characters `\n`, so printf gets `\\n` there and a real newline at the end.
    printf '{"checks":[{"name":"CB-200: TDG Grade Gate","status":"Fail","severity":"Error","message":"%s function(s) below minimum grade A\\n    a.rs:1 f [F] (complexity: 40)"}]}\n' "$CB200_COUNT"
    ;;
  *) echo "shim: unexpected $*" >&2; exit 2 ;;
esac
"#;

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    cache_root: PathBuf,
    cache_dir: PathBuf,
    seen: PathBuf,
    bin: PathBuf,
}

/// One tracked source file, the real ratchet, a ceiling of 651, and a comply
/// cache directory named the way pmat names them: `<basename of root>-<hash>`.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("tree");
    std::fs::create_dir_all(&root).expect("mkdir");
    let script = std::fs::read_to_string(repo().join("scripts/cb200-ratchet.sh"))
        .expect("the ratchet under test");
    write(&root, "scripts/cb200-ratchet.sh", &script);
    write(
        &root,
        "scripts/ratchets/cb200-baseline.json",
        r#"{"check":"CB-200","ceiling":651,"rule":"MAY ONLY SHRINK"}"#,
    );
    write(&root, "src/lib.rs", "pub fn f() {}\n");
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(&root, &["config", "core.hooksPath", "/nonexistent"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "fixture"]);

    let bin = dir.path().join("bin");
    std::fs::create_dir_all(&bin).expect("mkdir bin");
    let shim = bin.join("pmat");
    std::fs::write(&shim, PMAT_SHIM).expect("shim");
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    let cache_root = dir.path().join("cache");
    let basename = root.file_name().unwrap().to_string_lossy().to_string();
    let cache_dir = cache_root.join(format!("{basename}-deadbeef"));
    std::fs::create_dir_all(&cache_dir).expect("cache dir");
    std::fs::write(cache_dir.join("context.db"), "stale").expect("cache file");
    Fixture {
        seen: dir.path().join("seen"),
        _dir: dir,
        root,
        cache_root,
        cache_dir,
        bin,
    }
}

/// Set the cache directory's mtime relative to the tree: a year ago (stale)
/// or a year ahead (fresh).
fn set_cache_age(fx: &Fixture, stamp: &str) {
    let ok = Command::new("touch")
        .args(["-d", stamp])
        .arg(&fx.cache_dir)
        .status()
        .expect("touch")
        .success();
    assert!(ok, "touch -d {stamp}");
}

fn run(fx: &Fixture, count: &str) -> (i32, String) {
    let path = format!(
        "{}:{}",
        fx.bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .arg("scripts/cb200-ratchet.sh")
        .current_dir(&fx.root)
        .env("PATH", path)
        .env("PMAT_COMPLY_CACHE", &fx.cache_root)
        .env("CB200_WATCH", &fx.cache_dir)
        .env("CB200_SEEN", &fx.seen)
        .env("CB200_COUNT", count)
        .output()
        .expect("run the ratchet");
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    (out.status.code().unwrap_or(-1), text)
}

fn seen(fx: &Fixture) -> String {
    std::fs::read_to_string(&fx.seen)
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// The defect: a cache older than the tree. It must be gone by the time
/// comply runs, and the script must say so.
#[test]
fn a_cache_older_than_the_tree_is_removed_before_comply_runs() {
    let fx = fixture();
    set_cache_age(&fx, "2000-01-01T00:00:00");
    let (code, text) = run(&fx, "651");
    assert_eq!(
        code, 0,
        "the ratchet went red on a fresh measurement: {text}"
    );
    assert!(
        text.contains("NOTE:"),
        "the staleness was not announced: {text}"
    );
    assert_eq!(
        seen(&fx),
        "absent",
        "comply ran with the stale cache still in place: {text}"
    );
    assert!(!fx.cache_dir.exists(), "the stale cache directory survived");
}

/// The control: a cache newer than the tree is what a fresh comply run leaves
/// behind, and it must be left alone — no NOTE, present when comply runs.
#[test]
fn a_cache_newer_than_the_tree_is_left_alone() {
    let fx = fixture();
    set_cache_age(&fx, "2100-01-01T00:00:00");
    let (code, text) = run(&fx, "651");
    assert_eq!(code, 0, "{text}");
    assert!(
        !text.contains("NOTE:"),
        "a fresh cache was called stale: {text}"
    );
    assert_eq!(seen(&fx), "present", "a fresh cache was removed: {text}");
}

/// The ceiling is still the ceiling: a count above it is REGRESSED and red,
/// whichever branch the cache rule took.
#[test]
fn the_ceiling_is_still_enforced_after_the_cache_rule() {
    let fx = fixture();
    set_cache_age(&fx, "2000-01-01T00:00:00");
    let (code, text) = run(&fx, "652");
    assert_ne!(code, 0, "652 against a ceiling of 651 passed: {text}");
    assert!(text.contains("REGRESSED"), "wrong reason: {text}");
}

/// And nothing outside the cache root can be touched: a cache root that does
/// not exist is simply skipped, and the run still measures.
#[test]
fn a_missing_cache_root_is_skipped_not_created_or_removed() {
    let fx = fixture();
    std::fs::remove_dir_all(&fx.cache_root).expect("remove cache root");
    let (code, text) = run(&fx, "651");
    assert_eq!(code, 0, "{text}");
    assert!(
        !fx.cache_root.exists(),
        "the script created a cache root it was not asked for"
    );
}

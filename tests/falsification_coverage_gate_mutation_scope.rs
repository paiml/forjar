//! PMAT-165: gate F must not call a branch with nothing to mutate UNMEASURED.
//!
//! # Why this test exists
//!
//! `scripts/dogfood/coverage.sh` Arm 3 runs `cargo mutants --in-diff` over the
//! branch's own diff and refuses a run that produced no `outcomes.json`, on the
//! ground that a mutation arm which did not run is not a clean one. That is the
//! right rule for a diff that changes library code. It is the wrong rule for a
//! diff whose only Rust change is a test: `cargo mutants` mutates library and
//! binary targets, not `tests/`, so it prints `INFO No mutants to filter`,
//! exits 0, and writes nothing.
//!
//! Measured on 2026-09-08 against the 1.26.0 release cut, whose single Rust
//! change is a falsification test:
//!
//! ```text
//! GATE F FAIL cargo mutants wrote no outcomes.json (exit 0) — the mutation arm is UNMEASURED
//! ```
//!
//! The release gate went red on a branch with nothing to mutate. Arm 3 now
//! counts the MUTABLE changed files — the ones under a `src/` directory — and
//! only demands mutants when that set is non-empty. Both halves are pinned
//! here: a tests-only diff passes, and a diff that touches `src/` still has to
//! produce mutants and kill them.
//!
//! The `cargo` on PATH is a shim: it answers the three subcommands this gate
//! calls and records whether `mutants` was invoked at all, which is the fact
//! that separates "nothing to mutate" from "the tool did not run".

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
    std::fs::write(p, body).expect("write");
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
    assert!(ok, "git {args:?}");
}

/// A `cargo` that answers exactly what gate F asks and records the `mutants`
/// call. `MUT_OUTCOMES` selects what the mutation run leaves behind: `none`
/// writes no outcomes.json at all (what the real tool does with nothing to
/// mutate), `clean` writes one caught mutant, `missed` writes one survivor.
const CARGO_SHIM: &str = r#"#!/usr/bin/env bash
set -uo pipefail
case "${1:-}" in
  test)
    echo "test result: ok. 1 passed; 0 failed; 43 ignored; 0 measured; 0 filtered out"
    ;;
  llvm-cov)
    printf 'TOTAL 100 0 100.00%% 100 0 100.00%% 100 0 96.40%% 100 0 100.00%%\n'
    ;;
  mutants)
    echo ran > "$MUT_CALLED"
    out=""
    while [ $# -gt 0 ]; do
      if [ "$1" = "--output" ]; then out="$2"; fi
      shift
    done
    case "${MUT_OUTCOMES}" in
      none) echo "INFO No mutants to filter" ;;
      clean)
        mkdir -p "$out/mutants.out"
        echo '{"total_mutants":1,"outcomes":[{"summary":"CaughtMutant"}]}' > "$out/mutants.out/outcomes.json"
        ;;
      missed)
        mkdir -p "$out/mutants.out"
        echo '{"total_mutants":1,"outcomes":[{"summary":"MissedMutant","scenario":{"Mutant":{"file":"src/lib.rs","span":{"start":{"line":1}},"genre":"g","replacement":"r"}}}]}' > "$out/mutants.out/outcomes.json"
        ;;
    esac
    ;;
  *) echo "cargo shim: unexpected $*" >&2; exit 2 ;;
esac
"#;

const PMAT_SHIM: &str = "#!/usr/bin/env bash\nexit 0\n";

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    bin: PathBuf,
    called: PathBuf,
}

/// A repo with `origin/main` behind it, the real gate, a Makefile carrying the
/// floor the gate insists on, and the contracts crate the gate greps.
fn fixture(second_commit: &[(&str, &str)]) -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("tree");
    std::fs::create_dir_all(&root).expect("mkdir");

    let gate = std::fs::read_to_string(repo().join("scripts/dogfood/coverage.sh"))
        .expect("the gate under test");
    write(&root, "scripts/dogfood/coverage.sh", &gate);
    write(
        &root,
        "Makefile",
        "coverage:\n\tcargo llvm-cov --workspace --locked --fail-under-lines 95\n",
    );
    // The gate counts `not(feature = "aprender-corpus")` annotations and holds
    // them at an exact number, so the fixture carries exactly that many.
    let annotations = std::fs::read_to_string(repo().join("scripts/dogfood/coverage.sh"))
        .expect("gate")
        .lines()
        .find_map(|l| {
            l.strip_prefix("APRENDER_ANNOTATIONS=")
                .map(|n| n.trim().to_string())
        })
        .expect("APRENDER_ANNOTATIONS in the gate")
        .parse::<usize>()
        .expect("a number");
    let mut body = String::new();
    for i in 0..annotations {
        body.push_str(&format!(
            "#[cfg_attr(not(feature = \"aprender-corpus\"), ignore = \"corpus\")]\nfn a{i}() {{}}\n"
        ));
    }
    write(&root, "crates/forjar-contracts/src/lib.rs", &body);
    write(&root, "src/lib.rs", "pub fn f() -> u32 { 1 }\n");
    git(&root, &["init", "-q", "-b", "main"]);
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(&root, &["config", "core.hooksPath", "/nonexistent"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "base"]);
    // `origin/main` without a remote: the gate only needs the ref to resolve.
    git(&root, &["update-ref", "refs/remotes/origin/main", "HEAD"]);

    for (rel, body) in second_commit {
        write(&root, rel, body);
    }
    if !second_commit.is_empty() {
        git(&root, &["add", "-A"]);
        git(&root, &["commit", "-qm", "branch"]);
    }

    let bin = dir.path().join("bin");
    std::fs::create_dir_all(&bin).expect("mkdir bin");
    for (name, body) in [("cargo", CARGO_SHIM), ("pmat", PMAT_SHIM)] {
        let p = bin.join(name);
        std::fs::write(&p, body).expect("shim");
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    Fixture {
        called: dir.path().join("mut-called"),
        _dir: dir,
        root,
        bin,
    }
}

fn run(fx: &Fixture, outcomes: &str) -> (i32, String) {
    let _ = std::fs::remove_file(&fx.called);
    let path = format!(
        "{}:{}",
        fx.bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .arg("scripts/dogfood/coverage.sh")
        .current_dir(&fx.root)
        .env("PATH", path)
        .env("MUT_OUTCOMES", outcomes)
        .env("MUT_CALLED", &fx.called)
        .output()
        .expect("run gate F");
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    (out.status.code().unwrap_or(-1), text)
}

fn mutants_ran(fx: &Fixture) -> bool {
    fx.called.exists()
}

/// The release cut's shape: the only Rust change is a test. `cargo mutants` has
/// no target to mutate, so the arm passes and says why — it does not report an
/// unmeasured run.
#[test]
fn a_diff_whose_only_rust_change_is_a_test_passes_and_says_why() {
    let fx = fixture(&[("tests/falsification_new.rs", "#[test]\nfn t() {}\n")]);
    let (code, text) = run(&fx, "none");
    assert_eq!(
        code, 0,
        "gate F went red on a branch with nothing to mutate: {text}"
    );
    assert!(
        text.contains("none under src/"),
        "it passed without saying why the mutation arm was empty: {text}"
    );
}

/// The half that must not slip: a diff that touches library code still has to
/// produce mutants, and a run that produced none is UNMEASURED.
#[test]
fn a_diff_touching_src_still_demands_mutants() {
    let fx = fixture(&[("src/lib.rs", "pub fn f() -> u32 { 2 }\n")]);
    let (code, text) = run(&fx, "none");
    assert_ne!(
        code, 0,
        "a src/ change with no mutation outcome passed: {text}"
    );
    assert!(text.contains("UNMEASURED"), "wrong reason: {text}");
    assert!(
        mutants_ran(&fx),
        "cargo mutants was never invoked for a src/ change"
    );
}

/// And a surviving mutant is still a failure.
#[test]
fn a_surviving_mutant_in_src_is_red() {
    let fx = fixture(&[("src/lib.rs", "pub fn f() -> u32 { 2 }\n")]);
    let (code, text) = run(&fx, "missed");
    assert_ne!(code, 0, "a survivor passed: {text}");
    assert!(text.contains("survived"), "wrong reason: {text}");
}

/// A src/ change whose mutants are all caught passes, which is what keeps the
/// two cases above from being vacuous.
#[test]
fn a_src_change_whose_mutants_are_caught_passes() {
    let fx = fixture(&[("src/lib.rs", "pub fn f() -> u32 { 2 }\n")]);
    let (code, text) = run(&fx, "clean");
    assert_eq!(code, 0, "{text}");
    assert!(
        mutants_ran(&fx),
        "the mutation arm was skipped for a src/ change: {text}"
    );
}

/// A branch with no Rust change at all was already handled; it must stay that
/// way, and without invoking the mutation tool.
#[test]
fn a_diff_with_no_rust_change_skips_the_mutation_arm() {
    let fx = fixture(&[("README.md", "prose\n")]);
    let (code, text) = run(&fx, "none");
    assert_eq!(code, 0, "{text}");
    assert!(
        !mutants_ran(&fx),
        "cargo mutants ran for a diff with no .rs file: {text}"
    );
}

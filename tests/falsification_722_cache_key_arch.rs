//! forjar#722 falsification: lint.yml and msrv.yml run on arch-neutral clean-room
//! runners, so every actions/cache key must carry `runner.arch`. The shell guard
//! `tests/cache-key-arch-guard.sh` decides; this test runs it on the real tree
//! (must pass) and on a copy with the arch segment reverted (must fail), so the
//! guard is shown capable of failing.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const WORKFLOWS: [&str; 2] = [".github/workflows/lint.yml", ".github/workflows/msrv.yml"];

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Run the guard from `root/tests/`, which is where it resolves its workflows.
fn run_guard(root: &Path) -> (bool, String) {
    let out = Command::new("bash")
        .arg(root.join("tests/cache-key-arch-guard.sh"))
        .output()
        .expect("bash runs the guard");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    (out.status.success(), text)
}

/// Copy the guard and both workflows into a scratch tree, optionally reverting the
/// arch segment of every cache key.
fn scratch_tree(name: &str, revert: bool) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("forjar-722-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    fs::copy(
        repo().join("tests/cache-key-arch-guard.sh"),
        dir.join("tests/cache-key-arch-guard.sh"),
    )
    .unwrap();
    for wf in WORKFLOWS {
        let mut text = fs::read_to_string(repo().join(wf)).unwrap();
        if revert {
            text = text.replace("${{ runner.arch }}-", "");
        }
        fs::write(dir.join(wf), text).unwrap();
    }
    dir
}

#[test]
fn every_cache_key_carries_runner_arch() {
    let (ok, text) = run_guard(&repo());
    assert!(ok, "guard failed on the real tree:\n{text}");
    for wf in WORKFLOWS {
        assert!(
            text.contains(&format!("ok {wf}")),
            "guard did not report {wf}:\n{text}"
        );
    }
}

#[test]
fn the_pre_722_cache_keys_are_caught() {
    let dir = scratch_tree("reverted", true);
    let (ok, text) = run_guard(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert!(!ok, "guard passed with runner.arch reverted:\n{text}");
    for wf in WORKFLOWS {
        assert!(
            text.contains(&format!("FAIL {wf}")),
            "guard did not name {wf}:\n{text}"
        );
    }
}

#[test]
fn an_unmodified_copy_passes() {
    let dir = scratch_tree("clean", false);
    let (ok, text) = run_guard(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert!(ok, "guard failed on an unmodified copy:\n{text}");
}

/// A scratch tree where only the `key:` line of lint.yml loses its arch segment
/// and a `restore-keys:` line that still carries it is added, so a comparison of
/// key-line count against arch-line count (1 == 1) would have passed it.
#[test]
fn a_key_without_arch_is_caught_even_if_restore_keys_has_it() {
    let dir = scratch_tree("mixed", false);
    let lint = dir.join(".github/workflows/lint.yml");
    let text = fs::read_to_string(&lint).unwrap();
    let mut out = String::new();
    for line in text.lines() {
        if line.trim_start().starts_with("key: ") {
            let pad = &line[..line.len() - line.trim_start().len()];
            out.push_str(&line.replace("${{ runner.arch }}-", ""));
            out.push('\n');
            out.push_str(&format!(
                "{pad}restore-keys: lint-${{{{ runner.arch }}}}-\n"
            ));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    fs::write(&lint, out).unwrap();
    let (ok, text) = run_guard(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert!(!ok, "guard passed a key with no arch:\n{text}");
    assert!(text.contains("FAIL .github/workflows/lint.yml"), "{text}");
}

#[test]
fn a_block_scalar_restore_keys_is_refused() {
    let dir = scratch_tree("block", false);
    let msrv = dir.join(".github/workflows/msrv.yml");
    let mut text = fs::read_to_string(&msrv).unwrap();
    text.push_str("          restore-keys: |\n            msrv-${{ runner.arch }}-\n");
    fs::write(&msrv, text).unwrap();
    let (ok, text) = run_guard(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert!(!ok, "guard accepted a block-scalar restore-keys:\n{text}");
    assert!(text.contains("FAIL .github/workflows/msrv.yml"), "{text}");
    assert!(text.contains("1 empty/block/sequence"), "{text}");
}

#[test]
fn a_sequence_restore_keys_is_refused() {
    let dir = scratch_tree("seq", false);
    let msrv = dir.join(".github/workflows/msrv.yml");
    let mut text = fs::read_to_string(&msrv).unwrap();
    text.push_str("          restore-keys:\n            - msrv-${{ runner.arch }}-\n");
    fs::write(&msrv, text).unwrap();
    let (ok, text) = run_guard(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert!(!ok, "guard accepted a sequence restore-keys:\n{text}");
    assert!(text.contains("1 empty/block/sequence"), "{text}");
}

#[test]
fn an_arch_only_in_a_comment_does_not_count() {
    let dir = scratch_tree("comment", false);
    let lint = dir.join(".github/workflows/lint.yml");
    let text = fs::read_to_string(&lint).unwrap();
    let text = text.replace(
        "${{ runner.os }}-${{ runner.arch }}-${{ matrix.features }}",
        "${{ runner.os }}-${{ matrix.features }}",
    );
    let text = text.replacen(
        "-${{ hashFiles('**/Cargo.lock') }}",
        "-${{ hashFiles('**/Cargo.lock') }}  # runner.arch",
        1,
    );
    fs::write(&lint, text).unwrap();
    let (ok, text) = run_guard(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert!(!ok, "guard counted runner.arch inside a comment:\n{text}");
    assert!(text.contains("FAIL .github/workflows/lint.yml"), "{text}");
}

#[test]
fn a_missing_workflow_is_a_failure_not_a_pass() {
    let dir = scratch_tree("missing", false);
    fs::remove_file(dir.join(".github/workflows/msrv.yml")).unwrap();
    let (ok, text) = run_guard(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert!(!ok, "guard passed with msrv.yml absent:\n{text}");
    assert!(
        text.contains("FAIL .github/workflows/msrv.yml: file is missing"),
        "{text}"
    );
}

/// The workflow files named in `ARCH_NEUTRAL_JOBS`, read from the source of
/// tests/falsification_every_ci_job_runs_on_the_fleet/main.rs (a separate test
/// binary, so it cannot be imported). Every non-blank line inside the array must be
/// one `("file.yml", "job"),` tuple, and the tuple count must equal the declared
/// array length, so a reformatted entry fails loudly instead of being dropped.
fn arch_neutral_files() -> Vec<String> {
    let src = fs::read_to_string(
        repo().join("tests/falsification_every_ci_job_runs_on_the_fleet/main.rs"),
    )
    .unwrap();
    let decl = src
        .find("const ARCH_NEUTRAL_JOBS: [(&str, &str); ")
        .expect("ARCH_NEUTRAL_JOBS is declared with a length");
    let rest = &src[decl + "const ARCH_NEUTRAL_JOBS: [(&str, &str); ".len()..];
    let declared: usize = rest[..rest.find(']').unwrap()].trim().parse().unwrap();
    let body = &rest[rest.find("= [").expect("array opens") + 3..];
    let body = &body[..body.find("\n];").expect("array closes")];
    let mut files = Vec::new();
    for line in body.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let tuple = line
            .strip_prefix("(\"")
            .and_then(|l| l.strip_suffix("\"),"))
            .and_then(|l| l.split_once("\", \""))
            .unwrap_or_else(|| panic!("unparsable ARCH_NEUTRAL_JOBS line: {line}"));
        files.push(tuple.0.to_string());
    }
    assert_eq!(
        files.len(),
        declared,
        "parsed {} ARCH_NEUTRAL_JOBS entries, declared {declared}",
        files.len()
    );
    assert!(!files.is_empty(), "read no files out of ARCH_NEUTRAL_JOBS");
    files.sort();
    files.dedup();
    files
}

/// One list is authoritative: every workflow file with an arch-neutral job must be
/// checked by BOTH this test's WORKFLOWS and the shell guard, or its cache key
/// could drop runner.arch with every test green.
#[test]
fn every_arch_neutral_workflow_is_in_the_guarded_set() {
    let guard = fs::read_to_string(repo().join("tests/cache-key-arch-guard.sh")).unwrap();
    let code: String = guard
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .map(|l| l.split(" #").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let (ok, text) = run_guard(&repo());
    assert!(ok, "guard failed on the real tree:\n{text}");
    for f in arch_neutral_files() {
        let path = format!(".github/workflows/{f}");
        assert!(
            WORKFLOWS.contains(&path.as_str()),
            "{path} has an ARCH_NEUTRAL_JOBS entry but is not in WORKFLOWS"
        );
        assert!(
            code.contains(&path),
            "{path} has an ARCH_NEUTRAL_JOBS entry but is not in cache-key-arch-guard.sh code"
        );
        assert!(
            text.lines().any(|l| l.starts_with(&format!("ok {path}:"))),
            "guard did not report {path}:\n{text}"
        );
    }
}

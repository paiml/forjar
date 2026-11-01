//! A kind: code RECEIPT MAY FALSIFY WITH A LIB UNIT TEST (forjar#846).
//!
//! `scripts/quorum-gate.sh` ran only `cargo test --test <target>`, so a fix whose
//! natural test lives in a `src/**/tests*.rs` module (a private function, no
//! integration surface) could not be falsified honestly. The gate now runs
//! `cargo test --lib <filter>` when `test_file` is such a module and the branch
//! touched it, and refuses a filter that matches no test (0 passed proves
//! nothing). Same fixtures conventions as the sibling gate tests: the helpers are
//! duplicated rather than lifted into `tests/common/`, for the reason they state.

//!
//! Every fixture pins `QUORUM_ACTOR` and the repo-local `user.email`: the
//! fixture's `enforce.json` lists only the fixture actor, so a run inheriting
//! the machine's email would fall to `advisory`, where `die` exits 0.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ACTOR: &str = "tester@example.com";
const BRANCH: &str = "PMAT-999-triage-fixture";
const DIGEST_PATH: &str = ".quorum/evidence/triage-claims.md";

fn gate() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/quorum-gate.sh")
}

fn git(repo: &Path, args: &[&str]) -> Output {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "tester")
        .env("GIT_AUTHOR_EMAIL", ACTOR)
        .env("GIT_COMMITTER_NAME", "tester")
        .env("GIT_COMMITTER_EMAIL", ACTOR)
        .output()
        .expect("git must run");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn write(repo: &Path, rel: &str, body: &str) {
    let p = repo.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

struct Run {
    code: i32,
    text: String,
}

fn scrub(cmd: &mut Command) -> &mut Command {
    // A GIT_DIR inherited from a pre-push hook would point every `git` at the
    // REAL repository, where the fixture's claims resolve for reasons it never
    // established.
    cmd.env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("QUORUM_SKIP")
}

/// Run the real gate script with the fixture as its working directory.
fn run_gate(repo: &Path, print_hash: bool, args: &[&str]) -> Run {
    let mut cmd = Command::new("bash");
    cmd.arg(gate())
        .args(args)
        .current_dir(repo)
        // A fleet-wide CARGO_TARGET_DIR is shared by every fixture crate named
        // "fixture" and cross-contaminates their builds: one target dir each.
        .env("CARGO_TARGET_DIR", repo.join("target"))
        .env("QUORUM_ACTOR", ACTOR)
        .env("PRINT_HASH", if print_hash { "1" } else { "0" });
    let out = scrub(&mut cmd).output().expect("the gate script must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        code: out.status.code().unwrap_or(-1),
        text,
    }
}

/// The subline lengths `quorum_evidence.py::truncated` reads as a writer's
/// budget rather than a sentence.
const TRUNC_BUDGETS: [usize; 12] = [
    200, 300, 400, 500, 600, 800, 1000, 1200, 1400, 1500, 2000, 4000,
];

fn not_round(mut s: String) -> String {
    while TRUNC_BUDGETS.contains(&s.len()) {
        s.push('.');
    }
    s
}

/// One adjudicated claim: a headline over the 180 B item floor and exactly one
/// `- evidence:` subline carrying `cite` verbatim.
fn claim(n: usize, headline: &str, cite: &str) -> String {
    let subline = not_round(format!(
        "{cite} is the row the panel read at the commit under review rather than \
         trusting the receipt's prose; a triage verdict is a classification, and the \
         classification is what the citation points at."
    ));
    format!(
        "{n}. [{headline}] A triage branch classifies and links: the ledger names each \
         issue's verdict and the roadmap row it was filed under, and the panel adjudicated \
         this claim against the pushed tree, which is the only tree a reviewer will see.\n\
         - evidence: {subline}\n\n"
    )
}

fn digest(confirmed: &[(&str, &str)], refuted: &[(&str, &str)]) -> String {
    let mut s = String::from("# Claims digest (fixture)\n\n## CONFIRMED\n\n");
    for (i, (h, c)) in confirmed.iter().enumerate() {
        s.push_str(&claim(i + 1, h, c));
    }
    s.push_str("## REFUTED\n\n");
    for (i, (h, c)) in refuted.iter().enumerate() {
        s.push_str(&claim(i + 1, h, c));
    }
    s
}

const TESTS_REL: &str = "src/tests_one.rs";
const BASE_LIB: &str = "pub fn one() -> u32 {\n    1\n}\n\n#[cfg(test)]\nmod tests_one;\n";
const FIXED_LIB: &str = "pub fn one() -> u32 {\n    2\n}\n\n#[cfg(test)]\nmod tests_one;\n";
const BASE_TESTS: &str = "use super::*;\n\n#[test]\nfn always_green() {\n    assert!(true);\n}\n";
const NEW_TESTS: &str = "use super::*;\n\n#[test]\nfn always_green() {\n    assert!(true);\n}\n\n#[test]\nfn one_is_two() {\n    assert_eq!(one(), 2);\n}\n\n#[test]\n#[allow(clippy::eq_op)]\nfn attr_between() {\n    assert_eq!(1, 1);\n}\n\nfn helper_not_a_test() {}\n";

struct Code {
    _dir: tempfile::TempDir,
    repo: PathBuf,
}

/// A real one-crate repo. The branch adds a lib unit test and, when `fixed`, the
/// src change it covers; `fixed == false` is the "src change reverted" tree.
fn code_fixture(fixed: bool, filter: &str, test_file: &str) -> Code {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().to_path_buf();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["config", "user.email", ACTOR]);
    git(&repo, &["config", "user.name", "tester"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);
    write(&repo, "README.md", "fixture\n");
    write(&repo, ".gitignore", "target/\n");
    write(
        &repo,
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    );
    write(
        &repo,
        ".quorum/enforce.json",
        &format!("{{\"enforced_for\": [\"{ACTOR}\"]}}\n"),
    );
    write(&repo, "src/lib.rs", BASE_LIB);
    write(&repo, TESTS_REL, BASE_TESTS);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "base"]);
    git(&repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    let base = stdout_of(&git(&repo, &["rev-parse", "HEAD"]));

    git(&repo, &["checkout", "-q", "-b", BRANCH]);
    if fixed {
        write(&repo, "src/lib.rs", FIXED_LIB);
    } else {
        // Keep src/lib.rs in the diff so the claims still anchor; only the
        // behaviour the test covers is left unfixed.
        write(&repo, "src/lib.rs", &format!("{BASE_LIB}// touched\n"));
    }
    write(&repo, TESTS_REL, NEW_TESTS);
    let d = digest(
        &[
            ("a", "src/lib.rs:1"),
            ("b", "src/tests_one.rs:4"),
            ("c", "src/lib.rs:2"),
        ],
        &[("d", "src/tests_one.rs:5")],
    );
    write(&repo, DIGEST_PATH, &d);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "fix: one() returns 2"]);

    let raw = std::fs::read(repo.join(DIGEST_PATH)).unwrap();
    let blob = stdout_of(&git(&repo, &["rev-parse", &format!("HEAD:{DIGEST_PATH}")]));
    let sha: String = Sha256::digest(&raw)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let hash = run_gate(
        &repo,
        true,
        &["--remote-ref", &format!("refs/heads/{BRANCH}")],
    );
    assert_eq!(hash.code, 0, "PRINT_HASH must succeed:\n{}", hash.text);
    let r = serde_json::json!({
        "issue": "PMAT-999 (fixture) -- one() returns 2",
        "base_commit": base,
        "diff_sha256": hash.text.trim(),
        "evidence": {
            "claims_digest": DIGEST_PATH,
            "total_bytes": raw.len(),
            "files": [{"path": DIGEST_PATH, "roles": ["claims","lanes","judges","agy"],
                       "blob": blob, "sha256": sha, "bytes": raw.len()}],
        },
        "quorum": {
            "lanes": ["review lane 1", "review lane 2", "review lane 3"],
            "refuters_per_claim": 3, "judges": 3,
            "claims_confirmed": 3, "claims_refuted": 1,
            "refuted_claims": ["one() was claimed to return 3 -- corrected: it returns 2"],
        },
        "crux": {"systems": ["a", "b", "c"], "verdict": "accept"},
        "agy_teamwork": {"ran": true, "verdict": "pass"},
        "pmat": {"tools": ["analyze_vacuous_tests"], "vacuous_tests_in_touched_paths": 0, "accepted": []},
        "falsification": {
            "test": "tests_one::one_is_two",
            "test_file": test_file,
            "cargo_test_target": filter,
            "reverted": "src/lib.rs one() reverted to return 1",
            "observed_failure": "assertion `left == right` failed: left 1, right 2",
        },
    });
    write(
        &repo,
        &format!(".quorum/{BRANCH}.json"),
        &serde_json::to_string_pretty(&r).unwrap(),
    );
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "quorum receipt"]);
    Code { _dir: dir, repo }
}

fn gate_on(c: &Code) -> Run {
    let tip = stdout_of(&git(&c.repo, &["rev-parse", "HEAD"]));
    run_gate(
        &c.repo,
        false,
        &[
            "--remote-ref",
            &format!("refs/heads/{BRANCH}"),
            "--local-sha",
            &tip,
        ],
    )
}

/// #784's shape: a private-fn fix with its unit test in a touched src/**/tests*.rs.
#[test]
fn a_lib_unit_test_falsifier_passes_the_whole_gate() {
    let c = code_fixture(true, "tests_one::one_is_two", TESTS_REL);
    let r = gate_on(&c);
    assert_eq!(r.code, 0, "{}", r.text);
    assert!(r.text.contains("cargo test --lib"), "{}", r.text);
}

/// The falsifier: with the src change reverted the same receipt must fail.
#[test]
fn the_same_receipt_fails_with_the_src_change_reverted() {
    let c = code_fixture(false, "tests_one::one_is_two", TESTS_REL);
    let r = gate_on(&c);
    assert_ne!(r.code, 0, "{}", r.text);
    assert!(r.text.contains("does NOT pass"), "{}", r.text);
}

/// A filter that matches nothing runs 0 tests and would pass cargo: refused.
#[test]
fn a_lib_filter_that_matches_no_test_is_refused() {
    let c = code_fixture(true, "tests_one::no_such_test", TESTS_REL);
    let r = gate_on(&c);
    assert_ne!(r.code, 0, "{}", r.text);
    assert!(r.text.contains("ran no test"), "{}", r.text);
}

/// r1 fix (2): a free-rider points the lib filter at a test that is not in the
/// branch's touched test_file; it must be refused before cargo runs.
#[test]
fn a_lib_filter_naming_a_test_outside_test_file_is_refused() {
    let c = code_fixture(true, "other_module::unrelated_green", TESTS_REL);
    let r = gate_on(&c);
    assert_ne!(r.code, 0, "{}", r.text);
    assert!(r.text.contains("does not name a test in"), "{}", r.text);
}

/// r1 fix (3): the success mark is not printed when the falsification test
/// fails (it used to print before the test ran).
#[test]
fn the_success_mark_is_absent_when_the_test_fails() {
    let c = code_fixture(false, "tests_one::one_is_two", TESTS_REL);
    let r = gate_on(&c);
    assert_ne!(r.code, 0, "{}", r.text);
    assert!(!r.text.contains("✓ quorum receipt valid"), "{}", r.text);
}

fn gate_without_jq(c: &Code, shim_jq: Option<&str>) -> Run {
    let tip = stdout_of(&git(&c.repo, &["rev-parse", "HEAD"]));
    let bin = c.repo.join(".git/nojq-bin");
    std::fs::create_dir_all(&bin).unwrap();
    for e in std::fs::read_dir("/usr/bin").unwrap().flatten() {
        let n = e.file_name();
        if n != "jq" {
            let _ = std::os::unix::fs::symlink(e.path(), bin.join(&n));
        }
    }
    if let Some(body) = shim_jq {
        std::fs::write(bin.join("jq"), body).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(bin.join("jq"), std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let mut cmd = Command::new("/usr/bin/bash");
    cmd.arg(gate())
        .args([
            "--remote-ref",
            &format!("refs/heads/{BRANCH}"),
            "--local-sha",
            &tip,
        ])
        .current_dir(&c.repo)
        .env("PATH", &bin)
        .env("QUORUM_ACTOR", ACTOR)
        .env("PRINT_HASH", "0");
    let out = scrub(&mut cmd).output().unwrap();
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        code: out.status.code().unwrap_or(-1),
        text,
    }
}

/// r1 fix (1): no jq, or a jq that cannot read the kind, is a refusal -- never a
/// silent default to kind=code.
#[test]
fn a_missing_or_broken_jq_is_refused_not_defaulted() {
    let c = code_fixture(true, "tests_one::one_is_two", TESTS_REL);
    let r = gate_without_jq(&c, None);
    assert_ne!(r.code, 0, "{}", r.text);
    assert!(r.text.contains("jq is required"), "{}", r.text);
    let r = gate_without_jq(&c, Some("#!/bin/sh\nexit 3\n"));
    assert_ne!(r.code, 0, "{}", r.text);
    assert!(r.text.contains("cannot read .kind"), "{}", r.text);
}

fn tie_refused(filter: &str) -> bool {
    let c = code_fixture(true, filter, TESTS_REL);
    let r = gate_on(&c);
    assert_ne!(r.code, 0, "{}", r.text);
    r.text.contains("does not name a test in")
}

/// r2 fix (1): only a fn directly under #[test] (other attributes allowed in
/// between) ties a filter to test_file; a same-named helper and a regex
/// metachar name do not.
#[test]
fn the_fn_branch_requires_a_real_test_and_matches_literally() {
    // Passes the tie (then cargo finds nothing under that module: a later refusal).
    assert!(!tie_refused("other_module::one_is_two"));
    assert!(!tie_refused("other_module::attr_between"));
    // A helper without #[test] is not a test.
    assert!(tie_refused("other_module::helper_not_a_test"));
    // Regex metacharacters are literal: `.*` / `one_is_.wo` match nothing.
    assert!(tie_refused("other_module::.*"));
    assert!(tie_refused("other_module::one_is_.wo"));
}

/// r2 fix (3): the lanes/refuters line main printed is back, exactly.
#[test]
fn the_lanes_line_is_printed_as_main_prints_it() {
    let c = code_fixture(true, "tests_one::one_is_two", TESTS_REL);
    let r = gate_on(&c);
    assert_eq!(r.code, 0, "{}", r.text);
    assert!(
        r.text
            .contains("    lanes=3 refuters=3 judges=3 confirmed=3 refuted=1\n"),
        "{}",
        r.text
    );
}

/// r2 fix (4): quorum.yml runs on the clean-room fleet (never hosted) and makes
/// jq present before the gate step reads the receipt with it.
#[test]
fn the_quorum_workflow_gets_jq_before_the_gate_on_a_self_hosted_runner() {
    let y = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/quorum.yml"),
    )
    .unwrap();
    assert!(y.contains("runs-on: [self-hosted, clean-room, X64]"));
    assert!(
        !y.contains("ubuntu-"),
        "a GitHub-hosted runner is forbidden"
    );
    let jq = y.find("name: Ensure jq").expect("an `Ensure jq` step");
    let gate = y.find("name: Verify quorum receipt").unwrap();
    assert!(jq < gate, "jq must be ensured before the gate runs");
    assert!(
        y[jq..gate].contains("command -v jq"),
        "the step must show presence"
    );
    assert!(
        y[jq..gate].contains("apt-get install"),
        "the step must install when absent"
    );
}

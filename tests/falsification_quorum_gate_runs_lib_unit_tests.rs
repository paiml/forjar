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
const NEW_TESTS: &str = "use super::*;\n\n#[test]\nfn always_green() {\n    assert!(true);\n}\n\n#[test]\nfn one_is_two() {\n    assert_eq!(one(), 2);\n}\n";

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
    let c = code_fixture(true, "no_such_test_anywhere", TESTS_REL);
    let r = gate_on(&c);
    assert_ne!(r.code, 0, "{}", r.text);
    assert!(r.text.contains("ran no test"), "{}", r.text);
}

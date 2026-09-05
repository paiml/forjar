//! A RELEASE-SHAPED DIFF MUST BE ABLE TO ANCHOR ITS OWN CLAIMS.
//!
//! PMAT-159. This is the SECOND subject of `scripts/quorum-gate.sh`'s evidence
//! pass. The first — the gate must judge the COMMIT BEING PUSHED, not the
//! working tree (forjar#400) — lives in the sibling file
//! `tests/falsification_quorum_gate_reads_the_pushed_ref.rs`, and these cases
//! were written there. They are here because the fixture block below is ~200
//! lines and that file sits against CB-040's 500-line ceiling: the two subjects
//! cannot share one file. The four small helpers (`git`, `write`, `stdout_of`,
//! `Run`) are duplicated from the sibling rather than lifted into
//! `tests/common/`, which is where this repository shares a harness between two
//! test binaries (`common/budget_harness.rs` names the same file-health limit as
//! its reason). If a third file ever needs them, that is where they belong.
//!
//! `scripts/quorum_evidence.py::check_anchors` requires that at least
//! `ANCHOR_MIN` (33%) of adjudicated claims cite a `file:line` that resolves at
//! the merge-base AND names a file inside this branch's diff. Two rules made
//! that floor UNREACHABLE for a release commit, which touches `Cargo.toml`,
//! `Cargo.lock`, `CHANGELOG.md` and (here) one new falsification test:
//!
//!   * `CIT_RE` matched only `(src|tests|scripts|benches)/….rs`, so a citation
//!     into any of the three root files was not a citation at all; and
//!   * a file ADDED by the branch has no blob at the merge-base, and the loop
//!     `continue`d past it.
//!
//! Measured on this repository's own v1.25.2 release commit: **0 of 19**
//! adjudicated claims anchored. v1.25.0 (`0b4f2e3e`) and v1.25.1 (`813159f2`)
//! were therefore pushed with `waived` receipts — the gate was not refusing bad
//! evidence, it was refusing a SHAPE, and the waiver is what a repo learns to
//! reach for when a gate cannot be passed honestly.
//!
//! The fix keeps the 33% floor, keeps the base-resolution rule for every file
//! that exists at the merge-base, and keeps refusing prose. For an ADDED file
//! the anchor's guarantee changes and is worth stating exactly: it can no
//! longer be "a tree the pusher did not author", because the pusher wrote every
//! line of it. What remains mechanically checkable is that the cited line EXISTS
//! in the pushed commit — the line is in the diff by construction — so a
//! citation past the end of a new file is refused by name rather than silently
//! scored as unanchored. `the_gate_refuses_a_citation_past_the_end_of_a_new_file`
//! pins that stricter reading, which is the same one the base branch has always
//! applied to pre-existing files.
//!
//! HOW THESE WERE SHOWN TO DISCRIMINATE, one half of the fix at a time:
//!
//!   * both halves reverted → `a_release_shaped_diff_can_anchor_its_claims`
//!     RED at `only 0/4 (0%)` — the shape this repository measured on its own
//!     release commit;
//!   * only the `CIT_RE` widening reverted → RED at `only 1/4 (25%)`. The added
//!     test anchors on its own, and one claim in four is still under the floor,
//!     so the root-file half is load-bearing rather than decorative;
//!   * only the added-file half reverted →
//!     `the_gate_refuses_a_citation_past_the_end_of_a_new_file` RED. The gate
//!     still refuses, but at `only 0/4 (0%)` — it never names line 9999, so the
//!     fabricated citation is invisible in the message a human reads.
//!
//! These cases drive `scripts/quorum_evidence.py` DIRECTLY rather than through
//! `scripts/quorum-gate.sh` as the sibling file's fixtures do. That is
//! deliberate: the gate reaches the evidence pass only after a full receipt
//! clears eight unrelated lane floors and a `cargo test --test <target>` run
//! inside the fixture repo, so a failure there would not name the anchor rule.
//! The invocation is byte-for-byte the one the gate makes (`quorum_evidence.py
//! <receipt> <confirmed> <refuted> <touched> <merge-base> <head>`).
//!
//! `QUORUM_ACTOR` and the repo-local `user.email` are pinned by the fixture for
//! the reason the sibling file states: a run that inherited the machine's global
//! email would fall to `advisory`, where `die` exits 0.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ACTOR: &str = "tester@example.com";

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

/// The subline lengths `quorum_evidence.py::truncated` reads as a writer's
/// budget rather than a sentence. A fixture that lands on one by accident would
/// be refused for a reason that has nothing to do with anchoring.
const TRUNC_BUDGETS: [usize; 12] = [
    200, 300, 400, 500, 600, 800, 1000, 1200, 1400, 1500, 2000, 4000,
];

fn not_round(mut s: String) -> String {
    while TRUNC_BUDGETS.contains(&s.len()) {
        s.push('.');
    }
    s
}

const DIGEST_PATH: &str = ".quorum/evidence/release-claims.md";
const NEW_TEST: &str = "tests/falsification_version_matches_manifest.rs";
const OLD_TEST: &str = "tests/falsification_pre_existing.rs";

const BASE_CARGO_TOML: &str =
    "[package]\nname = \"fixture\"\nversion = \"1.25.1\"\nedition = \"2021\"\n";

const BASE_CARGO_LOCK: &str = "# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n[[package]]\nname = \"fixture\"\nversion = \"1.25.1\"\n";

const BASE_CHANGELOG: &str =
    "# Changelog\n\n## [1.25.1] \u{2014} 2026-09-04\n\n- the previous release\n";

const BASE_TEST_FILE: &str =
    "//! A falsifier that predates this branch.\n\n#[test]\nfn it_holds() {\n    assert_eq!(1 + 1, 2);\n}\n";

const ADDED_TEST_FILE: &str = "//! The falsifier this release adds: every artefact that states the\n//! version must state the same one.\n\n#[test]\nfn the_lockfile_records_the_manifest_version() {\n    assert_eq!(\"1.25.2\", \"1.25.2\");\n}\n";

/// One adjudicated claim: a headline long enough to clear the 180 B item floor,
/// and exactly one `- evidence:` subline carrying `cite` verbatim.
fn claim(n: usize, headline: &str, cite: &str) -> String {
    let subline = not_round(format!(
        "{cite} states the version this release ships. The panel opened it at the \
         commit under review rather than trusting the receipt's own prose, which is \
         the whole reason a citation is worth more than a tally."
    ));
    format!(
        "{n}. [{headline}] A release-shaped diff moves the manifest, the lockfile, \
         the changelog and the falsifier that pins them together; that is the entire \
         shape of a version bump, and the panel adjudicated this claim against the \
         pushed tree rather than the working copy.\n- evidence: {subline}\n\n"
    )
}

/// A claims digest with `CONFIRMED` and `REFUTED` sections in the shape
/// `sections()`/`items()` parse.
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

struct Release {
    _dir: tempfile::TempDir,
    repo: PathBuf,
    base: String,
    head: String,
    touched: String,
    receipt: PathBuf,
}

/// A merge-base carrying the four files a release edits, then a release commit
/// that bumps three of them and ADDS a falsification test — plus the committed
/// claims digest the receipt manifests.
fn release_fixture(digest_text: &str) -> Release {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().to_path_buf();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["config", "user.email", ACTOR]);
    git(&repo, &["config", "user.name", "tester"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);

    write(&repo, "Cargo.toml", BASE_CARGO_TOML);
    write(&repo, "Cargo.lock", BASE_CARGO_LOCK);
    write(&repo, "CHANGELOG.md", BASE_CHANGELOG);
    write(&repo, OLD_TEST, BASE_TEST_FILE);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "base"]);
    git(&repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    let base = stdout_of(&git(&repo, &["rev-parse", "HEAD"]));

    git(&repo, &["checkout", "-q", "-b", "release/v1.25.2"]);
    write(
        &repo,
        "Cargo.toml",
        &BASE_CARGO_TOML.replace("1.25.1", "1.25.2"),
    );
    write(
        &repo,
        "Cargo.lock",
        &BASE_CARGO_LOCK.replace("1.25.1", "1.25.2"),
    );
    write(
        &repo,
        "CHANGELOG.md",
        &BASE_CHANGELOG.replace(
            "# Changelog\n",
            "# Changelog\n\n## [1.25.2] \u{2014} 2026-09-05\n\n- this release\n",
        ),
    );
    write(&repo, NEW_TEST, ADDED_TEST_FILE);
    write(&repo, DIGEST_PATH, digest_text);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "release: v1.25.2"]);
    let head = stdout_of(&git(&repo, &["rev-parse", "HEAD"]));
    let touched = stdout_of(&git(&repo, &["diff", "--name-only", &base, &head]));
    assert!(
        touched.contains(NEW_TEST) && touched.contains("Cargo.toml"),
        "fixture is wrong: the release diff does not carry the release files: {touched:?}"
    );

    // The receipt is handed to the script as a PATH, exactly as the gate hands
    // it the blob it extracted. Written after the commits, so it is untracked
    // and cannot itself appear in the diff.
    let raw = std::fs::read(repo.join(DIGEST_PATH)).expect("read the digest back");
    let blob = stdout_of(&git(&repo, &["rev-parse", &format!("HEAD:{DIGEST_PATH}")]));
    let sha: String = Sha256::digest(&raw)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let receipt = repo.join("receipt.json");
    std::fs::write(
        &receipt,
        serde_json::json!({
            "base_commit": base,
            "evidence": {
                "claims_digest": DIGEST_PATH,
                "total_bytes": raw.len(),
                "files": [{
                    "path": DIGEST_PATH,
                    "roles": ["claims", "lanes", "judges", "agy"],
                    "blob": blob,
                    "sha256": sha,
                    "bytes": raw.len(),
                }],
            },
        })
        .to_string(),
    )
    .expect("write the receipt");

    Release {
        _dir: dir,
        repo,
        base,
        head,
        touched,
        receipt,
    }
}

/// The evidence pass, invoked the way `scripts/quorum-gate.sh` invokes it.
fn run_evidence(f: &Release, confirmed: usize, refuted: usize) -> Run {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/quorum_evidence.py");
    let out = Command::new("python3")
        .arg(&script)
        .arg(&f.receipt)
        .arg(confirmed.to_string())
        .arg(refuted.to_string())
        .arg(&f.touched)
        .arg(&f.base)
        .arg(&f.head)
        .current_dir(&f.repo)
        // A GIT_DIR inherited from a pre-push hook would point every `git` the
        // script runs at the REAL repository, where the citations resolve for
        // reasons the fixture never established.
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("the evidence script must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        code: out.status.code().unwrap_or(-1),
        text,
    }
}

/// The release shape anchors: three root files and one added test.
///
/// Every line cited into a PRE-EXISTING file (`Cargo.toml:3`, `Cargo.lock:7`,
/// `CHANGELOG.md:3`) resolves at the merge-base, so the old rule is untouched;
/// only `tests/falsification_version_matches_manifest.rs:5` needs the added-file
/// rule.
#[test]
fn a_release_shaped_diff_can_anchor_its_claims() {
    let f = release_fixture(&digest(
        &[
            ("manifest", "Cargo.toml:3"),
            ("lockfile", "Cargo.lock:7"),
            ("changelog", "CHANGELOG.md:3"),
        ],
        &[(
            "falsifier",
            "tests/falsification_version_matches_manifest.rs:5",
        )],
    ));
    let r = run_evidence(&f, 3, 1);
    assert_eq!(
        r.code, 0,
        "a release commit cited its manifest, its lockfile, its changelog and the \
         falsifier it adds -- the only four files it touches -- and the evidence gate \
         still refused it. Nothing a release can honestly cite would anchor, so the \
         only way past this gate is `waived`, which is how v1.25.0 and v1.25.1 were \
         pushed:\n{}",
        r.text
    );
    assert!(
        r.text.contains("redaction clean"),
        "the script exited 0 without reaching its own summary line:\n{}",
        r.text
    );
}

/// Over-correction control: prose with no citation at all is still refused.
#[test]
fn a_claim_set_with_no_citations_is_still_refused() {
    let f = release_fixture(&digest(
        &[
            ("manifest", "the version was bumped"),
            ("lockfile", "the lock agrees with it"),
            ("changelog", "the changelog has a dated heading"),
        ],
        &[("falsifier", "a test was added for it")],
    ));
    let r = run_evidence(&f, 3, 1);
    assert_ne!(
        r.code, 0,
        "four claims citing nothing at all passed the anchor floor. Widening the \
         citation shapes must not turn the floor off:\n{}",
        r.text
    );
    assert!(
        r.text.contains("0/4"),
        "the refusal does not say how many claims anchored:\n{}",
        r.text
    );
}

/// Over-correction control: an `.rs` citation outside the diff still anchors
/// nothing.
///
/// `tests/falsification_pre_existing.rs` resolves at the merge-base — which is
/// exactly the free-rider the file-in-the-diff half of the rule exists to stop.
#[test]
fn an_rs_citation_outside_the_diff_is_still_refused() {
    let cite = "tests/falsification_pre_existing.rs:3";
    let f = release_fixture(&digest(
        &[("manifest", cite), ("lockfile", cite), ("changelog", cite)],
        &[("falsifier", cite)],
    ));
    let r = run_evidence(&f, 3, 1);
    assert_ne!(
        r.code, 0,
        "every claim cited a pre-existing file this branch never touched and the \
         gate accepted it. Prose about code the branch did not change is not \
         evidence FOR this change:\n{}",
        r.text
    );
    assert!(
        r.text.contains("0/4"),
        "the refusal does not say how many claims anchored:\n{}",
        r.text
    );
}

/// A citation past the end of an ADDED file is refused BY NAME.
///
/// The base rule already dies on an out-of-range line for a file that exists at
/// the merge-base; an added file gets the same treatment, read at the pushed
/// commit. Scoring it as merely unanchored would let a fabricated line number
/// hide inside the 67% of claims the floor does not require.
#[test]
fn the_gate_refuses_a_citation_past_the_end_of_a_new_file() {
    let cite = "tests/falsification_version_matches_manifest.rs:9999";
    let f = release_fixture(&digest(
        &[("manifest", cite), ("lockfile", cite), ("changelog", cite)],
        &[("falsifier", cite)],
    ));
    let r = run_evidence(&f, 3, 1);
    assert_ne!(
        r.code, 0,
        "a citation to line 9999 of a seven-line file the branch just added was \
         accepted:\n{}",
        r.text
    );
    assert!(
        r.text.contains("9999"),
        "the gate refused, but never named the citation that cannot exist -- a \
         refusal that does not name its cause is the one people learn to waive:\n{}",
        r.text
    );
}

//! THE COMMITTED-QUORUM GATE MUST HAVE A SHAPE A kind:triage BRANCH CAN SATISFY.
//!
//! forjar#491, PMAT-224. `scripts/quorum-gate.sh` requires a falsification test
//! the branch wrote or changed, and `scripts/quorum_evidence.py::check_anchors`
//! requires ≥33% of adjudicated claims to cite a `.rs` file (or a root manifest)
//! the branch touches. A `kind:triage` branch — classify and link, no diff —
//! touches only `docs/audits/**`, `docs/roadmaps/roadmap.yaml` and, since
//! PMAT-226, the release ledger `docs/roadmaps/releases.yaml`. It has no
//! Rust file to name and no citable path, so it can anchor 0% by construction
//! and can never satisfy the falsification block. Every triage PR was therefore
//! pushed `waived`, which the `CIT_RE` comment in `quorum_evidence.py` already
//! names as the failure mode a gate teaches when it cannot be passed honestly.
//!
//! The shape this pins: a receipt may declare `kind: triage`. Then a citation
//! into a DOCUMENTATION file the branch touches anchors a claim under the same
//! at-base / as-added / must-be-touched rules a `.rs` file has always had; the
//! diff must lie within `docs/audits/**`, `docs/roadmaps/roadmap.yaml`,
//! `docs/roadmaps/releases.yaml` and `.quorum/**` (a triage receipt over a
//! code diff is refused BY NAME, because
//! it would be the cheapest way to skip the falsification); and the
//! falsification block declares `not_applicable` with a reason, which the gate
//! PRINTS as what it did not verify. Every other floor — lanes, judges,
//! refuters, a refuted claim as text, crux, agy, pmat, provenance, redaction —
//! applies unchanged. A receipt without `kind` is a code receipt and nothing
//! about it changes: documentation cites anchor nothing for it (so a code branch
//! cannot anchor its claims on the receipt it wrote itself), and a code diff
//! with no falsification is still refused.
//!
//! The four small helpers are duplicated from the sibling gate tests rather
//! than lifted into `tests/common/`, for the reason those files state.
//!
//! Every fixture pins `QUORUM_ACTOR` and the repo-local `user.email`: the
//! fixture's `enforce.json` lists only the fixture actor, so a run inheriting
//! the machine's email would fall to `advisory`, where `die` exits 0.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ACTOR: &str = "tester@example.com";
const BRANCH: &str = "PMAT-999-triage-fixture";
const LEDGER: &str = "docs/audits/triage-ledger-999.md";
const ROADMAP: &str = "docs/roadmaps/roadmap.yaml";
const RELEASES: &str = "docs/roadmaps/releases.yaml";
/// The one edit a triage receipt must never cover.
const CODE_EDIT: (&str, &str) = ("src/lib.rs", "pub fn one() -> u32 {\n    2\n}\n");
const OTHER_DOC: &str = "docs/book/src/other.md";
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

const BASE_ROADMAP: &str = "roadmap:\n- id: PMAT-001\n  status: done\n";
const BASE_OTHER_DOC: &str = "# Another page\n\nnot touched by the branch\n";
const LEDGER_BODY: &str = "# Triage ledger — PMAT-999\n\n| issue | verdict | linked |\n|---|---|---|\n| #1 | duplicate of #2 | PMAT-001 |\n| #3 | deferred | PMAT-999 |\n";

struct Triage {
    _dir: tempfile::TempDir,
    repo: PathBuf,
    base: String,
    head: String,
    touched: String,
    receipt: PathBuf,
}

/// A merge-base with a roadmap, a doc page the branch never touches and one
/// Rust file; then a triage branch that ADDS a ledger, edits the roadmap and
/// commits its claims digest. `also` makes the branch write one more file —
/// `CODE_EDIT` is the shape a triage receipt must never cover, the release
/// ledger the shape it must (PMAT-226).
fn triage_fixture(digest_text: &str, also: Option<(&str, &str)>) -> Triage {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().to_path_buf();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["config", "user.email", ACTOR]);
    git(&repo, &["config", "user.name", "tester"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);
    write(&repo, "README.md", "fixture\n");
    write(
        &repo,
        ".quorum/enforce.json",
        &format!("{{\"enforced_for\": [\"{ACTOR}\"]}}\n"),
    );
    write(&repo, ROADMAP, BASE_ROADMAP);
    write(&repo, OTHER_DOC, BASE_OTHER_DOC);
    write(&repo, "src/lib.rs", "pub fn one() -> u32 {\n    1\n}\n");
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "base"]);
    git(&repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    let base = stdout_of(&git(&repo, &["rev-parse", "HEAD"]));

    git(&repo, &["checkout", "-q", "-b", BRANCH]);
    write(&repo, LEDGER, LEDGER_BODY);
    write(
        &repo,
        ROADMAP,
        &format!("{BASE_ROADMAP}- id: PMAT-999\n  kind: triage\n  status: done\n"),
    );
    if let Some((path, body)) = also {
        write(&repo, path, body);
    }
    write(&repo, DIGEST_PATH, digest_text);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "triage: PMAT-999"]);
    let head = stdout_of(&git(&repo, &["rev-parse", "HEAD"]));
    let touched = stdout_of(&git(&repo, &["diff", "--name-only", &base, &head]));
    assert!(
        touched.contains(LEDGER) && touched.contains(ROADMAP),
        "fixture is wrong: the triage diff does not carry the ledger and the roadmap: {touched:?}"
    );
    let raw = std::fs::read(repo.join(DIGEST_PATH)).expect("read the digest back");
    let blob = stdout_of(&git(&repo, &["rev-parse", &format!("HEAD:{DIGEST_PATH}")]));
    let sha: String = Sha256::digest(&raw)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    // Inside .git/ so `git add -A` for the later receipt commit never sweeps
    // this scratch file into the diff the gate hashes.
    let receipt = repo.join(".git/fixture-receipt.json");
    std::fs::write(
        &receipt,
        serde_json::json!({
            "kind": "triage",
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
    Triage {
        _dir: dir,
        repo,
        base,
        head,
        touched,
        receipt,
    }
}

/// The evidence pass, invoked the way `scripts/quorum-gate.sh` invokes it.
fn run_evidence(f: &Triage, confirmed: usize, refuted: usize) -> Run {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/quorum_evidence.py");
    let mut cmd = Command::new("python3");
    cmd.arg(&script)
        .arg(&f.receipt)
        .arg(confirmed.to_string())
        .arg(refuted.to_string())
        .arg(&f.touched)
        .arg(&f.base)
        .arg(&f.head)
        .current_dir(&f.repo);
    let out = scrub(&mut cmd)
        .output()
        .expect("the evidence script must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        code: out.status.code().unwrap_or(-1),
        text,
    }
}

/// Promote the evidence-only receipt to a full one, commit it under the branch's
/// slug, and hand back the fixture ready for the whole gate. `kind` is spliced in
/// verbatim so a test can drop it; `falsification` likewise.
fn commit_full_receipt(f: &Triage, kind: Option<&str>, falsification: serde_json::Value) {
    let hash = run_gate(
        &f.repo,
        true,
        &["--remote-ref", &format!("refs/heads/{BRANCH}")],
    );
    assert_eq!(hash.code, 0, "PRINT_HASH must succeed:\n{}", hash.text);
    let diff_sha = hash.text.trim().to_string();
    let mut r: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&f.receipt).unwrap()).unwrap();
    let obj = r.as_object_mut().unwrap();
    obj.remove("kind");
    if let Some(k) = kind {
        obj.insert("kind".into(), serde_json::json!(k));
    }
    obj.insert(
        "issue".into(),
        serde_json::json!("PMAT-999 (fixture) — a triage ledger"),
    );
    obj.insert("diff_sha256".into(), serde_json::json!(diff_sha));
    obj.insert(
        "quorum".into(),
        serde_json::json!({
            "lanes": ["review lane 1", "review lane 2", "review lane 3"],
            "refuters_per_claim": 3, "judges": 3,
            "claims_confirmed": 3, "claims_refuted": 1,
            "refuted_claims": ["issue #3 was a duplicate of #1 — corrected: it is deferred, its own defect"],
        }),
    );
    obj.insert(
        "crux".into(),
        serde_json::json!({"systems": ["Bugzilla", "Jira", "GitHub triage"], "verdict": "accept"}),
    );
    obj.insert(
        "agy_teamwork".into(),
        serde_json::json!({"ran": true, "verdict": "pass"}),
    );
    obj.insert(
        "pmat".into(),
        serde_json::json!({"tools": ["analyze_vacuous_tests"], "vacuous_tests_in_touched_paths": 0, "accepted": []}),
    );
    obj.insert("falsification".into(), falsification);
    write(
        &f.repo,
        &format!(".quorum/{BRANCH}.json"),
        &serde_json::to_string_pretty(&r).unwrap(),
    );
    git(&f.repo, &["add", "-A"]);
    git(&f.repo, &["commit", "-qm", "quorum receipt"]);
}

fn whole_gate(f: &Triage) -> Run {
    let tip = stdout_of(&git(&f.repo, &["rev-parse", "HEAD"]));
    run_gate(
        &f.repo,
        false,
        &[
            "--remote-ref",
            &format!("refs/heads/{BRANCH}"),
            "--local-sha",
            &tip,
        ],
    )
}

fn triage_digest() -> String {
    digest(
        &[
            ("ledger", &format!("{LEDGER}:5")),
            ("roadmap", &format!("{ROADMAP}:2")),
            ("ledger", &format!("{LEDGER}:6")),
        ],
        &[("verdict", &format!("{ROADMAP}:3"))],
    )
}

fn not_applicable() -> serde_json::Value {
    serde_json::json!({
        "not_applicable": "kind: triage — the branch writes no code; the ledger's read-backs are its evidence"
    })
}

/// THE ANCHOR HALF. A triage branch cites the ledger it adds and the roadmap
/// row it edits — the only two files it touches — and that must be enough.
#[test]
fn a_triage_shaped_diff_anchors_its_claims_on_the_documentation_it_touches() {
    let f = triage_fixture(&triage_digest(), None);
    let r = run_evidence(&f, 3, 1);
    assert_eq!(
        r.code, 0,
        "a triage branch cited its ledger and its roadmap row and the evidence gate \
         still refused it. Nothing a triage branch can honestly cite would anchor, so \
         the only way past this gate is `waived`:\n{}",
        r.text
    );
    assert!(r.text.contains("redaction clean"), "{}", r.text);
}

/// Over-correction control: a documentation citation OUTSIDE the diff still
/// anchors nothing, exactly as an `.rs` citation outside the diff never did.
#[test]
fn a_documentation_citation_outside_the_diff_anchors_nothing() {
    let cite = format!("{OTHER_DOC}:1");
    let f = triage_fixture(
        &digest(&[("a", &cite), ("b", &cite), ("c", &cite)], &[("d", &cite)]),
        None,
    );
    let r = run_evidence(&f, 3, 1);
    assert_ne!(r.code, 0, "{}", r.text);
    assert!(r.text.contains("0/4"), "{}", r.text);
}

/// Free-rider control: a CODE receipt cannot anchor on documentation, so a code
/// branch cannot cite the receipt or the ledger it wrote itself instead of code.
#[test]
fn a_code_receipt_cannot_anchor_its_claims_on_documentation() {
    let f = triage_fixture(&triage_digest(), None);
    let mut r: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&f.receipt).unwrap()).unwrap();
    r.as_object_mut().unwrap().remove("kind");
    std::fs::write(&f.receipt, r.to_string()).unwrap();
    let run = run_evidence(&f, 3, 1);
    assert_ne!(
        run.code, 0,
        "a receipt that declares no kind is a code receipt, and for a code receipt a \
         documentation citation must anchor nothing:\n{}",
        run.text
    );
}

/// THE GATE HALF. A triage receipt over a triage-shaped diff passes the whole
/// gate, and the gate says in its own output what it did not verify.
#[test]
fn a_triage_receipt_passes_the_whole_gate_and_says_what_it_did_not_verify() {
    let f = triage_fixture(&triage_digest(), None);
    commit_full_receipt(&f, Some("triage"), not_applicable());
    let r = whole_gate(&f);
    assert_eq!(
        r.code, 0,
        "a kind: triage receipt over a diff that touches only the ledger, the roadmap \
         and its own receipt was refused. The gate has no shape a triage branch can \
         satisfy, so every triage PR must be waived:\n{}",
        r.text
    );
    assert!(
        r.text.contains("NOT APPLICABLE") && r.text.contains("kind: triage"),
        "the gate passed a triage receipt without saying that it verified no \
         falsification — an unmeasured check must never print like a passed one:\n{}",
        r.text
    );
}

/// A triage receipt over a diff that touches code is refused BY NAME: declaring
/// `kind: triage` would otherwise be the cheapest way to skip the falsification.
#[test]
fn a_triage_receipt_over_a_code_diff_is_refused_by_name() {
    let f = triage_fixture(&triage_digest(), Some(CODE_EDIT));
    commit_full_receipt(&f, Some("triage"), not_applicable());
    let r = whole_gate(&f);
    assert_ne!(r.code, 0, "{}", r.text);
    assert!(
        r.text.contains("src/lib.rs"),
        "the refusal must name the file outside the triage rail, not fail for some \
         other reason:\n{}",
        r.text
    );
}

/// PMAT-226: the release ledger is on the rail. Booking a tag's row after the
/// cut is classify + link with no code in it, and it is the one PR every
/// release needs; before this the booking had no honest shape.
#[test]
fn a_triage_receipt_over_the_release_ledger_is_on_the_rail() {
    let f = triage_fixture(&triage_digest(), Some((RELEASES, "cadence_days: 2\n")));
    assert!(f.touched.contains(RELEASES), "{:?}", f.touched);
    commit_full_receipt(&f, Some("triage"), not_applicable());
    let r = whole_gate(&f);
    assert_eq!(
        r.code, 0,
        "the release ledger was refused as off the rail:\n{}",
        r.text
    );
}

/// The rail names the ledger by file, not the directory: any other path under
/// `docs/roadmaps/` is still refused by name.
#[test]
fn another_file_under_roadmaps_is_still_off_the_rail() {
    let f = triage_fixture(&triage_digest(), Some(("docs/roadmaps/notes.md", "x\n")));
    commit_full_receipt(&f, Some("triage"), not_applicable());
    let r = whole_gate(&f);
    assert_ne!(r.code, 0, "{}", r.text);
    assert!(r.text.contains("docs/roadmaps/notes.md"), "{}", r.text);
}

/// Nothing changes for a code receipt: no `kind`, no falsification, still refused.
#[test]
fn a_code_receipt_with_no_falsification_is_still_refused() {
    let f = triage_fixture(&triage_digest(), None);
    commit_full_receipt(&f, None, serde_json::json!({}));
    let r = whole_gate(&f);
    assert_ne!(r.code, 0, "{}", r.text);
    assert!(r.text.contains("falsification"), "{}", r.text);
}

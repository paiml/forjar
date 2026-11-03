//! C58-3 (Noah ruling C58 item 3, aligning with C48): the `quorum receipt` CI check
//! must turn green on ONE non-author PASS at the PR head.
//!
//! Today `quorum.yml` runs only `scripts/quorum-gate.sh`, which demands a committed
//! `.quorum/<branch>.json` with 3 lanes, 3 refuters, 3 judges, CRUX, agy and pmat lanes
//! and a diff hash. Nothing in the fleet's review flow writes that, so the check is red
//! on every PR. The fleet's review verdict is a head-bound receipt OUTSIDE the tree
//! (`<root>/<repo>/<pr>/<sha>.json`, the schema `arm-reviewed` reads); it cannot be
//! committed, because committing it would change the head it is bound to.
//!
//! `scripts/quorum-fleet-receipt.sh <repo> <pr> <sha>` is that check at the C48 LOW bar:
//! exit 0 only when the receipt is bound to exactly this repo, PR and head sha, has no
//! FAIL/BLOCKER verdict or non-empty blockers, and holds at least one PASS from a model
//! that is not `author_model`. Every other outcome, including no receipt, exits 1.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const REPO: &str = "paiml/forjar";
const PR: &str = "900";
const HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OLD: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/quorum-fleet-receipt.sh")
}

fn workflow() -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/quorum.yml"),
    )
    .expect("quorum.yml")
}

/// Write `<root>/<repo>/<pr>/<file_sha>.json` with `body`, run the check for `check_sha`.
fn run(file_sha: &str, check_sha: &str, body: &str) -> Output {
    let root = tempfile::tempdir().expect("tempdir");
    let dir = root.path().join(REPO).join(PR);
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join(format!("{file_sha}.json")), body).expect("write receipt");
    Command::new("bash")
        .arg(script())
        .args([REPO, PR, check_sha])
        .env("QUORUM_FLEET_ROOT", root.path())
        .output()
        .expect("run quorum-fleet-receipt.sh")
}

fn receipt(sha: &str, author: &str, verdicts: &str) -> String {
    format!(
        r#"{{"repo":"{REPO}","pr":{PR},"sha":"{sha}","author_model":"{author}","verdicts":[{verdicts}]}}"#
    )
}

fn v(model: &str, verdict: &str, blockers: &str) -> String {
    format!(r#"{{"lane":"l","model":"{model}","verdict":"{verdict}","blockers":[{blockers}]}}"#)
}

fn msg(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

#[test]
fn one_non_author_pass_at_head_is_green() {
    let r = receipt(HEAD, "claude-sonnet-5-5", &v("claude-opus-5-5", "PASS", ""));
    let o = run(HEAD, HEAD, &r);
    assert!(
        o.status.success(),
        "1 non-author PASS at head must be green: {}",
        msg(&o)
    );
}

#[test]
fn an_author_model_pass_alone_is_red() {
    let r = receipt(
        HEAD,
        "claude-sonnet-5-5",
        &v("claude-sonnet-5-5", "PASS", ""),
    );
    let o = run(HEAD, HEAD, &r);
    assert!(
        !o.status.success(),
        "the author's own PASS must not count: {}",
        msg(&o)
    );
}

#[test]
fn a_stale_head_pass_is_red() {
    // Receipt for an older head exists; the PR head moved. File for HEAD is absent.
    let r = receipt(OLD, "claude-sonnet-5-5", &v("claude-opus-5-5", "PASS", ""));
    let o = run(OLD, HEAD, &r);
    assert!(
        !o.status.success(),
        "a PASS for an old head must not clear the new head: {}",
        msg(&o)
    );
}

#[test]
fn a_receipt_filed_under_head_but_naming_another_sha_is_red() {
    let r = receipt(OLD, "claude-sonnet-5-5", &v("claude-opus-5-5", "PASS", ""));
    let o = run(HEAD, HEAD, &r);
    assert!(
        !o.status.success(),
        "body sha != head must be red: {}",
        msg(&o)
    );
}

#[test]
fn a_fail_verdict_is_red_even_beside_a_pass() {
    let vs = format!(
        "{},{}",
        v("claude-opus-5-5", "PASS", ""),
        v("gemini-3", "FAIL", "")
    );
    let o = run(HEAD, HEAD, &receipt(HEAD, "claude-sonnet-5-5", &vs));
    assert!(!o.status.success(), "FAIL must stay red: {}", msg(&o));
}

#[test]
fn a_blocker_verdict_or_nonempty_blockers_is_red() {
    let b = receipt(
        HEAD,
        "claude-sonnet-5-5",
        &v("claude-opus-5-5", "BLOCKER", ""),
    );
    assert!(
        !run(HEAD, HEAD, &b).status.success(),
        "BLOCKER verdict must be red"
    );
    let nb = receipt(
        HEAD,
        "claude-sonnet-5-5",
        &v("claude-opus-5-5", "PASS", r#""x""#),
    );
    assert!(
        !run(HEAD, HEAD, &nb).status.success(),
        "PASS with blockers must be red"
    );
}

#[test]
fn a_wrong_pr_or_repo_is_red() {
    let r = receipt(HEAD, "claude-sonnet-5-5", &v("claude-opus-5-5", "PASS", ""))
        .replace(&format!(r#""pr":{PR}"#), r#""pr":901"#);
    assert!(
        !run(HEAD, HEAD, &r).status.success(),
        "receipt for another PR must be red"
    );
}

#[test]
fn no_receipt_and_garbage_are_red() {
    let o = run(HEAD, HEAD, "not json");
    assert!(
        !o.status.success(),
        "unparseable receipt must be red: {}",
        msg(&o)
    );
    let o = run(OLD, HEAD, "{}");
    assert!(
        !o.status.success(),
        "no receipt at head must be red: {}",
        msg(&o)
    );
}

#[test]
fn the_workflow_runs_the_fleet_check_before_the_legacy_gate() {
    let w = workflow();
    let fleet = w
        .find("quorum-fleet-receipt.sh")
        .expect("quorum.yml must run quorum-fleet-receipt.sh");
    let legacy = w.find("quorum-gate.sh").expect("legacy gate stays");
    assert!(
        fleet < legacy,
        "fleet receipt check must come first so a C48 pass is green"
    );
    assert!(
        w.contains("github.event.pull_request.head.sha"),
        "must bind to the PR head sha"
    );
}

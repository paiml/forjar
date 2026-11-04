//! The CI quorum gate reads the receipt STORE, not a committed `.quorum/*.json`.
//!
//! A receipt binds to the PR head sha; committing the receipt moves the head,
//! so a committed receipt can never match the head it was written for (loop).
//! `scripts/quorum-store-gate.sh` therefore reads
//! `${QUORUM_STORE}/quorum/<repo>/<pr>/<headsha>.json` and fails closed on
//! every shortfall. UNMEASURED is failing.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SHA: &str = "141132a621af2843efe9d6689b2b469ef34f7ce4";

fn gate() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/quorum-store-gate.sh")
}

fn lane(name: &str, model: &str, verdict: &str) -> String {
    format!(r#"{{"lane":"{name}","model":"{model}","verdict":"{verdict}","blockers":[]}}"#)
}

fn receipt(sha: &str, pr: u32, author: &str, lanes: &[String], extra: &str) -> String {
    format!(
        r#"{{"repo":"paiml/forjar","pr":{pr},"sha":"{sha}","author_model":"{author}"{extra},"verdicts":[{}]}}"#,
        lanes.join(",")
    )
}

fn three_good() -> Vec<String> {
    vec![
        lane("a", "claude-opus-5-5", "PASS"),
        lane("b", "claude-haiku-5-5", "PASS"),
        lane("c", "gemini-3", "PASS"),
    ]
}

/// Write `body` (if any) as the receipt for (pr 798, `file_sha`), run the gate for head `head`.
fn run(body: Option<String>, file_sha: &str, head: &str) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    if let Some(b) = body {
        let d = store.join("quorum/paiml/forjar/798");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(format!("{file_sha}.json")), b).unwrap();
        // seat files must be ignored
        std::fs::write(d.join("seat-1.txt"), "noise").unwrap();
    }
    Command::new("bash")
        .arg(gate())
        .env("QUORUM_STORE", &store)
        .env("QUORUM_REPO", "paiml/forjar")
        .env("PR_NUMBER", "798")
        .env("PR_AUTHOR_MODELS", "Claude Sonnet 5.5")
        .env("PR_HEAD_SHA", head)
        .output()
        .unwrap()
}

fn ok(o: &Output) -> bool {
    o.status.success()
}

fn text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

#[test]
fn good_receipt_passes() {
    let o = run(
        Some(receipt(SHA, 798, "claude-sonnet-5-5", &three_good(), "")),
        SHA,
        SHA,
    );
    assert!(ok(&o), "{}", text(&o));
}

#[test]
fn missing_store_dir_fails() {
    let o = run(None, SHA, SHA);
    assert!(!ok(&o), "{}", text(&o));
}

#[test]
fn missing_file_for_head_fails() {
    let other = "acff20f1b5eb1991ab26fb03eb8076d1417d4d2c";
    let o = run(
        Some(receipt(other, 798, "claude-sonnet-5-5", &three_good(), "")),
        other,
        SHA,
    );
    assert!(!ok(&o), "{}", text(&o));
}

#[test]
fn sha_inside_file_must_equal_head() {
    let other = "acff20f1b5eb1991ab26fb03eb8076d1417d4d2c";
    // file is named for head but its body claims another sha
    let o = run(
        Some(receipt(other, 798, "claude-sonnet-5-5", &three_good(), "")),
        SHA,
        SHA,
    );
    assert!(!ok(&o), "{}", text(&o));
}

#[test]
fn pr_mismatch_fails() {
    let o = run(
        Some(receipt(SHA, 799, "claude-sonnet-5-5", &three_good(), "")),
        SHA,
        SHA,
    );
    assert!(!ok(&o), "{}", text(&o));
}

#[test]
fn fewer_than_three_pass_lanes_fails() {
    let l = vec![
        lane("a", "claude-opus-5-5", "PASS"),
        lane("b", "claude-haiku-5-5", "PASS"),
    ];
    let o = run(
        Some(receipt(SHA, 798, "claude-sonnet-5-5", &l, "")),
        SHA,
        SHA,
    );
    assert!(!ok(&o), "{}", text(&o));
}

#[test]
fn non_pass_verdict_does_not_count() {
    let mut l = three_good();
    l[2] = lane("c", "gemini-3", "FAIL");
    let o = run(
        Some(receipt(SHA, 798, "claude-sonnet-5-5", &l, "")),
        SHA,
        SHA,
    );
    assert!(!ok(&o), "{}", text(&o));
}

#[test]
fn duplicate_lane_names_count_once() {
    let l = vec![
        lane("a", "claude-opus-5-5", "PASS"),
        lane("a", "claude-haiku-5-5", "PASS"),
        lane("a", "gemini-3", "PASS"),
    ];
    let o = run(
        Some(receipt(SHA, 798, "claude-sonnet-5-5", &l, "")),
        SHA,
        SHA,
    );
    assert!(!ok(&o), "{}", text(&o));
}

#[test]
fn author_model_lane_is_excluded() {
    // 3 PASS lanes but one is the author's own model id: only 2 count.
    let l = vec![
        lane("a", "claude-opus-5-5", "PASS"),
        lane("b", "claude-haiku-5-5", "PASS"),
        lane("c", "claude-sonnet-5-5", "PASS"),
    ];
    let o = run(
        Some(receipt(SHA, 798, "claude-sonnet-5-5", &l, "")),
        SHA,
        SHA,
    );
    assert!(!ok(&o), "{}", text(&o));
}

#[test]
fn waiver_key_anywhere_fails() {
    for extra in [r#","waived":{"reason":"x"}"#, r#","override":true"#] {
        let o = run(
            Some(receipt(SHA, 798, "claude-sonnet-5-5", &three_good(), extra)),
            SHA,
            SHA,
        );
        assert!(!ok(&o), "{extra}: {}", text(&o));
    }
    let nested = receipt(
        SHA,
        798,
        "claude-sonnet-5-5",
        &three_good(),
        r#","meta":{"deep":{"waived":false}}"#,
    );
    let o = run(Some(nested), SHA, SHA);
    assert!(!ok(&o), "{}", text(&o));
}

#[test]
fn unparseable_receipt_fails() {
    let o = run(Some("not json".into()), SHA, SHA);
    assert!(!ok(&o), "{}", text(&o));
}

#[test]
fn missing_pr_env_fails() {
    let o = Command::new("bash")
        .arg(gate())
        .env_remove("PR_NUMBER")
        .env_remove("PR_HEAD_SHA")
        .output()
        .unwrap();
    assert!(!ok(&o), "{}", text(&o));
}

#[test]
fn workflow_runs_the_store_gate_not_the_committed_receipt_gate() {
    let wf = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/quorum.yml"),
    )
    .unwrap();
    assert!(wf.contains("scripts/quorum-store-gate.sh"));
    assert!(!wf.contains("./scripts/quorum-gate.sh"));
}

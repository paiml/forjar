//! Bypass falsifiers for `scripts/quorum-store-gate.sh` (PMAT-554 review round 2).
//!
//! Every test plants one forged or degenerate receipt and demands the gate
//! FAIL. Companion to `falsification_quorum_gate_reads_the_receipt_store.rs`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SHA: &str = "141132a621af2843efe9d6689b2b469ef34f7ce4";

fn gate() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/quorum-store-gate.sh")
}

fn lane(name: &str, model: &str, verdict: &str) -> String {
    format!(r#"{{"lane":"{name}","model":"{model}","verdict":"{verdict}","blockers":[]}}"#)
}

fn receipt_repo(repo: &str, author: &str, lanes: &[String], extra: &str) -> String {
    format!(
        r#"{{"repo":"{repo}","pr":798,"sha":"{SHA}","author_model":"{author}"{extra},"verdicts":[{}]}}"#,
        lanes.join(",")
    )
}

fn receipt(author: &str, lanes: &[String], extra: &str) -> String {
    receipt_repo("paiml/forjar", author, lanes, extra)
}

fn three_good() -> Vec<String> {
    vec![
        lane("a", "claude-opus-5-5", "PASS"),
        lane("b", "claude-haiku-5-5", "PASS"),
        lane("c", "gemini-3", "PASS"),
    ]
}

fn run_with(body: &str, repo_env: &str, symlink: bool) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    let d = store.join("quorum/paiml/forjar/798");
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join(format!("{SHA}.json"));
    if symlink {
        let real = dir.path().join("real.json");
        std::fs::write(&real, body).unwrap();
        std::os::unix::fs::symlink(&real, &f).unwrap();
    } else {
        std::fs::write(&f, body).unwrap();
    }
    Command::new("bash")
        .arg(gate())
        .env("QUORUM_STORE", &store)
        .env("QUORUM_REPO", repo_env)
        .env("PR_NUMBER", "798")
        .env("PR_HEAD_SHA", SHA)
        .output()
        .unwrap()
}

fn run(body: &str) -> Output {
    run_with(body, "paiml/forjar", false)
}

fn passes(body: &str) -> bool {
    run(body).status.success()
}

#[test]
fn control_good_receipt_passes() {
    assert!(passes(&receipt("claude-sonnet-5-5", &three_good(), "")));
}

#[test]
fn multi_document_file_fails() {
    let good = receipt("claude-sonnet-5-5", &three_good(), "");
    let waived = receipt("claude-sonnet-5-5", &three_good(), r#","waived":true"#);
    assert!(!passes(&format!("{good}\n{waived}\n")));
    assert!(!passes(&format!("{good}\n{good}\n")));
}

#[test]
fn same_model_under_three_lane_names_fails() {
    let l = vec![
        lane("a", "claude-opus-5-5", "PASS"),
        lane("b", "claude-opus-5-5", "PASS"),
        lane("c", "claude-opus-5-5", "PASS"),
    ];
    assert!(!passes(&receipt("claude-sonnet-5-5", &l, "")));
    // provider prefix / case / whitespace do not make a model distinct
    let l = vec![
        lane("a", "claude-opus-5-5", "PASS"),
        lane("b", "anthropic/claude-opus-5-5", "PASS"),
        lane("c", " Claude-Opus-5-5 ", "PASS"),
    ];
    assert!(!passes(&receipt("claude-sonnet-5-5", &l, "")));
}

#[test]
fn lane_name_variants_collapse() {
    let l = vec![
        lane("a", "claude-opus-5-5", "PASS"),
        lane("A", "claude-haiku-5-5", "PASS"),
        lane(" a ", "gemini-3", "PASS"),
    ];
    assert!(!passes(&receipt("claude-sonnet-5-5", &l, "")));
}

#[test]
fn author_model_variants_are_excluded() {
    // (author, lane model that is the author's own family)
    let cases = [
        ("Anthropic/Claude-Opus-5-5", "claude-opus-5-5"),
        ("claude-opus-5-5", "claude-opus-5-5-20260101"),
        ("claude-opus-5-5", "claude-opus-5"),
        ("claude-opus-5", "claude-opus-5-5"),
        ("opus", "claude-opus-5-5"),
        (" CLAUDE-OPUS-5-5 ", "claude-opus-5-5"),
    ];
    for (author, same) in cases {
        let l = vec![
            lane("a", same, "PASS"),
            lane("b", "claude-haiku-5-5", "PASS"),
            lane("c", "gemini-3", "PASS"),
        ];
        assert!(!passes(&receipt(author, &l, "")), "{author} vs {same}");
    }
}

#[test]
fn author_model_empty_or_unknown_fails() {
    for a in ["", "unknown", "UNKNOWN", " Unknown ", "  "] {
        assert!(!passes(&receipt(a, &three_good(), "")), "author '{a}'");
    }
}

#[test]
fn pass_lane_with_blockers_does_not_count() {
    let mut l = three_good();
    l[2] = r#"{"lane":"c","model":"gemini-3","verdict":"PASS","blockers":["race in x"]}"#.into();
    assert!(!passes(&receipt("claude-sonnet-5-5", &l, "")));
    l[2] = r#"{"lane":"c","model":"gemini-3","verdict":"PASS","blockers":"bad"}"#.into();
    assert!(!passes(&receipt("claude-sonnet-5-5", &l, "")));
}

#[test]
fn model_and_lane_must_be_nonempty_strings() {
    let bad = [
        r#"{"lane":"c","model":"","verdict":"PASS","blockers":[]}"#,
        r#"{"lane":"c","model":"   ","verdict":"PASS","blockers":[]}"#,
        r#"{"lane":"c","model":7,"verdict":"PASS","blockers":[]}"#,
        r#"{"lane":"","model":"gemini-3","verdict":"PASS","blockers":[]}"#,
        r#"{"lane":3,"model":"gemini-3","verdict":"PASS","blockers":[]}"#,
        r#"{"model":"gemini-3","verdict":"PASS","blockers":[]}"#,
    ];
    for b in bad {
        let mut l = three_good();
        l[2] = b.into();
        assert!(!passes(&receipt("claude-sonnet-5-5", &l, "")), "{b}");
    }
}

#[test]
fn lane_with_any_non_pass_verdict_is_disqualified() {
    for dup in ["a", "A", " a "] {
        let mut l = three_good();
        l.push(lane(dup, "claude-opus-5-5", "FAIL"));
        assert!(
            !passes(&receipt("claude-sonnet-5-5", &l, "")),
            "dup '{dup}'"
        );
    }
}

#[test]
fn waiver_words_in_values_fail() {
    for extra in [
        r#","note":"Waived by cop""#,
        r#","note":"OVERRIDE granted""#,
        r#","meta":{"deep":["x","manual Override"]}"#,
    ] {
        assert!(
            !passes(&receipt("claude-sonnet-5-5", &three_good(), extra)),
            "{extra}"
        );
    }
}

#[test]
fn symlinked_receipt_fails() {
    let body = receipt("claude-sonnet-5-5", &three_good(), "");
    assert!(run_with(&body, "paiml/forjar", false).status.success());
    assert!(!run_with(&body, "paiml/forjar", true).status.success());
}

#[test]
fn quorum_repo_is_validated() {
    let body = receipt("claude-sonnet-5-5", &three_good(), "");
    for r in [
        "../x",
        "paiml/../forjar",
        "a/b/c",
        "paiml",
        "paiml/for jar",
        "/x/y",
        "../..",
    ] {
        assert!(!run_with(&body, r, false).status.success(), "repo '{r}'");
    }
}

#[test]
fn repo_field_mismatch_fails() {
    let body = receipt_repo("paiml/other", "claude-sonnet-5-5", &three_good(), "");
    assert!(!passes(&body));
}

#[test]
fn missing_author_model_fails() {
    let body = format!(
        r#"{{"repo":"paiml/forjar","pr":798,"sha":"{SHA}","verdicts":[{}]}}"#,
        three_good().join(",")
    );
    assert!(!passes(&body));
}

//! Round-3 falsifiers for `scripts/quorum-store-gate.sh`: exact-id allowlist
//! (`scripts/quorum-models.json`), author derived from commit trailers, strict
//! receipt schema. Companion to `falsification_quorum_gate_bypasses.rs`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SHA: &str = "141132a621af2843efe9d6689b2b469ef34f7ce4";

fn gate() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/quorum-store-gate.sh")
}

fn lane(name: &str, model: &str, verdict: &str) -> String {
    format!(r#"{{"lane":"{name}","model":"{model}","verdict":"{verdict}","blockers":[]}}"#)
}

fn receipt(author: &str, lanes: &[String], extra: &str) -> String {
    format!(
        r#"{{"repo":"paiml/forjar","pr":798,"sha":"{SHA}","author_model":"{author}"{extra},"verdicts":[{}]}}"#,
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

// ---- round 3: exact-id allowlist, trailer-derived author, strict schema ----

/// Run with full control of the author env and working directory.
fn run_cfg(
    body: &[u8],
    authors: Option<&str>,
    extra_env: &[(&str, String)],
    cwd: Option<&Path>,
) -> Output {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    let d = store.join("quorum/paiml/forjar/798");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(format!("{SHA}.json")), body).unwrap();
    let mut c = Command::new("bash");
    c.arg(gate())
        .env_remove("PR_AUTHOR_MODELS")
        .env_remove("PR_BASE_SHA")
        .env("QUORUM_STORE", &store)
        .env("QUORUM_REPO", "paiml/forjar")
        .env("PR_NUMBER", "798")
        .env("PR_HEAD_SHA", SHA);
    if let Some(a) = authors {
        c.env("PR_AUTHOR_MODELS", a);
    }
    for (k, v) in extra_env {
        c.env(k, v);
    }
    if let Some(p) = cwd {
        c.current_dir(p);
    }
    c.output().unwrap()
}

fn sonnet_passes(body: &str) -> bool {
    run_cfg(body.as_bytes(), Some("Claude Sonnet 5.5"), &[], None)
        .status
        .success()
}

fn rec3(lanes: &[String]) -> String {
    receipt("claude-sonnet-5-5", lanes, "")
}

#[test]
fn model_spelling_variants_are_not_in_the_allowlist() {
    for m in [
        "Claude Sonnet 5.5",
        "claude_opus_5_5",
        "opus5.5",
        "Claude-Opus-5-5",
        "anthropic/claude-opus-5-5",
        " claude-opus-5-5",
        "claude-opus-5-5 ",
        "claude-opus-5",
        "claude-opus-5-5-20260101",
    ] {
        let l = vec![
            lane("a", m, "PASS"),
            lane("b", "claude-haiku-5-5", "PASS"),
            lane("c", "gemini-3", "PASS"),
        ];
        assert!(!sonnet_passes(&rec3(&l)), "model '{m}'");
    }
}

#[test]
fn non_ascii_anywhere_fails() {
    let good = rec3(&three_good());
    // U+2010 hyphen, U+200B zero-width space, Cyrillic a (U+0430), raw bytes
    for (from, to) in [
        ("claude-opus-5-5", "claude\u{2010}opus-5-5"),
        ("claude-opus-5-5", "claude-opus-5-5\u{200b}"),
        ("claude-haiku-5-5", "cl\u{0430}ude-haiku-5-5"),
        ("\"a\"", "\"\u{0430}\""),
    ] {
        assert!(!sonnet_passes(&good.replace(from, to)), "{to}");
    }
    // a non-ASCII byte in an unrelated field also fails (whole receipt is ASCII-only)
    assert!(!sonnet_passes(&receipt(
        "claude-sonnet-5-5",
        &three_good(),
        r#","note":"café""#
    )));
    assert!(!sonnet_passes(&receipt(
        "claude-sonnet-5-5",
        &three_good(),
        ",\"note\":\"caf\u{e9}\""
    )));
    // \u escapes that decode to a lookalike are refused as well
    assert!(!sonnet_passes(
        &good.replace("claude-opus-5-5", r"claude‐opus-5-5")
    ));
}

#[test]
fn same_model_under_three_lane_names_fails() {
    let l = vec![
        lane("a", "claude-opus-5-5", "PASS"),
        lane("b", "claude-opus-5-5", "PASS"),
        lane("c", "claude-opus-5-5", "PASS"),
    ];
    assert!(!sonnet_passes(&rec3(&l)));
}

#[test]
fn one_lane_with_three_models_fails() {
    let l = vec![
        lane("a", "claude-opus-5-5", "PASS"),
        lane("a", "claude-haiku-5-5", "PASS"),
        lane("a", "gemini-3", "PASS"),
    ];
    assert!(!sonnet_passes(&rec3(&l)));
}

#[test]
fn gpt_prefix_spellings_are_one_allowlisted_id_at_most() {
    let l = vec![
        lane("a", "gpt-5", "PASS"),
        lane("b", "x/openai/gpt-5", "PASS"),
        lane("c", "y/z/gpt-5", "PASS"),
    ];
    assert!(!sonnet_passes(&rec3(&l)));
}

#[test]
fn forged_or_missing_receipt_author_fails() {
    for a in ["none", "Unknown", "unknown", "", "gpt-4o"] {
        assert!(
            !sonnet_passes(&receipt(a, &three_good(), "")),
            "author '{a}'"
        );
    }
}

#[test]
fn receipt_author_must_match_a_commit_trailer() {
    // trailer says Opus authored; receipt claims Sonnet authored, so the Opus lane
    // would wrongly count if the receipt were believed. It must FAIL (mismatch).
    let body = rec3(&three_good());
    let o = run_cfg(body.as_bytes(), Some("Claude Opus 5.5"), &[], None);
    assert!(!o.status.success());
}

#[test]
fn trailer_author_lane_is_excluded_even_if_receipt_is_honest() {
    let l = vec![
        lane("a", "claude-opus-5-5", "PASS"),
        lane("b", "claude-haiku-5-5", "PASS"),
        lane("c", "gemini-3", "PASS"),
    ];
    let body = receipt("claude-opus-5-5", &l, "");
    let o = run_cfg(body.as_bytes(), Some("Claude Opus 5.5"), &[], None);
    assert!(!o.status.success(), "opus author + opus lane leaves 2");
    // control: Opus author with three non-opus lanes passes
    let l = vec![
        lane("a", "claude-sonnet-5-5", "PASS"),
        lane("b", "claude-haiku-5-5", "PASS"),
        lane("c", "gemini-3", "PASS"),
    ];
    let body = receipt("claude-opus-5-5", &l, "");
    assert!(run_cfg(body.as_bytes(), Some("Claude Opus 5.5"), &[], None)
        .status
        .success());
}

#[test]
fn two_trailer_authors_exclude_both_ids() {
    let body = rec3(&three_good());
    let o = run_cfg(
        body.as_bytes(),
        Some("Claude Sonnet 5.5\nClaude Opus 5.5"),
        &[],
        None,
    );
    assert!(!o.status.success());
}

#[test]
fn no_or_unmapped_trailer_fails() {
    let body = rec3(&three_good());
    for a in [
        None,
        Some(""),
        Some("\n"),
        Some("Some Human"),
        Some("Claude Sonnet 5.5\nSome Human"),
    ] {
        assert!(
            !run_cfg(body.as_bytes(), a, &[], None).status.success(),
            "{a:?}"
        );
    }
    // full "Name <email>" trailer values are accepted
    assert!(run_cfg(
        body.as_bytes(),
        Some("Claude Sonnet 5.5 <noreply@anthropic.com>"),
        &[],
        None
    )
    .status
    .success());
}

#[test]
fn author_is_computed_from_git_log_when_env_is_unset() {
    let dir = tempfile::tempdir().unwrap();
    let g = |args: &[&str]| {
        let o = Command::new("git")
            .args(args)
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(o.status.success(), "{args:?}");
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    };
    g(&["init", "-q"]);
    g(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        "base",
    ]);
    let base = g(&["rev-parse", "HEAD"]);
    let msg = "work\n\nPmat-Ticket: PMAT-554\n\nCo-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>\n";
    g(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        msg,
    ]);
    let head = g(&["rev-parse", "HEAD"]);
    let body = rec3(&three_good()).replace(SHA, &head);
    let go = |base: &str| {
        let store = dir.path().join("store");
        let d = store.join("quorum/paiml/forjar/798");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(format!("{head}.json")), &body).unwrap();
        Command::new("bash")
            .arg(gate())
            .env_remove("PR_AUTHOR_MODELS")
            .env("PR_BASE_SHA", base)
            .env("QUORUM_STORE", &store)
            .env("QUORUM_REPO", "paiml/forjar")
            .env("PR_NUMBER", "798")
            .env("PR_HEAD_SHA", &head)
            .current_dir(dir.path())
            .output()
            .unwrap()
    };
    let ok = go(&base);
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );
    // a range with no trailer fails (UNMEASURED)
    g(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-q",
        "--amend",
        "--allow-empty",
        "-m",
        "no trailer",
    ]);
    let head2 = g(&["rev-parse", "HEAD"]);
    let d = dir.path().join("store/quorum/paiml/forjar/798");
    std::fs::write(d.join(format!("{head2}.json")), body.replace(&head, &head2)).unwrap();
    let no = Command::new("bash")
        .arg(gate())
        .env_remove("PR_AUTHOR_MODELS")
        .env("PR_BASE_SHA", &base)
        .env("QUORUM_STORE", dir.path().join("store"))
        .env("QUORUM_REPO", "paiml/forjar")
        .env("PR_NUMBER", "798")
        .env("PR_HEAD_SHA", &head2)
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!no.status.success());
}

#[test]
fn pass_with_blocker_then_second_pass_entry_fails() {
    let l = vec![
        r#"{"lane":"a","model":"claude-opus-5-5","verdict":"PASS","blockers":["race"]}"#
            .to_string(),
        lane("a", "claude-opus-5-5", "PASS"),
        lane("b", "claude-haiku-5-5", "PASS"),
        lane("c", "gemini-3", "PASS"),
    ];
    assert!(!sonnet_passes(&rec3(&l)));
    // FAIL then PASS under the same lane name
    let l = vec![
        lane("a", "claude-opus-5-5", "FAIL"),
        lane("a", "claude-opus-5-5", "PASS"),
        lane("b", "claude-haiku-5-5", "PASS"),
        lane("c", "gemini-3", "PASS"),
    ];
    assert!(!sonnet_passes(&rec3(&l)));
}

#[test]
fn duplicate_json_keys_fail() {
    let good = rec3(&three_good());
    let verdict_dup = good.replacen(
        r#""verdict":"PASS""#,
        r#""verdict":"FAIL","verdict":"PASS""#,
        1,
    );
    let blockers_dup = good.replacen(r#""blockers":[]"#, r#""blockers":["x"],"blockers":[]"#, 1);
    let model_dup = good.replacen(
        r#""model":"claude-opus-5-5""#,
        r#""model":"claude-sonnet-5-5","model":"claude-opus-5-5""#,
        1,
    );
    let lane_dup = good.replacen(r#""lane":"a""#, r#""lane":"zz","lane":"a""#, 1);
    let repo_dup = good.replacen(
        r#""repo":"paiml/forjar""#,
        r#""repo":"x/y","repo":"paiml/forjar""#,
        1,
    );
    let author_dup = good.replacen(
        r#""author_model":"claude-sonnet-5-5""#,
        r#""author_model":"gpt-4o","author_model":"claude-sonnet-5-5""#,
        1,
    );
    let nested_then_scalar = receipt(
        "claude-sonnet-5-5",
        &three_good(),
        r#","n":{"k":[1]},"n":2"#,
    );
    for b in [
        verdict_dup,
        blockers_dup,
        model_dup,
        lane_dup,
        repo_dup,
        author_dup,
        nested_then_scalar,
    ] {
        assert!(!sonnet_passes(&b), "{b}");
    }
}

#[test]
fn blockers_must_be_an_array_present_and_spelled_right() {
    let ent = |tail: &str| format!(r#"{{"lane":"c","model":"gemini-3","verdict":"PASS"{tail}}}"#);
    for tail in [
        r#","blockers":null"#,
        "",
        r#","blocker":[]"#,
        r#","blockers":{}"#,
        r#","blockers":"""#,
        r#","blockers":false"#,
        r#","blockers":[],"blocker":[]"#,
    ] {
        let tail = if tail == r#","blockers":""# {
            r#","blockers":"""#
        } else {
            tail
        };
        let mut l = three_good();
        l[2] = ent(tail);
        assert!(!sonnet_passes(&rec3(&l)), "{tail}");
    }
}

#[test]
fn pr_must_be_a_json_integer_literal() {
    let good = rec3(&three_good());
    for lit in [
        "798.0", "7.98e2", "7980e-1", "\"798\"", "0798", "798.", "+798",
    ] {
        let b = good.replace("\"pr\":798", &format!("\"pr\":{lit}"));
        assert!(!sonnet_passes(&b), "pr {lit}");
    }
}

#[test]
fn sha_must_be_lowercase_40_hex_in_the_body() {
    let good = rec3(&three_good());
    assert!(!sonnet_passes(&good.replace(SHA, &SHA.to_uppercase())));
}

#[test]
fn lane_names_are_strict_ascii_tokens() {
    for n in ["A", "a b", "a.b", "a/b", ""] {
        let l = vec![
            lane(n, "claude-opus-5-5", "PASS"),
            lane("b", "claude-haiku-5-5", "PASS"),
            lane("c", "gemini-3", "PASS"),
        ];
        assert!(!sonnet_passes(&rec3(&l)), "lane '{n}'");
    }
    let l = vec![
        lane("a_1", "claude-opus-5-5", "PASS"),
        lane("b-2", "claude-haiku-5-5", "PASS"),
        lane("c3", "gemini-3", "PASS"),
    ];
    assert!(sonnet_passes(&rec3(&l)));
}

#[test]
fn waiver_spellings_fail_in_keys_and_values() {
    for extra in [
        r#","note":"Waived by cop""#,
        r#","note":"WAIVER""#,
        r#","note":"OVERRIDE granted""#,
        r#","note":"overridden""#,
        r#","note":"bypass""#,
        r#","note":"Skip_Quorum""#,
        r#","skip_quorum":true"#,
        r#","waived":false"#,
        r#","Override":1"#,
        r#","meta":{"deep":["x","manual Override"]}"#,
    ] {
        assert!(
            !sonnet_passes(&receipt("claude-sonnet-5-5", &three_good(), extra)),
            "{extra}"
        );
    }
}

#[test]
fn empty_or_missing_receipt_content_fails() {
    for b in ["", "\n", "{}", "[]", "null"] {
        assert!(!sonnet_passes(b), "body {b:?}");
    }
    assert!(!sonnet_passes(&rec3(&[])));
}

#[test]
fn real_shaped_two_lane_receipts_fail() {
    let l = vec![
        lane("delta-opus", "claude-opus-5-5", "PASS"),
        lane("delta-haiku", "claude-haiku-5-5", "PASS"),
    ];
    assert!(!sonnet_passes(&rec3(&l)));
}

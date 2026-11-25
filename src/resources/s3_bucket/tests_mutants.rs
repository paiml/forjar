//! Mutant proof for every check: neutralise ONE assertion in the generated check
//! script and the drift case for it must go from red to green. A case that stays red
//! under its own mutant is not testing that assertion.

use super::tests::cases;
use super::tests_fake::{res, Fake};
use super::*;

/// Assertion condition lines (the line after `if`) keyed by the marker they print.
fn neutralise(script: &str, converged_marker: &str) -> String {
    let lines: Vec<&str> = script.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        // pattern: "if" / "  <cond>" / "then" / "  echo '<marker>:B'"
        if lines[i] == "if"
            && lines
                .get(i + 3)
                .is_some_and(|l| l.contains(converged_marker))
        {
            out.push(lines[i]);
            out.push("  true");
            i += 2;
            continue;
        }
        out.push(lines[i]);
        i += 1;
    }
    out.join("\n")
}

#[test]
fn every_case_is_red_on_the_real_script_and_green_under_its_assertions_mutant() {
    let real = check_script(&res());
    for (label, divergent, break_it) in cases() {
        let converged = match divergent {
            "s3-bucket-absent" => "s3-bucket-present",
            "s3-sse-missing" => "s3-sse-aes256",
            "s3-public-access-open" => "s3-public-access-blocked",
            "s3-versioning-off" => "s3-versioning-enabled",
            "s3-lifecycle-missing" => "s3-lifecycle-present",
            "s3-tls-only-policy-missing" => "s3-tls-only-policy",
            other => panic!("unmapped marker {other}"),
        };
        let mutant = neutralise(&real, converged);
        assert_ne!(mutant, real, "{label}: mutation did not apply");
        let f = Fake::converged();
        break_it(&f);
        assert_eq!(f.run(&real).0, 1, "{label}: real script should be red");
        let f = Fake::converged();
        break_it(&f);
        assert_eq!(
            f.run(&mutant).0,
            0,
            "{label}: case stays red under the `{converged}` mutant, so it does not test it"
        );
    }
}

#[test]
fn a_check_with_every_assertion_neutralised_would_pass_an_empty_account() {
    // Pins that the neutraliser really removes the verdict: with all six removed an
    // empty account is "converged" — the shape every real check must not have.
    let mut s = check_script(&res());
    for m in [
        "s3-bucket-present",
        "s3-sse-aes256",
        "s3-public-access-blocked",
        "s3-versioning-enabled",
        "s3-lifecycle-present",
        "s3-tls-only-policy",
    ] {
        s = neutralise(&s, m);
    }
    assert_eq!(Fake::new().run(&s).0, 0);
    assert_ne!(Fake::new().run(&check_script(&res())).0, 0);
}

#[test]
fn the_lookup_guard_has_a_mutant_too() {
    // Mutant: treat every head-bucket failure as absent. apply then tries to create.
    let real = apply_script(&res());
    let mutant = real.replace(
        "*) echo 'bucket-lookup-failed'",
        "*) have=no;; xx) echo 'bucket-lookup-failed'",
    );
    assert_ne!(mutant, real);
    let f = Fake::converged();
    f.set(
        "head.err",
        "An error occurred (403) when calling the HeadBucket operation: Forbidden",
    );
    f.clear_log();
    assert_eq!(f.run(&real).0, 1);
    assert!(!f.log().contains("create-bucket"));
    f.clear_log();
    f.run(&mutant);
    assert!(
        f.log().contains("create-bucket"),
        "mutant should have tried to create"
    );
}

/// The head-bucket 404 pattern as shipped, and as it was before the fix (any "404").
const FIXED_404: &str = "*'An error occurred (404) when calling the HeadBucket operation'*|*'(NoSuchBucket)'*) have=no;;";
const OLD_404: &str = "*404*|*'Not Found'*) have=no;;";

fn bash_n(script: &str) {
    let out = std::process::Command::new("sh")
        .arg("-n")
        .arg("-c")
        .arg(script)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "mutant is not valid shell: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn a_network_error_mentioning_404_is_an_error_not_absent() {
    // Bug 1: the old pattern read any text holding "404" (a port, a request id, a
    // proxy page) as "no such bucket", so apply would create-bucket on a network error.
    for msg in [
        "Could not connect to the endpoint URL: \"https://s3.us-east-1.amazonaws.com:4040/paiml\"",
        "Connect timeout on endpoint URL: \"https://proxy.internal:404/\"",
        "An error occurred (RequestTimeout) when calling the HeadBucket operation: id 4041",
    ] {
        for (label, script) in [
            ("apply", apply_script(&res())),
            ("check", check_script(&res())),
        ] {
            let f = Fake::new();
            f.set("head.err", msg);
            f.clear_log();
            let (rc, out, _) = f.run(&script);
            assert_eq!(rc, 1, "{label}: {msg}");
            assert!(!out.contains("s3-bucket-absent"), "{label}: {msg}");
            assert!(
                !f.log().contains("create-"),
                "{label}: wrote on {msg}: {}",
                f.log()
            );
        }
    }
    // A real HeadBucket 404 and a NoSuchBucket code are still absent, so apply creates.
    for msg in [
        "An error occurred (404) when calling the HeadBucket operation: Not Found",
        "An error occurred (NoSuchBucket) when calling the HeadBucket operation: gone",
    ] {
        let f = Fake::new();
        f.set("head.err", msg);
        f.run(&apply_script(&res()));
        assert!(f.log().contains("create-bucket"), "{msg}: {}", f.log());
    }
}

#[test]
fn bug1_revert_proof_old_pattern_creates_on_a_network_error() {
    let real = apply_script(&res());
    assert!(real.contains(FIXED_404));
    let mutant = real.replace(FIXED_404, OLD_404);
    assert_ne!(mutant, real);
    bash_n(&mutant);
    let f = Fake::new();
    f.set(
        "head.err",
        "Could not connect to the endpoint URL: \"https://s3.us-east-1.amazonaws.com:4040/x\"",
    );
    f.run(&mutant);
    assert!(
        f.log().contains("create-bucket"),
        "reverted fix must reproduce the bug"
    );
}

/// (what to break in the stored policy, which predicate it is)
fn policy_breaks() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("Effect Deny", "\"Effect\":\"Deny\"", "\"Effect\":\"Allow\""),
        (
            "Principal *",
            "\"Principal\":\"*\"",
            "\"Principal\":\"nobody\"",
        ),
        (
            "Action s3:*",
            "\"Action\":\"s3:*\"",
            "\"Action\":\"s3:GetObject\"",
        ),
        (
            "bucket resource",
            "\"arn:aws:s3:::paiml-metrics-preview\",",
            "",
        ),
        (
            "object resource",
            ",\"arn:aws:s3:::paiml-metrics-preview/*\"",
            "",
        ),
        (
            "Bool SecureTransport false",
            "\"aws:SecureTransport\":\"false\"",
            "\"aws:SecureTransport\":\"true\"",
        ),
    ]
}

#[test]
fn state_query_reports_tls_only_no_when_any_of_the_six_predicates_is_missing() {
    let q = state_query_script(&res());
    let f = Fake::converged();
    assert!(f.run(&q).1.contains("tls_only=yes"));
    for (label, from, to) in policy_breaks() {
        let f = Fake::converged();
        let p = std::fs::read_to_string(f.file("pol"))
            .unwrap()
            .replace(['\n', ' '], "");
        assert!(p.contains(from), "{label}: fixture lacks {from}");
        f.set("pol", &p.replace(from, to));
        let out = f.run(&q).1;
        assert!(
            out.contains("tls_only=no"),
            "{label}: false tls_only=yes: {out}"
        );
    }
}

#[test]
fn bug2_revert_proof_two_predicate_state_query_reports_yes_for_a_broken_policy() {
    let real = state_query_script(&res());
    let cond = super::script::tls_condition(&res().s3.bucket.clone().unwrap());
    assert!(real.contains(&cond));
    let old = "has '\"aws:SecureTransport\":\"false\"' && has '\"Effect\":\"Deny\"'";
    let mutant = real.replace(&cond, old);
    assert_ne!(mutant, real);
    bash_n(&mutant);
    let mut wrong = 0;
    for (label, from, to) in policy_breaks() {
        let f = Fake::converged();
        let p = std::fs::read_to_string(f.file("pol"))
            .unwrap()
            .replace(['\n', ' '], "");
        f.set("pol", &p.replace(from, to));
        if f.run(&mutant).1.contains("tls_only=yes") {
            wrong += 1;
            eprintln!("reverted fix says yes for: {label}");
        }
    }
    assert!(
        wrong >= 4,
        "reverted fix must misreport at least the 4 unchecked predicates, got {wrong}"
    );
}

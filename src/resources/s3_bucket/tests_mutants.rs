//! Mutant proof for every check: neutralise ONE assertion in the generated check
//! script and the drift case for it must go from red to green. A case that stays red
//! under its own mutant is not testing that assertion.

use super::tests::{cases, res, Fake};
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

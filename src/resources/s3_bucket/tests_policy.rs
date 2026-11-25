//! The TLS policy check must hold inside ONE statement and reject the `Not*` keys.
//! Each case is run against the shipped check and state query, then against the previous
//! flat-substring condition as a mutant (checked with `sh -n`) that must misreport it.

use super::script::tls_condition;
use super::tests_fake::{res, Fake, BUCKET};
use super::*;
use crate::core::shell_escape::sh_squote;

fn stored(f: &Fake) -> String {
    std::fs::read_to_string(f.file("pol"))
        .unwrap()
        .replace(['\n', ' '], "")
}

fn stmt(extra: &str, cond: &str) -> String {
    format!(
        "{{\"Effect\":\"Deny\",\"Principal\":\"*\",{extra}\"Resource\":[\"arn:aws:s3:::{BUCKET}\",\"arn:aws:s3:::{BUCKET}/*\"]{cond}}}"
    )
}

fn doc(statements: &[String]) -> String {
    format!(
        "{{\"Version\":\"2012-10-17\",\"Statement\":[{}]}}",
        statements.join(",")
    )
}

const COND: &str = ",\"Condition\":{\"Bool\":{\"aws:SecureTransport\":\"false\"}}";

/// Policies that are NOT the TLS-only policy but would satisfy flat substring predicates.
fn impostors(f: &Fake) -> Vec<(&'static str, String)> {
    let good = stored(f);
    vec![
        (
            // Deny+SecureTransport in one statement, Action s3:* only in an Allow one.
            "cross-statement",
            doc(&[
                stmt("\"Action\":\"s3:GetObject\",", COND),
                "{\"Effect\":\"Allow\",\"Principal\":\"*\",\"Action\":\"s3:*\",\"Resource\":[\"arn:aws:s3:::paiml-metrics-preview\",\"arn:aws:s3:::paiml-metrics-preview/*\"]}".to_string(),
            ]),
        ),
        ("NotResource", good.replace("\"Resource\":[", "\"NotResource\":[")),
        ("NotPrincipal", good.replace("\"Principal\":", "\"NotPrincipal\":")),
        ("NotAction", good.replace("\"Action\":", "\"NotAction\":")),
        ("extra allow statement", good.replace("}}}]}", "}}},{\"Effect\":\"Allow\",\"Principal\":\"*\",\"Action\":\"s3:*\",\"Resource\":\"*\"}]}")),
    ]
}

fn flat_condition(b: &str) -> String {
    let q = |t: &str| sh_squote(t);
    format!(
        "has {} && has {} && has {} && has {} && has {} && {{ has {} || has {}; }}",
        q("\"Effect\":\"Deny\""),
        q("\"Bool\":{\"aws:SecureTransport\":\"false\"}"),
        q(&format!("\"arn:aws:s3:::{b}\"")),
        q(&format!("\"arn:aws:s3:::{b}/*\"")),
        q("\"Action\":\"s3:*\""),
        q("\"Principal\":\"*\""),
        q("\"Principal\":{\"AWS\":\"*\"}"),
    )
}

#[test]
fn a_policy_that_is_not_the_one_statement_tls_deny_is_never_tls_only() {
    let check = check_script(&res());
    let state = state_query_script(&res());
    assert_eq!(Fake::converged().run(&check).0, 0);
    assert!(Fake::converged().run(&state).1.contains("tls_only=yes"));
    // AWS echoing the principal as {"AWS":"*"} is still the same policy.
    let f = Fake::converged();
    f.set(
        "pol",
        &stored(&f).replace("\"Principal\":\"*\"", "\"Principal\":{\"AWS\":\"*\"}"),
    );
    assert_eq!(f.run(&check).0, 0);
    for (label, policy) in impostors(&Fake::converged()) {
        let f = Fake::converged();
        f.set("pol", &policy);
        let (rc, out, _) = f.run(&check);
        assert_eq!(rc, 1, "{label}: check accepted it: {out}");
        assert!(
            out.contains(&format!("s3-tls-only-policy-missing:{BUCKET}")),
            "{label}: {out}"
        );
        assert!(
            f.run(&state).1.contains("tls_only=no"),
            "{label}: false tls_only=yes"
        );
    }
}

#[test]
fn apply_replaces_an_impostor_policy_with_the_canonical_one() {
    let apply = apply_script(&res());
    for (label, policy) in impostors(&Fake::converged()) {
        let f = Fake::converged();
        f.set("pol", &policy);
        let (rc, out, err) = f.run(&apply);
        assert_eq!(rc, 0, "{label}: {out} {err}");
        assert_eq!(f.run(&check_script(&res())).0, 0, "{label}");
    }
}

#[test]
fn revert_proof_the_flat_substring_condition_accepts_the_impostors() {
    let cond = tls_condition(BUCKET);
    for (what, script) in [
        ("check", check_script(&res())),
        ("state", state_query_script(&res())),
    ] {
        assert!(script.contains(&cond), "{what} does not use tls_condition");
        let mutant = script.replace(&cond, &flat_condition(BUCKET));
        assert_ne!(mutant, script);
        let n = std::process::Command::new("sh")
            .arg("-n")
            .arg("-c")
            .arg(&mutant)
            .status()
            .unwrap();
        assert!(n.success(), "{what} mutant is not valid shell");
        let mut fooled = Vec::new();
        for (label, policy) in impostors(&Fake::converged()) {
            let f = Fake::converged();
            f.set("pol", &policy);
            let (rc, out, _) = f.run(&mutant);
            let yes = if what == "check" {
                rc == 0
            } else {
                out.contains("tls_only=yes")
            };
            if yes {
                fooled.push(label);
            }
        }
        assert!(
            fooled.contains(&"cross-statement") && fooled.contains(&"NotResource"),
            "{what}: reverted fix only fooled {fooled:?}"
        );
    }
}

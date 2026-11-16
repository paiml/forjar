//! #1134-#1137 falsifiers: client token, adopt by token, id line, capacity exit code.
//! The fake `aws` here replaces the shared stub: its `run-instances` and
//! client-token lookups are scripted by files in the sandbox.

use super::launch_io::{token, token_on, CAPACITY_EXIT};
use super::tests::{gated, res, Sandbox};
use super::*;

const ID: &str = "i-0123456789abcdef0";

const STUB: &str = r#"#!/bin/sh
echo "$@" >> "$STUB_LOG"
d=$(dirname "$STUB_LOG")
case "$*" in
*run-instances*)
  [ -e "$d/run.out" ] && cat "$d/run.out"
  [ -e "$d/run.err" ] && cat "$d/run.err" >&2
  exit "$(cat "$d/run.rc" 2>/dev/null || echo 0)";;
*Name=client-token*)
  if [ -e "$d/tok.fail" ]; then echo ExpiredToken >&2; exit 255; fi
  cat "$d/tok.ids" 2>/dev/null; exit 0;;
esac
exit 0
"#;

/// Apply a gated declaration with `run-instances` scripted: (rc, stdout, stderr)
/// and `tok` (the client-token lookup reply, or None for a failing lookup).
fn launch(run: (i32, &str, &str), tok: Option<&str>) -> (i32, String, String) {
    use std::os::unix::fs::PermissionsExt;
    let sb = Sandbox::new();
    sb.open_gate();
    let aws = sb.path().join("bin/aws");
    std::fs::write(&aws, STUB).unwrap();
    std::fs::set_permissions(&aws, std::fs::Permissions::from_mode(0o755)).unwrap();
    let w = |n: &str, c: &str| std::fs::write(sb.path().join(n), c).unwrap();
    w("run.rc", &run.0.to_string());
    w("run.out", run.1);
    w("run.err", run.2);
    match tok {
        Some(t) => w("tok.ids", t),
        None => w("tok.fail", ""),
    }
    let mut r = res();
    r.cloud.kill_switch = gated(sb.path()).cloud.kill_switch;
    r.cloud.parity_gate = gated(sb.path()).cloud.parity_gate;
    r.cloud.budget_cap_usd_day = Some("5".into());
    let (rc, out) = sb.run(&apply_script(&r), "");
    (rc, out, sb.log())
}

fn err(code: &str) -> String {
    format!("An error occurred ({code}) when calling the RunInstances operation: nope\n")
}

#[test]
fn client_token_is_derived_stable_and_valid() {
    let k = "ci-box-a@eu-south-2@0123456789ab";
    let t = token_on(k, 20400);
    assert!(!t.is_empty() && t.len() <= 64, "{t}");
    assert!(
        t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
        "{t}"
    );
    assert_eq!(t, token_on(k, 20400), "stable for the same inputs");
    assert_ne!(t, token_on("other@eu-south-2@0123456789ab", 20400));
    assert_ne!(t, token_on(k, 20401), "a new UTC day is a new token");
    assert!(!t.contains("ci-box-a"), "no user text in the token: {t}");
    let a = spec_of(&res()).unwrap();
    assert_eq!(token(&a), token(&a));
    let mut r = res();
    r.name = Some("ci-box-b".into());
    assert_ne!(token(&a), token(&spec_of(&r).unwrap()));
}

#[test]
fn run_instances_carries_the_client_token() {
    let (rc, out, log) = launch((0, &format!("{ID}\n"), ""), Some(""));
    assert_eq!(rc, 0, "{out}");
    let want = format!("--client-token {}", token(&spec_of(&res()).unwrap()));
    assert!(log.contains(&want), "{log}");
    assert_eq!(log.matches("run-instances").count(), 1, "{log}");
}

#[test]
fn apply_last_line_is_launched_id() {
    let (rc, out, log) = launch((0, &format!("{ID}\n"), ""), Some(""));
    assert_eq!(rc, 0, "{out}");
    assert_eq!(
        out.trim_end().lines().last(),
        Some(format!("launched {ID}").as_str())
    );
    assert!(
        log.contains("--query Instances[0].InstanceId --output text"),
        "{log}"
    );
    // a warning after the id hides it from the parse: the token lookup still finds the box
    let (rc, out, _) = launch((0, &format!("{ID}\n"), "warning: x\n"), Some(ID));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains(&format!("adopted {ID}")), "{out}");
}

#[test]
fn error_then_token_lookup_adopts_the_box() {
    let (rc, out, log) = launch((255, "", &err("RequestLimitExceeded")), Some(ID));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains(&format!("adopted {ID}\n")), "{out}");
    let want = format!(
        "Name=client-token,Values={}",
        token(&spec_of(&res()).unwrap())
    );
    assert!(log.contains(&want), "{log}");
    assert_eq!(
        log.matches("run-instances").count(),
        1,
        "never a second launch: {log}"
    );
    assert!(
        out.contains("RequestLimitExceeded"),
        "the aws error is still shown: {out}"
    );
}

#[test]
fn failed_or_malformed_adopt_lookup_is_an_error_never_no_instance() {
    let (rc, out, _) = launch((255, "", &err("RequestLimitExceeded")), None);
    assert_eq!(rc, 1, "{out}");
    assert!(
        out.contains("instance-lookup-failed") && !out.contains("adopted"),
        "{out}"
    );
    let (rc, out, _) = launch((255, "", &err("RequestLimitExceeded")), Some("i-0stub"));
    assert_eq!(rc, 1, "{out}");
    assert!(out.contains("instance-lookup-malformed"), "{out}");
}

#[test]
fn a_launch_without_a_usable_id_is_not_success() {
    let (rc, out, _) = launch((0, "i-0stub\n", ""), Some(""));
    assert_eq!(rc, 1, "{out}");
    assert!(!out.contains("launched"), "{out}");
    let (rc, out, _) = launch((0, "None\n", ""), Some(""));
    assert_eq!(rc, 1, "{out}");
    // but if the token finds the box, adopt it
    let (rc, out, _) = launch((0, "garbage\n", ""), Some(ID));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains(&format!("adopted {ID}")), "{out}");
}

#[test]
fn capacity_codes_exit_75_other_errors_keep_their_code() {
    assert_eq!(CAPACITY_EXIT, 75);
    for code in [
        "InsufficientInstanceCapacity",
        "SpotMaxPriceTooLow",
        "MaxSpotInstanceCountExceeded",
        "Unsupported",
    ] {
        let (rc, out, log) = launch((254, "", &err(code)), Some(""));
        assert_eq!(rc, CAPACITY_EXIT, "{code}: {out}");
        assert!(out.contains(code), "stderr still shown: {out}");
        assert!(
            !log.contains("client-token,Values"),
            "no lookup needed: {log}"
        );
    }
    // lookalikes and unknown codes are not capacity errors; the aws code passes through
    for e in [
        err("UnsupportedOperation"),
        err("UnauthorizedOperation"),
        err("InvalidParameterValue").replace("nope", "InsufficientInstanceCapacity"),
        "InsufficientInstanceCapacity\n".to_string(),
    ] {
        let (rc, out, _) = launch((254, "", &e), Some(""));
        assert_eq!(rc, 254, "{e}: {out}");
    }
}

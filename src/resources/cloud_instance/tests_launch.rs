//! #1134-#1137 falsifiers: client token, adopt by token, id line, capacity exit code.
//! The fake `aws` here replaces the shared stub: its `run-instances` and
//! client-token lookups are scripted by files in the sandbox.

use super::launch_io::{day_of, token_input, token_on, CAPACITY_EXIT};
use super::tests::{gated, install_tool, res, Sandbox, SPAWN_GATE};
use super::*;

pub(super) const ID: &str = "i-0123456789abcdef0";

/// The UTC day every script under test is generated for.
pub(super) const DAY: u64 = 20_400;

/// The client token `apply_script_on(r, DAY)` carries.
pub(super) fn tok(r: &Resource) -> String {
    token_on(&token_input(&spec_of(r).unwrap()), DAY)
}

const STUB: &str = r#"#!/bin/sh
echo "$@" >> "$STUB_LOG"
d=$(dirname "$STUB_LOG")
case "$*" in
*run-instances*)
  n=$(grep -c run-instances "$STUB_LOG")
  t=$(printf '%s\n' "$*" | sed -n 's/.*--client-token \([^ ]*\).*/\1/p')
  for k in out err; do
    f="$d/run.$k"; [ -e "$d/run.$k.$n" ] && f="$d/run.$k.$n"; [ -e "$d/rt.$t.$k" ] && f="$d/rt.$t.$k"
    if [ "$k" = out ]; then [ -e "$f" ] && cat "$f"; else [ -e "$f" ] && cat "$f" >&2; fi
  done
  f="$d/run.rc"; [ -e "$d/run.rc.$n" ] && f="$d/run.rc.$n"; [ -e "$d/rt.$t.rc" ] && f="$d/rt.$t.rc"
  exit "$(cat "$f" 2>/dev/null || echo 0)";;
*describe-instances*--instance-ids*)
  if [ -e "$d/state.fail" ]; then echo ExpiredToken >&2; exit 255; fi
  id=$(for a in "$@"; do case "$a" in i-*) echo "$a";; esac; done)
  k=$(grep -c -- "--instance-ids $id" "$STUB_LOG")
  for f in "$d/statefail.$id.$k" "$d/statefail.$id"; do
    if [ -e "$f" ]; then cat "$f" >&2; exit 254; fi
  done
  if [ -e "$d/state.$id.$k" ]; then cat "$d/state.$id.$k"
  elif [ -e "$d/state.$id" ]; then cat "$d/state.$id"; else echo running; fi; exit 0;;
*Name=client-token*)
  if [ -e "$d/tok.fail" ]; then echo ExpiredToken >&2; exit 255; fi
  t=$(printf '%s\n' "$*" | sed -n 's/.*Name=client-token,Values=\([^ ]*\).*/\1/p')
  if [ -e "$d/tok.$t" ]; then cat "$d/tok.$t"; else cat "$d/tok.ids" 2>/dev/null; fi; exit 0;;
esac
exit 0
"#;

pub(super) type Run<'a> = (i32, &'a str, &'a str);

/// Apply a gated declaration with `run-instances` scripted per call (the Nth
/// call gets the Nth reply, later calls the last one), `tok` (the client-token
/// lookup reply, or None for a failing lookup) and the instance states
/// (`(id, state)`, default `running`; `state_fail` makes the state read fail).
pub(super) fn launch_with(
    runs: &[Run],
    tok: Option<&str>,
    states: &[(&str, &str)],
    state_fail: bool,
) -> (i32, String, String) {
    let (sb, script) = fixture(runs, tok, states, state_fail);
    let (rc, out) = sb.run(&script, "");
    (rc, out, sb.log())
}

/// The sandbox and the apply script of `launch_with`, not yet run.
pub(super) fn fixture(
    runs: &[Run],
    tok: Option<&str>,
    states: &[(&str, &str)],
    state_fail: bool,
) -> (Sandbox, String) {
    let sb = Sandbox::new();
    sb.open_gate();
    install_tool(&sb.path().join("bin/aws"), STUB, sb.path());
    let w = |n: &str, c: &str| std::fs::write(sb.path().join(n), c).unwrap();
    for (i, run) in runs.iter().enumerate() {
        let n = i + 1;
        w(&format!("run.rc.{n}"), &run.0.to_string());
        w(&format!("run.out.{n}"), run.1);
        w(&format!("run.err.{n}"), run.2);
    }
    let last = runs[runs.len() - 1];
    w("run.rc", &last.0.to_string());
    w("run.out", last.1);
    w("run.err", last.2);
    for (id, st) in states {
        w(&format!("state.{id}"), st);
    }
    if state_fail {
        w("state.fail", "");
    }
    match tok {
        Some(t) => w("tok.ids", t),
        None => w("tok.fail", ""),
    }
    let mut r = res();
    r.cloud.kill_switch = gated(sb.path()).cloud.kill_switch;
    r.cloud.parity_gate = gated(sb.path()).cloud.parity_gate;
    r.cloud.budget_cap_usd_day = Some("5".into());
    let script = apply_script_on(&r, DAY);
    (sb, script)
}

pub(super) fn launch(run: Run, tok: Option<&str>) -> (i32, String, String) {
    launch_with(&[run], tok, &[], false)
}

/// The client tokens of the `run-instances` calls in an aws log, in order.
pub(super) fn run_tokens(log: &str) -> Vec<String> {
    // the logged command spans lines (user-data), so split the whole log
    log.split(" --client-token ")
        .skip(1)
        .filter_map(|r| r.split(' ').next())
        .map(String::from)
        .collect()
}

/// `ago` seconds before now as an EC2 launch time (`date` renders it).
pub(super) fn iso(ago: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let _gate = SPAWN_GATE.read().unwrap_or_else(|e| e.into_inner());
    let o = std::process::Command::new("date")
        .args([
            "-u",
            "-d",
            &format!("@{}", now - ago),
            "+%Y-%m-%dT%H:%M:%S+00:00",
        ])
        .output()
        .unwrap();
    String::from_utf8(o.stdout).unwrap().trim().to_string()
}

/// A well-formed `run-instances` reply: the id and a launch time one minute ago.
pub(super) fn rep(id: &str) -> String {
    format!("{id}\t{}\n", iso(60))
}

/// `first 64 hex of sha256(a + b)` through the same tools the script uses.
pub(super) fn next_token(a: &str, b: &str) -> String {
    let _gate = SPAWN_GATE.read().unwrap_or_else(|e| e.into_inner());
    let o = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "printf '%s%s' '{a}' '{b}' | sha256sum | cut -c1-64"
        ))
        .output()
        .unwrap();
    String::from_utf8(o.stdout).unwrap().trim().to_string()
}

pub(super) fn err(code: &str) -> String {
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
    assert_eq!(tok(&res()), tok(&res()));
    let mut r = res();
    r.name = Some("ci-box-b".into());
    assert_ne!(tok(&res()), tok(&r));
}

#[test]
fn the_day_number_turns_over_at_utc_midnight() {
    assert_eq!(day_of(0), 0);
    assert_eq!(day_of(86_399), 0);
    assert_eq!(day_of(86_400), 1);
    assert_eq!(day_of(1_763_251_200), 20_408);
}

#[test]
fn run_instances_carries_the_client_token() {
    let (rc, out, log) = launch((0, &rep(ID), ""), Some(""));
    assert_eq!(rc, 0, "{out}");
    assert_eq!(run_tokens(&log), [tok(&res())], "{log}");
    assert_eq!(log.matches("run-instances").count(), 1, "{log}");
}

#[test]
fn apply_last_line_is_launched_id() {
    let (rc, out, log) = launch((0, &rep(ID), ""), Some(""));
    assert_eq!(rc, 0, "{out}");
    assert_eq!(
        out.trim_end().lines().last(),
        Some(format!("launched {ID}").as_str())
    );
    assert!(
        log.contains("--query Instances[0].[InstanceId,LaunchTime] --output text"),
        "{log}"
    );
    // a warning after the id hides it from the parse: the token lookup still finds the box
    let (rc, out, _) = launch((0, &rep(ID), "warning: x\n"), Some(ID));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains(&format!("adopted {ID}")), "{out}");
}

#[test]
fn error_then_token_lookup_adopts_the_box() {
    let (rc, out, log) = launch((255, "", &err("RequestLimitExceeded")), Some(ID));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains(&format!("adopted {ID}\n")), "{out}");
    let toks = run_tokens(&log);
    let want = format!("Name=client-token,Values={}", toks[0]);
    assert!(
        log.contains(&want),
        "the lookup uses the launch token: {log}"
    );
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
    let (rc, out, _) = launch((0, &rep("i-0stub"), ""), Some(""));
    assert_eq!(rc, 1, "{out}");
    assert!(!out.contains("launched"), "{out}");
    let (rc, out, _) = launch((0, "None\n", ""), Some(""));
    assert_eq!(rc, 1, "{out}");
    for bad in [
        "i-0123456",
        "i-0123456789abcdef01",
        "i-0123456789abcdeg0",
        "x-0123456789abcdef0",
    ] {
        let (rc, out, _) = launch((0, &rep(bad), ""), Some(""));
        assert_eq!(rc, 1, "{bad}: {out}");
        assert!(!out.contains("launched"), "{bad}: {out}");
    }
    // a failed launch is never success, even if an id was printed
    let (rc, out, _) = launch((255, &rep(ID), ""), Some(""));
    assert_eq!(rc, 255, "{out}");
    assert!(!out.contains("launched"), "{out}");
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

//! #1134-#1137 falsifiers: client token, adopt by token, id line, capacity exit code.
//! The fake `aws` here replaces the shared stub: its `run-instances` and
//! client-token lookups are scripted by files in the sandbox.

use super::launch_io::{day_of, token, token_input, token_on, CAPACITY_EXIT};
use super::tests::{gated, res, Sandbox};
use super::*;

const ID: &str = "i-0123456789abcdef0";

const STUB: &str = r#"#!/bin/sh
echo "$@" >> "$STUB_LOG"
d=$(dirname "$STUB_LOG")
case "$*" in
*run-instances*)
  n=$(grep -c run-instances "$STUB_LOG")
  for k in out err; do
    f="$d/run.$k"; [ -e "$d/run.$k.$n" ] && f="$d/run.$k.$n"
    if [ "$k" = out ]; then [ -e "$f" ] && cat "$f"; else [ -e "$f" ] && cat "$f" >&2; fi
  done
  f="$d/run.rc"; [ -e "$d/run.rc.$n" ] && f="$d/run.rc.$n"
  exit "$(cat "$f" 2>/dev/null || echo 0)";;
*describe-instances*--instance-ids*)
  if [ -e "$d/state.fail" ]; then echo ExpiredToken >&2; exit 255; fi
  id=$(for a in "$@"; do case "$a" in i-*) echo "$a";; esac; done)
  if [ -e "$d/state.$id" ]; then cat "$d/state.$id"; else echo running; fi; exit 0;;
*Name=client-token*)
  if [ -e "$d/tok.fail" ]; then echo ExpiredToken >&2; exit 255; fi
  cat "$d/tok.ids" 2>/dev/null; exit 0;;
esac
exit 0
"#;

type Run<'a> = (i32, &'a str, &'a str);

/// Apply a gated declaration with `run-instances` scripted per call (the Nth
/// call gets the Nth reply, later calls the last one), `tok` (the client-token
/// lookup reply, or None for a failing lookup) and the instance states
/// (`(id, state)`, default `running`; `state_fail` makes the state read fail).
fn launch_with(
    runs: &[Run],
    tok: Option<&str>,
    states: &[(&str, &str)],
    state_fail: bool,
) -> (i32, String, String) {
    use std::os::unix::fs::PermissionsExt;
    let sb = Sandbox::new();
    sb.open_gate();
    let aws = sb.path().join("bin/aws");
    std::fs::write(&aws, STUB).unwrap();
    std::fs::set_permissions(&aws, std::fs::Permissions::from_mode(0o755)).unwrap();
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
    let (rc, out) = sb.run(&apply_script(&r), "");
    (rc, out, sb.log())
}

fn launch(run: Run, tok: Option<&str>) -> (i32, String, String) {
    launch_with(&[run], tok, &[], false)
}

/// The client tokens of the `run-instances` calls in an aws log, in order.
fn run_tokens(log: &str) -> Vec<String> {
    // the logged command spans lines (user-data), so split the whole log
    log.split(" --client-token ")
        .skip(1)
        .filter_map(|r| r.split(' ').next())
        .map(String::from)
        .collect()
}

/// A token is today's if it equals the one computed before the run or the one
/// computed after it (a run may straddle UTC midnight).
fn is_todays(t: &str, before: &str) -> bool {
    t == before || t == token(&spec_of(&res()).unwrap())
}

/// `first 64 hex of sha256(a + b)` through the same tools the script uses.
fn next_token(a: &str, b: &str) -> String {
    let o = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "printf '%s%s' '{a}' '{b}' | sha256sum | cut -c1-64"
        ))
        .output()
        .unwrap();
    String::from_utf8(o.stdout).unwrap().trim().to_string()
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
fn the_day_number_turns_over_at_utc_midnight() {
    assert_eq!(day_of(0), 0);
    assert_eq!(day_of(86_399), 0);
    assert_eq!(day_of(86_400), 1);
    assert_eq!(day_of(1_763_251_200), 20_408);
}

#[test]
fn run_instances_carries_the_client_token() {
    let before = token(&spec_of(&res()).unwrap());
    let (rc, out, log) = launch((0, &format!("{ID}\n"), ""), Some(""));
    assert_eq!(rc, 0, "{out}");
    let toks = run_tokens(&log);
    assert!(toks.len() == 1 && is_todays(&toks[0], &before), "{log}");
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
    let (rc, out, _) = launch((0, "i-0stub\n", ""), Some(""));
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
        let (rc, out, _) = launch((0, &format!("{bad}\n"), ""), Some(""));
        assert_eq!(rc, 1, "{bad}: {out}");
        assert!(!out.contains("launched"), "{bad}: {out}");
    }
    // a failed launch is never success, even if an id was printed
    let (rc, out, _) = launch((255, &format!("{ID}\n"), ""), Some(""));
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

const ID2: &str = "i-0fedcba9876543210";
const DEAD: [&str; 4] = ["shutting-down", "terminated", "stopping", "stopped"];

fn ok(id: &str) -> Run<'static> {
    (0, Box::leak(format!("{id}\n").into_boxed_str()), "")
}

#[test]
fn the_script_reads_the_state_and_walks_a_bounded_chain() {
    let s = apply_script(&gated(std::path::Path::new("/nonexistent")));
    assert!(
        s.contains("describe-instances --region 'eu-south-2' --instance-ids \"$new\"")
            && s.contains("Reservations[0].Instances[0].State.Name"),
        "{s}"
    );
    assert!(s.contains("sha256sum | cut -c1-64"), "{s}");
    assert!(s.contains("-ge 5 ]"), "{s}");
    assert!(s.contains("launch-token-chain-exhausted"), "{s}");
}

#[test]
fn a_dead_box_spends_the_token_and_the_next_token_relaunches() {
    for dead in DEAD {
        let (rc, out, log) = launch_with(&[ok(ID), ok(ID2)], Some(""), &[(ID, dead)], false);
        assert_eq!(rc, 0, "{dead}: {out}");
        assert_eq!(
            out.trim_end().lines().last(),
            Some(format!("launched {ID2}").as_str()),
            "{dead}: {out}"
        );
        let t = run_tokens(&log);
        assert_eq!(t.len(), 2, "{dead}: exactly two launches: {log}");
        assert_eq!(t[1], next_token(&t[0], ID), "{dead}: {log}");
        assert_ne!(t[0], t[1]);
        // a retried apply walks the same chain
        let (rc, _, log2) = launch_with(&[ok(ID), ok(ID2)], Some(""), &[(ID, dead)], false);
        assert_eq!(rc, 0);
        let t2 = run_tokens(&log2);
        assert_eq!(t2[1], next_token(&t2[0], ID), "{dead}");
        if t2[0] == t[0] {
            assert_eq!(t2, t, "{dead}: same tokens");
        }
    }
}

#[test]
fn a_live_box_is_launched_with_one_call() {
    for live in ["pending", "running"] {
        let (rc, out, log) = launch_with(&[ok(ID)], Some(""), &[(ID, live)], false);
        assert_eq!(rc, 0, "{live}: {out}");
        assert!(out.contains(&format!("launched {ID}\n")), "{live}: {out}");
        assert_eq!(run_tokens(&log).len(), 1, "{log}");
    }
}

#[test]
fn five_dead_boxes_in_a_row_exhaust_the_chain() {
    let ids = [
        "i-00000001",
        "i-00000002",
        "i-00000003",
        "i-00000004",
        "i-00000005",
        "i-00000006",
    ];
    let runs: Vec<Run> = ids.iter().map(|i| ok(i)).collect();
    let states: Vec<(&str, &str)> = ids.iter().map(|i| (*i, "terminated")).collect();
    let (rc, out, log) = launch_with(&runs, Some(""), &states, false);
    assert_ne!(rc, 0, "{out}");
    assert!(
        out.contains("REFUSED: launch-token-chain-exhausted"),
        "{out}"
    );
    assert!(!out.contains("launched"), "{out}");
    let t = run_tokens(&log);
    assert_eq!(t.len(), 5, "at most five generations: {log}");
    for k in 1..5 {
        assert_eq!(t[k], next_token(&t[k - 1], ids[k - 1]), "gen {k}");
    }
}

#[test]
fn a_failed_or_unknown_state_read_is_an_error_never_launched() {
    let (rc, out, log) = launch_with(&[ok(ID)], Some(""), &[], true);
    assert_eq!(rc, 1, "{out}");
    assert!(
        out.contains("instance-state-failed") && !out.contains("launched"),
        "{out}"
    );
    assert_eq!(run_tokens(&log).len(), 1, "{log}");
    for weird in ["", "None", "runningx", "Running"] {
        let (rc, out, log) = launch_with(&[ok(ID)], Some(""), &[(ID, weird)], false);
        assert_eq!(rc, 1, "{weird:?}: {out}");
        assert!(out.contains("instance-state-unknown"), "{weird:?}: {out}");
        assert!(!out.contains("launched"), "{weird:?}: {out}");
        assert_eq!(run_tokens(&log).len(), 1, "{log}");
    }
}

#[test]
fn the_capacity_classification_applies_to_every_call_of_the_chain() {
    let runs = [
        ok(ID),
        (
            254,
            "",
            Box::leak(err("InsufficientInstanceCapacity").into_boxed_str()) as &str,
        ),
    ];
    let (rc, out, log) = launch_with(&runs, Some(""), &[(ID, "terminated")], false);
    assert_eq!(rc, CAPACITY_EXIT, "{out}");
    assert_eq!(run_tokens(&log).len(), 2, "{log}");
}

#[test]
fn every_launch_parameter_the_resource_can_change_changes_the_token() {
    let base = token_input(&spec_of(&res()).unwrap());
    type Change = (&'static str, fn(&mut Resource));
    let changed: [Change; 5] = [
        ("instance_type", |r| {
            r.cloud.instance_type = Some("c7a.2xlarge".into())
        }),
        ("max_price", |r| r.cloud.max_price = Some("0.30".into())),
        ("max_lifetime_min", |r| r.cloud.max_lifetime_min = Some(90)),
        ("labels", |r| r.tags.push("x=y".into())),
        ("image name", |r| {
            r.image = Some(format!(
                "123456789012.dkr.ecr.eu-south-2.amazonaws.com/img002@{}",
                super::tests::DIGEST
            ));
        }),
    ];
    for (what, f) in changed {
        let mut r = res();
        f(&mut r);
        let s = spec_of(&r).unwrap();
        assert_ne!(token_input(&s), base, "{what}");
        assert_ne!(
            token_on(&token_input(&s), 20400),
            token_on(&base, 20400),
            "{what}"
        );
    }
}

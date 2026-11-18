//! Token chain after a launch (#1134-#1137): the state read, the chain past a
//! dead or forgotten box, the bounded state-read retry and the launch-time check.
//! The fake aws is the one of `tests_launch`; launch times are computed relative
//! to now by the test; the token's UTC day is injected (`DAY`).

use super::launch_io::{token_input, token_on, CAPACITY_EXIT};
use super::tests::{gated, res};
use super::tests_launch::{err, fixture, iso, launch_with, next_token, run_tokens, tok, Run, ID};
use super::*;

const ID2: &str = "i-0fedcba9876543210";
const DEAD: [&str; 4] = ["shutting-down", "terminated", "stopping", "stopped"];

fn ok(id: &str) -> Run<'static> {
    aged(id, 60)
}

/// A reply for `id` whose launch time is `ago` seconds old.
fn aged(id: &str, ago: u64) -> Run<'static> {
    (
        0,
        Box::leak(format!("{id}\t{}\n", iso(ago)).into_boxed_str()),
        "",
    )
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
    assert!(s.contains("-ge 32 ]"), "{s}");
    assert!(s.contains("Instances[0].[InstanceId,LaunchTime]"), "{s}");
    assert!(
        s.contains("An error occurred (InvalidInstanceID.NotFound)"),
        "{s}"
    );
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
fn a_chain_of_dead_boxes_stops_at_the_bound() {
    let ids: Vec<String> = (1..=33).map(|k| format!("i-{k:08x}")).collect();
    let runs: Vec<Run> = ids.iter().map(|i| ok(i)).collect();
    let states: Vec<(&str, &str)> = ids.iter().map(|i| (i.as_str(), "terminated")).collect();
    let (rc, out, log) = launch_with(&runs, Some(""), &states, false);
    assert_ne!(rc, 0, "{out}");
    assert!(
        out.contains("REFUSED: launch-token-chain-exhausted"),
        "{out}"
    );
    assert!(!out.contains("launched"), "{out}");
    let t = run_tokens(&log);
    assert_eq!(t.len(), 32, "at most 32 generations: {log}");
    for k in 1..32 {
        assert_eq!(t[k], next_token(&t[k - 1], &ids[k - 1]), "gen {k}");
    }
}

fn count(log: &str, what: &str) -> usize {
    log.matches(what).count()
}

#[test]
fn a_failed_state_read_is_retried_three_times_then_refused_naming_the_id() {
    let (sb, script) = fixture(&[ok(ID)], Some(""), &[], true);
    let (rc, out) = sb.run(&script, "");
    let log = sb.log();
    assert_eq!(rc, 1, "{out}");
    assert!(
        out.contains(&format!("REFUSED: instance-state-failed {ID}")) && !out.contains("launched"),
        "{out}"
    );
    assert_eq!(count(&log, "--instance-ids"), 3, "three tries: {log}");
    let naps = std::fs::read_to_string(sb.path().join("calls.log.sleep")).unwrap();
    assert_eq!(naps, "2\n2\n", "a fixed pause between the tries");
    assert_eq!(run_tokens(&log).len(), 1, "{log}");
}

#[test]
fn a_state_read_that_recovers_on_a_retry_is_launched() {
    for first in [
        format!("An error occurred (InvalidInstanceID.NotFound) when calling the DescribeInstances operation: The instance ID '{ID}' does not exist\n"),
        "An error occurred (RequestLimitExceeded) when calling the DescribeInstances operation: slow\n".to_string(),
    ] {
        let (sb, script) = fixture(&[ok(ID)], Some(""), &[], false);
        std::fs::write(sb.path().join(format!("statefail.{ID}.1")), first).unwrap();
        let (rc, out) = sb.run(&script, "");
        let log = sb.log();
        assert_eq!(rc, 0, "{out}");
        assert!(out.contains(&format!("launched {ID}\n")), "{out}");
        assert_eq!(count(&log, "--instance-ids"), 2, "{log}");
        assert_eq!(run_tokens(&log).len(), 1, "a retry is not a new launch");
    }
}

#[test]
fn an_unknown_state_is_an_error_without_retries() {
    for weird in ["runningx", "Running", "unknown"] {
        let (rc, out, log) = launch_with(&[ok(ID)], Some(""), &[(ID, weird)], false);
        assert_eq!(rc, 1, "{weird:?}: {out}");
        assert!(out.contains("instance-state-unknown"), "{weird:?}: {out}");
        assert!(!out.contains("launched"), "{weird:?}: {out}");
        assert_eq!(count(&log, "--instance-ids"), 1, "{log}");
    }
}

const NOTFOUND: &str = "An error occurred (InvalidInstanceID.NotFound) when calling the DescribeInstances operation: The instance ID 'i-0123456789abcdef0' does not exist\n";

/// The three shapes of "EC2 does not know this id": (statefail text, state text).
fn forgotten_shapes() -> [(&'static str, &'static str); 3] {
    [(NOTFOUND, ""), ("", ""), ("", "None")]
}

fn forgotten(ago: u64, shape: (&str, &str)) -> (i32, String, String) {
    let (sb, script) = fixture(&[aged(ID, ago), ok(ID2)], Some(""), &[(ID, shape.1)], false);
    if !shape.0.is_empty() {
        std::fs::write(sb.path().join(format!("statefail.{ID}")), shape.0).unwrap();
    }
    let (rc, out) = sb.run(&script, "");
    let naps = std::fs::read_to_string(sb.path().join("calls.log.sleep")).unwrap_or_default();
    (rc, out, format!("{}\nNAPS {}", sb.log(), naps.len()))
}

#[test]
fn a_forgotten_instance_with_an_old_launch_time_chains_and_relaunches() {
    for ago in [16 * 60 + 1, 3 * 3600] {
        for shape in forgotten_shapes() {
            let (rc, out, log) = forgotten(ago, shape);
            assert_eq!(rc, 0, "{ago} {shape:?}: {out}");
            assert_eq!(
                out.trim_end().lines().last(),
                Some(format!("launched {ID2}").as_str()),
                "{ago} {shape:?}: {out}"
            );
            let t = run_tokens(&log);
            assert_eq!(t.len(), 2, "exactly two launches: {log}");
            assert_eq!(t[1], next_token(&t[0], ID), "{log}");
            assert_eq!(
                count(&log, "NAPS 0"),
                1,
                "a conclusive answer is not retried"
            );
        }
    }
}

#[test]
fn a_not_found_for_a_fresh_launch_does_not_chain_and_names_the_id() {
    for ago in [1, 60, 14 * 60] {
        for shape in forgotten_shapes() {
            let (rc, out, log) = forgotten(ago, shape);
            assert_ne!(rc, 0, "{ago} {shape:?}: {out}");
            assert!(
                out.contains(&format!("REFUSED: instance-state-failed {ID}")),
                "{out}"
            );
            assert!(!out.contains("launched"), "{out}");
            assert_eq!(run_tokens(&log).len(), 1, "no second launch: {log}");
            assert_eq!(count(&log, "--instance-ids"), 3, "{log}");
        }
    }
}

#[test]
fn only_the_exact_not_found_error_counts_as_forgotten() {
    for e in [
        "An error occurred (RequestLimitExceeded) when calling the DescribeInstances operation: x\n",
        "An error occurred (InvalidInstanceID.Malformed) when calling the DescribeInstances operation: x\n",
        "An error occurred (InvalidInstanceID.NotFoundX) when calling the DescribeInstances operation: x\n",
        "InvalidInstanceID.NotFound\n",
        "ExpiredToken\n",
    ] {
        let (rc, out, log) = forgotten(3 * 3600, (e, ""));
        assert_eq!(rc, 1, "{e}: {out}");
        assert!(out.contains("instance-state-failed"), "{e}: {out}");
        assert_eq!(run_tokens(&log).len(), 1, "{e}: never a second launch: {log}");
    }
}

#[test]
fn a_malformed_launch_time_or_reply_is_refused_without_a_state_read() {
    let ok_t = iso(60);
    let day = ok_t.split('T').next().unwrap().to_string();
    let bad = [
        String::new(),
        "garbage".to_string(),
        "None".to_string(),
        day.clone(),
        ok_t.replace('T', " "),
        ok_t.replace("+00:00", ""),
        ok_t.replace("+00:00", "+01:00"),
        ok_t.replace("+00:00", "-00:00"),
        ok_t.replace("+00:00", ".Z"),
        ok_t.replace("+00:00", ".12x4Z"),
        ok_t.replacen("-", "/", 1),
        format!("{}-99T00:00:00Z", &day[..7]),
        format!("{day}T25:61:61Z"),
        format!("{ok_t}; rm -rf x"),
    ];
    for t in bad {
        let reply = format!("{ID}\t{t}\n");
        let r: Run = (0, Box::leak(reply.into_boxed_str()), "");
        let (rc, out, log) = launch_with(&[r], Some(""), &[], false);
        assert_eq!(rc, 1, "{t:?}: {out}");
        assert!(out.contains("launch-output-malformed"), "{t:?}: {out}");
        assert!(!out.contains("launched"), "{t:?}: {out}");
        assert_eq!(count(&log, "--instance-ids"), 0, "{t:?}: {log}");
    }
    // the old id-only reply is refused the same way
    let r: Run = (0, Box::leak(format!("{ID}\n").into_boxed_str()), "");
    let (rc, out, _) = launch_with(&[r], Some(""), &[], false);
    assert_eq!(rc, 1, "{out}");
    // but the box is adopted by its token when it exists
    let r: Run = (0, Box::leak(format!("{ID}\tnope\n").into_boxed_str()), "");
    let (rc, out, _) = launch_with(&[r], Some(ID), &[], false);
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains(&format!("adopted {ID}")), "{out}");
}

#[test]
fn the_accepted_launch_time_forms_are_launched() {
    let t = iso(60);
    for form in [
        t.clone(),
        t.replace("+00:00", "Z"),
        t.replace("+00:00", ".123456Z"),
        t.replace("+00:00", ".000+00:00"),
    ] {
        let r: Run = (0, Box::leak(format!("{ID}\t{form}\n").into_boxed_str()), "");
        let (rc, out, _) = launch_with(&[r], Some(""), &[], false);
        assert_eq!(rc, 0, "{form}: {out}");
        assert!(out.contains(&format!("launched {ID}\n")), "{form}: {out}");
    }
}

#[test]
fn adoption_uses_the_current_chain_token_and_a_rerun_walks_the_same_chain() {
    // generation 1 is dead; generation 2's launch is lost (rate limit) but its box exists
    let t1 = tok(&res());
    let t2 = next_token(&t1, ID);
    let lost: Run = (
        255,
        "",
        Box::leak(err("RequestLimitExceeded").into_boxed_str()),
    );
    let (sb, script) = fixture(&[ok(ID), lost], Some(""), &[(ID, "terminated")], false);
    std::fs::write(sb.path().join(format!("tok.{t2}")), ID2).unwrap();
    let (rc, out) = sb.run(&script, "");
    let log = sb.log();
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains(&format!("adopted {ID2}\n")), "{out}");
    assert!(log.contains(&format!("client-token,Values={t2}")), "{log}");
    assert!(!log.contains(&format!("client-token,Values={t1}")), "{log}");
    assert_eq!(run_tokens(&log).len(), 2, "{log}");
    // a table keyed by token: a rerun walks the same chain to the same live id
    let (sb, script) = fixture(&[ok(ID)], Some(""), &[(ID, "terminated")], false);
    let w = |n: String, c: String| std::fs::write(sb.path().join(n), c).unwrap();
    w(format!("rt.{t1}.out"), format!("{ID}\t{}\n", iso(3 * 3600)));
    w(format!("rt.{t1}.rc"), "0".into());
    w(format!("rt.{t1}.err"), String::new());
    w(format!("rt.{t2}.out"), format!("{ID2}\t{}\n", iso(60)));
    w(format!("rt.{t2}.rc"), "0".into());
    w(format!("rt.{t2}.err"), String::new());
    for round in 1..=2 {
        let (rc, out) = sb.run(&script, "");
        let log = sb.log();
        assert_eq!(rc, 0, "{out}");
        assert!(out.contains(&format!("launched {ID2}\n")), "{out}");
        let t = run_tokens(&log);
        assert_eq!(t.len(), 2 * round, "no extra launch on a rerun: {log}");
        assert_eq!(t[2 * round - 2..], [t1.clone(), t2.clone()], "{log}");
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
    let changed: [Change; 6] = [
        ("ami pin", |r| r.cloud.ami = Some("ami-0123abcd".into())),
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

#[test]
fn labels_are_hashed_sorted_and_the_default_ami_reference_is_in_the_token() {
    let mut a = res();
    a.tags = vec!["b=2".into(), "a=1".into()];
    let mut b = res();
    b.tags = vec!["a=1".into(), "b=2".into()];
    let (ia, ib) = (
        token_input(&spec_of(&a).unwrap()),
        token_input(&spec_of(&b).unwrap()),
    );
    assert_eq!(ia, ib, "declaration order does not matter");
    assert!(ia.ends_with("a=1\u{1f}b=2"), "{ia:?}");
    assert!(
        ia.contains("resolve:ssm:"),
        "the default AMI reference: {ia}"
    );
    assert_ne!(token_on(&ia, 1), token_on(&ia, 2));
}

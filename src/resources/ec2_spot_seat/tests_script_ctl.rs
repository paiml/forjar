//! Controller-phase runtime tests: `teardown` and `register` run for real under `sh`
//! against the fake `aws`/`gh` of [`tests_script`](super::tests_script).

use super::tests_script::{jits, Box_};
use super::*;

#[test]
fn teardown_runs_ship_cancel_terminate_and_read_back_in_order() {
    let b = Box_::new();
    std::fs::write(b.p("live"), "i-1\n").unwrap();
    std::fs::write(b.p("spotreqs"), "sir-1\n").unwrap();
    let (rc, out) = b.run(&apply_script(&b.res("teardown")));
    assert_eq!(rc, 0, "{out}");
    for line in [
        "evidence-shipped:i-1",
        "cancelled:sir-1",
        "terminated:i-1",
        "state shows 0",
    ] {
        assert!(out.contains(line), "missing {line}: {out}");
    }
    let log = b.log();
    let at = |needle: &str| {
        log.find(needle)
            .unwrap_or_else(|| panic!("missing `{needle}` in log:\n{log}"))
    };
    let ship = at("aws ssm send-command");
    let wait = at("aws ssm wait command-executed");
    let cancel = at("aws ec2 cancel-spot-instance-requests");
    let term = at("aws ec2 terminate-instances");
    assert!(ship < wait && wait < cancel && cancel < term, "{log}");
    // read-back runs after the terminate
    assert!(
        log.rfind("aws ec2 describe-instances").unwrap() > term,
        "{log}"
    );
    // every box and request read is filtered to this owner's tag
    let owner = "Name=tag:Owner,Values=forjar-spot";
    let describes: Vec<&str> = log
        .lines()
        .filter(|l| l.starts_with("aws ec2 describe-instances"))
        .collect();
    assert!(!describes.is_empty(), "{log}");
    assert!(describes.iter().all(|l| l.contains(owner)), "{log}");
    let reqs = log
        .lines()
        .find(|l| l.starts_with("aws ec2 describe-spot-instance-requests"))
        .unwrap_or_else(|| panic!("no spot request read in:\n{log}"));
    assert!(reqs.contains(owner), "{reqs}");
}

/// The `aws ec2 run-instances` line of the log, split into arguments.
fn run_args(log: &str) -> Vec<String> {
    let line = log
        .lines()
        .find(|l| l.starts_with("aws ec2 run-instances"))
        .unwrap_or_else(|| panic!("no run-instances in:\n{log}"));
    line.split(' ').map(str::to_string).collect()
}

fn value_after(args: &[String], flag: &str) -> String {
    let i = args
        .iter()
        .position(|a| a == flag)
        .unwrap_or_else(|| panic!("no {flag} in {args:?}"));
    args[i + 1].clone()
}

#[test]
fn launch_run_instances_args_pin_count_market_shutdown_and_tags() {
    let b = Box_::new();
    let (rc, out) = b.run(&apply_script(&b.res("launch")));
    assert_eq!(rc, 0, "{out}");
    let log = b.log();
    let a = run_args(&log);
    assert_eq!(value_after(&a, "--count"), "1");
    assert_eq!(value_after(&a, "--image-id"), "ami-0123456789abcdef0");
    assert_eq!(value_after(&a, "--instance-type"), "c6a.4xlarge");
    assert_eq!(value_after(&a, "--subnet-id"), "subnet-0abc1234");
    assert_eq!(
        value_after(&a, "--instance-initiated-shutdown-behavior"),
        "terminate"
    );
    assert!(
        value_after(&a, "--client-token").starts_with("fj-"),
        "{a:?}"
    );
    let market = value_after(&a, "--instance-market-options");
    assert!(market.contains("MarketType=spot"), "{market}");
    assert!(market.contains("MaxPrice=0.5,"), "{market}");
    assert!(market.contains("SpotInstanceType=one-time"), "{market}");
    assert!(
        market.contains("InstanceInterruptionBehavior=terminate}"),
        "{market}"
    );
    // the spot request is tagged in the same tag-specifications list as the instance
    let at = a.iter().position(|x| x == "--tag-specifications").unwrap();
    assert!(a[at + 1].starts_with("ResourceType=instance,"), "{a:?}");
    assert_eq!(
        a[at + 2],
        "ResourceType=spot-instances-request,Tags=[{Key=Owner,Value=forjar-spot}]"
    );
    // the instance read and the price read are owner-filtered too
    assert!(log
        .lines()
        .all(|l| !l.starts_with("aws ec2 describe-instances")
            || l.contains("Name=tag:Owner,Values=forjar-spot")
            || l.contains("client-token")));
}

#[test]
fn teardown_fails_when_terminate_leaves_the_box_alive() {
    let b = Box_::new();
    std::fs::write(b.p("live"), "i-1\n").unwrap();
    b.touch("terminatenoop");
    let (rc, out) = b.run(&apply_script(&b.res("teardown")));
    assert_ne!(rc, 0, "{out}");
    assert!(
        out.contains("state shows 1 boxes after teardown, want 0"),
        "{out}"
    );
    assert!(!out.contains("state shows 0"), "{out}");
}

#[test]
fn teardown_failed_ssm_wait_terminates_nothing() {
    let b = Box_::new();
    std::fs::write(b.p("live"), "i-1\n").unwrap();
    std::fs::write(b.p("spotreqs"), "sir-1\n").unwrap();
    b.touch("ssmwaitfail");
    let (rc, out) = b.run(&apply_script(&b.res("teardown")));
    assert_ne!(rc, 0, "{out}");
    assert!(
        out.contains("evidence-ship-failed:i-1; nothing terminated"),
        "{out}"
    );
    let log = b.log();
    assert!(log.contains("aws ssm wait command-executed"), "{log}");
    assert!(!log.contains("cancel-spot-instance-requests"), "{log}");
    assert!(!log.contains("terminate-instances"), "{log}");
    assert!(b.p("live").exists(), "the box must still be there");
}

#[test]
fn launch_aborts_when_the_price_read_fails_and_never_launches() {
    let b = Box_::new();
    // one live box: the spend is needed, so an unreadable price must not read as zero
    std::fs::write(b.p("live"), "i-1\n").unwrap();
    std::fs::write(b.p("prices"), "0.1\n").unwrap();
    b.touch("pricefail");
    let (rc, out) = b.run(&apply_script(&b.res("launch")));
    assert_ne!(rc, 0, "{out}");
    assert!(!b.log().contains("run-instances"), "{}", b.log());
}

#[test]
fn teardown_failed_evidence_ship_does_not_cancel_or_terminate() {
    let b = Box_::new();
    b.touch("shipfail");
    std::fs::write(b.p("live"), "i-1\n").unwrap();
    std::fs::write(b.p("spotreqs"), "sir-1\n").unwrap();
    let (rc, out) = b.run(&apply_script(&b.res("teardown")));
    assert_ne!(rc, 0, "{out}");
    assert!(
        out.contains("evidence-ship-failed:i-1; nothing terminated"),
        "{out}"
    );
    let log = b.log();
    assert!(!log.contains("cancel-spot-instance-requests"), "{log}");
    assert!(!log.contains("terminate-instances"), "{log}");
    assert!(b.p("live").exists(), "the box must still be there");
}

#[test]
fn register_script_purges_only_offline_idle_seats_of_this_owner() {
    let b = Box_::new();
    std::fs::write(
        b.p("runners.tsv"),
        "1\tseat-forjar-spot-01\toffline\tfalse\n\
         2\tseat-forjar-spot-02\toffline\ttrue\n\
         3\tother-box\toffline\tfalse\n\
         4\tseat-forjar-spot-03\tonline\tfalse\n",
    )
    .unwrap();
    jits(&b);
    let (rc, out) = b.run(&apply_script(&b.res("register")));
    assert_eq!(rc, 0, "{out}");
    // only the stale offline idle seat (id 1) is deleted
    assert_eq!(std::fs::read_to_string(b.p("deleted")).unwrap(), "1\n");
    assert!(out.contains("deregistered:seat-forjar-spot-01"), "{out}");
    assert!(!out.contains("deregistered:seat-forjar-spot-02"), "{out}");
    // the purge DELETE is logged before the first JIT mint
    let log = b.log();
    let del = log.find("-X DELETE").unwrap_or_else(|| panic!("{log}"));
    let mint = log
        .find("generate-jitconfig")
        .unwrap_or_else(|| panic!("{log}"));
    assert!(del < mint, "purge must precede mint:\n{log}");
    // seat 01 was purged, so it is minted again; seat 02 has no online runner either
    assert!(out.contains("jit-minted:seat-forjar-spot-01"), "{out}");
    assert!(out.contains("jit-minted:seat-forjar-spot-02"), "{out}");
    assert!(std::fs::read_to_string(b.p("jit/seat-forjar-spot-01.jit"))
        .unwrap()
        .contains("jit:seat-forjar-spot-01"));
}

#[test]
fn register_script_keeps_an_online_seat_and_mints_nothing_for_it() {
    let b = Box_::new();
    std::fs::write(
        b.p("runners.tsv"),
        "9\tseat-forjar-spot-01\tonline\tfalse\n",
    )
    .unwrap();
    jits(&b);
    let (rc, out) = b.run(&apply_script(&b.res("register")));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("registered:seat-forjar-spot-01"), "{out}");
    assert!(!out.contains("jit-minted:seat-forjar-spot-01"), "{out}");
    assert!(!b.p("deleted").exists(), "an online seat is never deleted");
}

#[test]
fn teardown_read_back_failure_exits_non_zero_and_never_reports_zero() {
    let b = Box_::new();
    std::fs::write(b.p("live"), "i-1\n").unwrap();
    b.touch("readbackfail");
    let (rc, out) = b.run(&apply_script(&b.res("teardown")));
    assert_ne!(rc, 0, "{out}");
    assert!(out.contains("read-back-failed"), "{out}");
    assert!(!out.contains("state shows 0"), "{out}");
    assert!(
        b.log().contains("aws ec2 terminate-instances"),
        "{}",
        b.log()
    );
}

#[test]
fn launch_adopt_describe_is_owner_filtered() {
    let b = Box_::new();
    let (rc, out) = b.run(&apply_script(&b.res("launch")));
    assert_eq!(rc, 0, "{out}");
    let log = b.log();
    let adopt = log
        .lines()
        .find(|l| l.starts_with("aws ec2 describe-instances") && l.contains("Name=client-token,"))
        .unwrap_or_else(|| panic!("no adopt describe in:\n{log}"));
    assert!(
        adopt.contains("Name=tag:Owner,Values=forjar-spot"),
        "{adopt}"
    );
}

#[test]
fn launch_instance_tag_spec_carries_owner_name_ttl_and_price() {
    let b = Box_::new();
    let (rc, out) = b.run(&apply_script(&b.res("launch")));
    assert_eq!(rc, 0, "{out}");
    let a = run_args(&b.log());
    let at = a.iter().position(|x| x == "--tag-specifications").unwrap();
    let inst = &a[at + 1];
    assert!(inst.starts_with("ResourceType=instance,"), "{inst}");
    for tag in [
        "Key=Owner,Value=forjar-spot",
        "Key=Name,Value=seat-forjar-spot",
        "Key=TtlMin,Value=120",
        "Key=Price,Value=0.5",
    ] {
        assert!(inst.contains(tag), "instance tags lack {tag}: {inst}");
    }
}

#[test]
fn launch_refuses_when_a_live_box_has_no_readable_price() {
    for prices in ["", "None\n", "abc\n", "0.1\t0.1\n"] {
        let b = Box_::new();
        std::fs::write(b.p("live"), "i-1\n").unwrap();
        std::fs::write(b.p("prices"), prices).unwrap();
        let (rc, out) = b.run(&apply_script(&b.res("launch")));
        assert_eq!(rc, 1, "prices {prices:?}: {out}");
        assert!(out.contains("price-unmeasured (not measured)"), "{out}");
        assert!(!b.log().contains("run-instances"), "{}", b.log());
    }
}

#[test]
fn launch_refuses_when_the_live_box_read_fails() {
    let b = Box_::new();
    b.touch("livefail");
    let (rc, out) = b.run(&apply_script(&b.res("launch")));
    assert_ne!(rc, 0, "{out}");
    assert!(out.contains("live-box-read-failed (not measured)"), "{out}");
    assert!(!b.log().contains("run-instances"), "{}", b.log());
}

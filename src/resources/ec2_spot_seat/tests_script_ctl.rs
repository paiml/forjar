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

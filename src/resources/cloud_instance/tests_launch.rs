//! #1136 / #1137: the launch result. Capacity errors exit 75, every other
//! error keeps its code, and `apply` ends on `launched i-<id>` or `adopted i-<id>`.

use super::tests::{gated, Sandbox};
use super::tests_b1::count;
use super::*;

const GOOD_ID: &str = "i-0a1b2c3d4e5f60718";

fn aws_err(code: &str) -> String {
    format!("\nAn error occurred ({code}) when calling the RunInstances operation: nope\n")
}

fn launch(sb: &Sandbox) -> (i32, String) {
    sb.open_gate();
    let script = apply_script(&gated(sb.path()));
    crate::core::purifier::validate_script(&script).expect("purifier accepts the launch script");
    sb.run(&script, "")
}

#[test]
fn capacity_codes_exit_75_other_errors_keep_their_code() {
    for code in [
        "InsufficientInstanceCapacity",
        "SpotMaxPriceTooLow",
        "MaxSpotInstanceCountExceeded",
        "Unsupported",
    ] {
        let sb = Sandbox::new();
        sb.fail_run(&aws_err(code), 254);
        let (rc, out) = launch(&sb);
        assert_eq!(rc, 75, "{code}: {out}");
        assert!(out.contains(code), "the aws error is still shown: {out}");
    }
    let keep: [(&str, i32); 7] = [
        (&aws_err("UnauthorizedOperation"), 254),
        (&aws_err("InsufficientInstanceCapacityX"), 253),
        (&aws_err("NotUnsupported"), 252),
        (&aws_err("UnsupportedOperation"), 254),
        (
            "Unsupported InsufficientInstanceCapacity SpotMaxPriceTooLow\n",
            2,
        ),
        ("", 254),
        (&aws_err("insufficientinstancecapacity"), 1),
    ];
    for (stderr, want) in keep {
        let sb = Sandbox::new();
        sb.fail_run(stderr, want);
        let (rc, out) = launch(&sb);
        assert_eq!(rc, want, "{stderr:?}: {out}");
    }
}

#[test]
fn apply_last_line_is_launched_id() {
    let sb = Sandbox::new();
    let (rc, out) = launch(&sb);
    assert_eq!(rc, 0, "{out}");
    assert_eq!(
        out.trim_end().lines().last(),
        Some(format!("launched {GOOD_ID}").as_str())
    );
    let log = sb.log();
    assert!(
        log.contains("--query Instances[0].InstanceId --output text"),
        "{log}"
    );
    for ok in ["i-0a1b2c3d", "i-0a1b2c3d4e5f60718"] {
        let sb = Sandbox::new();
        sb.set_run_output(&format!("{ok}\n"));
        let (rc, out) = launch(&sb);
        assert_eq!(rc, 0, "{out}");
        assert_eq!(
            out.trim_end().lines().last(),
            Some(format!("launched {ok}").as_str())
        );
    }
}

#[test]
fn a_malformed_launch_id_fails_and_is_never_printed_as_success() {
    for bad in [
        "",
        "None\n",
        "i-xyz\n",
        "i-123\n",
        "i-0a1b2c3d4e\n",
        "i-0a1b2c3d4e5f607189\n",
        "I-0a1b2c3d\n",
        "i-0A1B2C3D\n",
        "i-0a1b2c3d i-0a1b2c3e\n",
        "i-0a1b2c3d\ni-0a1b2c3e\n",
        "i-*\n",
        "launched i-0a1b2c3d\n",
    ] {
        let sb = Sandbox::new();
        sb.set_run_output(bad);
        let (rc, out) = launch(&sb);
        assert_ne!(rc, 0, "{bad:?}: {out}");
        assert!(!out.contains("launched i-"), "{bad:?}: {out}");
    }
}

#[test]
fn a_live_owned_box_is_adopted_on_the_last_line() {
    let sb = Sandbox::new();
    sb.open_gate();
    let fx = format!(
        "i-00a1b2c3 {} {} sir-0a0a0a0a\n",
        super::tests_b1::KEY_A,
        super::tests_b1::OWN_A
    );
    let (rc, out) = sb.run_with(&apply_script(&gated(sb.path())), "", Some(&fx));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("present i-00a1b2c3"), "{out}");
    assert_eq!(out.trim_end().lines().last(), Some("adopted i-00a1b2c3"));
    assert_eq!(count(&sb.log(), "run-instances"), 0);
}

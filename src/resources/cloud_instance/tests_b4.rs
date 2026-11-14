//! FJ-861 round 4: ownership across an image change, and small hardening.

use super::tests::{gated, Sandbox};
use super::tests_b1::{absent, count, KEY_A, OWN_A};
use super::*;

// ---- round 4: ownership across an image change, and small hardening ----

const OTHER_DIGEST: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

fn with_image(mut r: Resource, digest: &str) -> Resource {
    r.image = Some(format!("registry.example/img@sha256:{digest}"));
    r
}

fn old_image_box() -> String {
    format!("i-00a1b2c3 {KEY_A} {OWN_A} sir-0a\n")
}

#[test]
fn an_image_change_with_a_live_owned_instance_refuses_and_never_launches() {
    let sb = Sandbox::new();
    sb.open_gate();
    let r = with_image(gated(sb.path()), OTHER_DIGEST);
    assert_ne!(key_of(&spec_of(&r).unwrap()), KEY_A);
    let (rc, out) = sb.run_with(&apply_script(&r), "", Some(&old_image_box()));
    assert_ne!(rc, 0, "{out}");
    assert!(out.contains("image changed while instance"), "{out}");
    assert!(out.contains("i-00a1b2c3"), "{out}");
    assert!(out.contains("state: absent"), "{out}");
    assert_eq!(count(&sb.log(), "run-instances"), 0, "{}", sb.log());
}

#[test]
fn the_same_image_with_a_live_owned_instance_is_present_and_never_launches() {
    let sb = Sandbox::new();
    sb.open_gate();
    let (rc, out) = sb.run_with(&apply_script(&gated(sb.path())), "", Some(&old_image_box()));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("present i-00a1b2c3"), "{out}");
    assert_eq!(count(&sb.log(), "run-instances"), 0, "{}", sb.log());
}

#[test]
fn nothing_owned_launches_exactly_once() {
    let sb = Sandbox::new();
    sb.open_gate();
    // another resource's box is alive; it is not ours
    let fx = "i-00b2c3d4 ci-box-b@eu-south-2@0123456789ab ci-box-b@eu-south-2 sir-0o\n";
    let (rc, out) = sb.run_with(&apply_script(&gated(sb.path())), "", Some(fx));
    assert_eq!(rc, 0, "{out}");
    assert_eq!(count(&sb.log(), "run-instances"), 1, "{}", sb.log());
}

#[test]
fn an_owned_box_with_a_different_key_beside_a_same_key_box_still_refuses() {
    let sb = Sandbox::new();
    sb.open_gate();
    let fx = format!(
        "{}i-00b2c3d4 ci-box-a@eu-south-2@ffffffffffff {OWN_A} sir-0b\n",
        old_image_box()
    );
    let (rc, out) = sb.run_with(&apply_script(&gated(sb.path())), "", Some(&fx));
    assert_ne!(rc, 0, "{out}");
    assert!(
        out.contains("i-00b2c3d4") && !out.contains("alive i-00a1b2c3"),
        "{out}"
    );
    assert_eq!(count(&sb.log(), "run-instances"), 0);
}

#[test]
fn state_query_reports_by_owner_across_an_image_change() {
    let sb = Sandbox::new();
    let r = with_image(gated(sb.path()), OTHER_DIGEST);
    let (rc, out) = sb.run_with(&state_query_script(&r), "", Some(&old_image_box()));
    assert_eq!(rc, 0, "{out}");
    assert_eq!(out.trim(), "present i-00a1b2c3");
    assert!(sb
        .log()
        .contains(&format!("Name=tag:forjar-owner,Values={OWN_A}")));
    // absent state reads the same way
    let (rc, out) = sb.run_with(&state_query_script(&absent(r)), "", Some(&old_image_box()));
    assert_eq!(rc, 0, "{out}");
    assert_eq!(out.trim(), "present i-00a1b2c3");
    let (rc, out) = sb.run_with(&state_query_script(&gated(sb.path())), "", Some(""));
    assert_eq!(rc, 0, "{out}");
    assert_eq!(out.trim(), "absent");
}

#[test]
fn a_failed_owner_lookup_in_apply_never_launches() {
    let sb = Sandbox::new();
    sb.open_gate();
    sb.fail_describe();
    let (rc, _) = sb.run(&apply_script(&gated(sb.path())), "");
    assert_ne!(rc, 0);
    assert_eq!(count(&sb.log(), "run-instances"), 0);
}

#[test]
fn prices_must_be_finite_plain_decimals() {
    for bad in [
        ".5", "5.", ".", "0", "0.0", "1.2.3", "-1", "1e3", "", "+1", " 1",
    ] {
        assert!(!is_positive_decimal(bad), "{bad:?}");
    }
    let huge = "9".repeat(400);
    assert!(!is_positive_decimal(&huge));
    assert!(!is_positive_decimal(&"1".repeat(13)));
    for ok in ["1", "0.2", "0.20", "12.5", "100"] {
        assert!(is_positive_decimal(ok), "{ok:?}");
    }
    // an overflowing price and cap can never compare as "within budget"
    let sb = Sandbox::new();
    sb.open_gate();
    let mut r = gated(sb.path());
    r.cloud.max_price = Some(huge.clone());
    r.cloud.budget_cap_usd_day = Some(huge);
    let (rc, out) = sb.run(&apply_script(&r), "");
    assert_ne!(rc, 0, "{out}");
    assert_eq!(sb.log(), "");
}

#[test]
fn only_whole_instance_ids_pass_the_lookup() {
    for bad in [
        "i-",
        "i-abc",
        "i-0a",
        "i-0123abcdXYZ",
        "i-0123abcd!",
        "i-0123456789abcdef01",
        "I-0123abcd",
        "-r",
    ] {
        let sb = Sandbox::new();
        let (rc, out) = sb.run(&state_query_script(&gated(sb.path())), bad);
        assert_ne!(rc, 0, "{bad}: {out}");
        assert!(out.contains("not an instance id"), "{bad}: {out}");
    }
    for ok in [
        "i-0123abcd",
        "i-0123456789abcdef0",
        "i-0123abcd\ti-4567ef01",
    ] {
        let sb = Sandbox::new();
        let (rc, out) = sb.run(&state_query_script(&gated(sb.path())), ok);
        assert_eq!(rc, 0, "{ok}: {out}");
    }
}

#[test]
fn destroy_passes_the_validated_ids_as_separate_arguments() {
    let sb = Sandbox::new();
    let fx = format!("{}i-00b2c3d4 {KEY_A} {OWN_A} sir-0b\n", old_image_box());
    let (rc, out) = sb.run_with(&apply_script(&absent(gated(sb.path()))), "", Some(&fx));
    assert_eq!(rc, 0, "{out}");
    let log = sb.log();
    let term = log
        .lines()
        .find(|l| l.contains("terminate-instances"))
        .unwrap();
    assert!(
        term.ends_with("--instance-ids i-00a1b2c3 i-00b2c3d4"),
        "{term}"
    );
}

#[test]
fn name_and_region_cannot_blur_into_each_other() {
    let sb = Sandbox::new();
    let a = {
        let mut r = gated(sb.path());
        r.name = Some("box-us".into());
        r.cloud.region = Some("east-1".into());
        spec_of(&r).unwrap()
    };
    let b = {
        let mut r = gated(sb.path());
        r.name = Some("box".into());
        r.cloud.region = Some("us-east-1".into());
        spec_of(&r).unwrap()
    };
    assert_ne!(owner_of(&a), owner_of(&b));
    assert_ne!(key_of(&a), key_of(&b));
    assert!(!is_ident("a@b", 63) && !is_word("a@b", "-"));
}

#[test]
fn gate_and_kill_switch_paths_reject_control_characters() {
    let sb = Sandbox::new();
    sb.open_gate();
    for evil in ["/tmp/x\ny", "/tmp/x\ty", "/tmp/x\u{7f}", "/tmp/x\r"] {
        let mut r = gated(sb.path());
        r.cloud.kill_switch = Some(evil.into());
        let (rc, out) = sb.run(&apply_script(&r), "");
        assert_ne!(rc, 0, "{evil:?}: {out}");
        assert!(out.contains("kill_switch"), "{out}");
        let mut r = gated(sb.path());
        r.cloud.parity_gate = Some(evil.into());
        let (rc, out) = sb.run(&apply_script(&r), "");
        assert_ne!(rc, 0, "{evil:?}: {out}");
        assert!(out.contains("parity_gate"), "{out}");
    }
    assert_eq!(sb.log(), "");
}

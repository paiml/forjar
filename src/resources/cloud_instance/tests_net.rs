//! #1033-#1034 falsifiers: subnet and security groups, asserted on the fake aws call log.

use super::tests::{gated, res, Sandbox};
use super::*;

const SUBNET: &str = "subnet-0123abcd";
const SG1: &str = "sg-0123456789abcdef0";
const SG2: &str = "sg-89abcdef";

fn launch_log(r: &Resource, fixture: Option<&str>) -> (i32, String, String) {
    let sb = Sandbox::new();
    sb.open_gate();
    let mut r = r.clone();
    r.cloud.kill_switch = gated(sb.path()).cloud.kill_switch;
    r.cloud.parity_gate = gated(sb.path()).cloud.parity_gate;
    r.cloud.budget_cap_usd_day = Some("5".into());
    let (rc, out) = sb.run_with(&apply_script(&r), "", Some(fixture.unwrap_or("")));
    (rc, out, sb.log())
}

fn run_line(log: &str) -> String {
    log.lines()
        .find(|l| l.contains("run-instances"))
        .unwrap_or("")
        .to_string()
}

#[test]
fn nothing_declared_leaves_the_command_line_alone() {
    let (rc, out, log) = launch_log(&res(), None);
    assert_eq!(rc, 0, "{out}");
    let l = run_line(&log);
    for w in ["--subnet-id", "--security-group-ids", "--placement"] {
        assert!(!l.contains(w), "{l}");
    }
    assert!(!key_of(&spec_of(&res()).unwrap()).contains("@p"));
}

#[test]
fn subnet_is_passed_as_one_quoted_argument() {
    let mut r = res();
    r.cloud.subnet_id = Some(SUBNET.into());
    let (rc, out, log) = launch_log(&r, None);
    assert_eq!(rc, 0, "{out}");
    assert!(
        run_line(&log).contains(&format!("--subnet-id {SUBNET} ")),
        "{log}"
    );
    let script = apply_script(&{
        let mut g = gated(std::path::Path::new("/t"));
        g.cloud.subnet_id = Some(SUBNET.into());
        g
    });
    assert!(
        script.contains(&format!("--subnet-id '{SUBNET}'")),
        "{script}"
    );
}

#[test]
fn only_whole_subnet_ids_pass() {
    for bad in [
        "",
        "subnet-",
        "subnet-ABCDEF12",
        "subnet-0123abc",
        "subnet-0123abcd;x",
        "subnet-0123abcd ",
        "xsubnet-0123abcd",
        "subnet-0123abcdef",
    ] {
        let mut r = res();
        r.cloud.subnet_id = Some(bad.into());
        assert!(spec_of(&r).is_err(), "{bad:?}");
        let (rc, _, log) = launch_log(&r, None);
        assert_ne!(rc, 0, "{bad:?}");
        assert_eq!(log, "", "nothing may reach the provider for {bad:?}");
    }
}

#[test]
fn security_groups_are_each_passed_quoted() {
    let mut r = res();
    r.cloud.security_group_ids = Some(vec![SG1.into(), SG2.into()]);
    let (rc, out, log) = launch_log(&r, None);
    assert_eq!(rc, 0, "{out}");
    assert!(
        run_line(&log).contains(&format!("--security-group-ids {SG1} {SG2} ")),
        "{log}"
    );
}

#[test]
fn only_whole_distinct_security_group_ids_pass() {
    let six: Vec<String> = (0..6).map(|i| format!("sg-0000000{i}")).collect();
    let cases: Vec<Vec<String>> = vec![
        vec![],
        vec![SG1.into(), SG1.into()],
        vec!["sg-0123456".into()],
        vec!["sg-0123456G".into()],
        vec!["sg-01234567;x".into()],
        vec![SG1.into(), "$(id)".into()],
        six,
    ];
    for bad in cases {
        let mut r = res();
        r.cloud.security_group_ids = Some(bad.clone());
        assert!(spec_of(&r).is_err(), "{bad:?}");
    }
    let mut r = res();
    r.cloud.security_group_ids = Some((0..5).map(|i| format!("sg-0000000{i}")).collect());
    assert!(spec_of(&r).is_ok());
}

#[test]
fn a_changed_placement_on_a_live_instance_refuses_and_never_launches() {
    let before = res();
    let key = key_of(&spec_of(&before).unwrap());
    let own = owner_of(&spec_of(&before).unwrap());
    let mut moved = res();
    moved.cloud.subnet_id = Some(SUBNET.into());
    assert_ne!(key, key_of(&spec_of(&moved).unwrap()));
    let (rc, out, log) = launch_log(&moved, Some(&format!("i-0aaaaaaa {key} {own} sir-0a\n")));
    assert_ne!(rc, 0, "{out}");
    assert!(out.contains("REFUSED"), "{out}");
    assert_eq!(log.matches("run-instances").count(), 0, "{log}");
    let mut same = moved.clone();
    same.cloud.security_group_ids = None;
    let k2 = key_of(&spec_of(&same).unwrap());
    let (rc, out, log) = launch_log(&same, Some(&format!("i-0aaaaaaa {k2} {own} sir-0a\n")));
    assert_eq!(rc, 0, "{out}");
    assert_eq!(log.matches("run-instances").count(), 0, "{log}");
}

#[test]
fn security_group_order_does_not_change_the_key() {
    let (mut a, mut b) = (res(), res());
    a.cloud.security_group_ids = Some(vec![SG1.into(), SG2.into()]);
    b.cloud.security_group_ids = Some(vec![SG2.into(), SG1.into()]);
    assert_eq!(key_of(&spec_of(&a).unwrap()), key_of(&spec_of(&b).unwrap()));
}

//! FJ-861 falsifiers for `cloud_instance`. The provider CLI is a PATH stub in a
//! temp sandbox; no real `aws` is ever reachable (PATH is the sandbox + /usr/bin:/bin).

use super::*;
use crate::core::types::{CloudSpec, MachineTarget, ResourceType};
use std::path::Path;
use std::process::Command;

const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn res() -> Resource {
    Resource {
        resource_type: ResourceType::CloudInstance,
        machine: MachineTarget::Single("m".into()),
        provider: Some("aws".into()),
        image: Some(format!(
            "123456789012.dkr.ecr.eu-south-2.amazonaws.com/img001@{DIGEST}"
        )),
        tags: vec!["pool=spot-rmedia-pr".into()],
        cloud: CloudSpec {
            region: Some("eu-south-2".into()),
            instance_type: Some("c7a.xlarge".into()),
            max_price: Some("0.20".into()),
            max_lifetime_min: Some(60),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// `res()` plus the three apply gates, pointing into `dir`.
fn gated(dir: &Path) -> Resource {
    let mut r = res();
    r.cloud.budget_cap_usd_day = Some("5".into());
    r.cloud.kill_switch = Some(dir.join("KILL").to_str().unwrap().into());
    r.cloud.parity_gate = Some(dir.join("gate").to_str().unwrap().into());
    r
}

struct Sandbox {
    dir: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        let stub = bin.join("aws");
        std::fs::write(
            &stub,
            "#!/bin/sh\necho \"$@\" >> \"$STUB_LOG\"\n\
             case \"$*\" in\n*describe-instances*) printf '%s' \"$STUB_IDS\";;\n\
             *run-instances*) echo i-0stub;;\nesac\n",
        )
        .unwrap();
        Command::new("chmod")
            .arg("755")
            .arg(&stub)
            .status()
            .unwrap();
        Sandbox { dir }
    }
    fn path(&self) -> &Path {
        self.dir.path()
    }
    fn open_gate(&self) {
        std::fs::write(self.path().join("gate"), "parity-gate: pass\n").unwrap();
    }
    fn log(&self) -> String {
        std::fs::read_to_string(self.path().join("calls.log")).unwrap_or_default()
    }
    /// Run `script` with only the stub `aws` reachable; `ids` is what describe returns.
    fn run(&self, script: &str, ids: &str) -> (i32, String) {
        let o = Command::new("bash")
            .arg("-c")
            .arg(script)
            .env_clear()
            .env(
                "PATH",
                format!("{}:/usr/bin:/bin", self.path().join("bin").display()),
            )
            .env("STUB_LOG", self.path().join("calls.log"))
            .env("STUB_IDS", ids)
            .output()
            .unwrap();
        let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&o.stderr));
        (o.status.code().unwrap_or(-1), s)
    }
}

#[test]
fn spec_accepts_a_complete_declaration() {
    let s = spec_of(&res()).expect("valid");
    assert_eq!(s.region, "eu-south-2");
    assert_eq!(s.max_lifetime_min, 60);
}

#[test]
fn spec_refuses_each_bad_declaration() {
    let mut cases: Vec<(&str, Resource)> = Vec::new();
    let mut r = res();
    r.provider = Some("gcp".into());
    cases.push(("provider", r));
    let mut r = res();
    r.image = Some("img001:latest".into());
    cases.push(("tag-only image", r));
    let mut r = res();
    r.image = Some("img001@sha256:abc".into());
    cases.push(("short digest", r));
    let mut r = res();
    r.cloud.region = None;
    cases.push(("region", r));
    let mut r = res();
    r.cloud.max_price = Some("0".into());
    cases.push(("zero price", r));
    let mut r = res();
    r.cloud.max_price = Some("free".into());
    cases.push(("non-numeric price", r));
    let mut r = res();
    r.cloud.max_lifetime_min = Some(0);
    cases.push(("zero lifetime", r));
    for (what, r) in cases {
        assert!(spec_of(&r).is_err(), "{what} must be refused");
    }
}

#[test]
fn plan_diff_shows_create_and_destroy_without_gates() {
    let d = plan_diff(&res());
    for want in [
        "+ cloud_instance",
        "eu-south-2",
        "c7a.xlarge",
        "0.20",
        DIGEST,
        "- destroy",
        "60",
    ] {
        assert!(d.contains(want), "missing {want:?} in:\n{d}");
    }
    assert!(d.contains("parity_gate: MISSING"), "{d}");
}

#[test]
fn plan_diff_reports_a_bad_declaration_instead_of_hiding_it() {
    let mut r = res();
    r.image = None;
    assert!(plan_diff(&r).contains("ERROR"));
}

#[test]
fn apply_refuses_when_any_gate_is_undeclared_and_never_calls_the_provider() {
    let sb = Sandbox::new();
    sb.open_gate();
    let full = gated(sb.path());
    let mut no_gate = full.clone();
    no_gate.cloud.parity_gate = None;
    let mut no_budget = full.clone();
    no_budget.cloud.budget_cap_usd_day = None;
    let mut no_kill = full;
    no_kill.cloud.kill_switch = None;
    for (name, r) in [
        ("parity_gate", no_gate),
        ("budget", no_budget),
        ("kill_switch", no_kill),
    ] {
        let (rc, out) = sb.run(&apply_script(&r), "");
        assert_ne!(rc, 0, "{name}: {out}");
        assert!(
            out.contains("REFUSED") && out.contains(name.split('_').next().unwrap()),
            "{out}"
        );
    }
    assert_eq!(sb.log(), "", "a refused apply must not reach the provider");
}

#[test]
fn apply_refuses_unless_the_gate_file_says_pass_or_when_the_kill_switch_exists() {
    let sb = Sandbox::new();
    let r = gated(sb.path());
    let (rc, out) = sb.run(&apply_script(&r), "");
    assert_ne!(rc, 0, "gate file absent: {out}");
    std::fs::write(sb.path().join("gate"), "parity-gate: fail\n").unwrap();
    let (rc, out) = sb.run(&apply_script(&r), "");
    assert_ne!(rc, 0, "gate says fail: {out}");
    sb.open_gate();
    std::fs::write(sb.path().join("KILL"), "").unwrap();
    let (rc, out) = sb.run(&apply_script(&r), "");
    assert_ne!(rc, 0, "kill switch thrown: {out}");
    assert_eq!(sb.log(), "", "no provider call on any refusal");
}

#[test]
fn apply_creates_one_bounded_spot_instance_when_every_gate_passes() {
    let sb = Sandbox::new();
    sb.open_gate();
    let (rc, out) = sb.run(&apply_script(&gated(sb.path())), "");
    assert_eq!(rc, 0, "{out}");
    let log = sb.log();
    for want in [
        "ec2 run-instances",
        "--region eu-south-2",
        "--instance-type c7a.xlarge",
        "MaxPrice=0.20",
        "terminate",
        "shutdown -h +60",
        DIGEST,
        "pool=spot-rmedia-pr",
    ] {
        assert!(log.contains(want), "missing {want:?} in:\n{log}");
    }
}

#[test]
fn apply_is_idempotent_when_the_instance_already_exists() {
    let sb = Sandbox::new();
    sb.open_gate();
    let (rc, out) = sb.run(&apply_script(&gated(sb.path())), "i-0live");
    assert_eq!(rc, 0, "{out}");
    assert!(!sb.log().contains("run-instances"), "{}", sb.log());
}

#[test]
fn check_asks_the_provider_and_fails_when_absent() {
    let sb = Sandbox::new();
    let (rc, out) = sb.run(&check_script(&res()), "i-0live");
    assert_eq!(rc, 0, "{out}");
    assert!(sb.log().contains("describe-instances"), "{}", sb.log());
    let (rc, out) = sb.run(&check_script(&res()), "");
    assert_ne!(rc, 0, "{out}");
}

#[test]
fn state_query_is_read_only() {
    let sb = Sandbox::new();
    let (_, _) = sb.run(&state_query_script(&res()), "");
    let log = sb.log();
    assert!(log.contains("describe-instances"), "{log}");
    assert!(
        !log.contains("run-instances") && !log.contains("terminate-instances"),
        "{log}"
    );
}

#[test]
fn a_bad_declaration_makes_every_script_refuse() {
    let mut r = res();
    r.image = None;
    for s in [check_script(&r), apply_script(&r), state_query_script(&r)] {
        let sb = Sandbox::new();
        let (rc, _) = sb.run(&s, "");
        assert_ne!(rc, 0);
        assert_eq!(sb.log(), "");
    }
}

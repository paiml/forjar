//! FJ-861 falsifiers for `cloud_instance`. The provider CLI is a PATH stub in a
//! temp sandbox. PATH is the sandbox `bin` ONLY (coreutils are symlinked in, `aws`
//! never is), so a missing stub is `command not found`, never a real `aws`.

use super::*;
use crate::core::types::{CloudSpec, MachineTarget, ResourceType};
use std::path::Path;
use std::process::Command;

pub(super) const DIGEST: &str =
    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

pub(super) fn res() -> Resource {
    Resource {
        resource_type: ResourceType::CloudInstance,
        name: Some("ci-box-a".into()),
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
pub(super) fn gated(dir: &Path) -> Resource {
    let mut r = res();
    r.cloud.budget_cap_usd_day = Some("5".into());
    r.cloud.kill_switch = Some(dir.join("KILL").to_str().unwrap().into());
    r.cloud.parity_gate = Some(dir.join("gate").to_str().unwrap().into());
    r
}

pub(super) struct Sandbox {
    pub(super) dir: tempfile::TempDir,
}

/// Fake `aws`. Refuses to run without STUB_LOG. With STUB_FIXTURE (lines of
/// `id key owner sir`) `describe-instances` honours the tag filter and skips
/// terminated ids, so ownership scoping is exercised, not assumed.
const STUB: &str = r#"#!/bin/sh
: "${STUB_LOG:?STUB_LOG unset: refusing to act as aws}"
echo "$@" >> "$STUB_LOG"
case "$*" in
*describe-instances*)
  if [ -e "$STUB_LOG.descfail" ]; then echo "ExpiredToken" >&2; exit 255; fi
  if [ -e "$STUB_LOG.descfail2" ] && [ "$(grep -c describe-instances "$STUB_LOG")" -ge 2 ]; then echo "ExpiredToken" >&2; exit 255; fi
  if [ -n "${STUB_FIXTURE:-}" ]; then
    want=$(printf '%s\n' "$*" | sed -n 's/.*Name=tag:\(forjar-[a-z]*\),Values=\([^ ]*\).*/\1 \2/p')
    col=1; case "$*" in *SpotInstanceRequestId*) col=4;; esac
    awk -v want="$want" -v col="$col" -v term="$STUB_LOG.term" '
      BEGIN { split(want, w, " "); while ((getline l < term) > 0) t[l] = 1 }
      { c = (w[1] == "forjar-key") ? 2 : 3
        if ($c == w[2] && !($1 in t)) { printf "%s%s", sep, $col; sep = "\t" } }' "$STUB_FIXTURE"
  else
    printf '%s' "$STUB_IDS"
  fi;;
*terminate-instances*)
  if [ ! -e "$STUB_LOG.noterm" ]; then
    for a in "$@"; do case "$a" in i-*) echo "$a" >> "$STUB_LOG.term";; esac; done
  fi;;
*run-instances*) echo i-0stub;;
esac
"#;

const TOOLS: [&str; 16] = [
    "sh", "bash", "grep", "awk", "sed", "cat", "touch", "tr", "cut", "head", "sort", "dirname",
    "basename", "mkdir", "rm", "wc",
];

impl Sandbox {
    pub(super) fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir(&bin).unwrap();
        for t in TOOLS {
            for d in ["/usr/bin", "/bin"] {
                let src = Path::new(d).join(t);
                if src.exists() {
                    std::os::unix::fs::symlink(&src, bin.join(t)).unwrap();
                    break;
                }
            }
        }
        let stub = bin.join("aws");
        std::fs::write(&stub, STUB).unwrap();
        Command::new("chmod")
            .arg("755")
            .arg(&stub)
            .status()
            .unwrap();
        assert!(stub.exists(), "the fake aws must exist");
        Sandbox { dir }
    }
    /// Make every later `describe-instances` exit 255 (expired credentials).
    pub(super) fn fail_describe(&self) {
        std::fs::write(self.path().join("calls.log.descfail"), "").unwrap();
    }
    /// Make `describe-instances` exit 255 from its SECOND call on (the first succeeds).
    pub(super) fn fail_describe_from_second(&self) {
        std::fs::write(self.path().join("calls.log.descfail2"), "").unwrap();
    }
    pub(super) fn path(&self) -> &Path {
        self.dir.path()
    }
    pub(super) fn open_gate(&self) {
        std::fs::write(self.path().join("gate"), "parity-gate: pass\n").unwrap();
    }
    pub(super) fn log(&self) -> String {
        std::fs::read_to_string(self.path().join("calls.log")).unwrap_or_default()
    }
    /// Run `script` with only the stub `aws` reachable; `ids` is what describe returns.
    pub(super) fn run(&self, script: &str, ids: &str) -> (i32, String) {
        self.run_with(script, ids, None)
    }
    /// As `run`, but describe answers from a tagged fixture (`id key owner sir` lines).
    pub(super) fn run_with(&self, script: &str, ids: &str, fixture: Option<&str>) -> (i32, String) {
        let mut cmd = Command::new("/bin/bash");
        cmd.arg("-c")
            .arg(script)
            .env_clear()
            .env("PATH", self.path().join("bin"))
            .env("STUB_LOG", self.path().join("calls.log"))
            .env("STUB_IDS", ids);
        if let Some(f) = fixture {
            let fx = self.path().join("fixture");
            std::fs::write(&fx, f).unwrap();
            cmd.env("STUB_FIXTURE", fx);
        }
        let o = cmd.output().unwrap();
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
        "--count 1",
        "MarketType=spot",
        "SpotInstanceType=one-time",
        "--instance-initiated-shutdown-behavior terminate",
        "HttpEndpoint=enabled",
        "shutdown -h +60",
        "HttpTokens=required",
        "HttpPutResponseHopLimit=1",
        DIGEST,
        "Key=pool,Value=spot-rmedia-pr",
    ] {
        assert!(log.contains(want), "missing {want:?} in:\n{log}");
    }
}

#[test]
fn apply_is_idempotent_when_the_instance_already_exists() {
    let sb = Sandbox::new();
    sb.open_gate();
    let (rc, out) = sb.run(&apply_script(&gated(sb.path())), "i-0123abcd");
    assert_eq!(rc, 0, "{out}");
    assert!(!sb.log().contains("run-instances"), "{}", sb.log());
}

#[test]
fn check_asks_the_provider_and_fails_when_absent() {
    let sb = Sandbox::new();
    let (rc, out) = sb.run(&check_script(&res()), "i-0123abcd");
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

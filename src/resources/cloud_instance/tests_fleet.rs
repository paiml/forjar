//! #1265: spot fleet launch — multi-type, multi-AZ, count, plan-time caps,
//! and exit-75 capacity-fallback loop.

use super::tests::{gated, install_tool, res, Sandbox, DIGEST};
use super::tests_b1::{absent, count};
use super::tests_launch::{err, next_token, run_tokens, DAY};
use super::*;
use crate::core::purifier::validate_script as validate_before_exec;
use std::collections::BTreeSet;

/// Fake `aws` for fleet tests: tracks `run-instances` call index, returns distinct
/// instance ids `i-0000000<n>`, and supports per-`(itype, zone)` or per-call capacity/error
/// overrides in `STUB_LOG` sibling files (`cap.<itype>`, `cap.<itype>.<zone>`, `fail.<n>`,
/// `dead.<n>`, `cap.all`).
const FLEET_STUB: &str = r#"#!/bin/sh
echo "$@" >> "$STUB_LOG"
d=$(dirname "$STUB_LOG")
case "$*" in
*run-instances*)
  n=$(grep -c run-instances "$STUB_LOG")
  itype=$(printf '%s\n' "$*" | sed -n 's/.*--instance-type \([^ ]*\).*/\1/p')
  az=$(printf '%s\n' "$*" | sed -n 's/.*AvailabilityZone=\([^ ]*\).*/\1/p')
  if [ -e "$d/fail.$n" ]; then cat "$d/fail.$n" >&2; exit 254; fi
  if [ -e "$d/cap.all" ] || [ -e "$d/cap.$itype" ] || { [ -n "$az" ] && [ -e "$d/cap.$itype.$az" ]; } || [ -e "$d/cap.call.$n" ]; then
    echo "An error occurred (InsufficientInstanceCapacity) when calling the RunInstances operation: nope" >&2
    exit 254
  fi
  id=$(printf 'i-%08x' "$n")
  printf '%s\t%s\n' "$id" "$(date -u -d '@1700000000' +%Y-%m-%dT%H:%M:%S+00:00)"
  exit 0;;
*describe-instances*--instance-ids*)
  id=$(for a in "$@"; do case "$a" in i-*) echo "$a";; esac; done)
  if [ -e "$d/state.$id" ]; then cat "$d/state.$id"; else echo running; fi
  exit 0;;
*Name=client-token*)
  exit 0;;
*describe-instances*)
  if [ -n "${STUB_FIXTURE:-}" ]; then
    want=$(printf '%s\n' "$*" | sed -n 's/.*Name=tag:\(forjar-[a-z]*\),Values=\([^ ]*\).*/\1 \2/p')
    col=1; case "$*" in *SpotInstanceRequestId*) col=4;; esac
    awk -v want="$want" -v col="$col" -v term="$STUB_LOG.term" '
      BEGIN { split(want, w, " "); while ((getline l < term) > 0) t[l] = 1 }
      { c = (w[1] == "forjar-key") ? 2 : 3
        if ($c == w[2] && !($1 in t)) { printf "%s%s", sep, $col; sep = "\t" } }' "$STUB_FIXTURE"
  else
    printf '%s' "${STUB_IDS:-}"
  fi;;
*terminate-instances*)
  for a in "$@"; do case "$a" in i-*) echo "$a" >> "$STUB_LOG.term";; esac; done;;
esac
exit 0
"#;

fn fleet_sandbox() -> Sandbox {
    let sb = Sandbox::new();
    sb.open_gate();
    install_tool(&sb.path().join("bin/aws"), FLEET_STUB, sb.path());
    sb
}

fn fleet_res(dir: &std::path::Path) -> Resource {
    let mut r = gated(dir);
    r.cloud.instance_type = None;
    r.cloud.instance_types = Some(vec!["c7a.xlarge".into(), "c7i.xlarge".into()]);
    r.cloud.availability_zones = Some(vec!["eu-south-2a".into(), "eu-south-2b".into()]);
    r.cloud.count = Some(3);
    r.cloud.budget_cap_usd_day = Some("10".into());
    r
}

#[test]
fn single_value_fields_keep_working_and_multi_lists_work() {
    let single = spec_of(&res()).unwrap();
    assert_eq!(single.instance_types, vec!["c7a.xlarge"]);
    assert_eq!(single.count, 1);
    assert!(single.placement.zones().is_empty());

    let sb = Sandbox::new();
    let fleet = spec_of(&fleet_res(sb.path())).unwrap();
    assert_eq!(fleet.instance_types, vec!["c7a.xlarge", "c7i.xlarge"]);
    assert_eq!(fleet.placement.zones(), ["eu-south-2a", "eu-south-2b"]);
    assert_eq!(fleet.count, 3);
    assert_eq!(
        fleet.candidates(),
        vec![
            ("c7a.xlarge", Some("eu-south-2a")),
            ("c7a.xlarge", Some("eu-south-2b")),
            ("c7i.xlarge", Some("eu-south-2a")),
            ("c7i.xlarge", Some("eu-south-2b")),
        ]
    );
}

#[test]
fn invalid_type_or_zone_lists_and_ambiguous_singles_are_refused() {
    let mut both_types = res();
    both_types.cloud.instance_types = Some(vec!["c7i.xlarge".into()]);
    assert!(spec_of(&both_types).unwrap_err().contains("both"));

    let mut neither_type = res();
    neither_type.cloud.instance_type = None;
    assert!(spec_of(&neither_type).is_err());

    for bad in [
        vec![],
        vec!["c7a.xlarge".into(), "c7a.xlarge".into()],
        vec!["bad type".into()],
        vec!["c7a.xlarge;id".into()],
    ] {
        let mut r = res();
        r.cloud.instance_type = None;
        r.cloud.instance_types = Some(bad.clone());
        assert!(spec_of(&r).is_err(), "{bad:?}");
    }

    let mut both_zones = res();
    both_zones.cloud.availability_zone = Some("eu-south-2a".into());
    both_zones.cloud.availability_zones = Some(vec!["eu-south-2b".into()]);
    assert!(spec_of(&both_zones).unwrap_err().contains("both"));

    for bad in [
        vec![],
        vec!["eu-south-2a".into(), "eu-south-2a".into()],
        vec!["us-east-1a".into()],
        vec!["eu-south-2A".into()],
    ] {
        let mut r = res();
        r.cloud.availability_zones = Some(bad.clone());
        assert!(spec_of(&r).is_err(), "{bad:?}");
    }
}

/// Guard test: fails if the `MAX_FLEET_COUNT` (6) or `count == 0` guard is removed.
#[test]
fn count_hard_cap_guard_refuses_zero_and_above_six_at_plan_time() {
    assert_eq!(MAX_FLEET_COUNT, 6);
    for ok_count in [1, 6] {
        let mut r = res();
        r.cloud.count = Some(ok_count);
        assert!(spec_of(&r).is_ok(), "count={ok_count}");
        assert!(!plan_diff(&r).starts_with("ERROR"), "count={ok_count}");
    }
    for bad_count in [0, 7, 100] {
        let mut r = res();
        r.cloud.count = Some(bad_count);
        let e = spec_of(&r).expect_err(&format!("count={bad_count} must fail"));
        assert!(e.contains("cloud_count"), "{e}");
        assert!(plan_diff(&r).starts_with("ERROR"), "count={bad_count}");
        let sb = Sandbox::new();
        sb.open_gate();
        let mut g = gated(sb.path());
        g.cloud.count = Some(bad_count);
        let (rc, out) = sb.run(&apply_script(&g), "");
        assert_ne!(rc, 0);
        assert!(out.contains("REFUSED"), "{out}");
        assert_eq!(sb.log(), "");
    }
}

/// Guard test: fails if the daily budget check ignores `cloud_count`.
#[test]
fn daily_budget_cap_multiplies_by_fleet_count_at_apply_time() {
    let sb = Sandbox::new();
    sb.open_gate();
    let mut g = gated(sb.path());
    g.cloud.max_price = Some("1.0".into());
    g.cloud.count = Some(4);
    g.cloud.max_lifetime_min = Some(60);
    g.cloud.budget_cap_usd_day = Some("3.0".into());
    let (rc, out) = sb.run(&apply_script(&g), "");
    assert_ne!(rc, 0, "{out}");
    assert!(out.contains("exceeds cloud_budget_cap_usd_day"), "{out}");
    assert_eq!(sb.log(), "");
}

#[test]
fn fleet_launches_count_instances_with_distinct_tokens_and_spot_options() {
    let sb = fleet_sandbox();
    let r = fleet_res(sb.path());
    let (rc, out) = sb.run(&apply_script_on(&r, DAY), "");
    assert_eq!(rc, 0, "{out}");
    assert_eq!(
        out.trim_end().lines().last(),
        Some("launched i-00000001 i-00000002 i-00000003"),
        "{out}"
    );
    let log = sb.log();
    assert_eq!(count(&log, "run-instances"), 3, "{log}");
    let toks = run_tokens(&log);
    assert_eq!(toks.len(), 3, "{log}");
    assert_eq!(toks.iter().collect::<BTreeSet<_>>().len(), 3, "{toks:?}");
    assert!(log.contains("MarketType=spot"), "{log}");
    assert!(log.contains("AvailabilityZone=eu-south-2a"), "{log}");
}

#[test]
fn exit_75_capacity_fallback_walks_across_types_and_zones() {
    let sb = fleet_sandbox();
    // c7a.xlarge/2a has capacity for 0; c7a.xlarge/2b has capacity for 1 (call 3 hits cap);
    // c7i.xlarge/2a serves the remaining 2 slots!
    std::fs::write(sb.path().join("cap.c7a.xlarge.eu-south-2a"), "").unwrap();
    std::fs::write(sb.path().join("cap.call.3"), "").unwrap();
    let r = fleet_res(sb.path());
    let (rc, out) = sb.run(&apply_script_on(&r, DAY), "");
    assert_eq!(rc, 0, "{out}");
    let log = sb.log();
    let runs: Vec<&str> = log
        .lines()
        .filter(|l| l.contains("run-instances"))
        .collect();
    // Call 1: c7a/2a (cap) -> Call 2: c7a/2b (ok #1) -> Call 3: c7a/2b (cap) -> Call 4: c7i/2a (ok #2) -> Call 5: c7i/2a (ok #3)
    assert_eq!(runs.len(), 5, "{log}");
    assert!(runs[0].contains("c7a.xlarge") && runs[0].contains("eu-south-2a"));
    assert!(runs[1].contains("c7a.xlarge") && runs[1].contains("eu-south-2b"));
    assert!(runs[2].contains("c7a.xlarge") && runs[2].contains("eu-south-2b"));
    assert!(runs[3].contains("c7i.xlarge") && runs[3].contains("eu-south-2a"));
    assert!(runs[4].contains("c7i.xlarge") && runs[4].contains("eu-south-2a"));
    let toks = run_tokens(&log);
    assert_eq!(toks.iter().collect::<BTreeSet<_>>().len(), 5, "{toks:?}");
}

#[test]
fn all_candidates_exhausted_exits_75_and_non_capacity_error_stops_immediately() {
    let sb = fleet_sandbox();
    std::fs::write(sb.path().join("cap.all"), "").unwrap();
    let r = fleet_res(sb.path());
    let (rc, out) = sb.run(&apply_script_on(&r, DAY), "");
    assert_eq!(rc, 75, "{out}");
    assert!(out.contains("launch-capacity-unavailable"), "{out}");
    // 2 types x 2 AZs = 4 candidates tried
    assert_eq!(count(&sb.log(), "run-instances"), 4, "{}", sb.log());

    // Non-capacity error (UnauthorizedOperation) aborts on call 1 without fallback
    let sb2 = fleet_sandbox();
    std::fs::write(sb2.path().join("fail.1"), err("UnauthorizedOperation")).unwrap();
    let r2 = fleet_res(sb2.path());
    let (rc, out) = sb2.run(&apply_script_on(&r2, DAY), "");
    assert_eq!(rc, 254, "{out}");
    assert_eq!(count(&sb2.log(), "run-instances"), 1, "{}", sb2.log());
}

#[test]
fn fleet_is_idempotent_scales_up_partial_and_tears_down_all() {
    let sb = fleet_sandbox();
    let r = fleet_res(sb.path());
    let key = key_of(&spec_of(&r).unwrap());
    let own = owner_of(&spec_of(&r).unwrap());

    // 3 already live -> 0 launches
    let full_fx = format!(
        "i-00000001 {key} {own} sir-00000001\n\
         i-00000002 {key} {own} sir-00000002\n\
         i-00000003 {key} {own} sir-00000003\n"
    );
    let (rc, out) = sb.run_with(&apply_script_on(&r, DAY), "", Some(&full_fx));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("present"), "{out}");
    assert_eq!(count(&sb.log(), "run-instances"), 0);

    // 2 live, count=3 -> launches exactly 1 more
    let sb2 = fleet_sandbox();
    let r2 = fleet_res(sb2.path());
    let part_fx = format!(
        "i-0000f001 {key} {own} sir-00000001\n\
         i-0000f002 {key} {own} sir-00000002\n"
    );
    let (rc, out) = sb2.run_with(&apply_script_on(&r2, DAY), "", Some(&part_fx));
    assert_eq!(rc, 0, "{out}");
    assert_eq!(count(&sb2.log(), "run-instances"), 1, "{}", sb2.log());

    // check_script fails with 2 live, passes with 3 live
    let (rc, _) = sb2.run_with(&check_script(&r2), "", Some(&part_fx));
    assert_ne!(rc, 0);
    let (rc, _) = sb2.run_with(&check_script(&r2), "", Some(&full_fx));
    assert_eq!(rc, 0);

    // state: absent terminates all 3
    let sb3 = fleet_sandbox();
    let r3 = absent(fleet_res(sb3.path()));
    let (rc, out) = sb3.run_with(&apply_script_on(&r3, DAY), "", Some(&full_fx));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("destroyed"), "{out}");
    assert_eq!(count(&sb3.log(), "terminate-instances"), 1);
    assert_eq!(count(&sb3.log(), "cancel-spot-instance-requests"), 3);
}

#[test]
fn fleet_dead_box_token_chain_works_and_scripts_pass_i8_gate() {
    let sb = fleet_sandbox();
    std::fs::write(sb.path().join("state.i-00000001"), "terminated").unwrap();
    let mut r = fleet_res(sb.path());
    r.cloud.count = Some(1);
    r.cloud.availability_zones = None;
    let script = apply_script_on(&r, DAY);
    validate_before_exec(&script).expect("fleet apply script passes I8 gate");
    validate_before_exec(&check_script(&fleet_res(sb.path()))).expect("fleet check passes I8");
    validate_before_exec(&state_query_script(&fleet_res(sb.path())))
        .expect("fleet query passes I8");

    let (rc, out) = sb.run(&script, "");
    assert_eq!(rc, 0, "{out}");
    assert_eq!(out.trim_end().lines().last(), Some("launched i-00000002"));
    let toks = run_tokens(&sb.log());
    assert_eq!(toks.len(), 2);
    assert_eq!(toks[1], next_token(&toks[0], "i-00000001"));
}

#[test]
fn fleet_yaml_parses_validates_resolves_and_plans() {
    let yaml = format!(
        "version: \"1.0\"\nname: fleet\nmachines:\n  m: {{hostname: m, addr: 127.0.0.1}}\nparams:\n  t1: c7a.xlarge\nresources:\n  w:\n    type: cloud_instance\n    machine: m\n    provider: aws\n    name: workers\n    image: img@{DIGEST}\n    cloud_region: eu-south-2\n    cloud_instance_types: [\"{{{{params.t1}}}}\", c7i.xlarge]\n    cloud_availability_zones: [eu-south-2a, eu-south-2b]\n    cloud_count: 4\n    cloud_max_price: \"0.25\"\n    cloud_max_lifetime_min: 60\n"
    );
    let cfg = crate::core::parser::parse_config(&yaml).expect("parses");
    let errs = crate::core::parser::validate_config(&cfg);
    assert!(errs.is_empty(), "{errs:?}");
    let resolved = crate::core::resolver::resolve_all(
        &cfg.resources,
        &cfg.params,
        &cfg.machines,
        &cfg.secrets,
    );
    let plan = plan_diff(&resolved["w"]);
    assert!(plan.contains("c7a.xlarge,c7i.xlarge x4"), "{plan}");
    assert!(
        plan.contains("availability_zones: eu-south-2a,eu-south-2b"),
        "{plan}"
    );
}

/// Guard test: fails if a duplicate id from the fleet loop lets the launch exit 0 short of `cloud_count`
/// (the slot nonce must not steal an iteration from the live-instance count).
#[test]
fn fleet_duplicate_id_does_not_exit_short_of_count() {
    let sb = fleet_sandbox();
    let r = fleet_res(sb.path());
    let key = key_of(&spec_of(&r).unwrap());
    let own = owner_of(&spec_of(&r).unwrap());
    // 1 live; the first run-instances call returns i-00000001 again (a duplicate).
    let fx = format!("i-00000001 {key} {own} sir-00000001\n");
    let (rc, out) = sb.run_with(&apply_script_on(&r, DAY), "", Some(&fx));
    assert_eq!(rc, 0, "{out}");
    assert_eq!(
        out.trim_end().lines().last(),
        Some("launched i-00000002 i-00000003"),
        "{out}"
    );
    assert_eq!(count(&sb.log(), "run-instances"), 3, "{}", sb.log());
}

/// Regression (#1273 lane 1): one subnet is one AZ, so it cannot combine with a multi-AZ list.
#[test]
fn subnet_refuses_multi_az_fleet_at_plan_time() {
    let sb = fleet_sandbox();
    let mut r = fleet_res(sb.path());
    r.cloud.subnet_id = Some("subnet-01234567".into());
    r.cloud.availability_zones = Some(vec!["eu-south-2a".into(), "eu-south-2b".into()]);
    let err = spec_of(&r).expect_err("subnet with two AZs must refuse");
    assert!(err.contains("one AZ"), "{err}");
}

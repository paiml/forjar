//! #1037: a template that resolves to a bad value, or stays unresolved, is
//! refused by every script generator and never reaches the provider CLI.

use super::tests::{gated, Sandbox};
use super::*;
use std::collections::HashMap;

type Setter = fn(&mut Resource, &str);

/// Fields the resolver fills in: a bad resolved value must be refused.
const RESOLVED: [(&str, Setter); 6] = [
    ("region", |r, v| r.cloud.region = Some(v.into())),
    ("type", |r, v| r.cloud.instance_type = Some(v.into())),
    ("types", |r, v| {
        r.cloud.instance_type = None;
        r.cloud.instance_types = Some(vec![v.into()]);
    }),
    ("price", |r, v| r.cloud.max_price = Some(v.into())),
    ("name", |r, v| r.name = Some(v.into())),
    ("image", |r, v| r.image = Some(v.into())),
];

/// Fields the resolver does not touch: a template stays literally unresolved.
const UNTOUCHED: [(&str, Setter); 6] = [
    ("ami", |r, v| r.cloud.ami = Some(v.into())),
    ("subnet", |r, v| r.cloud.subnet_id = Some(v.into())),
    ("groups", |r, v| {
        r.cloud.security_group_ids = Some(vec![v.into()])
    }),
    ("zone", |r, v| r.cloud.availability_zone = Some(v.into())),
    ("zones", |r, v| {
        r.cloud.availability_zones = Some(vec![v.into()])
    }),
    ("tag", |r, v| r.tags.push(format!("k={v}"))),
];

/// Every generator, for state present and absent, run against the fake aws.
fn assert_refused_everywhere(r: &Resource, what: &str) {
    for state in [None, Some("absent")] {
        let mut r = r.clone();
        r.state = state.map(String::from);
        refused_scripts(&r, &format!("{what}/{state:?}"));
    }
}

fn refused_scripts(r: &Resource, what: &str) {
    let scripts = [
        ("apply", apply_script(r)),
        ("check", check_script(r)),
        ("state_query", state_query_script(r)),
    ];
    for (kind, script) in scripts {
        let label = format!("{what}/{kind}");
        assert!(script.contains("REFUSED"), "{label}: {script}");
        assert!(!script.contains("aws ec2"), "{label}: {script}");
        let sb = Sandbox::new();
        let (rc, _) = sb.run(&script, "i-0123456789abcdef0");
        assert_ne!(rc, 0, "{label}");
        assert_eq!(sb.log(), "", "{label}: the provider CLI ran");
    }
}

fn params(bad: &str) -> HashMap<String, serde_yaml_ng::Value> {
    HashMap::from([("bad".to_string(), serde_yaml_ng::Value::String(bad.into()))])
}

#[test]
fn a_template_resolving_to_a_bad_value_produces_no_provider_command() {
    let sb = Sandbox::new();
    assert!(spec_of(&gated(sb.path())).is_ok(), "baseline must be valid");
    let machines = indexmap::IndexMap::new();
    let bad_values = ["x y", "a;rm -rf /", "$(id)", "../x", "", "{{params.bad}}"];
    // syntactically valid for most fields, bad for one
    let extra = |field: &str| match field {
        "price" => vec!["free", "0", "-1"],
        "image" => vec!["img", "img@sha256:abc"],
        _ => vec![],
    };
    for (field, set) in RESOLVED {
        for bad in bad_values.into_iter().chain(extra(field)) {
            let mut r = gated(sb.path());
            set(&mut r, "{{params.bad}}");
            let r = crate::core::resolver::resolve_resource_templates(&r, &params(bad), &machines)
                .expect("the param is defined, so resolution succeeds");
            assert_refused_everywhere(&r, &format!("{field}={bad:?}"));
        }
    }
}

#[test]
fn a_good_resolved_value_still_launches() {
    let sb = Sandbox::new();
    sb.open_gate();
    let mut r = gated(sb.path());
    r.cloud.region = Some("{{params.bad}}".into());
    let r = crate::core::resolver::resolve_resource_templates(
        &r,
        &params("eu-south-2"),
        &indexmap::IndexMap::new(),
    )
    .unwrap();
    let (rc, _) = sb.run(&apply_script(&r), "");
    assert_eq!(rc, 0);
    assert!(sb.log().contains("run-instances"), "{}", sb.log());
}

#[test]
fn an_unresolved_template_reaching_codegen_is_refused() {
    let sb = Sandbox::new();
    for (field, set) in RESOLVED.iter().chain(UNTOUCHED.iter()) {
        let mut r = gated(sb.path());
        set(&mut r, "{{params.x}}");
        assert_refused_everywhere(&r, field);
    }
    for (state, provider) in [("{{params.x}}", "aws"), ("present", "{{params.x}}")] {
        let mut r = gated(sb.path());
        r.state = Some(state.into());
        r.provider = Some(provider.into());
        refused_scripts(&r, "state/provider");
    }
}

#[test]
fn the_dispatcher_refuses_what_the_generators_refuse() {
    use crate::core::codegen as cg;
    let sb = Sandbox::new();
    let mut inspected = 0;
    for (field, set) in RESOLVED.iter().chain(UNTOUCHED.iter()) {
        for state in [None, Some("absent")] {
            let mut r = gated(sb.path());
            set(&mut r, "{{params.x}}");
            r.state = state.map(String::from);
            let outs = [
                cg::apply_script(&r),
                cg::check_script(&r),
                cg::state_query_script(&r),
            ];
            // an Err is a refusal too; an Ok script must say REFUSED
            for script in outs.into_iter().flatten() {
                inspected += 1;
                let label = format!("{field}/{state:?}: {script}");
                assert!(script.contains("REFUSED"), "{label}");
                assert!(!script.contains("aws ec2"), "{label}");
            }
        }
    }
    assert!(
        inspected > 0,
        "every dispatcher result was an Err: nothing was inspected"
    );
}

/// Gate fields: a template left in a gate path or the budget must refuse the
/// launch. Only apply (present) reads the gates; check, state_query and the
/// absent/destroy script never launch and never read them.
#[test]
fn an_unresolved_template_in_a_gate_is_refused_never_an_absent_kill_switch() {
    let sb = Sandbox::new();
    sb.open_gate();
    let setters: [(&str, Setter); 3] = [
        ("kill_switch", |r, v| r.cloud.kill_switch = Some(v.into())),
        ("parity_gate", |r, v| r.cloud.parity_gate = Some(v.into())),
        ("budget", |r, v| r.cloud.budget_cap_usd_day = Some(v.into())),
    ];
    let (rc, _) = sb.run(&apply_script(&gated(sb.path())), "");
    assert_eq!(rc, 0, "baseline launches");
    for (field, set) in setters {
        for bad in ["/run/{{params.k}}", "{{params.k}}", "/run/}}x", "/run/{{"] {
            let mut r = gated(sb.path());
            set(&mut r, bad);
            let script = apply_script(&r);
            let label = format!("{field}={bad}: {script}");
            assert!(script.contains("REFUSED"), "{label}");
            assert!(!script.contains("aws ec2"), "{label}");
        }
    }
}

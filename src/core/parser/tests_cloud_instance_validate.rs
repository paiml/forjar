//! `validate_cloud_instance` at parse time (#1046, #1047, #1038).

use super::cloud_instance_validate::validate_cloud_instance;
use crate::core::types::*;

const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn valid() -> Resource {
    let mut r = Resource {
        resource_type: ResourceType::CloudInstance,
        machine: MachineTarget::Single("local".to_string()),
        provider: Some("aws".into()),
        name: Some("web".into()),
        image: Some(format!("nginx@sha256:{DIGEST}")),
        ..Resource::default()
    };
    r.cloud.region = Some("us-east-1".into());
    r.cloud.instance_type = Some("m5.large".into());
    r.cloud.max_price = Some("0.5".into());
    r.cloud.max_lifetime_min = Some(60);
    r
}

fn errs(r: &Resource) -> Vec<String> {
    let mut e = Vec::new();
    validate_cloud_instance("ci", r, &mut e);
    e.into_iter().map(|x| x.message).collect()
}

#[test]
fn moved_function_accepts_valid_and_reports_bad_region() {
    assert!(errs(&valid()).is_empty());
    let mut r = valid();
    r.cloud.region = Some("US East!".into());
    let e = errs(&r);
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(e[0].contains("cloud_region"), "{e:?}");
}

type Set = fn(&mut Resource);

fn t() -> String {
    "{{params.x}}".into()
}

/// Parse-time errors expected as (field alone, field plus a bad literal sibling).
/// The sibling is a missing `cloud_max_lifetime_min`, which no template can fill.
type Seen = (bool, bool);
/// Rejected by itself, and so is the sibling.
const REJECTS: Seen = (true, true);
/// A template stands in for its own field only: accepted alone, the sibling is caught.
const CHECKED: Seen = (false, true);

fn table() -> Vec<(&'static str, Set, Seen)> {
    vec![
        ("name", |r| r.name = Some(t()), CHECKED),
        ("image", |r| r.image = Some(t()), CHECKED),
        ("cloud_region", |r| r.cloud.region = Some(t()), CHECKED),
        (
            "cloud_instance_type",
            |r| r.cloud.instance_type = Some(t()),
            CHECKED,
        ),
        (
            "cloud_max_price",
            |r| r.cloud.max_price = Some(t()),
            CHECKED,
        ),
        ("tags", |r| r.tags = vec![format!("k={}", t())], REJECTS),
        ("name_unclosed", |r| r.name = Some("a{{b".into()), REJECTS),
        ("tag_braced", |r| r.tags = vec!["k={{v}}".into()], REJECTS),
        ("provider", |r| r.provider = Some(t()), REJECTS),
        ("state", |r| r.state = Some(t()), REJECTS),
        ("cloud_ami", |r| r.cloud.ami = Some(t()), REJECTS),
        (
            "cloud_subnet_id",
            |r| r.cloud.subnet_id = Some(t()),
            REJECTS,
        ),
        (
            "cloud_security_group_ids",
            |r| r.cloud.security_group_ids = Some(vec![t()]),
            REJECTS,
        ),
        (
            "cloud_availability_zone",
            |r| r.cloud.availability_zone = Some(t()),
            REJECTS,
        ),
        (
            "cloud_kill_switch",
            |r| r.cloud.kill_switch = Some(t()),
            CHECKED,
        ),
        (
            "cloud_parity_gate",
            |r| r.cloud.parity_gate = Some(t()),
            CHECKED,
        ),
        (
            "cloud_budget_cap_usd_day",
            |r| r.cloud.budget_cap_usd_day = Some(t()),
            CHECKED,
        ),
    ]
}

#[test]
fn templated_vs_literal_fields_table() {
    for (label, set, (alone, with_bad_sibling)) in table() {
        let mut r = valid();
        set(&mut r);
        assert_eq!(!errs(&r).is_empty(), alone, "{label}: alone");
        r.cloud.max_lifetime_min = None;
        assert_eq!(!errs(&r).is_empty(), with_bad_sibling, "{label}: sibling");
    }
}

#[test]
fn a_templated_field_does_not_hide_a_bad_literal_sibling() {
    let mut r = valid();
    r.name = Some(t());
    r.cloud.region = Some("US East!".into());
    let e = errs(&r);
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(e[0].contains("cloud_region"), "{e:?}");
    // a templated region keeps a literal zone's shape check
    let mut r = valid();
    r.cloud.region = Some(t());
    r.cloud.availability_zone = Some("us-west-2a".into());
    assert!(errs(&r).is_empty());
    r.cloud.availability_zone = Some("us-west-2".into());
    assert!(!errs(&r).is_empty());
}

#[test]
fn an_unclosed_brace_pair_is_a_literal_and_is_checked() {
    let mut r = valid();
    r.cloud.instance_type = Some("m5.{{large".into());
    assert!(errs(&r)[0].contains("cloud_instance_type"));
    r.cloud.instance_type = Some("}}m5{{".into());
    assert!(!errs(&r).is_empty());
}

#[test]
fn a_tag_with_braces_is_rejected_because_tags_are_never_resolved() {
    let mut r = valid();
    r.tags = vec!["env={{params.env}}".into()];
    let e = errs(&r);
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(e[0].contains("tag"), "{e:?}");
}

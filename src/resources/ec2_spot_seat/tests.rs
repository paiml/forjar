//! `ec2_spot_seat` validation, pin and plan-level tests.

use super::*;
use crate::core::error::ErrorClass;
use crate::core::types::{Resource, ResourceType, SpotSeatSpec};

pub(super) const ID: &str =
    "sha256:bbc07e6620e8451b781ca0c05903c8126bb05fa85c2ba0b3b58502ebc5c56543";
pub(super) const SHA: &str = "c9749e14fe23d7d63b4753f98a59eeb9f69a48c729c0d20af5208adb1b91b558";

pub(super) fn pin_text() -> String {
    format!(
        "# comment\nimage\tversion\tv11\nimage\tid\t{ID}\nimage\ttar_sha256\t{SHA}\nimage\ts3_key\timage/burst-runner-bbc07e6620e8.tar.zst\n"
    )
}

pub(super) fn res() -> Resource {
    Resource {
        resource_type: ResourceType::Ec2SpotSeat,
        spot: SpotSeatSpec {
            phase: Some("launch".into()),
            pin_file: Some("fleet-image.tsv".into()),
            image_bucket: Some("img-bucket".into()),
            instance_types: vec!["c6a.4xlarge".into(), "m6a.4xlarge".into()],
            region: Some("us-east-1".into()),
            az: Some("us-east-1a".into()),
            subnet: Some("subnet-0abc1234".into()),
            ami: Some("ami-0123456789abcdef0".into()),
            ttl_min: Some(120),
            seats: Some(2),
            owner: Some("forjar-spot".into()),
            max_boxes: Some(2),
            max_usd_per_hour: Some(1.0),
            price_usd_per_hour: Some(0.5),
            market: Some("spot".into()),
            runner_org: Some("paiml".into()),
            runner_group: Some("tier2".into()),
            runner_labels: vec!["self-hosted".into(), "tier2".into()],
            jit_source: Some("gh-api".into()),
            evidence_dest: Some("s3://evidence-bucket/spot".into()),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn err_of(f: impl FnOnce(&mut Resource)) -> String {
    let mut r = res();
    f(&mut r);
    spec_of(&r).expect_err("must be refused").into_message()
}

#[test]
fn a_refused_declaration_or_pin_is_the_validation_class() {
    let mut r = res();
    r.spot.market = Some("on-demand".into());
    assert_eq!(spec_of(&r).unwrap_err().class(), ErrorClass::Validation);
    let e = parse_pin(&pin_text(), "nope", "s3_key", "tar_sha256").unwrap_err();
    assert_eq!(e.class(), ErrorClass::Validation);
}

#[test]
fn valid_declaration_parses() {
    let s = spec_of(&res()).expect("valid");
    assert_eq!(s.phase, Phase::Launch);
    assert_eq!(s.pin_id_key, "id");
    assert_eq!(s.pin_s3_key_key, "s3_key");
    assert_eq!(s.seats, 2);
}

#[test]
fn on_demand_market_is_a_validation_error() {
    assert!(err_of(|r| r.spot.market = Some("on-demand".into())).contains("only spot"));
}

#[test]
fn one_box_over_its_own_usd_cap_is_refused() {
    assert!(err_of(|r| r.spot.price_usd_per_hour = Some(1.5)).contains("exceeds"));
}

#[test]
fn field_errors_are_named() {
    assert!(err_of(|r| r.spot.az = Some("us-west-2a".into())).contains("not a zone"));
    assert!(err_of(|r| r.spot.subnet = Some("sn-1".into())).contains("subnet"));
    assert!(err_of(|r| r.spot.instance_types.clear()).contains("spot_instance_types"));
    assert!(err_of(|r| r.spot.seats = Some(0)).contains("spot_seats"));
    assert!(err_of(|r| r.spot.max_boxes = Some(0)).contains("positive"));
    assert!(err_of(|r| r.spot.runner_labels = vec!["a b".into()]).contains("labels"));
    assert!(err_of(|r| r.spot.jit_source = Some("dir:rel".into())).contains("spot_jit_source"));
    assert!(err_of(|r| r.spot.owner = None).contains("spot_owner"));
    assert!(err_of(|r| r.spot.owner = Some("x;rm".into())).contains("plain word"));
    assert!(err_of(|r| r.spot.region = Some("nowhere".into())).contains("region"));
    assert!(err_of(|r| r.spot.pin_file = None).contains("spot_pin_file"));
    assert!(err_of(|r| r.spot.phase = Some("boot".into())).contains("not launch"));
}

#[test]
fn absent_means_teardown() {
    let mut r = res();
    r.spot.phase = None;
    r.state = Some("absent".into());
    assert_eq!(spec_of(&r).unwrap().phase, Phase::Teardown);
    r.spot.phase = Some("launch".into());
    assert!(spec_of(&r).is_err());
}

#[test]
fn pin_is_parsed_from_the_declared_keys() {
    let p = parse_pin(&pin_text(), "id", "s3_key", "tar_sha256").unwrap();
    assert_eq!(p.id, ID);
    assert_eq!(p.sha256, SHA);
    assert_eq!(p.s3_key, "image/burst-runner-bbc07e6620e8.tar.zst");
}

#[test]
fn bad_pins_are_refused() {
    assert!(parse_pin(&pin_text(), "nope", "s3_key", "tar_sha256")
        .unwrap_err()
        .message()
        .contains("missing"));
    let dup = format!("{}image\tid\t{ID}\n", pin_text());
    assert!(parse_pin(&dup, "id", "s3_key", "tar_sha256")
        .unwrap_err()
        .message()
        .contains("twice"));
    let bad = pin_text().replace(ID, "sha256:zz");
    assert!(parse_pin(&bad, "id", "s3_key", "tar_sha256")
        .unwrap_err()
        .message()
        .contains("image id"));
    let bad = pin_text().replace(SHA, "abc");
    assert!(parse_pin(&bad, "id", "s3_key", "tar_sha256")
        .unwrap_err()
        .message()
        .contains("64 hex"));
    let bad = pin_text().replace("image/burst", "image/$(x)");
    assert!(parse_pin(&bad, "id", "s3_key", "tar_sha256")
        .unwrap_err()
        .message()
        .contains("unsafe"));
}

/// One valid `ec2_spot_seat` declaration: one seat, max 1 box, 1.0 USD/h cap, 0.5 USD/h box.
fn spot_yaml() -> String {
    "version: '1.0'\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  seat:\n    type: ec2_spot_seat\n    machine: m\n    spot_pin_file: p.tsv\n    spot_instance_types: [c6a.4xlarge]\n    spot_region: us-east-1\n    spot_az: us-east-1a\n    spot_subnet: subnet-0abc\n    spot_ami: ami-1\n    spot_ttl_min: 60\n    spot_seats: 1\n    spot_owner: forjar-spot\n    spot_max_boxes: 1\n    spot_max_usd_per_hour: 1.0\n    spot_price_usd_per_hour: 0.5\n    spot_market: spot\n    spot_runner_org: paiml\n    spot_runner_group: g\n    spot_runner_labels: [x]\n    spot_jit_source: gh-api\n".to_string()
}

#[test]
fn cap_exceeding_spec_is_refused_at_plan_time_by_the_parser() {
    // one box alone above its USD cap: refused by validation, before any launch
    let y = spot_yaml().replace(
        "spot_price_usd_per_hour: 0.5",
        "spot_price_usd_per_hour: 2.0",
    );
    let cfg = crate::core::parser::parse_config(&y).expect("parses");
    let errs = crate::core::parser::validate_config(&cfg);
    assert!(
        errs.iter().any(|e| e.message.contains("exceeds")),
        "{errs:?}"
    );
}

#[test]
fn cap_exceeding_spec_is_refused_before_planning_by_parse_and_validate() {
    // `parse_and_validate` is the gate every plan and apply command runs before
    // `planner::plan`; the core planner itself never calls this resource.
    let y = spot_yaml().replace(
        "spot_price_usd_per_hour: 0.5",
        "spot_price_usd_per_hour: 2.0",
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.yaml");
    std::fs::write(&path, y).unwrap();
    let e = match crate::core::parser::parse_and_validate(&path) {
        Ok(_) => panic!("an over-cap declaration must not reach the planner"),
        Err(e) => e,
    };
    assert!(e.contains("ec2_spot_seat") && e.contains("exceeds"), "{e}");
}

#[test]
fn box_cap_exceeding_spec_is_refused_by_launch_before_run() {
    use super::api::LiveBox;
    use super::lifecycle::launch;
    use super::tests_lifecycle::FakeEc2;
    let cfg = crate::core::parser::parse_config(&spot_yaml()).expect("parses");
    let r = cfg.resources.get("seat").expect("seat resource");
    let s = spec_of(r).expect("valid spec");
    assert_eq!(s.max_boxes, 1);
    // one live box already holds the only slot: launch must refuse before run_spot
    let f = FakeEc2 {
        live: std::cell::RefCell::new(vec![LiveBox {
            id: "i-1".into(),
            price_usd_per_hour: 0.1,
        }]),
        ..Default::default()
    };
    let e = launch(&s, &f, "tok").expect_err("over box cap must be refused before run");
    assert_eq!(
        e.message(),
        "cap: 2 boxes for owner forjar-spot exceeds spot_max_boxes 1"
    );
    assert_eq!(
        f.launched.borrow().len(),
        0,
        "run_spot was reached: a launch would have counted here"
    );
}

#[test]
fn yaml_validates_through_the_parser() {
    let y = spot_yaml();
    let cfg = crate::core::parser::parse_config(&y).expect("parses");
    assert!(
        crate::core::parser::validate_config(&cfg).is_empty(),
        "spot validates clean"
    );
    let bad = crate::core::parser::parse_config(
        &y.replace("spot_market: spot", "spot_market: on-demand"),
    )
    .expect("parses");
    let errs = crate::core::parser::validate_config(&bad);
    assert!(
        errs.iter().any(|e| e.message.contains("only spot")),
        "{errs:?}"
    );
}

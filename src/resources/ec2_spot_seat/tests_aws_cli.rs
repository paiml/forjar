//! `AwsCli` against a fake runner: no aws binary, no network.

use super::api::{Ec2Api, LaunchError, LaunchRequest};
use super::aws_cli::{run_instances_args, AwsCli, CliRunner};
use crate::core::error::ForjarError;
use std::cell::RefCell;

struct Fake {
    calls: RefCell<Vec<Vec<String>>>,
    reply: Result<String, ForjarError>,
}

impl Fake {
    fn new(reply: Result<&str, &str>) -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            reply: reply.map(str::to_string).map_err(ForjarError::connection),
        }
    }
}

impl CliRunner for &Fake {
    fn run(&self, args: &[&str]) -> Result<String, ForjarError> {
        self.calls
            .borrow_mut()
            .push(args.iter().map(|s| (*s).to_string()).collect());
        self.reply.clone()
    }
}

fn req() -> LaunchRequest {
    LaunchRequest {
        instance_type: "c6a.4xlarge".into(),
        az: "us-east-1a".into(),
        subnet: "subnet-0abc1234".into(),
        ami: Some("ami-0123".into()),
        owner: "forjar-spot".into(),
        ttl_min: 120,
        price_usd_per_hour: 0.5,
        client_token: "tok-1".into(),
    }
}

const LIVE: &str = r#"{"Reservations":[{"Instances":[
 {"InstanceId":"i-1","Tags":[{"Key":"Owner","Value":"forjar-spot"},{"Key":"PriceUsdPerHour","Value":"0.5"}]},
 {"InstanceId":"i-2","Tags":[{"Key":"PriceUsdPerHour","Value":"0.25"}]}]}]}"#;

#[test]
fn live_boxes_parses_boxes_and_filters_by_owner() {
    let f = Fake::new(Ok(LIVE));
    let boxes = AwsCli::with_runner(&f, "us-east-1")
        .live_boxes("forjar-spot")
        .unwrap();
    assert_eq!(boxes.len(), 2);
    assert_eq!(
        (boxes[0].id.as_str(), boxes[0].price_usd_per_hour),
        ("i-1", 0.5)
    );
    let call = f.calls.borrow()[0].join(" ");
    assert!(call.contains("describe-instances"));
    assert!(call.contains("Name=tag:Owner,Values=forjar-spot"));
    assert!(call.contains("pending,running"));
    assert!(call.contains("--region us-east-1"));
}

#[test]
fn live_boxes_failed_command_is_err_not_empty() {
    let f = Fake::new(Err("AuthFailure"));
    let r = AwsCli::with_runner(&f, "us-east-1").live_boxes("o");
    assert_eq!(r, Err(ForjarError::connection("AuthFailure")));
}

#[test]
fn live_boxes_bad_json_and_missing_price_are_err() {
    let f = Fake::new(Ok("not json"));
    assert!(AwsCli::with_runner(&f, "r").live_boxes("o").is_err());
    let f = Fake::new(Ok(
        r#"{"Reservations":[{"Instances":[{"InstanceId":"i-9"}]}]}"#,
    ));
    assert!(AwsCli::with_runner(&f, "r").live_boxes("o").is_err());
}

#[test]
fn live_boxes_empty_reservations_is_empty_ok() {
    let f = Fake::new(Ok(r#"{"Reservations":[]}"#));
    assert_eq!(AwsCli::with_runner(&f, "r").live_boxes("o"), Ok(vec![]));
}

#[test]
fn run_spot_passes_token_spot_market_and_owner_tag() {
    let f = Fake::new(Ok(r#"{"Instances":[{"InstanceId":"i-77"}]}"#));
    let id = AwsCli::with_runner(&f, "us-east-1")
        .run_spot(&req())
        .unwrap();
    assert_eq!(id, "i-77");
    let call = f.calls.borrow()[0].join(" ");
    assert!(call.contains("run-instances"));
    assert!(call.contains("--client-token tok-1"));
    assert!(call.contains(r#""MarketType":"spot""#));
    assert!(call.contains(r#""SpotInstanceType":"one-time""#));
    assert!(call.contains(r#""InstanceInterruptionBehavior":"terminate""#));
    assert!(call.contains(r#""Key":"Owner","Value":"forjar-spot""#));
    assert!(call.contains("AvailabilityZone=us-east-1a"));
    assert!(call.contains("--subnet-id subnet-0abc1234"));
}

#[test]
fn run_spot_capacity_errors_map_to_capacity() {
    for msg in [
        "An error occurred (InsufficientInstanceCapacity) when calling RunInstances",
        "An error occurred (SpotMaxPriceTooLow) when calling RunInstances",
    ] {
        let f = Fake::new(Err(msg));
        let e = AwsCli::with_runner(&f, "r").run_spot(&req()).unwrap_err();
        assert_eq!(e, LaunchError::Capacity(msg.to_string()));
    }
}

#[test]
fn run_spot_other_error_maps_to_other() {
    let f = Fake::new(Err("UnauthorizedOperation"));
    let e = AwsCli::with_runner(&f, "r").run_spot(&req()).unwrap_err();
    assert_eq!(e, LaunchError::Other("UnauthorizedOperation".into()));
}

#[test]
fn run_spot_without_ami_or_instance_id_is_other_and_makes_no_bad_call() {
    let f = Fake::new(Ok("{}"));
    let mut r = req();
    r.ami = None;
    assert!(matches!(
        AwsCli::with_runner(&f, "r").run_spot(&r),
        Err(LaunchError::Other(_))
    ));
    assert!(f.calls.borrow().is_empty());
    assert!(matches!(
        AwsCli::with_runner(&f, "r").run_spot(&req()),
        Err(LaunchError::Other(_))
    ));
}

#[test]
fn run_instances_refuses_on_demand() {
    assert!(run_instances_args(&req(), "on-demand").is_err());
    assert!(run_instances_args(&req(), "spot").is_ok());
}

#[test]
fn open_spot_requests_parses_ids() {
    let f = Fake::new(Ok(
        r#"{"SpotInstanceRequests":[{"SpotInstanceRequestId":"sir-1"},{"SpotInstanceRequestId":"sir-2"}]}"#,
    ));
    let ids = AwsCli::with_runner(&f, "r")
        .open_spot_requests("o")
        .unwrap();
    assert_eq!(ids, vec!["sir-1", "sir-2"]);
    assert!(f.calls.borrow()[0]
        .join(" ")
        .contains("Name=state,Values=open"));
    let f = Fake::new(Err("boom"));
    assert!(AwsCli::with_runner(&f, "r")
        .open_spot_requests("o")
        .is_err());
}

#[test]
fn empty_id_lists_make_no_call() {
    let f = Fake::new(Err("must not be called"));
    let api = AwsCli::with_runner(&f, "r");
    assert_eq!(api.terminate(&[]), Ok(()));
    assert_eq!(api.cancel_spot_requests(&[]), Ok(()));
    assert!(f.calls.borrow().is_empty());
}

#[test]
fn terminate_and_cancel_pass_ids_and_surface_errors() {
    let f = Fake::new(Ok("{}"));
    let api = AwsCli::with_runner(&f, "r");
    api.terminate(&["i-1".into(), "i-2".into()]).unwrap();
    api.cancel_spot_requests(&["sir-1".into()]).unwrap();
    let calls = f.calls.borrow();
    assert!(calls[0]
        .join(" ")
        .contains("terminate-instances --instance-ids i-1 i-2"));
    assert!(calls[1]
        .join(" ")
        .contains("cancel-spot-instance-requests --spot-instance-request-ids sir-1"));
    let f = Fake::new(Err("denied"));
    assert!(AwsCli::with_runner(&f, "r")
        .terminate(&["i-1".into()])
        .is_err());
}

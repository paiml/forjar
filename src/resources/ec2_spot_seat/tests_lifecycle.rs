//! Lifecycle tests against fakes: no AWS, no GitHub.

use super::api::*;
use super::lifecycle::*;
use super::tests::res;
use super::*;
use crate::core::error::{ErrorClass, ForjarError};
use std::cell::RefCell;

/// The only owner tag the fake EC2 holds: boxes and requests carry `Owner=forjar-spot`.
const FAKE_OWNER: &str = "forjar-spot";

#[derive(Default)]
pub(super) struct FakeEc2 {
    pub live: RefCell<Vec<LiveBox>>,
    pub reqs: RefCell<Vec<String>>,
    pub capacity_fail: Vec<String>,
    pub other_fail: bool,
    pub read_fail: bool,
    pub launched: RefCell<Vec<LaunchRequest>>,
    pub calls: RefCell<Vec<String>>,
    pub terminate_noop: bool,
}

impl Ec2Api for FakeEc2 {
    fn live_boxes(&self, owner: &str) -> Result<Vec<LiveBox>, ForjarError> {
        if self.read_fail {
            return Err(ForjarError::connection("throttled"));
        }
        if owner != FAKE_OWNER {
            return Ok(vec![]);
        }
        Ok(self.live.borrow().clone())
    }
    fn run_spot(&self, r: &LaunchRequest) -> Result<String, LaunchError> {
        self.launched.borrow_mut().push(r.clone());
        if self.other_fail {
            return Err(LaunchError::Other("UnauthorizedOperation".into()));
        }
        if self.capacity_fail.contains(&r.instance_type) {
            return Err(LaunchError::Capacity("InsufficientInstanceCapacity".into()));
        }
        self.live.borrow_mut().push(LiveBox {
            id: "i-new".into(),
            price_usd_per_hour: r.price_usd_per_hour,
        });
        Ok("i-new".into())
    }
    fn open_spot_requests(&self, owner: &str) -> Result<Vec<String>, ForjarError> {
        if owner != FAKE_OWNER {
            return Ok(vec![]);
        }
        Ok(self.reqs.borrow().clone())
    }
    fn cancel_spot_requests(&self, ids: &[String]) -> Result<(), ForjarError> {
        self.calls
            .borrow_mut()
            .push(format!("cancel {}", ids.join(",")));
        self.reqs.borrow_mut().clear();
        Ok(())
    }
    fn terminate(&self, ids: &[String]) -> Result<(), ForjarError> {
        self.calls
            .borrow_mut()
            .push(format!("terminate {}", ids.join(",")));
        if !self.terminate_noop {
            self.live.borrow_mut().clear();
        }
        Ok(())
    }
}

struct Sink<'a>(&'a FakeEc2, bool);
impl EvidenceSink for Sink<'_> {
    fn ship(&self, id: &str, dest: &str) -> Result<(), ForjarError> {
        self.0.calls.borrow_mut().push(format!("ship {id} {dest}"));
        if self.1 {
            Err(ForjarError::connection("s3 down"))
        } else {
            Ok(())
        }
    }
}

fn b(id: &str, p: f64) -> LiveBox {
    LiveBox {
        id: id.into(),
        price_usd_per_hour: p,
    }
}

fn spec() -> SpotSeat {
    spec_of(&res()).unwrap()
}

#[test]
fn fake_owner_matches_the_declared_owner() {
    assert_eq!(spec().owner, FAKE_OWNER);
}

#[test]
fn plan_counts_live_boxes() {
    let f = FakeEc2::default();
    f.live.borrow_mut().push(b("i-1", 0.4));
    let p = plan_launch(&spec(), &f).unwrap();
    assert_eq!((p.live_boxes, p.projected_boxes), (1, 2));
    assert!((p.projected_usd_per_hour - 0.9).abs() < 1e-9);
}

#[test]
fn plan_fails_over_box_cap() {
    let f = FakeEc2::default();
    f.live.borrow_mut().extend([b("i-1", 0.1), b("i-2", 0.1)]);
    assert!(plan_launch(&spec(), &f)
        .unwrap_err()
        .message()
        .contains("spot_max_boxes"));
}

#[test]
fn plan_fails_over_usd_cap() {
    let f = FakeEc2::default();
    f.live.borrow_mut().push(b("i-1", 0.6));
    assert!(plan_launch(&spec(), &f)
        .unwrap_err()
        .message()
        .contains("spot_max_usd_per_hour"));
}

#[test]
fn unreadable_fleet_is_not_zero_boxes() {
    let f = FakeEc2 {
        read_fail: true,
        ..Default::default()
    };
    let e = plan_launch(&spec(), &f).unwrap_err();
    assert_eq!(
        e.class(),
        ErrorClass::Connection,
        "the cause keeps its class"
    );
    assert!(e.message().contains("not measured"));
}

#[test]
fn launch_falls_through_capacity_errors_and_stays_spot_only() {
    let f = FakeEc2 {
        capacity_fail: vec!["c6a.4xlarge".into()],
        ..Default::default()
    };
    assert_eq!(launch(&spec(), &f, "t").unwrap(), "i-new");
    let l = f.launched.borrow();
    assert_eq!(l.len(), 2);
    assert_eq!(l[1].instance_type, "m6a.4xlarge");
    assert_ne!(l[0].client_token, l[1].client_token);
}

#[test]
fn launch_stops_on_other_errors_and_when_all_types_are_out() {
    let f = FakeEc2 {
        other_fail: true,
        ..Default::default()
    };
    assert!(launch(&spec(), &f, "t")
        .unwrap_err()
        .message()
        .contains("UnauthorizedOperation"));
    assert_eq!(f.launched.borrow().len(), 1);
    let f = FakeEc2 {
        capacity_fail: vec!["c6a.4xlarge".into(), "m6a.4xlarge".into()],
        ..Default::default()
    };
    assert!(launch(&spec(), &f, "t")
        .unwrap_err()
        .message()
        .contains("no capacity"));
}

#[test]
fn launch_over_cap_never_calls_run() {
    let f = FakeEc2::default();
    f.live.borrow_mut().extend([b("i-1", 0.1), b("i-2", 0.1)]);
    assert!(launch(&spec(), &f, "t").is_err());
    assert!(f.launched.borrow().is_empty());
}

#[test]
fn teardown_ships_then_cancels_then_terminates() {
    let f = FakeEc2::default();
    f.live.borrow_mut().push(b("i-1", 0.4));
    f.reqs.borrow_mut().push("sir-1".into());
    let t = teardown(&spec(), &f, &Sink(&f, false)).unwrap();
    assert_eq!(t.terminated, vec!["i-1"]);
    assert_eq!(
        *f.calls.borrow(),
        vec![
            "ship i-1 s3://evidence-bucket/spot",
            "cancel sir-1",
            "terminate i-1"
        ]
    );
}

#[test]
fn teardown_with_failed_ship_terminates_nothing() {
    let f = FakeEc2::default();
    f.live.borrow_mut().push(b("i-1", 0.4));
    f.reqs.borrow_mut().push("sir-1".into());
    let e = teardown(&spec(), &f, &Sink(&f, true)).unwrap_err();
    assert_eq!(
        e.class(),
        ErrorClass::Connection,
        "the cause keeps its class"
    );
    assert!(e.message().contains("nothing terminated"));
    assert!(!f.calls.borrow().iter().any(|c| c.starts_with("terminate")));
    // a failed ship must not cancel spot requests either (open requests exist here)
    assert!(
        !f.calls.borrow().iter().any(|c| c.starts_with("cancel")),
        "cancel must not run after a failed ship: {:?}",
        f.calls.borrow()
    );
    assert_eq!(
        *f.calls.borrow(),
        vec!["ship i-1 s3://evidence-bucket/spot"]
    );
}

#[test]
fn teardown_fails_when_state_does_not_show_zero() {
    let f = FakeEc2 {
        terminate_noop: true,
        ..Default::default()
    };
    f.live.borrow_mut().push(b("i-1", 0.4));
    assert!(teardown(&spec(), &f, &Sink(&f, false))
        .unwrap_err()
        .message()
        .contains("want 0"));
}

#[test]
fn teardown_needs_an_evidence_destination() {
    let mut r = res();
    r.spot.evidence_dest = None;
    let f = FakeEc2::default();
    assert!(teardown(&spec_of(&r).unwrap(), &f, &Sink(&f, false)).is_err());
}

/// Fake GitHub: `calls` records every delete and mint in the order they happen.
#[derive(Default)]
struct FakeGh {
    runners: RefCell<Vec<Runner>>,
    deleted: RefCell<Vec<u64>>,
    calls: RefCell<Vec<String>>,
}
impl RunnerApi for FakeGh {
    fn list_runners(&self, _o: &str) -> Result<Vec<Runner>, ForjarError> {
        Ok(self.runners.borrow().clone())
    }
    fn delete_runner(&self, _o: &str, id: u64) -> Result<(), ForjarError> {
        self.calls.borrow_mut().push(format!("delete {id}"));
        self.deleted.borrow_mut().push(id);
        self.runners.borrow_mut().retain(|r| r.id != id);
        Ok(())
    }
    fn mint_jit(&self, _o: &str, g: &str, n: &str, l: &[String]) -> Result<String, ForjarError> {
        self.calls.borrow_mut().push(format!("mint {n}"));
        Ok(format!("jit:{g}:{n}:{}", l.join("+")))
    }
}

fn runner(id: u64, name: &str, online: bool, busy: bool) -> Runner {
    Runner {
        id,
        name: name.into(),
        online,
        busy,
    }
}

#[test]
fn stale_offline_registrations_are_removed_but_not_busy_or_foreign_ones() {
    let gh = FakeGh {
        runners: RefCell::new(vec![
            runner(1, "seat-forjar-spot-01", false, false),
            runner(2, "seat-forjar-spot-02", false, true),
            runner(3, "other-box", false, false),
            runner(4, "seat-forjar-spot-03", true, false),
        ]),
        ..Default::default()
    };
    assert_eq!(
        deregister_stale(&spec(), &gh).unwrap(),
        vec!["seat-forjar-spot-01"]
    );
    assert_eq!(*gh.deleted.borrow(), vec![1]);
}

#[test]
fn register_purges_a_stale_offline_seat_before_minting_its_replacement() {
    let gh = FakeGh {
        runners: RefCell::new(vec![runner(8, "seat-forjar-spot-01", false, false)]),
        ..Default::default()
    };
    let out = register(&spec(), &gh).unwrap();
    assert!(
        !gh.deleted.borrow().is_empty(),
        "a stale offline seat must be deregistered"
    );
    assert_eq!(*gh.deleted.borrow(), vec![8]);
    // the purge of the stale registration happens before every replacement mint
    assert_eq!(
        *gh.calls.borrow(),
        vec![
            "delete 8",
            "mint seat-forjar-spot-01",
            "mint seat-forjar-spot-02"
        ]
    );
    assert_eq!(
        out.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
        vec!["seat-forjar-spot-01", "seat-forjar-spot-02"]
    );
}

#[test]
fn a_wrong_owner_sees_no_boxes_and_no_requests_in_plan_or_teardown() {
    let f = FakeEc2::default();
    f.live.borrow_mut().push(b("i-1", 0.4));
    f.reqs.borrow_mut().push("sir-1".into());
    let mut other = spec();
    other.owner = "someone-else".into();
    // the plan counts zero boxes for this owner, so it would not refuse on the box cap
    let p = plan_launch(&other, &f).unwrap();
    assert_eq!((p.live_boxes, p.projected_boxes), (0, 1));
    // teardown touches nothing of this owner: no ship, cancel or terminate
    let t = teardown(&other, &f, &Sink(&f, false)).unwrap();
    assert!(t.shipped.is_empty() && t.cancelled.is_empty() && t.terminated.is_empty());
    assert!(f.calls.borrow().is_empty(), "{:?}", f.calls.borrow());
    assert_eq!(f.live.borrow().len(), 1, "the foreign box is untouched");
}

#[test]
fn register_mints_only_missing_seats_with_the_declared_labels() {
    let gh = FakeGh {
        runners: RefCell::new(vec![runner(9, "seat-forjar-spot-01", true, false)]),
        ..Default::default()
    };
    let out = register(&spec(), &gh).unwrap();
    assert_eq!(
        out,
        vec![(
            "seat-forjar-spot-02".to_string(),
            "jit:tier2:seat-forjar-spot-02:self-hosted+tier2".to_string()
        )]
    );
}

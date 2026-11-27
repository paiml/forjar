//! Unit tests for the ami_bake spot builder plan.

use super::*;

fn good() -> AmiBakeSpec {
    AmiBakeSpec {
        base: Some("ami-0123456789abcdef0".into()),
        image_file: Some("images/fleet.img".into()),
        region: Some("us-east-1".into()),
        fleet_digest: Some(format!("sha256:{}", "a".repeat(64))),
        keep_last: Some(2),
    }
}

#[derive(Default)]
struct Fake {
    calls: Vec<String>,
    fail_at: Option<&'static str>,
}

impl Fake {
    fn step(&mut self, name: &str) -> Result<(), String> {
        self.calls.push(name.to_string());
        if self.fail_at == Some(name) {
            Err(format!("{name} failed"))
        } else {
            Ok(())
        }
    }
}

impl BakeBackend for Fake {
    fn launch_spot(&mut self, _: &str, _: &str) -> Result<(), String> {
        self.step("launch")
    }
    fn bootstrap(&mut self) -> Result<(), String> {
        self.step("bootstrap")
    }
    fn load_image(&mut self, _: &str) -> Result<(), String> {
        self.step("load")
    }
    fn stop_builder(&mut self) -> Result<(), String> {
        self.step("stop")
    }
    fn register_image(&mut self, _: &str, _: &str) -> Result<(), String> {
        self.step("register")
    }
    fn cleanup_old(&mut self, _: u32) -> Result<(), String> {
        self.step("cleanup")
    }
    fn terminate_builder(&mut self) -> Result<(), String> {
        self.step("terminate")
    }
}

#[test]
fn failure_after_launch_terminates_the_builder() {
    let steps = plan_bake(&good()).unwrap();
    let mut f = Fake {
        fail_at: Some("bootstrap"),
        ..Default::default()
    };
    let err = run_bake(&steps, &mut f).unwrap_err();
    assert_eq!(err.0, 1);
    assert_eq!(f.calls, ["launch", "bootstrap", "terminate"]);
}

#[test]
fn launch_failure_does_not_try_to_terminate() {
    let steps = plan_bake(&good()).unwrap();
    let mut f = Fake {
        fail_at: Some("launch"),
        ..Default::default()
    };
    assert!(run_bake(&steps, &mut f).is_err());
    assert_eq!(f.calls, ["launch"]);
}

#[test]
fn refuses_image_path_with_control_characters_or_padding() {
    for name in [
        "images/fleet.img\n",
        " images/fleet.img",
        "x\0.img",
        "a\nb.key",
    ] {
        let mut s = good();
        s.image_file = Some(name.into());
        assert!(
            matches!(
                plan_bake(&s),
                Err(BakePlanError::BadImageFile(_) | BakePlanError::SecretImageFile(_))
            ),
            "{name:?}"
        );
    }
}

#[test]
fn secret_detection_ignores_backslashes_and_trailing_slashes() {
    for name in ["deploy\\.env", "keys\\id_rsa", "ci\\credentials.json"] {
        let mut s = good();
        s.image_file = Some(name.into());
        assert!(
            matches!(plan_bake(&s), Err(BakePlanError::SecretImageFile(_))),
            "{name}"
        );
    }
    // A trailing slash is a directory, refused before the secret check.
    for name in [".env/", "ci\\credentials.json/"] {
        let mut s = good();
        s.image_file = Some(name.into());
        assert!(
            matches!(plan_bake(&s), Err(BakePlanError::BadImageFile(_))),
            "{name}"
        );
    }
}

#[test]
fn plans_steps_in_order() {
    let steps = plan_bake(&good()).expect("valid spec plans");
    assert_eq!(steps.len(), 7);
    assert!(matches!(steps[0], BakeStep::LaunchSpot { .. }));
    assert!(matches!(steps[4], BakeStep::RegisterImage { .. }));
    assert_eq!(steps[5], BakeStep::TerminateBuilder);
    assert_eq!(steps[6], BakeStep::CleanupOld { keep_last: 2 });
}

#[test]
fn run_executes_every_step_in_order() {
    let steps = plan_bake(&good()).unwrap();
    let mut f = Fake::default();
    run_bake(&steps, &mut f).unwrap();
    assert_eq!(
        f.calls,
        [
            "launch",
            "bootstrap",
            "load",
            "stop",
            "register",
            "terminate",
            "cleanup"
        ]
    );
}

#[test]
fn failure_after_builder_terminated_does_not_terminate_twice() {
    let steps = plan_bake(&good()).unwrap();
    let mut f = Fake {
        fail_at: Some("cleanup"),
        ..Default::default()
    };
    assert!(run_bake(&steps, &mut f).is_err());
    assert_eq!(
        f.calls,
        [
            "launch",
            "bootstrap",
            "load",
            "stop",
            "register",
            "terminate",
            "cleanup"
        ]
    );
}

#[test]
fn failed_terminate_step_is_not_retried() {
    let steps = plan_bake(&good()).unwrap();
    let mut f = Fake {
        fail_at: Some("terminate"),
        ..Default::default()
    };
    let err = run_bake(&steps, &mut f).unwrap_err();
    assert_eq!(err.0, 5);
    assert_eq!(
        f.calls,
        [
            "launch",
            "bootstrap",
            "load",
            "stop",
            "register",
            "terminate"
        ]
    );
}

#[test]
fn refuses_nine_char_ami_suffix_and_accepts_seventeen() {
    let mut s = good();
    s.base = Some("ami-012345678".into());
    assert!(matches!(plan_bake(&s), Err(BakePlanError::BadBaseAmi(_))));
    s.base = Some(format!("ami-{}", "0".repeat(17)));
    assert!(plan_bake(&s).is_ok());
}

#[test]
fn refuses_directory_and_degenerate_image_paths() {
    for name in ["/", "images/", ".", "..", "images/.."] {
        let mut s = good();
        s.image_file = Some(name.into());
        assert!(
            matches!(plan_bake(&s), Err(BakePlanError::BadImageFile(_))),
            "{name}"
        );
    }
}

#[test]
fn run_stops_at_first_failure_and_reports_index() {
    let steps = plan_bake(&good()).unwrap();
    let mut f = Fake {
        fail_at: Some("stop"),
        ..Default::default()
    };
    let err = run_bake(&steps, &mut f).unwrap_err();
    assert_eq!(err.0, 3);
    assert_eq!(
        f.calls,
        ["launch", "bootstrap", "load", "stop", "terminate"]
    );
}

#[test]
fn refuses_each_missing_required_field() {
    type Mutate = fn(&mut AmiBakeSpec);
    let cases: [(Mutate, &str); 5] = [
        (|s| s.base = None, "ami_base"),
        (|s| s.image_file = None, "ami_image_file"),
        (|s| s.region = None, "ami_region"),
        (|s| s.fleet_digest = None, "ami_fleet_digest"),
        (|s| s.keep_last = None, "ami_keep_last"),
    ];
    for (mutate, field) in cases {
        let mut s = good();
        mutate(&mut s);
        assert_eq!(
            plan_bake(&s),
            Err(BakePlanError::MissingField(field)),
            "field {field}"
        );
    }
}

#[test]
fn refuses_blank_image_file() {
    let mut s = good();
    s.image_file = Some("   ".into());
    assert_eq!(
        plan_bake(&s),
        Err(BakePlanError::MissingField("ami_image_file"))
    );
}

#[test]
fn refuses_non_ami_base() {
    let mut s = good();
    s.base = Some("ubuntu-22.04".into());
    assert!(matches!(plan_bake(&s), Err(BakePlanError::BadBaseAmi(_))));
}

#[test]
fn refuses_secret_image_file_even_under_a_directory() {
    for name in [
        ".env",
        "deploy/.env.prod",
        "ci/credentials.json",
        "keys/server.pem",
        "id_rsa.pub",
    ] {
        let mut s = good();
        s.image_file = Some(name.into());
        assert!(
            matches!(plan_bake(&s), Err(BakePlanError::SecretImageFile(_))),
            "{name}"
        );
    }
}

#[test]
fn refuses_bad_region() {
    let mut s = good();
    s.region = Some("useast1".into());
    assert!(matches!(plan_bake(&s), Err(BakePlanError::BadRegion(_))));
}

#[test]
fn refuses_digest_without_64_lowercase_hex() {
    let mut s = good();
    s.fleet_digest = Some(format!("sha256:{}", "A".repeat(64)));
    assert!(matches!(plan_bake(&s), Err(BakePlanError::BadDigest(_))));
}

#[test]
fn refuses_keep_last_zero() {
    let mut s = good();
    s.keep_last = Some(0);
    assert_eq!(plan_bake(&s), Err(BakePlanError::KeepLastZero));
}

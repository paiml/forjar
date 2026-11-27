//! #1310: the register-image call, old-AMI cleanup and the no-secrets screen,
//! each run through `run_bake` against a fake backend. No AWS call is made.

use super::*;

fn spec() -> AmiBakeSpec {
    AmiBakeSpec {
        base: Some("ami-0123456789abcdef0".into()),
        image_file: Some("images/fleet.img".into()),
        region: Some("us-east-1".into()),
        fleet_digest: Some(format!("sha256:{}", "c".repeat(64))),
        keep_last: Some(1),
    }
}

const NEW_AMI: &str = "ami-0000000000000000f";

fn old(id: &str, created: u64) -> BakedImage {
    BakedImage {
        ami_id: id.into(),
        created,
        kind: Some(KIND_VALUE.into()),
        fleet_digest: Some(format!("sha256:{}", "d".repeat(64))),
    }
}

#[derive(Default)]
struct Cloud {
    files: Vec<String>,
    roots: Vec<String>,
    returned_id: Option<String>,
    images: Vec<BakedImage>,
    tags: Vec<(&'static str, String)>,
    deregistered: Vec<(String, String)>,
    registered: bool,
    terminated: bool,
}

impl BakeBackend for Cloud {
    fn launch_spot(&mut self, _: &str, _: &str) -> Result<(), BakeFailure> {
        Ok(())
    }
    fn bootstrap(&mut self) -> Result<(), BakeFailure> {
        Ok(())
    }
    fn load_image(&mut self, _: &str) -> Result<(), BakeFailure> {
        Ok(())
    }
    fn builder_files(&mut self, roots: &[&str]) -> Result<Vec<String>, BakeFailure> {
        self.roots = roots.iter().map(ToString::to_string).collect();
        Ok(self.files.clone())
    }
    fn stop_builder(&mut self) -> Result<(), BakeFailure> {
        Ok(())
    }
    fn register_image(
        &mut self,
        _: &str,
        tags: &[(&'static str, String)],
    ) -> Result<String, BakeFailure> {
        self.registered = true;
        self.tags = tags.to_vec();
        Ok(self.returned_id.clone().unwrap_or_else(|| NEW_AMI.into()))
    }
    fn list_images(&mut self, _: &str) -> Result<Vec<BakedImage>, BakeFailure> {
        Ok(self.images.clone())
    }
    fn deregister_image(&mut self, region: &str, ami_id: &str) -> Result<(), BakeFailure> {
        self.deregistered.push((region.into(), ami_id.into()));
        Ok(())
    }
    fn terminate_builder(&mut self) -> Result<(), BakeFailure> {
        self.terminated = true;
        Ok(())
    }
}

fn bake(cloud: &mut Cloud) -> Result<(), (usize, String)> {
    run_bake(&plan_bake(&spec()).unwrap(), cloud)
}

#[test]
fn register_image_receives_the_fleet_digest_and_kind_tags() {
    let mut c = Cloud::default();
    bake(&mut c).unwrap();
    assert_eq!(
        c.tags,
        register_tags(&spec().fleet_digest.unwrap()).unwrap()
    );
}

#[test]
fn a_bad_digest_in_a_hand_built_plan_is_never_registered() {
    let mut steps = plan_bake(&spec()).unwrap();
    for s in &mut steps {
        if let BakeStep::RegisterImage { fleet_digest, .. } = s {
            *fleet_digest = "latest".into();
        }
    }
    let mut c = Cloud::default();
    assert!(run_bake(&steps, &mut c).is_err());
    assert!(!c.registered);
    assert!(c.terminated);
}

#[test]
fn a_secret_on_the_builder_blocks_registration() {
    for path in [
        "/root/.aws/credentials",
        "/home/ubuntu/.ssh/id_ed25519",
        "/opt/runner/.credentials",
        "/home/ubuntu/app/.env",
        "/tmp/deploy.pem",
    ] {
        let mut c = Cloud {
            files: vec!["/opt/runner/run.sh".into(), path.into()],
            ..Default::default()
        };
        let err = bake(&mut c).unwrap_err();
        assert!(err.1.contains(path), "{}", err.1);
        assert!(!c.registered, "{path} was baked");
        assert!(c.terminated, "{path} leaked the builder");
    }
}

#[test]
fn the_secret_screen_covers_every_declared_root() {
    let mut c = Cloud::default();
    bake(&mut c).unwrap();
    assert_eq!(c.roots, SECRET_SCAN_ROOTS);
    for root in ["/root", "/home", "/tmp", "/opt"] {
        assert!(c.roots.iter().any(|r| r == root), "{root}");
    }
}

#[test]
fn a_plan_without_the_secret_screen_is_never_registered() {
    let mut steps = plan_bake(&spec()).unwrap();
    steps.retain(|s| *s != BakeStep::VerifyNoSecrets);
    let mut c = Cloud::default();
    assert!(run_bake(&steps, &mut c).is_err());
    assert!(!c.registered);
}

#[test]
fn cleanup_deregisters_only_the_victims_in_the_declared_region() {
    let mut c = Cloud {
        images: vec![old(NEW_AMI, 300), old("ami-old", 100)],
        ..Default::default()
    };
    bake(&mut c).unwrap();
    assert_eq!(c.deregistered, [("us-east-1".into(), "ami-old".into())]);
}

#[test]
fn cleanup_never_deregisters_the_image_just_registered() {
    // A listing with a skewed creation time must not cost the new image.
    let mut c = Cloud {
        images: vec![old(NEW_AMI, 1), old("ami-old", 100)],
        ..Default::default()
    };
    bake(&mut c).unwrap();
    assert!(c.deregistered.iter().all(|(_, id)| id != NEW_AMI));
}

#[test]
fn keep_last_zero_in_a_hand_built_plan_deregisters_nothing() {
    let mut steps = plan_bake(&spec()).unwrap();
    for s in &mut steps {
        if let BakeStep::CleanupOld { keep_last, .. } = s {
            *keep_last = 0;
        }
    }
    let mut c = Cloud {
        images: vec![old("ami-old", 100)],
        ..Default::default()
    };
    assert!(run_bake(&steps, &mut c).is_err());
    assert!(c.deregistered.is_empty());
}

#[test]
fn a_register_that_returns_no_ami_id_fails_before_cleanup() {
    let mut c = Cloud {
        returned_id: Some(String::new()),
        images: vec![old("ami-old", 100), old("ami-older", 50)],
        ..Default::default()
    };
    assert!(bake(&mut c).is_err());
    assert!(c.deregistered.is_empty());
    assert!(c.terminated);
}

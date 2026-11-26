//! #1266 slice 1: `validate_ami_bake` at parse time. Each guard has a test
//! that fails when the guard is removed.

use super::ami_bake_validate::validate_ami_bake;
use crate::core::types::*;

const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn valid() -> Resource {
    let mut r = Resource {
        resource_type: ResourceType::AmiBake,
        machine: MachineTarget::Single("local".to_string()),
        ..Resource::default()
    };
    r.ami.base = Some("ami-0123456789abcdef0".into());
    r.ami.image_file = Some("fleet/image.tar".into());
    r.ami.region = Some("us-east-1".into());
    r.ami.fleet_digest = Some(DIGEST.into());
    r.ami.keep_last = Some(3);
    r
}

fn errs(r: &Resource) -> Vec<String> {
    let mut e = Vec::new();
    validate_ami_bake("bake", r, &mut e);
    e.into_iter().map(|x| x.message).collect()
}

#[test]
fn a_complete_declaration_is_accepted() {
    assert!(errs(&valid()).is_empty(), "{:?}", errs(&valid()));
}

#[test]
fn each_required_field_is_refused_when_missing() {
    type Clear = fn(&mut Resource);
    let cases: [(Clear, &str); 5] = [
        (|r| r.ami.base = None, "ami_base"),
        (|r| r.ami.image_file = None, "ami_image_file"),
        (|r| r.ami.region = None, "ami_region"),
        (|r| r.ami.fleet_digest = None, "ami_fleet_digest"),
        (|r| r.ami.keep_last = None, "ami_keep_last"),
    ];
    for (clear, field) in cases {
        let mut r = valid();
        clear(&mut r);
        let e = errs(&r);
        assert_eq!(e.len(), 1, "clearing {field}: {e:?}");
        assert!(e[0].contains(field), "clearing {field}: {e:?}");
    }
}

#[test]
fn base_must_be_a_stock_ami_id() {
    let mut r = valid();
    r.ami.base = Some("ubuntu-22.04".into());
    let e = errs(&r);
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(e[0].contains("not an AMI id"), "{e:?}");
}

#[test]
fn a_templated_base_is_accepted_until_resolve() {
    let mut r = valid();
    r.ami.base = Some("{{ params.base_ami }}".into());
    assert!(errs(&r).is_empty(), "{:?}", errs(&r));
}

#[test]
fn a_secret_file_is_never_baked() {
    for path in [
        ".env",
        "deploy/.env.prod",
        "keys/id_rsa",
        "aws_credentials.json",
        "tls/server.pem",
        "keys/deploy.key",
        "{{ params.dir }}/.env",
    ] {
        let mut r = valid();
        r.ami.image_file = Some(path.into());
        let e = errs(&r);
        assert!(e.iter().any(|m| m.contains("secret")), "{path}: {e:?}");
    }
}

#[test]
fn an_ordinary_image_file_is_not_mistaken_for_a_secret() {
    let mut r = valid();
    r.ami.image_file = Some("fleet/envoy-image.tar".into());
    assert!(errs(&r).is_empty(), "{:?}", errs(&r));
}

#[test]
fn region_must_be_a_real_region_shape() {
    let mut r = valid();
    r.ami.region = Some("US East!".into());
    let e = errs(&r);
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(e[0].contains("ami_region"), "{e:?}");
}

#[test]
fn fleet_digest_must_be_sha256_with_64_hex() {
    let md5 = DIGEST.replace("sha256:", "md5:");
    let non_hex = format!("sha256:{}", "z".repeat(64));
    for bad in ["0123", "sha256:abc", md5.as_str(), non_hex.as_str()] {
        let mut r = valid();
        r.ami.fleet_digest = Some(bad.into());
        let e = errs(&r);
        assert_eq!(e.len(), 1, "{bad}: {e:?}");
        assert!(e[0].contains("ami_fleet_digest"), "{bad}: {e:?}");
    }
}

#[test]
fn an_empty_image_file_is_refused() {
    for empty in ["", "   "] {
        let mut r = valid();
        r.ami.image_file = Some(empty.into());
        let e = errs(&r);
        assert_eq!(e.len(), 1, "{empty:?}: {e:?}");
        assert!(e[0].contains("ami_image_file"), "{empty:?}: {e:?}");
    }
}

#[test]
fn the_declaration_is_checked_through_the_type_dispatch() {
    // The real entry point is validate_resource_type, which the match arm feeds.
    use super::resource_types::validate_resource_type;
    let mut r = valid();
    r.ami.base = None;
    let mut e = Vec::new();
    validate_resource_type("bake", &r, &mut e);
    assert!(
        e.iter().any(|x| x.message.contains("ami_base")),
        "dispatch did not reach validate_ami_bake: {:?}",
        e.iter().map(|x| &x.message).collect::<Vec<_>>()
    );
}

#[test]
fn cleanup_can_never_keep_zero_images() {
    let mut r = valid();
    r.ami.keep_last = Some(0);
    let e = errs(&r);
    assert_eq!(e.len(), 1, "{e:?}");
    assert!(e[0].contains("at least 1"), "{e:?}");
}

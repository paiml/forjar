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

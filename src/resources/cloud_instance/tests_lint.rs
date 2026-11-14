//! #1025 follow-ups: every script `cloud_instance` emits must pass the I8 gate
//! (`validate_before_exec`) that stands in front of execution, and uniqueness
//! of `name@region` must hold after templates resolve.

use super::tests::{gated, res};
use super::tests_b1::absent;
use super::*;
use crate::core::purifier::validate_script as validate_before_exec;

fn assert_accepted(which: &str, script: &str) {
    if let Err(e) = validate_before_exec(script) {
        panic!("the I8 gate rejects the {which} script:\n{e}\n--- script ---\n{script}");
    }
}

#[test]
fn every_cloud_instance_script_passes_the_i8_gate() {
    let dir = std::path::Path::new("/tmp/ci-lint");
    for (label, r) in [("present", gated(dir)), ("absent", absent(gated(dir)))] {
        assert_accepted(&format!("{label} apply"), &apply_script(&r));
        assert_accepted(&format!("{label} check"), &check_script(&r));
        assert_accepted(&format!("{label} state_query"), &state_query_script(&r));
    }
    assert_accepted("ungated check", &check_script(&res()));
}

fn two_templated(region_b: &str) -> crate::core::types::ForjarConfig {
    let yaml = format!(
        "version: \"1.0\"\nname: t\nparams:\n  region: eu-south-2\nmachines:\n  m: {{hostname: m, addr: 127.0.0.1}}\nresources:\n{}",
        [("a", "{{params.region}}"), ("b", region_b)]
            .iter()
            .map(|(id, region)| format!(
                "  {id}:\n    type: cloud_instance\n    machine: m\n    provider: aws\n    name: same\n    \
                 image: img@{}\n    cloud_region: \"{region}\"\n    cloud_instance_type: c7a.xlarge\n    \
                 cloud_max_price: \"0.2\"\n    cloud_max_lifetime_min: 60\n",
                super::tests::DIGEST
            ))
            .collect::<String>()
    );
    crate::core::parser::parse_config(&yaml).expect("parses")
}

#[test]
fn a_templated_region_colliding_after_resolution_is_refused() {
    let cfg = two_templated("eu-south-2");
    assert!(
        validate_unique_names(&cfg).is_empty(),
        "parse-time skips templates"
    );
    let errs = validate_unique_names_resolved(&cfg);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(errs[0].contains("already used"), "{errs:?}");
}

#[test]
fn distinct_resolved_regions_are_accepted() {
    let cfg = two_templated("us-east-1");
    assert!(validate_unique_names_resolved(&cfg).is_empty());
}

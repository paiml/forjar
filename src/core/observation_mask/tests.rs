//! Unit tests for the per-field observation mask (forjar#360).

use super::*;
use crate::core::types::{LifecycleRules, MachineTarget, Resource};

fn ignored(fields: &[&str]) -> Vec<String> {
    fields.iter().map(|s| (*s).to_string()).collect()
}

/// The measured stdout of `file::state_query_script` for a regular file.
const FILE_OBSERVATION: &str = "owner=noah group=noah mode=644 size=19\n\
                                44e549f65a3acc7ad10f19e754137dd5110b07607ba5c8ffa12b98478400783a\n";

#[test]
fn an_empty_mask_returns_the_observation_verbatim() {
    // Load-bearing: every resource that does not declare ignore_drift must
    // hash exactly the bytes it always has, or the fleet drifts on upgrade.
    assert_eq!(mask_observation(FILE_OBSERVATION, &[]), FILE_OBSERVATION);
}

#[test]
fn masking_mode_drops_only_the_mode_token() {
    let out = mask_observation(FILE_OBSERVATION, &ignored(&["mode"]));
    assert_eq!(
        out,
        "owner=noah group=noah size=19\n\
         44e549f65a3acc7ad10f19e754137dd5110b07607ba5c8ffa12b98478400783a\n"
    );
}

#[test]
fn a_mode_change_is_invisible_once_mode_is_masked() {
    let before = mask_observation(FILE_OBSERVATION, &ignored(&["mode"]));
    let tampered = FILE_OBSERVATION.replace("mode=644", "mode=600");
    assert_eq!(before, mask_observation(&tampered, &ignored(&["mode"])));
}

#[test]
fn a_content_change_is_still_visible_when_mode_is_masked() {
    let tampered = FILE_OBSERVATION.replace("44e549f6", "deadbeef");
    assert_ne!(
        mask_observation(FILE_OBSERVATION, &ignored(&["mode"])),
        mask_observation(&tampered, &ignored(&["mode"]))
    );
}

#[test]
fn the_existence_sentinel_survives_every_mask() {
    // `echo 'MISSING'` is how a file reports that it is not there. It carries
    // no `=`, so no field list may erase it — masking existence away would turn
    // a deleted file into a clean bill of health.
    for fields in [ignored(&["mode"]), ignored(&["owner", "group", "size"])] {
        assert_eq!(mask_observation("MISSING\n", &fields), "MISSING\n");
    }
}

#[test]
fn a_line_of_only_masked_tokens_is_dropped_not_left_blank() {
    let service = "active=running\nenabled=enabled\n";
    assert_eq!(
        mask_observation(service, &ignored(&["active"])),
        "enabled=enabled\n"
    );
}

#[test]
fn masking_is_token_anchored_not_substring() {
    // A value that merely CONTAINS the field name must survive.
    let line = "owner=mode group=modes mode=644\n";
    assert_eq!(
        mask_observation(line, &ignored(&["mode"])),
        "owner=mode group=modes\n"
    );
}

fn file_resource(ignore: &[&str]) -> Resource {
    Resource {
        resource_type: ResourceType::File,
        machine: MachineTarget::Single("m1".to_string()),
        path: Some("/etc/app.conf".to_string()),
        lifecycle: Some(LifecycleRules {
            ignore_drift: ignore.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn only_vocabulary_entries_become_a_mask() {
    assert_eq!(ignored_fields(&file_resource(&["mode"])), vec!["mode"]);
    // `content` is deliberately outside the File vocabulary; a mask cannot be
    // built from it, so it stays a validation error rather than silently
    // masking nothing.
    assert!(ignored_fields(&file_resource(&["content"])).is_empty());
    // The wildcard suppresses the whole resource elsewhere and must not become
    // a field name here.
    assert!(ignored_fields(&file_resource(&["*"])).is_empty());
}

#[test]
fn the_mask_key_is_order_independent() {
    let a = mask_key(&file_resource(&["mode", "owner"]));
    let b = mask_key(&file_resource(&["owner", "mode"]));
    assert_eq!(a, b);
    assert_eq!(a, "mode,owner");
    assert_eq!(mask_key(&file_resource(&[])), "");
}

/// THE VOCABULARY MUST MATCH THE GENERATORS.
///
/// The table here and the shell in `src/resources/` are two hand-maintained
/// lists; nothing else ties them together. That is the bashrs#266 shape
/// `drift::census`'s module doc warns about — a whitelist and a dispatch that
/// drifted apart while the tests asserted the wrong half. So: for every type
/// with a vocabulary, generate its real state query and prove each field it
/// claims to mask is actually emitted as `field=`.
#[test]
fn every_vocabulary_field_is_emitted_by_its_generator() {
    let file = file_resource(&[]);
    let service = Resource {
        resource_type: ResourceType::Service,
        machine: MachineTarget::Single("m1".to_string()),
        name: Some("nginx".to_string()),
        ..Default::default()
    };
    for resource in [file, service] {
        let vocab = vocabulary(&resource.resource_type).expect("type has a vocabulary");
        let script = crate::core::codegen::state_query_script(&resource).expect("state query");
        for field in vocab {
            assert!(
                script.contains(&format!("{field}=")),
                "{:?} claims to mask {field}, but its state query never emits it:\n{script}",
                resource.resource_type
            );
        }
    }
}

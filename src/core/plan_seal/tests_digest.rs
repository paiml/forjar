//! Unit tests for the three legs and the composition.

use super::digest::*;
use crate::core::plan_selectors::PlanSelectors;
use crate::core::types::{ExecutionPlan, ForjarConfig, PlanAction, PlannedChange, ResourceType};
use std::path::Path;

pub(super) fn cfg(name: &str) -> ForjarConfig {
    let yaml = format!(
        "version: \"1.0\"\n\
         name: {name}\n\
         machines:\n\
         \x20 alpha:\n\
         \x20   hostname: alpha\n\
         \x20   addr: 127.0.0.1\n\
         \x20 bravo:\n\
         \x20   hostname: bravo\n\
         \x20   addr: 127.0.0.2\n\
         resources:\n\
         \x20 a:\n\
         \x20   type: file\n\
         \x20   machine: alpha\n\
         \x20   path: /tmp/forjar-seal-a\n\
         \x20   content: hello\n"
    );
    crate::core::parser::parse_config(&yaml).expect("test config parses")
}

pub(super) fn plan_of(to_create: u32) -> ExecutionPlan {
    ExecutionPlan {
        name: "seal".to_string(),
        changes: (0..to_create)
            .map(|i| PlannedChange {
                resource_id: format!("a{i}"),
                machine: "alpha".to_string(),
                resource_type: ResourceType::File,
                action: PlanAction::Create,
                description: format!("a{i}: create"),
            })
            .collect(),
        execution_order: (0..to_create).map(|i| format!("a{i}")).collect(),
        to_create,
        to_update: 0,
        to_destroy: 0,
        unchanged: 0,
    }
}

fn write_lock(state_dir: &Path, machine: &str, body: &str) {
    let path = crate::core::state::lock_file_path(state_dir, machine);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, body).unwrap();
}

#[test]
fn config_leg_tracks_the_config_and_nothing_else() {
    let a = config_leg(&cfg("one")).unwrap();
    let b = config_leg(&cfg("one")).unwrap();
    let c = config_leg(&cfg("two")).unwrap();
    assert_eq!(a, b, "same config must hash the same");
    assert_ne!(a, c, "a changed config must change the config leg");
}

#[test]
fn state_leg_changes_when_a_lock_changes() {
    let d = tempfile::tempdir().unwrap();
    let sd = d.path();
    let config = cfg("one");

    let empty = state_leg(&config, sd).unwrap();
    write_lock(sd, "alpha", "machine: alpha\nresources: {}\n");
    let seeded = state_leg(&config, sd).unwrap();
    assert_ne!(empty, seeded, "writing a lock must move the state leg");

    write_lock(sd, "alpha", "machine: alpha\nresources: {tampered: 1}\n");
    let tampered = state_leg(&config, sd).unwrap();
    assert_ne!(seeded, tampered, "editing a lock must move the state leg");
}

#[test]
fn an_absent_lock_and_an_empty_lock_do_not_hash_the_same() {
    let d = tempfile::tempdir().unwrap();
    let config = cfg("one");
    let absent = state_leg(&config, d.path()).unwrap();
    write_lock(d.path(), "alpha", "");
    let empty = state_leg(&config, d.path()).unwrap();
    assert_ne!(
        absent, empty,
        "'never applied' and 'applied, empty lock' are different worlds"
    );
}

#[test]
fn state_leg_ignores_a_machine_the_config_does_not_declare() {
    let d = tempfile::tempdir().unwrap();
    let config = cfg("one");
    let before = state_leg(&config, d.path()).unwrap();
    write_lock(d.path(), "ghost", "machine: ghost\n");
    assert_eq!(before, state_leg(&config, d.path()).unwrap());
}

/// The unfiltered selector record, which is what every plan not written with
/// `-m`/`-r`/`-t`/`-g` carries.
fn unfiltered() -> PlanSelectors {
    PlanSelectors::default()
}

/// Refs #358: the diff leg covers the SELECTORS as well as the body, so a
/// document that relabels itself "this was a `-r bravo` plan" to make the
/// re-plan agree with it is a hash mismatch rather than a quiet success.
#[test]
fn diff_leg_changes_with_the_selectors() {
    let plan = plan_of(1);
    let narrow = PlanSelectors::new(None, Some("bravo"), None, None);
    assert_ne!(
        diff_leg(&plan, &unfiltered()).unwrap(),
        diff_leg(&plan, &narrow).unwrap(),
        "the selectors are part of the sealed body"
    );
}

/// Framing: a selector record must not be swappable for a change list that
/// happens to serialise to the same bytes.
#[test]
fn the_body_and_the_selectors_are_framed_apart() {
    let a = PlanSelectors::new(Some("web"), None, None, None);
    let b = PlanSelectors::new(None, Some("web"), None, None);
    assert_ne!(
        diff_leg(&plan_of(1), &a).unwrap(),
        diff_leg(&plan_of(1), &b).unwrap()
    );
}

#[test]
fn diff_leg_changes_with_the_body() {
    assert_eq!(
        diff_leg(&plan_of(1), &unfiltered()).unwrap(),
        diff_leg(&plan_of(1), &unfiltered()).unwrap()
    );
    assert_ne!(
        diff_leg(&plan_of(1), &unfiltered()).unwrap(),
        diff_leg(&plan_of(2), &unfiltered()).unwrap()
    );
}

#[test]
fn diff_leg_changes_when_only_a_counter_is_edited() {
    let honest = plan_of(1);
    let mut lying = honest.clone();
    lying.to_create = 0;
    assert_ne!(
        diff_leg(&honest, &unfiltered()).unwrap(),
        diff_leg(&lying, &unfiltered()).unwrap(),
        "the counters are part of the sealed body"
    );
}

#[test]
fn composition_is_not_confusable_by_concatenation() {
    assert_ne!(
        compose("ab", "c", "d", 0, 0),
        compose("a", "bc", "d", 0, 0),
        "NUL delimiters must make the split unambiguous"
    );
}

#[test]
fn composition_covers_the_validity_window() {
    let base = compose("c", "s", "d", 1_000_000, 0);
    assert_ne!(
        base,
        compose("c", "s", "d", 1_000_001, 0),
        "sealed_at is inside"
    );
    assert_ne!(
        base,
        compose("c", "s", "d", 1_000_000, 900),
        "ttl is inside"
    );
}

#[test]
fn plan_id_is_sixteen_bytes_of_the_seal() {
    let seal = compose("c", "s", "d", 1, 2);
    let id = plan_id(&seal);
    assert_eq!(id.len(), 32, "16 bytes, hex");
    assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    let hex = seal.strip_prefix("blake3:").expect("prefixed");
    assert!(hex.starts_with(&id), "the id is the seal's own prefix");
}

#[test]
fn legs_are_stable_across_repeated_computation() {
    let d = tempfile::tempdir().unwrap();
    write_lock(d.path(), "alpha", "machine: alpha\nresources: {}\n");
    let config = cfg("stable");
    let plan = plan_of(3);
    let first = (
        config_leg(&config).unwrap(),
        state_leg(&config, d.path()).unwrap(),
        diff_leg(&plan, &unfiltered()).unwrap(),
    );
    for _ in 0..50 {
        assert_eq!(config_leg(&config).unwrap(), first.0);
        assert_eq!(state_leg(&config, d.path()).unwrap(), first.1);
        assert_eq!(diff_leg(&plan, &unfiltered()).unwrap(), first.2);
    }
}

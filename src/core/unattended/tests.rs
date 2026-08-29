//! forjar#372: unit cover for the sanitiser. The end-to-end proof that the
//! SHIPPED binary no longer executes any of this lives in
//! `tests/falsification_readonly_surface_executes_nothing.rs`, which drives
//! real `forjar mcp` stdio against a hostile config.

use super::*;
use crate::core::types::{ForjarConfig, Machine, Resource, ResourceType};

fn cfg_with(provider: Option<&str>, resource: Resource) -> ForjarConfig {
    let mut c = ForjarConfig {
        name: "t".into(),
        ..Default::default()
    };
    c.secrets.provider = provider.map(str::to_string);
    c.machines
        .insert("m".into(), Machine::ssh("m", "127.0.0.1", "root"));
    c.resources.insert("r".into(), resource);
    c
}

fn task(f: impl FnOnce(&mut Resource)) -> Resource {
    let mut r = Resource {
        resource_type: ResourceType::Task,
        ..Default::default()
    };
    f(&mut r);
    r
}

#[test]
fn ambient_inputs_are_removed_and_named() {
    let c = cfg_with(
        None,
        task(|r| r.ambient_inputs = vec!["touch PWNED".into(), "id".into()]),
    );
    let (out, skipped) = sanitize_config(&c);
    assert!(
        out.resources["r"].ambient_inputs.is_empty(),
        "a command left in ambient_inputs is a command the probe will run"
    );
    assert_eq!(
        skipped.len(),
        1,
        "the removal must be disclosed: {skipped:?}"
    );
    assert!(skipped[0].contains("ambient_inputs"), "{skipped:?}");
}

#[test]
fn a_subprocess_secret_provider_is_replaced_with_one_that_cannot_exec() {
    for provider in ["sops", "op"] {
        let (out, skipped) = sanitize_config(&cfg_with(Some(provider), task(|_| {})));
        assert_eq!(
            out.secrets.provider.as_deref(),
            Some(NO_EXEC_SECRET_PROVIDER),
            "{provider} must not survive into the planner"
        );
        assert!(
            skipped.iter().any(|s| s.contains(provider)),
            "the skipped provider must be named: {skipped:?}"
        );
    }
}

#[test]
fn env_and_file_providers_are_untouched() {
    // The guard against "fixed" meaning "no secret ever resolves". Neither of
    // these spawns anything, so neither is this file's business.
    for provider in [None, Some("env"), Some("file")] {
        let (out, skipped) = sanitize_config(&cfg_with(provider, task(|_| {})));
        assert_eq!(out.secrets.provider.as_deref(), provider);
        assert!(skipped.is_empty(), "{provider:?} skipped {skipped:?}");
    }
}

#[test]
fn an_output_normaliser_is_downgraded_to_none_not_to_bytes() {
    let c = cfg_with(
        None,
        task(|r| {
            r.output_artifacts = vec!["a.bin".into()];
            r.output_equivalence.insert(
                "a.bin".into(),
                OutputEquivalence::Command("touch PWNED".into()),
            );
        }),
    );
    let (out, skipped) = sanitize_config(&c);
    assert_eq!(
        out.resources["r"].output_equivalence["a.bin"],
        OutputEquivalence::None,
        "`bytes` would compare content under a predicate the author replaced; \
         `none` says truthfully that the content was not compared"
    );
    assert_eq!(skipped.len(), 1, "{skipped:?}");
}

#[test]
fn a_clean_config_is_unchanged_and_discloses_nothing() {
    // Without this the fix could be "always disclose", which teaches an agent
    // to ignore the field.
    let c = cfg_with(Some("env"), task(|r| r.task_inputs = vec!["src/**".into()]));
    let (out, skipped) = sanitize_config(&c);
    assert_eq!(out.resources["r"].task_inputs, vec!["src/**".to_string()]);
    assert!(skipped.is_empty());
    assert_eq!(disclosure(&skipped), None);
}

#[test]
fn the_disclosure_names_the_command_that_can_answer_instead() {
    let d = disclosure(&["r: 1 ambient_inputs command(s) not executed".into()])
        .expect("a non-empty skip list must disclose");
    assert!(d.contains("forjar drift"), "{d}");
    assert!(d.contains("lock-relative"), "{d}");
}

#[test]
fn the_two_disclosures_compose_rather_than_overwrite() {
    let scope = Some("scope sentence.".to_string());
    let un = Some("unattended sentence".to_string());
    let merged = merge_disclosures(scope, un).expect("both present");
    assert!(
        merged.contains("scope sentence") && merged.contains("unattended sentence"),
        "a consumer reading only `disclosure` must learn both: {merged}"
    );
    assert_eq!(merge_disclosures(None, None), None);
}

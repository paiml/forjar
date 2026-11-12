//! `apply -t <tag>` must honour `depends_on`: the tagged resources' prerequisite
//! closure is selected too, otherwise a tagged deploy converges `b` (the tagged
//! file) against an `a` (the untagged directory/package it needs) that was never
//! applied. Choice: pull the closure IN (as `-r` and `--subset` already do)
//! rather than refuse on an unmet dependency.

use super::apply_selection::{resolve_selection, Selectors};
use crate::core::types;

fn res(deps: &[&str], tags: &[&str]) -> types::Resource {
    types::Resource {
        resource_type: types::ResourceType::File,
        machine: types::MachineTarget::Multiple(vec!["local".to_string()]),
        depends_on: deps.iter().map(|d| (*d).to_string()).collect(),
        tags: tags.iter().map(|t| (*t).to_string()).collect(),
        ..Default::default()
    }
}

/// `b` (tagged `deploy`) depends on `a` (untagged); `c` is unrelated and untagged.
fn cfg() -> types::ForjarConfig {
    let mut config = types::ForjarConfig {
        version: "1.0".to_string(),
        name: "tag-closure".to_string(),
        ..Default::default()
    };
    config.machines.insert(
        "local".to_string(),
        serde_yaml_ng::from_str("hostname: h\naddr: localhost\n").expect("machine parses"),
    );
    config.resources.insert("a".to_string(), res(&[], &[]));
    config
        .resources
        .insert("b".to_string(), res(&["a"], &["deploy"]));
    config.resources.insert("c".to_string(), res(&[], &[]));
    config
}

#[test]
fn tag_selection_pulls_in_untagged_dependency() {
    let mut config = cfg();
    let sel = Selectors {
        tag: Some("deploy"),
        ..Default::default()
    };
    let out = resolve_selection(&mut config, &sel, false).expect("resolves");
    let mut ids: Vec<&str> = config.resources.keys().map(String::as_str).collect();
    ids.sort_unstable();
    assert_eq!(ids, ["a", "b"], "a is b's prerequisite; c is unrelated");
    assert_eq!(out.selected, 1);
    assert_eq!(out.dependencies_added, 1);
}

#[test]
fn no_tag_selects_everything() {
    let mut config = cfg();
    resolve_selection(&mut config, &Selectors::default(), false).expect("resolves");
    assert_eq!(config.resources.len(), 3);
}

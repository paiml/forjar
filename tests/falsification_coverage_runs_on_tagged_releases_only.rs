//! forjar#604: Coverage and Benchmarks run on tagged releases only.
//!
//! # The decision
//!
//! Operator decision 2026-09-21, "YES, coverage on tags release only"
//! (paiml/aprender#3676). `coverage.yml` and `bench.yml` ran on every push to
//! main and every PR, each holding a bare-metal intel runner for ~50 minutes on
//! a fleet shared with every other paiml repo. They now run on a `v*` tag push
//! and on a deliberate `workflow_dispatch`, and nowhere else.
//!
//! sovereign-ci's own coverage job follows the same rule through its
//! `coverage_on: tag` input (paiml/.github#74), and opting in is TWO edits in
//! the caller: the input AND a `v*` tag trigger on the caller's own `push:`. A
//! `push:` filtered to branches never fires for a tag, so the input alone
//! would leave coverage running on manual dispatch only — never on a release.
//!
//! # What this does NOT weaken
//!
//! Nothing ships unmeasured: gate F (`scripts/dogfood/coverage.sh`) enforces
//! the 95% line floor and runs `cargo mutants` before every tag, and on the
//! tag itself sovereign-ci's gate is RED when coverage is skipped.
//!
//! # Why this parses the YAML
//!
//! Every assertion reads a parsed field (`on`, `jobs.ci.with`), which a comment
//! cannot satisfy — the forjar#567 lesson. Each predicate is also driven with a
//! fabricated trigger block that must FAIL it, so a predicate that accepts
//! everything is caught here rather than by a reader.
//!
//! # mutations — one per arm
//!
//! 1. Add `pull_request:` back to coverage.yml's `on:` →
//!    `coverage_and_benchmarks_trigger_on_tags_and_dispatch_only` goes RED.
//! 2. Add `branches: [main]` under bench.yml's `push:` → the same test goes RED.
//! 3. Delete `coverage_on: tag` from ci.yml →
//!    `sovereign_ci_coverage_is_opted_into_tags_only` goes RED.
//! 4. Delete `tags: ['v*']` from ci.yml's `push:` → the same test goes RED:
//!    the input alone never fires on a release.

use serde_yaml_ng::Value;

fn parse(text: &str) -> Value {
    serde_yaml_ng::from_str(text).unwrap_or_else(|e| panic!("parse: {e}\n{text}"))
}

fn workflow(name: &str) -> Value {
    let path = format!("{}/.github/workflows/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    parse(&text)
}

fn string_seq(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_sequence)
        .map(|s| {
            s.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn trigger_names(on: &Value) -> Vec<String> {
    on.as_mapping()
        .map(|m| {
            m.keys()
                .filter_map(|k| k.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn push_is_tags_only(on: &Value) -> bool {
    let Some(push) = on.get("push").and_then(Value::as_mapping) else {
        return false;
    };
    let keys: Vec<&str> = push.keys().filter_map(Value::as_str).collect();
    keys == ["tags"] && string_seq(on.get("push").and_then(|p| p.get("tags"))) == ["v*"]
}

/// `None` when the workflow runs on a `v*` tag push and `workflow_dispatch`
/// and on nothing else; otherwise the reason it does not.
fn tags_and_dispatch_only(wf: &Value) -> Option<String> {
    let Some(on) = wf.get("on") else {
        return Some("no `on:` block".to_string());
    };
    let mut names = trigger_names(on);
    names.sort();
    if names != ["push", "workflow_dispatch"] {
        return Some(format!(
            "triggers are {names:?}, want [push, workflow_dispatch]"
        ));
    }
    if !push_is_tags_only(on) {
        return Some(format!(
            "push is {:?}, want tags: ['v*'] alone",
            on.get("push")
        ));
    }
    None
}

/// `None` when ci.yml hands sovereign-ci `coverage_on: tag` AND its own
/// `push:` fires on a `v*` tag; otherwise the reason it does not.
fn coverage_opted_into_tags(wf: &Value) -> Option<String> {
    let with = wf
        .get("jobs")
        .and_then(|j| j.get("ci"))
        .and_then(|c| c.get("with"));
    let input = with
        .and_then(|w| w.get("coverage_on"))
        .and_then(Value::as_str);
    if input != Some("tag") {
        return Some(format!(
            "jobs.ci.with.coverage_on is {input:?}, want \"tag\""
        ));
    }
    let tags = string_seq(
        wf.get("on")
            .and_then(|o| o.get("push"))
            .and_then(|p| p.get("tags")),
    );
    if !tags.iter().any(|t| t == "v*") {
        return Some(format!(
            "on.push.tags is {tags:?}: the input alone never fires on a tag"
        ));
    }
    None
}

#[test]
fn coverage_and_benchmarks_trigger_on_tags_and_dispatch_only() {
    for name in ["coverage.yml", "bench.yml"] {
        if let Some(why) = tags_and_dispatch_only(&workflow(name)) {
            panic!("{name} must run on tagged releases only (forjar#604): {why}");
        }
    }
}

#[test]
fn sovereign_ci_coverage_is_opted_into_tags_only() {
    if let Some(why) = coverage_opted_into_tags(&workflow("ci.yml")) {
        panic!("ci.yml (forjar#604): {why}");
    }
}

#[test]
fn the_trigger_predicate_refuses_every_shape_the_old_file_had() {
    let good = "on:\n  push:\n    tags: ['v*']\n  workflow_dispatch:\n";
    assert_eq!(
        tags_and_dispatch_only(&parse(good)),
        None,
        "control must pass"
    );
    let refused = [
        "on:\n  push:\n    branches: [main]\n  pull_request:\n    branches: [main]\n",
        "on:\n  push:\n    tags: ['v*']\n  pull_request:\n  workflow_dispatch:\n",
        "on:\n  push:\n    branches: [main]\n    tags: ['v*']\n  workflow_dispatch:\n",
        "on:\n  push:\n    tags: ['*']\n  workflow_dispatch:\n",
        "on:\n  push:\n  workflow_dispatch:\n",
        "on:\n  schedule:\n    - cron: '0 0 * * *'\n",
        "name: no trigger\n",
    ];
    for text in refused {
        assert!(
            tags_and_dispatch_only(&parse(text)).is_some(),
            "predicate accepted a trigger block it must refuse:\n{text}"
        );
    }
}

#[test]
fn the_opt_in_predicate_needs_both_edits() {
    let both = "on:\n  push:\n    branches: [main]\n    tags: ['v*']\njobs:\n  ci:\n    with:\n      coverage_on: tag\n";
    assert_eq!(
        coverage_opted_into_tags(&parse(both)),
        None,
        "control must pass"
    );
    let refused = [
        // the input alone: coverage would run on manual dispatch only
        "on:\n  push:\n    branches: [main]\njobs:\n  ci:\n    with:\n      coverage_on: tag\n",
        // the trigger alone: coverage runs on every event, as before
        "on:\n  push:\n    tags: ['v*']\njobs:\n  ci:\n    with:\n      use_nextest: true\n",
        "on:\n  push:\n    tags: ['v*']\njobs:\n  ci:\n    with:\n      coverage_on: always\n",
    ];
    for text in refused {
        assert!(
            coverage_opted_into_tags(&parse(text)).is_some(),
            "predicate accepted an opt-in it must refuse:\n{text}"
        );
    }
}

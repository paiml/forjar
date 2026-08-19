//! GH-241: purity's monotonicity invariant must hold on real recipes.
//!
//! `src/core/store/purity.rs` documents the rule —
//!
//! > A recipe's purity level is the **maximum** (least pure) of all its
//! > transitive dependencies (monotonicity invariant).
//!
//! — and `classify` implements it as `max(own_level, max(dep_levels))`. It was
//! unit-tested and correct. The only production caller then passed
//! `dep_levels: vec![]`, unconditionally, so `max(dep_levels)` was always `None`
//! and the rule never fired outside those unit tests. A Pure resource depending
//! on an Impure one reported Pure.
//!
//! That is the shape worth guarding against: an invariant that is documented,
//! implemented, unit-tested, and then bypassed at the one place it decides
//! anything. These tests drive the real binary over real config files, so they
//! cannot pass by exercising `classify` directly.

use std::process::Command;

fn run(yaml: &str, extra: &[&str]) -> (bool, String) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .arg("validate")
        .arg("-f")
        .arg(&file)
        .arg("--check-recipe-purity")
        .args(extra)
        .output()
        .expect("forjar must run");
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

const HEAD: &str = r#"version: "1.0"
name: purity-monotonicity
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    user: noah
resources:
"#;

/// A pinned resource that depends on a curl|bash resource.
fn pinned_depending_on_impure() -> String {
    format!(
        "{HEAD}{}",
        r#"  danger:
    type: task
    machine: local
    command: "curl -sSfL https://example.com/install.sh | bash"
  downstream:
    type: task
    machine: local
    version: "1.0"
    store: true
    command: "echo downstream"
    depends_on:
      - danger
"#
    )
}

#[test]
fn a_resource_is_at_least_as_impure_as_its_dependency() {
    // The regression. `downstream` is version-pinned and store-backed — on its
    // own signals it classifies well above Impure. It depends on a curl|bash
    // task, so monotonicity must drag it down.
    let (_, out) = run(&pinned_depending_on_impure(), &[]);
    let line = out
        .lines()
        .find(|l| l.trim_start().starts_with("downstream:"))
        .unwrap_or_else(|| panic!("no downstream line in:\n{out}"));
    assert!(
        line.contains("Impure"),
        "a resource depending on an Impure one must not report better than \
         Impure — this is `dep_levels: vec![]` again: {line}"
    );
}

#[test]
fn the_elevation_is_explained_not_silent() {
    // A level that changed because of a dependency, with no reason given, is
    // indistinguishable from a misclassification. `classify` already emits the
    // reason; this pins that it survives to the report.
    let (_, out) = run(&pinned_depending_on_impure(), &[]);
    assert!(
        out.contains("dependency at level Impure elevates purity"),
        "the monotonicity reason must reach the report:\n{out}"
    );
}

#[test]
fn monotonicity_is_transitive_across_a_chain() {
    // a -> b -> c. Only `a` is impure by its own signals. Classifying in
    // dependency order makes the propagation transitive; classifying in map
    // order would leave `c` clean roughly half the time, which is a flake, not
    // a failure.
    let yaml = format!(
        "{HEAD}{}",
        r#"  a:
    type: task
    machine: local
    command: "curl -sSfL https://example.com/x.sh | bash"
  b:
    type: task
    machine: local
    version: "1.0"
    store: true
    command: "echo b"
    depends_on:
      - a
  c:
    type: task
    machine: local
    version: "1.0"
    store: true
    command: "echo c"
    depends_on:
      - b
"#
    );
    let (_, out) = run(&yaml, &[]);
    for name in ["a", "b", "c"] {
        let line = out
            .lines()
            .find(|l| l.trim_start().starts_with(&format!("{name}:")))
            .unwrap_or_else(|| panic!("no {name} line in:\n{out}"));
        assert!(
            line.contains("Impure"),
            "monotonicity must reach {name} transitively: {line}"
        );
    }
}

#[test]
fn an_independent_resource_is_not_dragged_down() {
    // The converse. If propagation were implemented by taking the recipe-wide
    // worst level and stamping it on everything, the tests above would pass and
    // the report would be useless.
    let yaml = format!(
        "{HEAD}{}",
        r#"  danger:
    type: task
    machine: local
    command: "curl -sSfL https://example.com/x.sh | bash"
  unrelated:
    type: task
    machine: local
    version: "1.0"
    store: true
    command: "echo unrelated"
"#
    );
    let (_, out) = run(&yaml, &[]);
    let line = out
        .lines()
        .find(|l| l.trim_start().starts_with("unrelated:"))
        .unwrap_or_else(|| panic!("no unrelated line in:\n{out}"));
    assert!(
        !line.contains("Impure"),
        "a resource with no dependency on the impure one must keep its own \
         level; propagation is per-edge, not recipe-wide: {line}"
    );
}

#[test]
fn purity_is_only_a_gate_when_you_ask_for_one() {
    // `--check-recipe-purity` alone exits 0 even on the worst result. That is
    // deliberate and now documented — but it means the flag cannot gate CI
    // without parsing JSON, which is what `--min-purity` is for.
    let (ok_without, _) = run(&pinned_depending_on_impure(), &[]);
    assert!(
        ok_without,
        "the bare report must keep exiting 0; it is a report"
    );

    let (ok_with, out) = run(&pinned_depending_on_impure(), &["--min-purity", "pinned"]);
    assert!(
        !ok_with,
        "--min-purity pinned must FAIL an Impure recipe, or it gates nothing:\n{out}"
    );
}

#[test]
fn a_recipe_that_meets_the_threshold_passes_the_gate() {
    // The gate must be passable, or it trains people to remove it.
    let yaml = format!(
        "{HEAD}{}",
        r#"  fine:
    type: task
    machine: local
    version: "1.0"
    store: true
    command: "echo fine"
"#
    );
    let (ok, out) = run(&yaml, &["--min-purity", "pinned"]);
    assert!(ok, "a Pinned recipe must pass --min-purity pinned:\n{out}");
}

#[test]
fn the_json_report_carries_the_verdict() {
    // A CI consumer reads JSON. If `pass` were absent it would have to
    // re-derive the comparison, which is how the "parse the JSON yourself"
    // workaround got established in the first place.
    let (_, out) = run(
        &pinned_depending_on_impure(),
        &["--json", "--min-purity", "pinned"],
    );
    assert!(out.contains("\"pass\""), "JSON must report pass:\n{out}");
    assert!(
        out.contains("\"min_purity\""),
        "JSON must report the threshold it was judged against:\n{out}"
    );
}

/// GH-241 follow-up finding: `Pure` is currently unreachable.
///
/// `classify` awards `Pure` only when `has_sandbox` is true, and `has_sandbox`
/// reads a `sandbox:` key on the resource. `sandbox` is **not** in
/// `RESOURCE_FIELDS`, so the parser rejects any config that sets it:
///
/// ```text
/// error: unknown field errors:
///   - unknown field 'sandbox' at 'resources.downstream.sandbox'
/// ```
///
/// Every resource in a config that actually validates therefore tops out at
/// `Pinned`. This test pins the finding rather than papering over it: adding
/// `sandbox` to the resource schema would be inventing a feature, and changing
/// the classifier's ladder would change what every existing report means.
/// Whichever way that is resolved, this test must be updated deliberately.
#[test]
fn pure_is_unreachable_because_sandbox_is_not_a_resource_field() {
    let yaml = format!(
        "{HEAD}{}",
        r#"  best_effort:
    type: task
    machine: local
    version: "1.0"
    store: true
    sandbox: {}
    command: "echo best"
"#
    );
    let (ok, out) = run(&yaml, &[]);
    assert!(
        !ok && out.contains("unknown field 'sandbox'"),
        "if `sandbox` became a real resource field, the Pure level is now \
         reachable and this test and the purity ladder need revisiting:\n{out}"
    );
}

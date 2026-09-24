//! #615: `plan -r` and `apply -r` selected different resource sets.
//!
//! `apply` resolves `-r`/`-g` through `resolve_selection`, which closes the
//! positive set over `depends_on` (FJ-331, #468). `plan` kept its GH-214
//! exact-id retain, so on a two-resource chain the two disagreed. Measured on
//! 1.32.0 with `leaf-file` depending on `base-dir`:
//!
//! ```text
//!   $ forjar plan  -r leaf-file              → 1 to add   (leaf-file)
//!   $ forjar apply -r leaf-file --dry-run    → 2 to add   (base-dir, leaf-file)
//! ```
//!
//! The plan an operator reviews was not the set the apply would converge, and
//! `apply --plan-file` re-plans through the same `plan_filtered` to check a
//! saved `-r` plan still holds.
//!
//! Every assertion below compares against `resolve_selection` itself — the
//! apply path — not against a hand-written expected list, so a change to
//! apply's semantics moves both sides together and a second predicate on
//! either side goes red.

use super::apply_selection::{resolve_selection, Selectors};
use crate::core::plan_selectors::PlanSelectors;
use crate::core::types::ForjarConfig;
use std::collections::{BTreeSet, HashMap};

fn chain_config() -> ForjarConfig {
    serde_yaml_ng::from_str(
        "version: \"1.0\"\n\
         name: fj-615\n\
         machines:\n\
         \x20 local:\n\
         \x20   hostname: localhost\n\
         \x20   addr: 127.0.0.1\n\
         \x20   user: nobody\n\
         \x20   arch: x86_64\n\
         resources:\n\
         \x20 base-dir:\n\
         \x20   type: file\n\
         \x20   machine: local\n\
         \x20   state: directory\n\
         \x20   path: /tmp/fj-615/out\n\
         \x20 leaf-file:\n\
         \x20   type: file\n\
         \x20   machine: local\n\
         \x20   resource_group: alpha\n\
         \x20   path: /tmp/fj-615/out/leaf.txt\n\
         \x20   content: \"leaf\\n\"\n\
         \x20   depends_on: [base-dir]\n\
         \x20 other-file:\n\
         \x20   type: file\n\
         \x20   machine: local\n\
         \x20   path: /tmp/fj-615/other.txt\n\
         \x20   content: \"other\\n\"\n",
    )
    .expect("fixture parses")
}

/// The set `apply` converges under these selectors.
fn apply_set(resource: Option<&str>, group: Option<&str>) -> BTreeSet<String> {
    let mut config = chain_config();
    let sel = Selectors {
        resource,
        group,
        ..Default::default()
    };
    resolve_selection(&mut config, &sel, false).expect("apply selection resolves");
    config.resources.keys().cloned().collect()
}

/// The set `plan` shows under the same selectors (empty lock: all creates).
fn plan_set(resource: Option<&str>, group: Option<&str>) -> (BTreeSet<String>, BTreeSet<String>) {
    let sel = PlanSelectors::new(None, resource, None, group);
    let plan = super::plan_compute::plan_filtered(&chain_config(), &HashMap::new(), &sel)
        .expect("plan resolves");
    let changes = plan.changes.iter().map(|c| c.resource_id.clone()).collect();
    let order = plan.execution_order.iter().cloned().collect();
    (changes, order)
}

fn assert_same_selection(resource: Option<&str>, group: Option<&str>) {
    let want = apply_set(resource, group);
    assert!(
        want.contains("base-dir"),
        "fixture precondition: apply closes over depends_on, got {want:?}"
    );
    let (changes, order) = plan_set(resource, group);
    assert_eq!(
        changes, want,
        "plan -r {resource:?} -g {group:?} must list exactly what apply converges"
    );
    assert_eq!(order, want, "execution_order must be the same set");
}

#[test]
fn plan_r_selects_the_same_set_as_apply_r() {
    // 1.32.0: plan listed {leaf-file}, apply converged {base-dir, leaf-file}.
    assert_same_selection(Some("leaf-file"), None);
}

#[test]
fn plan_g_selects_the_same_set_as_apply_g() {
    // Same defect via the group selector: `alpha` holds only leaf-file.
    assert_same_selection(None, Some("alpha"));
}

#[test]
fn plan_counters_agree_with_the_closed_selection() {
    let sel = PlanSelectors::new(None, Some("leaf-file"), None, None);
    let plan = super::plan_compute::plan_filtered(&chain_config(), &HashMap::new(), &sel)
        .expect("plan resolves");
    assert_eq!(
        plan.to_create, 2,
        "the summary line counts the dependency too"
    );
}

#[test]
fn an_unknown_r_is_still_an_error_with_the_known_list() {
    let sel = PlanSelectors::new(None, Some("leaf-fil"), None, None);
    let err = super::plan_compute::plan_filtered(&chain_config(), &HashMap::new(), &sel)
        .expect_err("a typo must not be an empty successful plan");
    assert!(
        err.contains("--resource 'leaf-fil' matches no resource"),
        "{err}"
    );
    assert!(err.contains("base-dir, leaf-file, other-file"), "{err}");
}

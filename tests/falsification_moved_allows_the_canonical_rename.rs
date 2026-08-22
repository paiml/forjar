//! A `moved` block whose `to` is the renamed resource must VALIDATE.
//!
//! THE BUG.
//!
//! `moved` exists to rename a resource without a destroy+create cycle. The
//! canonical usage — the only usage, really — is:
//!
//!     moved:
//!       - from: webserver-pkg
//!         to:   nginx-web
//!     resources:
//!       nginx-web: { ... }        # the SAME resource, under its new name
//!
//! The `to` target MUST be declared in config; otherwise the rename points at
//! nothing and the resource would be destroyed. That is how OpenTofu/Terraform
//! `moved` blocks work and how forjar's own cookbook documents them
//! (paiml/forjar-cookbook recipes/34-moved-blocks.yaml).
//!
//! `validate_moved_blocks` rejected exactly that:
//!
//!     moved 'to: nginx-web' collides with existing resource 'nginx-web' —
//!     renaming onto a managed resource would overwrite its converged state
//!
//! WHY THE CHECK CANNOT WORK WHERE IT SITS.
//!
//! "Overwrite its converged state" is a claim about the LOCK. The check tests
//! `config.resources`. Those differ precisely where it matters:
//!
//!     canonical rename   config has `to`: YES   lock has `to`: no    safe
//!     genuine clobber    config has `to`: YES   lock has `to`: YES   destructive
//!
//! Config-contains-`to` is true in BOTH, so it cannot discriminate — and since
//! the safe case is the normal case, the rule rejects every correct use of the
//! feature. It is a proxy standing in for the thing it means to test, which is
//! the same shape as `mountpoint -q` standing in for "the declared filesystem
//! is mounted".
//!
//! The real collision is detectable only against the lock, which validation
//! does not have. `apply_moved_blocks` does, and defends there.
//!
//! The rule WAS tested — by two tests asserting it rejects the canonical
//! rename (`validate_rejects_to_colliding_with_managed_resource` and its
//! post-expansion twin from #165). They passed for as long as the rule existed
//! and encoded its premise as the expected result, which is how a rule
//! forbidding its feature's only correct usage shipped in 1.15.0. Both are now
//! inverted. A test can only guard behaviour someone questioned first.

use forjar::core::parser::validate_config;

fn cfg(moved: &str, resources: &str) -> String {
    format!(
        r#"version: "1.0"
name: t
machines:
  m: {{ hostname: h, addr: 127.0.0.1 }}
{moved}
resources:
{resources}
"#
    )
}

#[test]
fn the_canonical_rename_validates() {
    // THE REGRESSION. This is the documented pattern and the cookbook's own
    // recipe 34. It must not be an error.
    let yaml = cfg(
        "moved:\n  - from: webserver-pkg\n    to: nginx-web\n",
        "  nginx-web:\n    type: file\n    machine: m\n    path: /tmp/x\n    content: \"y\"\n",
    );
    let config: forjar::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&yaml).expect("fixture must parse");
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "the canonical moved-block rename must validate — `to` is REQUIRED to be \
         a declared resource, or the rename points at nothing.\nerrors: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

#[test]
fn a_no_op_move_is_still_rejected() {
    // The gate must keep catching what it legitimately caught.
    let yaml = cfg(
        "moved:\n  - from: same\n    to: same\n",
        "  same:\n    type: file\n    machine: m\n    path: /tmp/x\n    content: \"y\"\n",
    );
    let config: forjar::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&yaml).expect("fixture must parse");
    assert!(
        !validate_config(&config).is_empty(),
        "from == to is a no-op and must still be rejected"
    );
}

#[test]
fn two_moves_onto_one_target_are_still_rejected() {
    // x→z and y→z would clobber in the lock regardless of config, and that IS
    // decidable from config alone. It must keep failing.
    let yaml = cfg(
        "moved:\n  - from: a\n    to: z\n  - from: b\n    to: z\n",
        "  z:\n    type: file\n    machine: m\n    path: /tmp/x\n    content: \"y\"\n",
    );
    let config: forjar::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&yaml).expect("fixture must parse");
    assert!(
        !validate_config(&config).is_empty(),
        "two moves onto one target must still be rejected"
    );
}

#[test]
fn a_chain_is_still_rejected() {
    // a→b, b→c is order-dependent and must stay an error.
    let yaml = cfg(
        "moved:\n  - from: a\n    to: b\n  - from: b\n    to: c\n",
        "  c:\n    type: file\n    machine: m\n    path: /tmp/x\n    content: \"y\"\n",
    );
    let config: forjar::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&yaml).expect("fixture must parse");
    assert!(
        !validate_config(&config).is_empty(),
        "chained moves must still be rejected"
    );
}

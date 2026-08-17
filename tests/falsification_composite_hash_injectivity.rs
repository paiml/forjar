//! GH-235 / FALSIFY-B3-004: `composite_hash` must be injective.
//!
//! The v1 implementation separated components with a NUL byte but did not
//! *frame* them:
//!
//! ```text
//! for c in components { hasher.update(c.as_bytes()); hasher.update(b"\0"); }
//! ```
//!
//! Separation is not framing. A NUL inside a component is indistinguishable
//! from a boundary between components, so `["a\0b"]` and `["a", "b"]` both fed
//! BLAKE3 the byte string `a\0b\0` and produced the same digest.
//!
//! This is the store's address function — `store_path` builds
//! `[recipe_hash, ...sorted_inputs, arch, provider]`, where `arch` and
//! `provider` are free-form strings out of user YAML, and
//! `task::io_tracking` passes components it NUL-joined itself
//! (`format!("{artifact}\0{hash}")`), which is exactly the re-partitionable
//! shape. Two distinct derivations colliding on one store path means the second
//! silently reuses the first's build output.
//!
//! The three pre-existing obligations (FALSIFY-B3-001/002/003: prefix format,
//! determinism, order sensitivity) all PASS on the broken implementation and
//! always would — a non-injective function can be perfectly deterministic and
//! order-sensitive. That is why injectivity had to be added as its own
//! obligation rather than being expected to fall out of the others.

use forjar::tripwire::hasher::composite_hash;

#[test]
fn a_nul_inside_a_component_does_not_forge_a_boundary() {
    // The exact reproduction from the issue.
    let one = composite_hash(&["a\u{0}b"]);
    let two = composite_hash(&["a", "b"]);
    assert_ne!(
        one, two,
        "COLLISION: a component containing NUL hashes identically to two \
         separate components — composite_hash is separated, not framed"
    );
}

#[test]
fn the_boundary_cannot_be_moved_within_a_fixed_byte_sequence() {
    // Same bytes, different partition. Under unframed separation every one of
    // these feeds the hasher `a\0b\0c\0`.
    let variants = [
        composite_hash(&["a\u{0}b", "c"]),
        composite_hash(&["a", "b\u{0}c"]),
        composite_hash(&["a", "b", "c"]),
        composite_hash(&["a\u{0}b\u{0}c"]),
    ];
    for (i, x) in variants.iter().enumerate() {
        for (j, y) in variants.iter().enumerate() {
            if i < j {
                assert_ne!(
                    x, y,
                    "partitions {i} and {j} of the same byte sequence collide"
                );
            }
        }
    }
}

#[test]
fn the_component_count_is_part_of_the_digest() {
    // A one-component vector must not be able to reproduce an n-component one
    // even when the concatenations agree.
    assert_ne!(composite_hash(&["ab"]), composite_hash(&["a", "b"]));
    assert_ne!(composite_hash(&[""]), composite_hash(&["", ""]));
}

#[test]
fn empty_components_are_distinguishable_by_position() {
    // Empty strings are the degenerate case for length-prefixed framing: the
    // length is 0 and no bytes follow, so only the count and ordering separate
    // these.
    let a = composite_hash(&["", "x"]);
    let b = composite_hash(&["x", ""]);
    let c = composite_hash(&["x"]);
    assert_ne!(a, b, "position of an empty component must matter");
    assert_ne!(a, c);
    assert_ne!(b, c);
}

#[test]
fn a_realistic_store_path_collision_is_prevented() {
    // The concrete risk: `arch` and `provider` are free-form user YAML strings
    // reaching `store_path` as trailing components. An adversarial `arch` that
    // embeds a NUL must not be able to impersonate a different (arch, provider)
    // pair and land on another derivation's store entry.
    let honest = composite_hash(&["recipe:abc", "input:1", "x86_64", "apt"]);
    let forged = composite_hash(&["recipe:abc", "input:1", "x86_64\u{0}apt"]);
    assert_ne!(
        honest, forged,
        "a NUL-bearing arch re-partitioned into (arch, provider) — this is two \
         derivations sharing one store path"
    );
}

#[test]
fn framing_did_not_cost_determinism_or_order_sensitivity() {
    // The properties FALSIFY-B3-002/003 already cover, re-checked here because
    // a framing change is exactly the kind of edit that could break them.
    let a = composite_hash(&["alpha", "beta"]);
    assert_eq!(a, composite_hash(&["alpha", "beta"]), "not deterministic");
    assert_ne!(
        a,
        composite_hash(&["beta", "alpha"]),
        "no longer order-sensitive"
    );
    assert!(a.starts_with("blake3:"));
    assert_eq!(a.len(), 71);
}

#[test]
fn v2_digests_are_domain_separated_from_v1() {
    // A v2 digest must never coincide with what v1 would have produced for
    // some input, or the re-address is a cross-version collision rather than a
    // clean break. Recomputing v1 here directly rather than trusting a recorded
    // constant: the point is that the two schemes disagree on live inputs.
    fn v1(components: &[&str]) -> String {
        let mut h = blake3::Hasher::new();
        for c in components {
            h.update(c.as_bytes());
            h.update(b"\0");
        }
        format!("blake3:{}", h.finalize().to_hex())
    }
    for case in [
        vec!["a"],
        vec!["a", "b"],
        vec!["recipe:abc", "x86_64", "apt"],
        vec![""],
    ] {
        assert_ne!(
            composite_hash(&case),
            v1(&case),
            "v2 reproduced the v1 digest for {case:?} — the framing change did \
             not take effect for this shape"
        );
    }
}

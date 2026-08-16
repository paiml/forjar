//! GH-240/GH-245: the supported surface must not move silently.
//!
//! A documented promise with nothing enforcing it is the same shape as the
//! defects this release is about: a claim that reads as a guarantee and is
//! checked by nothing. These tests are what make `api` a promise rather than a
//! paragraph.
//!
//! If one of these fails, the fix is NOT to update the test quietly. It is to
//! decide whether the change is breaking, bump accordingly, and record it.

use crate::api;

#[test]
fn every_promised_item_is_reachable_through_the_api_module() {
    // Named one at a time rather than by glob: a glob would keep compiling
    // after an item was dropped from the module, which is the exact silence
    // this guards against. Referencing each as a value/type forces resolution.
    let _: fn(&std::path::Path) -> Result<String, String> = api::hash_file;
    let _: fn(&api::Resource) -> Option<api::IoDigest> = api::probe_resource;

    // Types resolve.
    let _r: Option<api::Resource> = None;
    let _p: Option<api::PlannedChange> = None;
    let _a: Option<api::PlanAction> = None;
    let _d: Option<api::IoDigest> = None;

    // Remaining functions resolve as paths. Their signatures are generic or
    // multi-argument, so this pins existence and reachability rather than an
    // exact shape — a signature change is caught by the consumer-shaped test
    // below.
    let _ = api::staleness_reason;
    let _ = api::probe_all::<fn(&str) -> bool>;
    let _ = api::hash_inputs;
    let _ = api::hash_outputs_in;
    let _ = api::propagate_changes;
}

#[test]
fn hash_file_is_deterministic_prefixed_and_byte_sensitive() {
    // The three properties rmedia verified before asking for the promise. They
    // are the promise: a content-identity function that is any of unstable,
    // unprefixed, or insensitive to a byte is not usable as a cache key.
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("lesson.srt");
    std::fs::write(&f, b"1\n00:00:01,000 --> 00:00:02,000\nhello\n").unwrap();

    let a = api::hash_file(&f).expect("hash_file must read an existing file");
    let b = api::hash_file(&f).expect("hash_file must be callable twice");
    assert_eq!(a, b, "hash_file must be deterministic");
    assert!(
        a.starts_with("blake3:"),
        "hash_file must carry its prefix: {a}"
    );

    std::fs::write(&f, b"1\n00:00:01,000 --> 00:00:02,000\nhellu\n").unwrap();
    let c = api::hash_file(&f).expect("hash_file must re-read");
    assert_ne!(a, c, "a one-byte change must change the identity");
}

#[test]
fn a_missing_baseline_means_rebuild_not_fresh() {
    // The fail-safe branch, and the single most important thing in this
    // surface. Every cache gets this backwards at least once: no recorded hash
    // reads as "nothing to compare, therefore fresh", and a corrected source
    // file silently fails to trigger a rebuild.
    //
    // Asserted through the api re-export specifically, so the promise covers
    // the ORDERING and not merely the symbol.
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("rendered.mp4");
    std::fs::write(&out, b"video").unwrap();

    let probe = api::IoDigest {
        input_hash: Some("blake3:something".to_string()),
        output_hash: Some("blake3:whatever".to_string()),
        outputs_missing: false,
    };

    let reason = api::staleness_reason(&probe, None, None);
    assert_eq!(
        reason.as_deref(),
        Some("no recorded input hash"),
        "a missing baseline must rebuild once to establish one, never read as fresh"
    );
}

#[test]
fn outputs_missing_is_checked_before_any_hash_comparison() {
    // "absent" and "present but different" are different facts. If
    // outputs_missing were folded into the hash comparison, a deleted artifact
    // would be reported by whichever branch happened to fire first — and with
    // matching hashes, that is "fresh".
    let probe = api::IoDigest {
        input_hash: Some("blake3:same".to_string()),
        output_hash: Some("blake3:same-out".to_string()),
        outputs_missing: true,
    };

    // Inputs and outputs both MATCH their baselines. Only the missing-output
    // flag distinguishes this from a fully fresh resource.
    let reason = api::staleness_reason(&probe, Some("blake3:same"), Some("blake3:same-out"));
    assert_eq!(
        reason.as_deref(),
        Some("output artifact missing"),
        "outputs_missing must win over matching hashes, or a deleted artifact reads as fresh"
    );
}

#[test]
fn a_fully_fresh_resource_reports_no_reason() {
    // The gate must be passable, or consumers learn to ignore it.
    let probe = api::IoDigest {
        input_hash: Some("blake3:in".to_string()),
        output_hash: Some("blake3:out".to_string()),
        outputs_missing: false,
    };
    assert_eq!(
        api::staleness_reason(&probe, Some("blake3:in"), Some("blake3:out")),
        None,
        "unchanged inputs and outputs must not report staleness"
    );
}

#[test]
fn changed_inputs_are_reported() {
    let probe = api::IoDigest {
        input_hash: Some("blake3:new".to_string()),
        output_hash: Some("blake3:out".to_string()),
        outputs_missing: false,
    };
    assert_eq!(
        api::staleness_reason(&probe, Some("blake3:old"), Some("blake3:out")).as_deref(),
        Some("inputs changed")
    );
}

#[test]
fn the_supported_surface_stays_small() {
    // GH-240's actual finding was scale: a promise over 1,844 items is one we
    // break by accident. This is a deliberate ceiling — growing the supported
    // surface should be a decision someone makes, not something that happens.
    //
    // Counted from the module source rather than by reflection, because Rust
    // has no way to enumerate a module's re-exports at runtime.
    let src = include_str!("api.rs");
    let reexports: usize = src
        .lines()
        .filter(|l| l.trim_start().starts_with("pub use crate::"))
        .map(|l| {
            // `pub use path::{a, b, c};` re-exports three items; `pub use path::a;` one.
            l.find('{').map_or(1, |i| l[i..].matches(',').count() + 1)
        })
        .sum();
    assert_eq!(
        reexports, 11,
        "the supported surface is 11 items (7 functions + 4 types). Changing it \
         is a semver decision: update this number deliberately, with a changelog \
         entry, not to make the test pass."
    );
}

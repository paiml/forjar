//! The gate: no desired-state field may be unclassified.

use super::{classify, hashed_fields, Observability};

#[test]
fn every_hashed_field_is_classified() {
    // THE FORCING FUNCTION. The field set comes from the HASHER by reflection,
    // so adding a field to the desired state adds it here automatically and
    // this test fails until someone decides whether the host can be asked about
    // it. That decision is the thing forjar has never had: before this, an
    // unobserved field defaulted to converged, silently and forever.
    let unclassified: Vec<String> = hashed_fields()
        .into_iter()
        .filter(|f| classify(f).is_none())
        .collect();

    assert!(
        unclassified.is_empty(),
        "\n{} desired-state field(s) are not classified in the observability \
         registry:\n\n{}\n\nEach changes hash_desired_state, so forjar will \
         re-apply when it changes — but nothing decides whether the HOST can be \
         asked about it. Add each to src/core/observe/mod.rs::classify as \
         Observed{{alt}} (the host can be asked; `alt` is a different valid \
         value the behavioural gate mutates to), or Unobservable(reason) with a \
         real reason.\n",
        unclassified.len(),
        unclassified
            .iter()
            .map(|f| format!("  - {f}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_reflection_actually_finds_fields() {
    // A gate that discovers nothing passes vacuously. Pin that reflection sees
    // the fields the 2026-08-19 defects lived in — if this ever returns an empty
    // or tiny set, `every_hashed_field_is_classified` has become a no-op.
    let fields = hashed_fields();
    assert!(
        fields.len() >= 8,
        "reflection found only {} hashed field(s); the probe has stopped \
         working and the gate above is vacuous: {fields:?}",
        fields.len()
    );
    for required in ["source", "content", "version", "path"] {
        assert!(
            fields.iter().any(|f| f == required),
            "`{required}` changes the desired-state hash but reflection did not \
             find it — the probe cannot represent its type. found: {fields:?}"
        );
    }
}

#[test]
fn unobservable_must_carry_a_reason() {
    // Unobservable is the escape hatch; an empty reason turns it into a shrug.
    for f in hashed_fields() {
        if let Some(Observability::Unobservable(reason)) = classify(&f) {
            assert!(
                reason.len() > 20,
                "`{f}` is Unobservable with no real reason: {reason:?}"
            );
        }
    }
}

#[test]
fn every_observed_alt_actually_dirties_the_baseline() {
    // `alt` exists so the behavioural gate can seed a PRESENT-BUT-WRONG host:
    // apply with `alt`, then apply the declaration, then assert the host matches
    // the declaration. If `alt` did not actually change the desired state, that
    // sequence would pass without ever dirtying anything — the same vacuity that
    // let `--refresh` ship doing nothing at all.
    //
    // So assert the PROPERTY, not the spelling. A first version of this test
    // whitelisted alt strings and failed the moment new fields were classified,
    // which is the text-pinning antipattern this whole change exists to kill.
    use crate::core::planner::hashing::hash_desired_state;
    use crate::core::types::Resource;

    for f in hashed_fields() {
        let Some(Observability::Observed { alt }) = classify(&f) else {
            continue;
        };
        assert!(!alt.is_empty(), "`{f}` has an empty alt value");

        let base = Resource::default();
        let Ok(serde_json::Value::Object(mut m)) = serde_json::to_value(&base) else {
            continue;
        };
        m.insert(f.clone(), serde_json::json!(alt));
        let scalar = serde_json::from_value::<Resource>(serde_json::Value::Object(m.clone()));

        let mut ml = m.clone();
        ml.insert(f.clone(), serde_json::json!([alt]));
        let list = serde_json::from_value::<Resource>(serde_json::Value::Object(ml));

        let moved = [scalar, list].iter().any(|r| {
            r.as_ref()
                .is_ok_and(|r| hash_desired_state(r) != hash_desired_state(&base))
        });
        assert!(
            moved,
            "`{f}` is Observed with alt {alt:?}, but setting the field to that \
             value does not change the desired-state hash — so it cannot dirty a \
             baseline and any convergence test using it would pass vacuously"
        );
    }
}

//! forjar#308: a contract macro that expands to nothing is documentation
//! wearing the costume of verification.
//!
//! Rust takes the LAST `macro_rules!` definition of a name. This file carried
//! 492 duplicate definitions, and where the later one had an empty body it
//! SHADOWED a real assertion — `contract_pre_serialize_roundtrip` had a
//! `debug_assert!(input.len() > 0)` at one definition and `{}` at a later one,
//! so the live call site in `state_encryption.rs` compiled to nothing.
//!
//! This test does not demand that every call site assert. Most contracts state
//! no checkable precondition, and a macro that honestly expands to nothing is
//! fine. What it forbids is the number of INERT LIVE CALL SITES GROWING, and it
//! forbids a name being defined twice at all — because that is the mechanism by
//! which a real assertion silently disappears.

use std::collections::HashMap;

const GENERATED: &str = include_str!("../src/generated_contracts.rs");

/// name -> body of every `macro_rules!` definition, in file order.
fn definitions() -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    let mut rest = GENERATED;
    while let Some(i) = rest.find("macro_rules! ") {
        let after = &rest[i + "macro_rules! ".len()..];
        let name: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let body_start = match after.find('{') {
            Some(b) => b,
            None => break,
        };
        let body_end = after[body_start..]
            .find("\n}\n")
            .map(|e| body_start + e)
            .unwrap_or(after.len());
        out.entry(name)
            .or_default()
            .push(after[body_start..body_end].to_string());
        rest = &after[body_end.min(after.len())..];
    }
    out
}

/// THE MECHANISM ITSELF. A duplicated name is how a real assertion vanishes
/// without anyone editing it — the later definition simply wins.
#[test]
fn no_macro_name_is_defined_twice() {
    let defs = definitions();
    let dup: Vec<&String> = defs
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, _)| k)
        .collect();
    assert!(
        dup.is_empty(),
        "{} macro name(s) are defined more than once. Rust takes the LAST \
         definition, so any earlier one that asserts is silently dead. \
         Examples: {:?}",
        dup.len(),
        dup.iter().take(5).collect::<Vec<_>>()
    );
}

/// The assertion that was found shadowed — and turned out to be WRONG.
///
/// Deduplicating restored `debug_assert!(input.len() > 0)` and
/// `hash_data_empty` panicked instantly: `hash_data(b"")` is legitimate, since
/// BLAKE3 of empty input is well defined. The precondition came from
/// `apr-format-invariants-v1.yaml` — a contract about a SERIALIZATION ROUNDTRIP,
/// from a corpus this repo does not contain — and was being asserted on a
/// hashing function it does not describe.
///
/// **A dead assertion hid a false assertion.** The call site is removed; the
/// macro keeps its restored body so the next caller gets the real check, and
/// this test pins that the deduplication itself did not regress.
#[test]
fn the_shadowed_precondition_is_restored_and_uncalled() {
    let defs = definitions();
    let bodies = defs
        .get("contract_pre_serialize_roundtrip")
        .expect("contract_pre_serialize_roundtrip is not defined");
    assert_eq!(bodies.len(), 1, "it is defined more than once again");
    assert!(
        bodies[0].contains("debug_assert"),
        "the restored length precondition is gone again:\n{}",
        bodies[0]
    );

    // And it must NOT be INVOKED on `hash_data`, whose empty input is valid.
    //
    // Checked per non-comment line, not with a plain `contains`. The first cut
    // used `contains` and failed against the comment that explains the removal —
    // a text assertion catching its own documentation. That is the same class of
    // mistake this file exists to catch, one level up.
    let enc = include_str!("../src/core/state_encryption.rs");
    let invoked = enc.lines().any(|l| {
        let t = l.trim_start();
        !t.starts_with("//") && t.contains("contract_pre_serialize_roundtrip!")
    });
    assert!(
        !invoked,
        "a serialization-roundtrip precondition is asserted on a hashing \
         function again — hash_data(b\"\") is legitimate"
    );
}

/// A RATCHET, NOT A DEMAND. Most contracts state no checkable precondition, so
/// requiring every call site to assert would be false precision. What must not
/// happen is the inert count creeping up — that is verification quietly being
/// replaced by decoration.
///
/// Lower this number when a call site is made to assert. Never raise it.
#[test]
fn inert_live_call_sites_do_not_increase() {
    const CEILING: usize = 16;

    let defs = definitions();
    let mut inert = 0usize;
    let mut total = 0usize;
    for entry in walk("src") {
        if entry.ends_with("generated_contracts.rs") {
            continue;
        }
        let Ok(src) = std::fs::read_to_string(&entry) else {
            continue;
        };
        for line in src.lines() {
            let Some(i) = line
                .find("contract_pre_")
                .or_else(|| line.find("contract_post_"))
            else {
                continue;
            };
            let name: String = line[i..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            // Skip the doc-comment mentions; only real invocations count.
            if !line[i + name.len()..].starts_with('!') {
                continue;
            }
            total += 1;
            if let Some(bodies) = defs.get(&name) {
                if !bodies.last().is_some_and(|b| b.contains("assert")) {
                    inert += 1;
                }
            }
        }
    }

    // PRINT THE DENOMINATOR. A count with no total cannot be told from a scan
    // that found nothing.
    eprintln!("contract call sites: {total} live, {inert} inert (ceiling {CEILING})");
    assert!(total > 0, "found no call sites at all — the scan is broken");
    assert!(
        inert <= CEILING,
        "inert contract call sites rose to {inert} (ceiling {CEILING}). A macro \
         that expands to nothing looks like verification and is not."
    );
}

fn walk(dir: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(p.to_str().unwrap_or("")));
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p.to_string_lossy().into_owned());
        }
    }
    out
}

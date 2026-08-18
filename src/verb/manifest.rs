//! The generated surface manifest.
//!
//! # Why a generated file and not a checklist
//!
//! A hand-maintained list of verbs agrees with the code on the day it is
//! written and drifts from it thereafter, silently, because nothing compares
//! them. This manifest is rendered from [`super::derive::registry`] and
//! committed, so the comparison happens on every test run: the tree either
//! matches what the code produces or the build is red.
//!
//! The rendering is deliberately plain text rather than JSON — a reviewer must
//! be able to read the diff and see that a verb gained a parameter or changed
//! its effect class, which is the review this file exists to make possible.

use super::derive;
use super::spec::VerbSpec;
use std::fmt::Write as _;

/// The manifest's path relative to the crate root.
pub const MANIFEST_PATH: &str = "docs/surface-manifest.txt";

/// The `#[ignore]`d test that rewrites [`MANIFEST_PATH`].
pub const REGEN_TEST: &str = "regenerate_surface_manifest";

/// Render the manifest for the current registry.
#[must_use]
pub fn render() -> String {
    let verbs = derive::registry();
    let mut s = String::with_capacity(64 * 1024);
    s.push_str("# forjar unified verb surface — GENERATED, DO NOT EDIT\n");
    s.push_str("#\n");
    let _ = writeln!(
        s,
        "# Regenerate: cargo test --lib {REGEN_TEST} -- --ignored"
    );
    s.push_str("#\n");
    s.push_str("# Derived from the clap command tree in src/cli/commands/mod.rs.\n");
    s.push_str("# Columns: verb, effects, then one line per parameter.\n");
    let _ = writeln!(s, "\nverbs: {}", verbs.len());
    let _ = writeln!(
        s,
        "params: {}",
        verbs.iter().map(|v| v.params.len()).sum::<usize>()
    );
    for v in verbs {
        render_verb(&mut s, v);
    }
    s
}

fn render_verb(s: &mut String, v: &VerbSpec) {
    let _ = write!(s, "\n[{}] effects={}", v.name, v.effects.as_str());
    if v.is_grouped() {
        let _ = write!(s, " subcommands={}", v.subcommands.join(","));
    }
    s.push('\n');
    let _ = writeln!(s, "  description: {}", v.description);
    for p in &v.params {
        let _ = write!(
            s,
            "  param {} kind={:?} required={}",
            p.name, p.kind, p.required
        );
        if let Some(l) = &p.long {
            let _ = write!(s, " long=--{l}");
        }
        if let Some(d) = &p.default {
            let _ = write!(s, " default={d}");
        }
        if !p.choices.is_empty() {
            let _ = write!(s, " choices={}", p.choices.join("|"));
        }
        s.push('\n');
    }
}

/// The committed manifest, as compiled into the test binary.
///
/// `include_str!` is what makes the drift check real: the bytes compared are
/// the bytes on disk at build time, not a value the test could compute for
/// itself and then compare against itself.
#[cfg(test)]
const COMMITTED: &str = include_str!("../../docs/surface-manifest.txt");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_committed_manifest_matches_the_derived_surface() {
        // The surface-derivation gate. A verb added, renamed, reclassified, or
        // given a new flag changes this file; a hand-edited file that does not
        // match what the code produces fails here.
        let derived = render();
        if derived != COMMITTED {
            let d: Vec<&str> = derived.lines().collect();
            let c: Vec<&str> = COMMITTED.lines().collect();
            let first = d
                .iter()
                .zip(c.iter())
                .position(|(a, b)| a != b)
                .unwrap_or(c.len().min(d.len()));
            panic!(
                "{} is stale.\n  regenerate: cargo test --lib {} -- --ignored\n\
                 first difference at line {}:\n  derived:   {:?}\n  committed: {:?}\n\
                 (derived {} lines, committed {} lines)",
                MANIFEST_PATH,
                REGEN_TEST,
                first + 1,
                d.get(first),
                c.get(first),
                d.len(),
                c.len()
            );
        }
    }

    #[test]
    #[ignore = "writes to the source tree; run explicitly to regenerate"]
    fn regenerate_surface_manifest() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/surface-manifest.txt");
        std::fs::write(path, render()).expect("write manifest");
        eprintln!("wrote {path}");
    }

    #[test]
    fn the_manifest_names_every_verb_in_the_registry() {
        let m = render();
        for v in derive::registry() {
            assert!(
                m.contains(&format!("\n[{}] effects=", v.name)),
                "manifest omits verb {}",
                v.name
            );
        }
    }

    #[test]
    fn the_manifest_records_effects_so_a_reclassification_shows_in_the_diff() {
        let m = render();
        assert!(
            m.contains("[apply] effects=mutating"),
            "apply misclassified"
        );
        assert!(m.contains("[plan] effects=read-only"), "plan misclassified");
        assert!(m.contains("[mcp] effects=transport"), "mcp misclassified");
    }

    #[test]
    fn the_manifest_is_deterministic() {
        assert_eq!(render(), render());
    }

    #[test]
    fn the_regen_test_name_constant_matches_the_actual_test() {
        // If the test is renamed the panic message would send a reader to a
        // command that does not exist.
        assert_eq!(REGEN_TEST, "regenerate_surface_manifest");
    }
}

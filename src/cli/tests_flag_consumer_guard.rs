//! GH-211 / FALSIFY-FLAG-A16: the guard that stops the SIXTEENTH inert flag.
//!
//! Family A of the 1.12.3 dogfood is 15 defects with a single shape: a field is
//! added to a clap args struct, so the flag appears in `--help` with an FJ-
//! ticket number and is accepted — and then nothing ever reads it. rustc has no
//! objection: an `Option<String>` that is never matched on is a struct field,
//! not dead code. So the DECLARED surface and the DISPATCH surface drift apart
//! silently, and every smoke test that runs the flag and checks `rc == 0`
//! passes.
//!
//! Fifteen hand-written per-flag tests would not stop the sixteenth. This test
//! is the only form of the check that scales: it derives the flag set from the
//! source rather than from a list a human maintains, so a NEW inert flag fails
//! CI on the commit that adds it.
//!
//! # The three states, and why there is no fourth
//!
//! For every option field on every `clap::Args` struct:
//!
//! * **consumed** — read somewhere outside `commands/` and outside
//!   `inert_flags.rs`. The flag reaches a dispatcher.
//! * **refused** — read ONLY by `inert_flags.rs`, whose entire job is to exit
//!   non-zero naming the flag. Honest interim state: `--pre-check` exiting 1
//!   with "not implemented" is strictly better than `--pre-check` exiting 0 as
//!   a gate that can never block.
//! * **inert** — read by nothing. This is the defect, and the test names it.
//!
//! Reading a field in BOTH places also fails: that is a flag that was
//! implemented while its refusal was left behind to block it.
//!
//! # What this test cannot see (stated, not hidden)
//!
//! It proves a field is READ, not that reading it changes behaviour. A field
//! forwarded into a function that ignores it still counts as consumed —
//! `--notify-custom-headers` is assigned into `NotifyOpts` and never becomes a
//! `-H` argument, and this test is blind to that. Partial consumption is
//! covered by the process-level falsification tests in
//! `contracts/flag-has-effect-v1.yaml`, not here. What this test does cover
//! completely is the "read by nothing at all" class, which is 15 of the 15
//! confirmed defects' root cause.
//!
//! Two Rust patterns already make a field impossible to leave silently inert,
//! and the parser below models both: a bare destructuring binding (`let Args {
//! foo, .. } = args`) is caught by rustc's unused-variable lint, so it counts
//! as consumed; a binding written `foo: _foo` or elided by the `..` rest
//! pattern is NOT, which is exactly how `lint --rules`, `validate
//! --schema-version` and `status --dependency-count` hid.

use super::tests_flag_surface::*;
use std::collections::{BTreeMap, BTreeSet};

/// The verdict for one declared flag.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    Consumed,
    Refused,
    Inert,
    Both,
}

fn classify(consumed: bool, refused: bool) -> Verdict {
    match (consumed, refused) {
        (true, true) => Verdict::Both,
        (true, false) => Verdict::Consumed,
        (false, true) => Verdict::Refused,
        (false, false) => Verdict::Inert,
    }
}

/// Flag names passed to `reject_inert_flag("--x", ..)` anywhere in the crate,
/// normalised to field spelling. Those call sites ARE the refusal for flags on
/// structs that are destructured rather than passed whole, so the field name
/// itself never appears in `inert_flags.rs`.
///
/// Derived from the source, not from a list here — a hand-written list of
/// refusals has precisely the failure mode of the hand-written list of
/// consumers that let 15 flags ship inert.
fn refused_by_name(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let needle = "reject_inert_flag(\"--";
    for (i, _) in text.match_indices(needle) {
        let rest = &text[i + needle.len()..];
        let flag: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '-')
            .collect();
        if !flag.is_empty() {
            out.insert(flag.replace('-', "_"));
        }
    }
    out
}

/// The audited surface: every declared flag with its verdict.
struct Audit {
    fields: Vec<FieldDecl>,
    verdicts: BTreeMap<(String, String), Verdict>,
}

impl Audit {
    fn verdict(&self, ty: &str, field: &str) -> &Verdict {
        self.verdicts
            .get(&(ty.to_string(), field.to_string()))
            .unwrap_or_else(|| panic!("{ty}.{field} is not a declared flag"))
    }
}

/// Compute the verdict for every declared flag on every `clap::Args` struct.
fn audit() -> Audit {
    let sources = read_sources();
    let fields = declared_flags(&sources);
    let variants = variant_types(&sources);

    let mut consumer_text = String::new();
    for (path, src) in &sources {
        if !is_declaration(path) && !is_excluded(path) {
            consumer_text.push_str(src);
            consumer_text.push('\n');
        }
    }
    let refusal_text = sources
        .get("src/cli/inert_flags.rs")
        .cloned()
        .unwrap_or_default();

    let consumer_binds = bindings_for(&consumer_text, &variants);
    let refusal_binds = bindings_for(&refusal_text, &variants);
    let by_name = refused_by_name(&consumer_text);

    let mut per_ty: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
    let mut verdicts = BTreeMap::new();
    for f in &fields {
        let (consumed, refused) = per_ty.entry(f.ty.clone()).or_insert_with(|| {
            (
                fields_read(&consumer_text, &f.ty, &consumer_binds),
                fields_read(&refusal_text, &f.ty, &refusal_binds),
            )
        });
        let mut v = classify(consumed.contains(&f.name), refused.contains(&f.name));
        // A `reject_inert_flag("--x", ..)` call site refuses a flag whose field
        // is destructured rather than read through the struct, so it can only
        // ever upgrade Inert — never override a real dispatch site elsewhere
        // (`--dependency-count` exists on two structs and works on one of them).
        if v == Verdict::Inert && by_name.contains(&f.name) {
            v = Verdict::Refused;
        }
        verdicts.insert((f.ty.clone(), f.name.clone()), v);
    }
    Audit { fields, verdicts }
}

#[test]
fn the_parser_finds_the_flag_surface_it_is_supposed_to_guard() {
    let sources = read_sources();
    let fields = declared_flags(&sources);
    let structs: BTreeSet<&str> = fields.iter().map(|f| f.ty.as_str()).collect();
    assert!(
        structs.len() >= 100,
        "parser found only {} args structs — the parser is broken, not the code",
        structs.len()
    );
    let apply: Vec<&FieldDecl> = fields.iter().filter(|f| f.ty == "ApplyArgs").collect();
    assert!(
        apply.len() >= 150,
        "ApplyArgs has {} fields — the parser is broken",
        apply.len()
    );
    for want in ["skip", "only_machine", "exclude_machine", "resource_filter"] {
        assert!(
            apply.iter().any(|f| f.name == want),
            "field {want} not extracted from ApplyArgs"
        );
    }
    assert!(
        apply
            .iter()
            .any(|f| f.name == "notify_file" && f.marked_unimplemented),
        "the [UNIMPLEMENTED] doc marker is not being read"
    );
    assert!(
        apply
            .iter()
            .any(|f| f.name == "subset" && !f.marked_unimplemented),
        "a working flag was mis-parsed as unimplemented"
    );
}

/// FALSIFY-FLAG-A16. RED condition: a `clap::Args` field that no dispatcher
/// reads and `inert_flags.rs` does not refuse. Adding `--foo` to `ApplyArgs`
/// and forgetting to wire it fails HERE, on that commit.
#[test]
fn every_declared_flag_is_consumed_or_refused() {
    let audit = audit();
    let inert: Vec<String> = audit
        .fields
        .iter()
        .filter(|f| *audit.verdict(&f.ty, &f.name) == Verdict::Inert)
        .map(|f| format!("  {}.{}  (--{})", f.ty, f.name, f.name.replace('_', "-")))
        .collect();

    assert!(
        inert.is_empty(),
        "{} declared flag(s) are read by NOTHING — accepted, printed in --help, \
         and inert. Wire each to its dispatcher, or refuse it in \
         src/cli/inert_flags.rs so it exits non-zero naming itself (GH-211):\n{}",
        inert.len(),
        inert.join("\n")
    );
}

/// RED condition: a flag is implemented AND still refused, so the refusal
/// blocks the implementation. This is the failure mode of the fix, and it
/// fails as loudly as the bug it fixes.
#[test]
fn no_flag_is_both_implemented_and_refused() {
    let audit = audit();
    let both: Vec<String> = audit
        .fields
        .iter()
        .filter(|f| *audit.verdict(&f.ty, &f.name) == Verdict::Both)
        .map(|f| format!("  {}.{}", f.ty, f.name))
        .collect();
    assert!(
        both.is_empty(),
        "{} flag(s) are read by a dispatcher AND refused by src/cli/inert_flags.rs. \
         The refusal wins, so the implementation is unreachable — delete the \
         refusal entry:\n{}",
        both.len(),
        both.join("\n")
    );
}

/// The four scope selectors are the ones that were IMPLEMENTED rather than
/// refused. RED condition: one of them regresses to inert, or gets refused.
#[test]
fn the_four_scope_selectors_are_implemented_not_refused() {
    let audit = audit();
    for f in ["skip", "only_machine", "exclude_machine", "resource_filter"] {
        assert_eq!(
            *audit.verdict("ApplyArgs", f),
            Verdict::Consumed,
            "--{} must reach the selector, not be refused",
            f.replace('_', "-")
        );
    }
}

/// Refusing a flag is only honest if `--help` says so. RED condition: a refused
/// flag whose doc comment still promises the effect, or a working flag wrongly
/// marked `[UNIMPLEMENTED]`.
#[test]
fn the_help_text_and_the_dispatcher_agree() {
    let audit = audit();
    let mut wrong = Vec::new();
    for f in &audit.fields {
        let refused = *audit.verdict(&f.ty, &f.name) == Verdict::Refused;
        if refused && !f.marked_unimplemented {
            wrong.push(format!(
                "  {}.{} is refused but --help still promises the effect",
                f.ty, f.name
            ));
        }
        if !refused && f.marked_unimplemented {
            wrong.push(format!(
                "  {}.{} works but --help says [UNIMPLEMENTED]",
                f.ty, f.name
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} flag(s) whose help text disagrees with the dispatcher:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}



#[test]
fn refused_by_name_reads_the_call_sites() {
    let text = r#"reject_inert_flag("--dependency-count", x)?; reject_inert_flag("--rules", y)?;"#;
    let got = refused_by_name(text);
    assert!(got.contains("dependency_count"));
    assert!(got.contains("rules"));
    assert_eq!(got.len(), 2);
}


#[test]
fn classify_covers_all_four_states() {
    assert_eq!(classify(true, false), Verdict::Consumed);
    assert_eq!(classify(false, true), Verdict::Refused);
    assert_eq!(classify(false, false), Verdict::Inert);
    assert_eq!(classify(true, true), Verdict::Both);
}

//! Refs #390: the executor turns a stream into operator text in ONE place.
//!
//! THE FLAW THIS CLOSES.
//!
//! #390 was not one bad line, it was five constructors that disagreed. Two of
//! them — `resource_ops.rs` and `machine_wave.rs` — held byte-identical copies
//! of `format!("exit code {}: {}", out.exit_code, out.stderr.trim())`, free to
//! drift apart. A third, `output_verify::verify_against_host`, reported
//! `out.stdout.trim()` and destroyed `out.stderr` — the exact mirror image, on
//! the branch every `type: task` without a `completion_check` reaches. Whichever
//! half of a failure mattered, some path was built to throw it away.
//!
//! Fixing the instance and leaving the class is how the next reporter starts
//! where this one did. This is a ratchet: it counts the places inside the
//! executor that read `.stdout` / `.stderr` off an `ExecOutput` and holds each
//! to a budget, so a sixth stderr-only message cannot be added quietly.
//!
//! WHAT THIS TEST MUST NOT BECOME. A scanner that finds nothing always passes.
//! `the_ratchet_is_actually_looking_at_something` guards the guard.

use std::collections::BTreeMap;
use std::path::Path;

/// Per-file budget of `.stdout` / `.stderr` mentions in NON-test executor code.
///
/// Each number is a reason, not a high-water mark:
///   failure_text  — the choke point; turning streams into text IS its job
///   run_capture   — persistence: the unelided record the excerpt points at,
///                   plus the two Refs #406 redaction writes that strike a
///                   resolved secret out of each stream BEFORE that record is
///                   written. Both persist bytes; neither renders a message.
///   resource_ops  — `record_success` hashes the state query's stdout
///   helpers       — content hashing and `parse_signature`: data, not messages
fn budget() -> BTreeMap<&'static str, usize> {
    BTreeMap::from([
        ("failure_text.rs", 3),
        ("run_capture.rs", 4),
        ("resource_ops.rs", 1),
        ("helpers.rs", 2),
    ])
}

/// Count `.stdout` / `.stderr` occurrences outside comments.
fn stream_mentions(src: &str) -> usize {
    src.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("//") && !l.starts_with("//!"))
        .map(|l| l.matches(".stdout").count() + l.matches(".stderr").count())
        .sum()
}

fn executor_sources() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/core/executor");
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("executor dir must exist") {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !name.ends_with(".rs") || name.starts_with("tests_") || name == "test_fixtures.rs" {
            continue;
        }
        out.push((name, std::fs::read_to_string(&path).unwrap()));
    }
    out
}

#[test]
fn only_one_file_in_the_executor_turns_a_stream_into_text() {
    let budget = budget();
    let mut violations = Vec::new();

    for (name, src) in executor_sources() {
        let found = stream_mentions(&src);
        let allowed = budget.get(name.as_str()).copied().unwrap_or(0);
        if found > allowed {
            violations.push(format!(
                "  {name}: {found} stream mentions, budget {allowed}"
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "a new place in the executor reads a stream directly instead of going \
         through `failure_text`. That is how #390 happened: five constructors, \
         each with its own opinion about which half of a failure to discard.\n\
         If the new code renders text for a human, call `failure_text`. If it \
         genuinely hashes or persists bytes, raise that file's budget here and \
         say why.\n{}",
        violations.join("\n")
    );
}

#[test]
fn the_ratchet_is_actually_looking_at_something() {
    // GUARD THE GUARD. A scanner whose glob silently stops matching passes
    // forever while proving nothing.
    let sources = executor_sources();
    assert!(
        sources.len() > 5,
        "scanner found almost no executor sources"
    );

    let total: usize = sources.iter().map(|(_, s)| stream_mentions(s)).sum();
    assert!(
        total >= 8,
        "the scanner found only {total} stream mentions; it has probably stopped \
         matching real code"
    );

    assert!(
        sources.iter().any(|(n, _)| n == "failure_text.rs"),
        "the choke point #390 introduced is gone; the ratchet is meaningless \
         without it"
    );
}

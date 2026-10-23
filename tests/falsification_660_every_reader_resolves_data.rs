//! forjar#660: every reader, on a manifest whose `data:` source SUCCEEDS.
//!
//! The command list is derived from the CLAP definition (`common_660::file_commands`),
//! never from `--help` text and never with a hand-set floor: the number of
//! probes run must equal the number derived, minus the rows in `NOT_RUN`, each of
//! which must itself be a derived command. The failing-source half of the
//! contract is `falsification_660_f_commands.rs`; the scan half is
//! `falsification_660_scan.rs`; rules 3 to 6 are `falsification_660_resolution.rs`.

mod common_660;

use common_660::*;
use std::collections::BTreeSet;

/// Subcommands that change a machine, a state directory or a lock, or block
/// on a server. They are not run against the fixture. Every entry must still
/// be a real subcommand — a stale line fails `skip_list_names_real_commands`.
const NOT_RUN: &[&str] = &[
    "apply",
    "destroy",
    "undo",
    "rollback",
    "exec",
    "run",
    "make",
    "retry-failed",
    "rolling",
    "canary",
    "multi-apply",
    "parallel-apply",
    "saga",
    "watch",
    "trigger",
    "agent",
    "fault-inject",
    "promote",
    "fmt",
    "migrate",
    "reseal",
    // `-f` is an ARCHIVE here, not a manifest.
    "archive",
    // `rules` files are rulebooks (`rulebooks:` key), not manifests.
    "rules",
    // Re-encrypt / rewrite the manifest's secrets in place.
    "secrets rekey",
    "secrets rotate",
    // Needs the `encryption` cargo feature, which the test binary lacks.
    "secrets view",
];

/// Subcommands that need more than `-f` to reach the loader.
fn extra_args(cmd: &str) -> &'static [&'static str] {
    match cmd {
        "explain" => &["cfg"],
        "codegen" | "build" | "impact" => &["--resource", "cfg"],
        "lock-rebase" => &["state-from"],
        "extract" => &["--glob", "*"],
        "iso-export" => &["-o", "iso-out"],
        "convert" => &["--reproducible"],
        "facts" => &["m1"],
        "environments diff" => &["a", "b"],
        _ => &[],
    }
}

const FLAG_GATED: &[&[&str]] = &[
    &["apply", "--check"],
    &["apply", "--preview"],
    &["apply", "--dry-run"],
    &["apply", "--dry-run-cost"],
    &["validate", "--dry-expand"],
    &["validate", "--deep"],
];

/// Readers that PRINT a rendered template, with a substring only the resolved
/// value produces. `template` is probed too: it was the unprobed row.
/// (`codegen` ships base64: base64("greeting=hello-from-data").)
const PRINTS_THE_VALUE: &[(&str, &str)] = &[
    ("show", "hello-from-data"),
    ("output", "hello-from-data"),
    ("explain", "hello-from-data"),
    ("template", "hello-from-data"),
    ("codegen", "Z3JlZXRpbmc9aGVsbG8tZnJvbS1kYXRh"),
];

/// `(label, argv)` for every derived `-f` command not in `NOT_RUN`, then `FLAG_GATED`.
fn probes() -> (usize, Vec<(String, Vec<String>)>) {
    let cmds = file_commands();
    let mut out = Vec::new();
    let mut skipped = 0;
    for c in &cmds {
        let top = c.path[0].as_str();
        if NOT_RUN.contains(&c.label.as_str()) || NOT_RUN.contains(&top) {
            skipped += 1;
            continue;
        }
        let mut args = c.path.clone();
        match &c.flag {
            Some(f) => args.extend([f.clone(), "forjar.yaml".into()]),
            None => args.push("forjar.yaml".into()),
        }
        args.extend(extra_args(&c.label).iter().map(|s| s.to_string()));
        out.push((c.label.clone(), args));
    }
    for row in FLAG_GATED {
        let mut args: Vec<String> = row.iter().map(|s| s.to_string()).collect();
        args.extend(["-f".into(), "forjar.yaml".into()]);
        out.push((row.join(" "), args));
    }
    (cmds.len() - skipped, out)
}

#[test]
fn not_run_rows_name_derived_commands() {
    let labels: BTreeSet<String> = file_commands().into_iter().map(|c| c.label).collect();
    let tops: BTreeSet<String> = file_commands()
        .into_iter()
        .map(|c| c.path[0].clone())
        .collect();
    let stale: Vec<&&str> = NOT_RUN
        .iter()
        .filter(|c| !labels.contains(**c) && !tops.contains(**c))
        .collect();
    assert!(
        stale.is_empty(),
        "NOT_RUN rows match no `-f` command: {stale:?}"
    );
}

/// Every probed reader on a manifest whose `data:` source SUCCEEDS: no leaked
/// `{{data.` text, no `__data__` key, and either the resolved value (where the
/// command prints templates) or exit 0.
#[test]
fn every_reader_on_a_good_source_prints_the_value_or_exits_clean() {
    let dir = fixture();
    let (derived, all) = probes();
    let flag_rows = FLAG_GATED.len();
    // The count is DERIVED from clap, not floored.
    assert_eq!(all.len(), derived + flag_rows, "probes != derived commands");
    assert!(derived > 0);
    let mut bad = Vec::new();
    let mut printed = BTreeSet::new();
    for (label, args) in &all {
        let argv: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = run(dir.path(), &argv);
        let body = text(&out);
        let mut why = Vec::new();
        if body.contains("{{data.") && !RAW_BY_DESIGN.contains(&label.as_str()) {
            why.push("left `{{data.` in the output");
        }
        if body.contains("__data__") {
            why.push("printed a `__data__` key");
        }
        if let Some((_, want)) = PRINTS_THE_VALUE.iter().find(|(l, _)| l == label) {
            printed.insert(label.clone());
            if !body.contains(want) {
                why.push("did not print the resolved value");
            }
        } else if !out.status.success()
            && !EXITS_NONZERO_ON_THIS_FIXTURE
                .iter()
                .any(|(l, _)| l == label)
        {
            why.push("exited non-zero");
        }
        if !why.is_empty() {
            bad.push(format!(
                "forjar {}: {} (exit {:?})\n    {}",
                argv.join(" "),
                why.join(", "),
                out.status.code(),
                body.trim().lines().next().unwrap_or("")
            ));
        }
    }
    // Every PRINTS_THE_VALUE row must have been probed, `template` included.
    for (l, _) in PRINTS_THE_VALUE {
        assert!(
            printed.contains(*l),
            "PRINTS_THE_VALUE row `{l}` was never probed"
        );
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

const EXITS_NONZERO_ON_THIS_FIXTURE: &[(&str, &str)] = &[
    ("drift", "no lock yet: drift exits 2 on a fresh machine"),
    ("status", "no state directory yet"),
    (
        "check",
        "the target file does not exist yet, which is a failed check",
    ),
    (
        "test",
        "the target file does not exist yet, which is a failed test",
    ),
    (
        "apply --check",
        "the target file does not exist yet, which is a failed check",
    ),
    (
        "apply --preview",
        "stdin is closed, so the confirmation prompt aborts",
    ),
    (
        "score",
        "a fresh manifest with no runtime data grades F, below the threshold",
    ),
    ("build", "the fixture resource is not type: image"),
    ("environments diff", "the fixture has no environments"),
    ("dist", "the fixture has no dist: section"),
];

/// Readers whose output is the manifest (or its text) by design, so it keeps
/// `{{...}}`: `explain` prints the raw definition beside the resolved one,
/// `remediate` copies the source bytes through, `extract` emits a sub-manifest.
const RAW_BY_DESIGN: &[&str] = &["explain", "remediate", "extract"];

/// `show` without `--resource` prints the RESOLVED outputs, and no `__data__`.
#[test]
fn show_prints_resolved_outputs_and_no_data_key() {
    let dir = fixture();
    let out = run(dir.path(), &["show", "-f", "forjar.yaml"]);
    let body = text(&out);
    assert!(out.status.success(), "{body}");
    assert!(
        body.contains("g: hello-from-data"),
        "outputs unresolved: {body}"
    );
    assert!(
        !body.contains("__data__"),
        "show leaked a __data__ key: {body}"
    );
    assert!(!body.contains("{{"), "show left a template: {body}");
}

/// `validate --strict|--exhaustive|--deep` must not call the data value an
/// unused or unknown param.
#[test]
fn validate_modes_do_not_see_a_data_param() {
    let dir = fixture();
    let yaml = std::fs::read_to_string(dir.path().join("forjar.yaml")).unwrap();
    std::fs::write(
        dir.path().join("forjar.yaml"),
        format!("description: data fixture\n{yaml}"),
    )
    .unwrap();
    for flag in ["--strict", "--exhaustive", "--deep"] {
        let out = run(dir.path(), &["validate", flag, "-f", "forjar.yaml"]);
        let body = text(&out);
        assert!(out.status.success(), "validate {flag} failed: {body}");
        assert!(
            !body.contains("__data__") && !body.to_lowercase().contains("unused param"),
            "validate {flag} reported the data value as a param: {body}"
        );
    }
}

/// An empty-`resources` manifest whose `data:` source fails still errors in
/// `contracts`: the source is resolved at load, not only when a resource needs it.
#[test]
fn contracts_with_no_resources_and_a_failing_source_errors() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("forjar.yaml"),
        "version: \"1.0\"\nname: empty-660\nmachines: {}\nresources: {}\ndata:\n  greeting:\n    type: command\n    value: \"exit 3\"\n",
    )
    .unwrap();
    let out = run(dir.path(), &["contracts", "-f", "forjar.yaml"]);
    let body = text(&out);
    assert!(!out.status.success(), "contracts exited 0: {body}");
    assert!(names_the_failing_source(&body), "{body}");
}

/// `remediate` never writes, so it resolves like MCP `forjar_remediate`: a
/// failing source stops it, a good one lets it run.
#[test]
fn remediate_resolves_data_sources() {
    let bad = fixture_with("exit 3");
    let out = run(bad.path(), &["remediate", "-f", "forjar.yaml"]);
    assert!(!out.status.success(), "remediate exited 0: {}", text(&out));
    assert!(names_the_failing_source(&text(&out)), "{}", text(&out));
    let good = fixture();
    let out = run(good.path(), &["remediate", "-f", "forjar.yaml"]);
    assert!(out.status.success(), "{}", text(&out));
}

fn names_the_failing_source(out: &str) -> bool {
    out.contains("data source 'greeting'")
}

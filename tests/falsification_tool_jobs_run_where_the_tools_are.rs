//! forjar#598: a CI job that requires a tool must be routed to a runner class
//! that has the tool.
//!
//! # The defect
//!
//! `proofs/kani`, `proofs/lean` and `coverage.yml` asked
//! `runs-on: [self-hosted, clean-room, X64]`. That set matches the intel runners
//! (bare metal; lean, kani and rsync present) AND yoga-build/-build2/-build3
//! (containers; all three ABSENT by an inventory run in the job's own venue).
//! Each job was deterministic per runner and a coin flip per run — on commit
//! 537252d3 four `proofs` runs went fail/pass/fail/pass purely by runner. A
//! re-run landing on intel is a green nobody can reconstruct.
//!
//! # Why this parses the YAML instead of searching the text
//!
//! The previous falsification test in this family (forjar#567) asserted a
//! substring, and the workflow's own comments contained it — so the declared
//! mutation left the test GREEN. Every assertion here reads a parsed field
//! (`jobs.<id>.runs-on`, `steps[].name`, `steps[].run`), which a comment cannot
//! satisfy.
//!
//! # The coupling this pins, on purpose
//!
//! `intel` is a HOSTNAME standing in for a CAPABILITY. When a capability label
//! (`provers` / `bare-metal`) exists on the fleet, the right edit is to change
//! the label here and in the workflows together — this test is the reminder
//! that the three must move as one.
//!
//! # mutations — one per arm, each measured to redden exactly its own test
//!
//! An earlier draft declared ONE mutation here while its commit message claimed
//! four; all three review lanes refuted that independently. Every arm now has
//! its address in the file, where a reader of the test finds it.
//!
//! 1. Delete `, intel` from the `lean` job's `runs-on` in proofs.yml →
//!    `every_tool_asserting_proof_job_runs_on_bare_metal` goes RED alone.
//! 2. Delete `, intel` from `stress.yml`'s (or `mutation.yml`'s, or
//!    `coverage.yml`'s) `runs-on` → `every_unfiltered_lib_test_job_runs_where_rsync_is`
//!    goes RED alone.
//! 3. Restore `is not in the CI image` in the kani assertion →
//!    `tool_assertions_name_the_runner_not_the_image` goes RED alone.
//! 4. Rename both `Assert <tool> is available` steps → the two tests that count
//!    them go RED on their anti-vacuity floors, which is the point of the floors.

use serde_yaml_ng::Value;

const BARE_METAL: &str = "intel";

fn workflow(name: &str) -> Value {
    let path = format!("{}/.github/workflows/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_yaml_ng::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

fn runs_on(job: &Value) -> Vec<String> {
    job.get("runs-on")
        .and_then(Value::as_sequence)
        .map(|s| {
            s.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Steps of `job` whose name is `Assert <tool> is available`.
fn tool_assertions(job: &Value) -> Vec<&Value> {
    job.get("steps")
        .and_then(Value::as_sequence)
        .map(|steps| {
            steps
                .iter()
                .filter(|s| {
                    s.get("name")
                        .and_then(Value::as_str)
                        .is_some_and(|n| n.starts_with("Assert ") && n.ends_with(" is available"))
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn every_tool_asserting_proof_job_runs_on_bare_metal() {
    let wf = workflow("proofs.yml");
    let jobs = wf
        .get("jobs")
        .and_then(Value::as_mapping)
        .expect("proofs.yml has jobs");

    let mut checked = Vec::new();
    for (id, job) in jobs {
        if tool_assertions(job).is_empty() {
            continue;
        }
        let id = id.as_str().unwrap_or("?");
        let labels = runs_on(job);
        assert!(
            labels.iter().any(|l| l == BARE_METAL),
            "proofs.yml job `{id}` asserts a tool is present but runs-on {labels:?} also matches \
             yoga-build containers, where the provers are ABSENT by inventory — the job becomes a \
             per-run lottery (forjar#598)"
        );
        checked.push(id.to_string());
    }

    // ANTI-VACUITY: renaming the assertion steps would make the loop above check
    // nothing and pass. kani and lean are the two measured today.
    assert!(
        checked.len() >= 2,
        "expected at least the kani and lean jobs to carry an `Assert <tool> is available` step; \
         found {checked:?}, so the assertion above may be checking nothing"
    );
}

/// Does this `run:` block execute UNFILTERED lib tests — the set that includes
/// nas_archive's rsync-requiring safety test?
///
/// Name-filtered runs (`cargo test --lib convergence`) and targeted ones
/// (`--test <target>`, `--doc`) are deliberately NOT matched: they cannot reach
/// that test, and pinning them would spend bare-metal capacity for nothing.
///
/// Tokenized rather than substring-matched. The round-2 review lane found the
/// first version's blind spots — bare `cargo test`, `--workspace`,
/// `--all-targets`, `cargo nextest run`, and `--locked --lib` (a flag between
/// `--lib` and the separator). Each is a FALSE NEGATIVE, the dangerous
/// direction: a future job using it would be silently left unpinned, which is
/// exactly what discovering jobs instead of listing them promises cannot happen.
fn runs_unfiltered_lib_tests(run: &str) -> bool {
    run.lines()
        .map(str::trim)
        .filter(|l| !l.starts_with('#'))
        .flat_map(shell_commands)
        .any(line_runs_unfiltered_lib_tests)
}

/// One shell line as its separate commands. The round-3 lane reported
/// `cargo test --lib && cargo test --doc` as classified correctly; measured, it
/// was not — the tokenizer read `&&` as a positional name filter and called the
/// unfiltered first command narrowed. A false negative, and a lane's hand-trace
/// that was wrong: the reason every trace here is backed by an asserted case.
fn shell_commands(line: &str) -> impl Iterator<Item = &str> {
    line.split(['&', '|', ';'])
        .map(str::trim)
        .filter(|c| !c.is_empty())
}

/// One command line. `cargo llvm-cov` and `cargo mutants` run the whole suite;
/// `cargo test` and `cargo nextest run` do unless their arguments narrow them.
fn line_runs_unfiltered_lib_tests(line: &str) -> bool {
    if line.contains("cargo llvm-cov") || line.contains("cargo mutants") {
        return true;
    }
    test_args(line).is_some_and(args_are_unfiltered)
}

/// The arguments after `cargo nextest run` or `cargo test`, if the line has either.
fn test_args(line: &str) -> Option<&str> {
    line.split_once("cargo nextest run")
        .or_else(|| line.split_once("cargo test"))
        .map(|(_, rest)| rest)
}

/// Flags that consume the next token as their value, so it is not a filter.
const TAKES_VALUE: &[&str] = &[
    "-p",
    "--package",
    "--features",
    "-F",
    "-j",
    "--jobs",
    "--target",
    "--manifest-path",
    "--profile",
    "--exclude",
    "--config",
    "-Z",
];

/// Flags that narrow the run to targets that cannot contain a lib unit test.
const TARGETED: &[&str] = &[
    "--doc",
    "--test",
    "--bench",
    "--example",
    "--bin",
    "--benches",
    "--examples",
    "--tests",
];

/// Libtest flags (after `--`) that consume the next token as their value. The
/// round-3 lane found the round-2 version read those values as name filters:
/// `cargo test --lib -- --skip slow` was classed as narrowed, when `--skip`
/// EXCLUDES and the run still reaches nas_archive's test — a false negative in
/// the dangerous direction, introduced by the fix for the previous one.
const BINARY_TAKES_VALUE: &[&str] = &[
    "--skip",
    "--test-threads",
    "--format",
    "--color",
    "--logfile",
    "-Z",
];

/// Cargo's arguments narrow the run if one targets a non-lib target or is a bare
/// positional name filter. The test binary's arguments (after `--`) narrow it if
/// one is a bare positional, since libtest reads that as a name filter too —
/// `cargo test --lib -- specific_test` runs one test, while `--test-threads=1`,
/// `--nocapture` and `--skip slow` narrow nothing.
fn args_are_unfiltered(args: &str) -> bool {
    let (cargo_args, binary_args) = match args.split_once(" -- ") {
        Some((c, b)) => (c, b),
        None => (args.strip_suffix(" --").unwrap_or(args), ""),
    };
    cargo_args_are_unfiltered(cargo_args) && binary_args_are_unfiltered(binary_args)
}

fn binary_args_are_unfiltered(args: &str) -> bool {
    let mut toks = args.split_whitespace();
    while let Some(tok) = toks.next() {
        if !tok.starts_with('-') {
            return false;
        }
        if BINARY_TAKES_VALUE.contains(&tok) {
            toks.next();
        }
    }
    true
}

fn cargo_args_are_unfiltered(args: &str) -> bool {
    let mut toks = args.split_whitespace();
    while let Some(tok) = toks.next() {
        if TARGETED.contains(&tok) || !tok.starts_with('-') {
            return false;
        }
        if TAKES_VALUE.contains(&tok) {
            toks.next();
        }
    }
    true
}

#[test]
fn every_unfiltered_lib_test_job_runs_where_rsync_is() {
    // nas_archive's the_environment_can_exercise_the_safety_property FAILS
    // rather than skips when rsync is absent (#289), and rsync is absent from
    // every runner container. So ANY job running unfiltered lib tests must be
    // routed to bare metal — discovered from the workflows, not listed by hand,
    // because the first draft of this fix listed coverage by hand and missed
    // stress.yml and mutation.yml, which a review lane found.
    let dir = format!("{}/.github/workflows", env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&dir).expect("workflows dir") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
            continue;
        }
        let file = path.file_name().unwrap().to_string_lossy().to_string();
        let wf = workflow(&file);
        let Some(jobs) = wf.get("jobs").and_then(Value::as_mapping) else {
            continue;
        };
        for (id, job) in jobs {
            let runs: String = job
                .get("steps")
                .and_then(Value::as_sequence)
                .map(|s| {
                    s.iter()
                        .filter_map(|st| st.get("run").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            if !runs_unfiltered_lib_tests(&runs) {
                continue;
            }
            let id = id.as_str().unwrap_or("?");
            let labels = runs_on(job);
            assert!(
                labels.iter().any(|l| l == BARE_METAL),
                "{file} job `{id}` runs unfiltered lib tests — including nas_archive's rsync-requiring \
                 safety test — on runs-on {labels:?}, which also matches rsync-less containers \
                 (forjar#598)"
            );
            found.push(format!("{file}:{id}"));
        }
    }
    // ANTI-VACUITY: the three measured today. If a rewrite of the detector
    // stops finding them, the assertion above is checking nothing.
    for want in [
        "coverage.yml:coverage",
        "stress.yml:stress",
        "mutation.yml:mutation",
    ] {
        assert!(
            found.iter().any(|f| f == want),
            "expected {want} to be detected as running unfiltered lib tests; found {found:?}"
        );
    }
}

#[test]
fn the_detector_does_not_pin_what_cannot_reach_the_test() {
    // Both edges of the detector's contract, including every form the round-2
    // review lane named. A detector that matched every `cargo test` would pass
    // the discovery test while pinning half the matrix to one host; one that
    // missed these forms would leave a future job silently unpinned.
    for unfiltered in [
        "cargo test",
        "cargo test --workspace",
        "cargo test --all-targets",
        "cargo nextest run",
        "cargo test --lib",
        "cargo test --lib --locked",
        "cargo test --locked --lib",
        "cargo test --lib -- --test-threads=1",
        "cargo test --lib -- --nocapture",
        "cargo test --lib -- --skip slow",
        "cargo test --lib -- --test-threads 4",
        "cargo test --lib -- --format json --color always",
        "cargo test --release --lib",
        "RUST_LOG=debug cargo test --lib",
        "cargo test --lib && cargo test --doc",
        "cargo test --doc; cargo test --lib",
        "cargo build || cargo test --lib",
        "cargo test -p forjar --lib",
        "cargo llvm-cov --lcov --output-path lcov.info",
        "cargo mutants --timeout 120",
    ] {
        assert!(
            runs_unfiltered_lib_tests(unfiltered),
            "missed an unfiltered run: {unfiltered}"
        );
    }
    for narrowed in [
        "cargo test --lib convergence -- --nocapture",
        "cargo test --lib hash_stability -- --nocapture",
        "cargo test --locked --test examples_validate",
        "cargo test --locked --doc",
        "cargo test -p forjar --lib hash_stability",
        "cargo nextest run some_filter",
        "cargo test --lib -- specific_test",
        "cargo test --lib -- --nocapture specific_test",
        "# cargo mutants would go here",
        "echo cargo is fine",
        "cargo test --doc && cargo test --locked --test foo",
    ] {
        assert!(
            !runs_unfiltered_lib_tests(narrowed),
            "matched a run that cannot reach nas_archive's test: {narrowed}"
        );
    }
}

#[test]
fn tool_assertions_name_the_runner_not_the_image() {
    // An absence on ONE runner is not evidence about the image. The message
    // said "not in the CI image" and pointed everyone at the image for months
    // while the cause was one runner class.
    let wf = workflow("proofs.yml");
    let jobs = wf.get("jobs").and_then(Value::as_mapping).expect("jobs");
    let mut seen = 0;
    for (id, job) in jobs {
        for step in tool_assertions(job) {
            seen += 1;
            let run = step.get("run").and_then(Value::as_str).unwrap_or("");
            assert!(
                run.contains("RUNNER_NAME"),
                "job `{}`'s tool assertion does not name the runner it failed on:\n{run}",
                id.as_str().unwrap_or("?")
            );
            assert!(
                !run.contains("not in the CI image"),
                "job `{}`'s tool assertion still claims the IMAGE lacks the tool, a cause it did \
                 not measure:\n{run}",
                id.as_str().unwrap_or("?")
            );
        }
    }
    assert!(
        seen >= 2,
        "found {seen} tool assertions; expected kani and lean"
    );
}

//! forjar#412 (CRUX audit E09): TWO schedulers, drifted apart.
//!
//! WHAT WAS OBSERVABLY WRONG. `apply` has a sequential loop
//! (`executor/machine.rs::execute_sequential` -> `resource_ops.rs`) and a
//! parallel wave loop (`executor/machine_wave.rs`), and the same fixture goes
//! through different code depending on one flag. Measured on 1.24.0 with two
//! independent `type: file` resources whose hooks append one line each to a
//! counter file, the ONLY difference between the runs being `--parallel`:
//!
//! ```text
//!   apply             -> pre-alpha 1  post-alpha 1  pre-beta 1  post-beta 1
//!   apply --parallel  -> pre-alpha 1  post-alpha 2  pre-beta 1  post-beta 2
//! ```
//!
//! `post_apply` ran TWICE per resource on the wave path: once inside the
//! spawned thread (`run_post_hook_if_success`) and once again in the record
//! phase through `output_verify::post_apply_failure`. A `post_apply` that
//! restarts a service, appends to a ledger or bumps a counter did it twice.
//!
//! Four more behaviours existed on ONE path only, all of them user-visible:
//!
//! ```text
//!   --retry N              sequential retried, --parallel did not
//!   runs/<id>/meta.yaml    sequential wrote `resources:`, --parallel wrote {}
//!   --trace                sequential printed the script, --parallel did not
//!   [n/total] progress     sequential printed it, --parallel did not
//! ```
//!
//! and a `post_apply` failure was REPORTED differently by each path: the
//! sequential path recorded the verify-failure text naming the resource's run
//! log, the wave path recorded `transport error: ...` plus the claim
//! "no exit code, no output and no run log exist for this resource" — for a
//! resource whose apply had in fact succeeded and whose transcript existed.
//!
//! WHY THESE ASSERTIONS. Each one is a COUNT or a BYTE COMPARISON of the two
//! paths over the SAME fixture in the SAME directories, so neither can pass
//! vacuously and neither depends on timing:
//!
//!   * `hooks_fire_exactly_once_per_resource_on_both_paths` — the counter file
//!     is written by the hooks themselves. 2 is 2 whatever the scheduler.
//!   * `the_lock_is_identical_between_the_two_paths` — bytes of
//!     `state.lock.yaml`, timestamps/durations/run-ids scrubbed. The failing
//!     fixture is used deliberately: `details.error` is where the two failure
//!     texts diverged.
//!   * `the_event_stream_and_the_run_log_agree_between_the_two_paths` — events
//!     are compared as a SORTED SET, because a wave genuinely interleaves two
//!     concurrent resources and demanding one order would be asserting that
//!     parallelism does not happen. The run log's `meta.yaml` is compared
//!     per-resource, which is where the wave path recorded nothing at all.
//!   * `a_failure_is_attributed_to_the_resource_that_failed` — the failing
//!     resource is the SECOND of three in one wave, so an index-0
//!     misattribution (the wave path's thread-panic arm hard-codes index 0)
//!     lands on `alpha` and is visible in the lock.
//!   * `retry_reruns_the_failed_resource_the_same_number_of_times` — attempts
//!     are counted by the resource's own command.
//!   * `trace_prints_the_generated_script_on_both_paths` — a literal marker
//!     in stderr.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

struct Fixture {
    dir: tempfile::TempDir,
}

struct Run {
    stderr: String,
    code: i32,
}

impl Fixture {
    /// `body` is a config with `{ROOT}` standing for this fixture's directory,
    /// so every fixture path is absolute and inside the tempdir.
    fn new(body: &str) -> Self {
        let f = Self {
            dir: tempfile::tempdir().expect("tempdir"),
        };
        let root = f.root().display().to_string();
        fs::write(f.path("forjar.yaml"), body.replace("{ROOT}", &root)).expect("write config");
        f
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    /// Run `apply` from a clean slate: no state, no produced files, no counters.
    ///
    /// Both schedulers are exercised over the SAME absolute paths, which is what
    /// lets the locks be compared byte for byte — a second tempdir would change
    /// every `hash_desired_state` in the file.
    fn run(&self, args: &[&str]) -> Run {
        for rel in ["state", "work"] {
            let _ = fs::remove_dir_all(self.path(rel));
        }
        for rel in ["hooks.log", "attempts.log"] {
            let _ = fs::remove_file(self.path(rel));
        }
        fs::create_dir_all(self.path("work")).expect("create work dir");

        let out = Command::new(forjar())
            .arg("apply")
            .arg("-f")
            .arg(self.path("forjar.yaml"))
            .arg("--state-dir")
            .arg(self.path("state"))
            .arg("--yes")
            .args(args)
            .current_dir(self.root())
            .env("HOME", self.root())
            .env("NO_COLOR", "1")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .output()
            .expect("run forjar apply");
        Run {
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            code: out.status.code().unwrap_or(-1),
        }
    }

    fn lock_text(&self) -> String {
        fs::read_to_string(self.path("state/local/state.lock.yaml")).expect("read machine lock")
    }

    fn events_text(&self) -> String {
        fs::read_to_string(self.path("state/local/events.jsonl")).expect("read event log")
    }

    /// The single run directory this apply created.
    fn run_dir(&self) -> PathBuf {
        let runs = self.path("state/local/runs");
        let mut dirs: Vec<PathBuf> = fs::read_dir(&runs)
            .expect("read runs dir")
            .map(|e| e.expect("run dir entry").path())
            .collect();
        dirs.sort();
        assert_eq!(dirs.len(), 1, "expected exactly one run dir in {runs:?}");
        dirs.pop().expect("one run dir")
    }

    fn meta_text(&self) -> String {
        fs::read_to_string(self.run_dir().join("meta.yaml")).expect("read run meta.yaml")
    }

    /// How many times each line appears in a counter file the fixture's own
    /// hooks/commands append to.
    fn counts(&self, rel: &str) -> BTreeMap<String, usize> {
        let text = fs::read_to_string(self.path(rel)).unwrap_or_default();
        let mut counts = BTreeMap::new();
        for line in text.lines().filter(|l| !l.trim().is_empty()) {
            *counts.entry(line.trim().to_string()).or_insert(0) += 1;
        }
        counts
    }
}

// --- normalisation -------------------------------------------------------

/// Replace every `r-<12 hex>` run id with a constant.
///
/// Run ids appear in the lock (inside failure text naming a run log), in the
/// events and in `meta.yaml`; they differ per invocation by construction.
fn scrub_run_ids(text: &str) -> String {
    let bytes: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        let looks_like_id = bytes[i] == 'r'
            && i + 13 < bytes.len()
            && bytes[i + 1] == '-'
            && bytes[i + 2..i + 14].iter().all(|c| c.is_ascii_hexdigit());
        if looks_like_id {
            out.push_str("r-RUNID");
            i += 14;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

/// Blank the value of any YAML key whose value is a wall-clock fact.
fn scrub_yaml(text: &str) -> String {
    const VOLATILE: [&str; 6] = [
        "generated_at:",
        "applied_at:",
        "duration_seconds:",
        "duration_secs:",
        "started_at:",
        "finished_at:",
    ];
    let scrubbed = scrub_run_ids(text);
    scrubbed
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            match VOLATILE.iter().find(|k| trimmed.starts_with(**k)) {
                Some(key) => format!("{}{key} <SCRUBBED>", &line[..line.len() - trimmed.len()]),
                None => line.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Remove one scalar JSON field, value included, from an object line.
fn strip_json_field(line: &str, key: &str) -> String {
    let needle = format!("\"{key}\":");
    let Some(start) = line.find(&needle) else {
        return line.to_string();
    };
    let rest = &line[start + needle.len()..];
    let end = if rest.starts_with('"') {
        rest[1..].find('"').map(|i| i + 2).unwrap_or(rest.len())
    } else {
        rest.find([',', '}']).unwrap_or(rest.len())
    };
    let mut tail = &rest[end..];
    if let Some(stripped) = tail.strip_prefix(',') {
        tail = stripped;
    }
    let head = line[..start].to_string();
    let joined = format!("{head}{tail}");
    // A stripped trailing field leaves `,}` behind.
    joined.replace(",}", "}")
}

/// The event stream as a comparable SET: volatile fields removed, sorted.
fn scrub_events(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut line = scrub_run_ids(l).to_string();
            for key in ["ts", "run_id", "duration_seconds", "total_seconds"] {
                line = strip_json_field(&line, key);
            }
            line
        })
        .collect();
    lines.sort();
    lines
}

/// The `resources:` block of a run's `meta.yaml`, per resource, durations gone.
fn meta_resources(meta: &str) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current: Option<String> = None;
    let mut in_resources = false;
    for line in scrub_yaml(meta).lines() {
        if !line.starts_with(' ') {
            in_resources = line.starts_with("resources:");
            current = None;
            continue;
        }
        if !in_resources {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        if indent == 2 {
            current = Some(line.trim().trim_end_matches(':').to_string());
            out.entry(current.clone().expect("resource key"))
                .or_default();
        } else if let Some(ref id) = current {
            out.entry(id.clone())
                .or_default()
                .push(line.trim().to_string());
        }
    }
    out
}

// --- fixtures ------------------------------------------------------------

const HEADER: &str = "version: \"1.0\"\nname: e09\n\
     machines:\n  local: { hostname: localhost, addr: 127.0.0.1, user: root }\n";

/// Two INDEPENDENT resources, so `--parallel` builds one wave of width 2 —
/// the only shape in which the wave path's multi-resource branch runs at all.
fn hooked_pair() -> Fixture {
    Fixture::new(&format!(
        "{HEADER}resources:\n\
         \x20 alpha:\n    type: file\n    machine: local\n    path: {{ROOT}}/work/alpha.txt\n\
         \x20   content: \"alpha\\n\"\n    mode: \"0644\"\n\
         \x20   pre_apply: echo pre-alpha >> {{ROOT}}/hooks.log\n\
         \x20   post_apply: echo post-alpha >> {{ROOT}}/hooks.log\n\
         \x20 beta:\n    type: file\n    machine: local\n    path: {{ROOT}}/work/beta.txt\n\
         \x20   content: \"beta\\n\"\n    mode: \"0644\"\n\
         \x20   pre_apply: echo pre-beta >> {{ROOT}}/hooks.log\n\
         \x20   post_apply: echo post-beta >> {{ROOT}}/hooks.log\n"
    ))
}

/// Three resources in one wave; the MIDDLE one's `post_apply` rejects the
/// result. Index 0 (`alpha`) must come out converged.
fn failing_middle() -> Fixture {
    Fixture::new(&format!(
        "{HEADER}resources:\n\
         \x20 alpha:\n    type: file\n    machine: local\n    path: {{ROOT}}/work/alpha.txt\n\
         \x20   content: \"alpha\\n\"\n    mode: \"0644\"\n\
         \x20 boom:\n    type: file\n    machine: local\n    path: {{ROOT}}/work/boom.txt\n\
         \x20   content: \"boom\\n\"\n    mode: \"0644\"\n\
         \x20   post_apply: |\n      echo 'post-hook says no' >&2\n      exit 3\n\
         \x20 gamma:\n    type: file\n    machine: local\n    path: {{ROOT}}/work/gamma.txt\n\
         \x20   content: \"gamma\\n\"\n    mode: \"0644\"\n"
    ))
}

/// A task that always fails, next to one that always succeeds, under
/// `continue_independent` — the policy under which `--retry` is live at all
/// (`StopOnFirst` sets `should_stop`, which ends the retry loop immediately).
fn retryable_pair() -> Fixture {
    Fixture::new(&format!(
        "{HEADER}policy:\n  failure: continue_independent\nresources:\n\
         \x20 keeper:\n    type: file\n    machine: local\n    path: {{ROOT}}/work/keeper.txt\n\
         \x20   content: \"k\\n\"\n    mode: \"0644\"\n\
         \x20 flaky:\n    type: task\n    machine: local\n    working_dir: {{ROOT}}\n\
         \x20   command: |\n      echo attempt >> {{ROOT}}/attempts.log\n      exit 7\n"
    ))
}

// --- the tests -----------------------------------------------------------

/// One scheduler means one hook per hook, whichever width the wave has.
#[test]
fn hooks_fire_exactly_once_per_resource_on_both_paths() {
    let fx = hooked_pair();

    let seq_run = fx.run(&[]);
    let sequential = fx.counts("hooks.log");
    let par_run = fx.run(&["--parallel"]);
    let parallel = fx.counts("hooks.log");

    assert_eq!(
        (seq_run.code, par_run.code),
        (0, 0),
        "fixture must converge on both paths.\nseq:\n{}\npar:\n{}",
        seq_run.stderr,
        par_run.stderr
    );
    assert_eq!(
        sequential.len(),
        4,
        "fixture is inert: expected 4 distinct hook lines, got {sequential:?}"
    );
    for (line, n) in &sequential {
        assert_eq!(*n, 1, "sequential ran '{line}' {n} times, expected once");
    }
    assert_eq!(
        parallel, sequential,
        "the two schedulers ran the hooks a different number of times.\n\
         sequential: {sequential:?}\n--parallel: {parallel:?}"
    );
}

/// The lock is the artefact everything downstream reads. It must not depend on
/// which scheduler produced it — including `details.error` on a failure.
#[test]
fn the_lock_is_identical_between_the_two_paths() {
    let fx = failing_middle();

    fx.run(&[]);
    let sequential = scrub_yaml(&fx.lock_text());
    fx.run(&["--parallel"]);
    let parallel = scrub_yaml(&fx.lock_text());

    assert!(
        sequential.contains("boom") && sequential.contains("status: failed"),
        "fixture is inert: the sequential lock records no failure:\n{sequential}"
    );
    assert_eq!(
        sequential, parallel,
        "state.lock.yaml differs between the schedulers.\n\
         --- sequential ---\n{sequential}\n--- --parallel ---\n{parallel}"
    );
}

/// The event stream and the run log are the audit surface. Both paths must
/// emit the same events and record the same per-resource run metadata.
#[test]
fn the_event_stream_and_the_run_log_agree_between_the_two_paths() {
    let fx = hooked_pair();

    fx.run(&[]);
    let seq_events = scrub_events(&fx.events_text());
    let seq_meta = meta_resources(&fx.meta_text());
    fx.run(&["--parallel"]);
    let par_events = scrub_events(&fx.events_text());
    let par_meta = meta_resources(&fx.meta_text());

    assert!(
        seq_events.iter().any(|e| e.contains("resource_converged")),
        "fixture is inert: no converge events at all:\n{seq_events:#?}"
    );
    assert_eq!(
        seq_events, par_events,
        "the event streams differ (sorted, timestamps and run ids scrubbed).\n\
         --- sequential ---\n{seq_events:#?}\n--- --parallel ---\n{par_events:#?}"
    );

    assert_eq!(
        seq_meta.len(),
        2,
        "fixture is inert: sequential meta.yaml names {} resources: {seq_meta:?}",
        seq_meta.len()
    );
    assert_eq!(
        seq_meta, par_meta,
        "runs/<id>/meta.yaml differs between the schedulers.\n\
         --- sequential ---\n{seq_meta:#?}\n--- --parallel ---\n{par_meta:#?}"
    );
}

/// A failure belongs to the resource that failed, and reads the same either way.
#[test]
fn a_failure_is_attributed_to_the_resource_that_failed() {
    let fx = failing_middle();

    let seq = fx.run(&[]);
    let seq_lock = fx.lock_text();
    let par = fx.run(&["--parallel"]);
    let par_lock = fx.lock_text();

    assert_eq!(
        (seq.code, par.code),
        (1, 1),
        "a failing post_apply must fail the apply on both paths"
    );
    for (label, lock) in [("sequential", &seq_lock), ("--parallel", &par_lock)] {
        let boom = lock.split("  boom:").nth(1).unwrap_or_default();
        let alpha = lock.split("  alpha:").nth(1).unwrap_or_default();
        assert!(
            boom.contains("status: failed"),
            "{label}: boom is not recorded failed:\n{lock}"
        );
        assert!(
            alpha.contains("status: converged"),
            "{label}: the failure was attributed to alpha, the resource at \
             index 0, which converged:\n{lock}"
        );
    }
    for (label, err) in [("sequential", &seq.stderr), ("--parallel", &par.stderr)] {
        assert!(
            err.contains("local/boom failed"),
            "{label}: the failure line does not name boom:\n{err}"
        );
        assert!(
            err.contains("post_apply hook failed (exit 3)"),
            "{label}: the failure text loses the hook's own verdict:\n{err}"
        );
        assert!(
            !err.contains("no run log exist"),
            "{label}: the failure text claims no run log exists for a resource \
             whose apply succeeded and whose transcript was written:\n{err}"
        );
    }
}

/// `--retry` is a promise about attempts, not about which scheduler ran.
#[test]
fn retry_reruns_the_failed_resource_the_same_number_of_times() {
    let fx = retryable_pair();

    fx.run(&["--retry", "1"]);
    let sequential = fx.counts("attempts.log");
    fx.run(&["--retry", "1", "--parallel"]);
    let parallel = fx.counts("attempts.log");

    assert_eq!(
        sequential.get("attempt").copied(),
        Some(2),
        "fixture is inert: sequential --retry 1 did not produce 2 attempts: {sequential:?}"
    );
    assert_eq!(
        parallel, sequential,
        "--retry 1 ran the failing resource {parallel:?} times under --parallel \
         and {sequential:?} times without it"
    );
}

/// `--trace` prints the script that is about to run — on either scheduler.
#[test]
fn trace_prints_the_generated_script_on_both_paths() {
    let fx = hooked_pair();

    let seq = fx.run(&["--trace"]);
    let par = fx.run(&["--trace", "--parallel"]);

    for (label, err) in [("sequential", &seq.stderr), ("--parallel", &par.stderr)] {
        for id in ["alpha", "beta"] {
            assert!(
                err.contains(&format!("[TRACE] {id} script:")),
                "{label}: --trace printed no script for {id}:\n{err}"
            );
        }
    }
}

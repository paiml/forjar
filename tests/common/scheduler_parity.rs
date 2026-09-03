//! Shared harness for the #412 scheduler-parity falsifiers (E09). Included
//! by `#[path]` from both test binaries so each stays under the 500-line
//! budget while reading the same fixtures and scrubbers.
#![allow(dead_code)]
#![allow(unused_imports)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

pub struct Fixture {
    pub dir: tempfile::TempDir,
}

pub struct Run {
    pub stderr: String,
    pub code: i32,
}

impl Fixture {
    /// `body` is a config with `{ROOT}` standing for this fixture's directory,
    /// so every fixture path is absolute and inside the tempdir.
    pub fn new(body: &str) -> Self {
        let f = Self {
            dir: tempfile::tempdir().expect("tempdir"),
        };
        let root = f.root().display().to_string();
        fs::write(f.path("forjar.yaml"), body.replace("{ROOT}", &root)).expect("write config");
        f
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    pub fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    /// Run `apply` from a clean slate: no state, no produced files, no counters.
    ///
    /// Both schedulers are exercised over the SAME absolute paths, which is what
    /// lets the locks be compared byte for byte — a second tempdir would change
    /// every `hash_desired_state` in the file.
    pub fn run(&self, args: &[&str]) -> Run {
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

    /// Run `apply` WITHOUT clearing state — the input cache is a statement
    /// about the previous run's lock, so it cannot be observed from scratch.
    pub fn run_keeping_state(&self, args: &[&str]) -> Run {
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

    pub fn rewrite_config(&self, body: &str) {
        let root = self.root().display().to_string();
        fs::write(self.path("forjar.yaml"), body.replace("{ROOT}", &root)).expect("rewrite config");
    }

    pub fn lock_text(&self) -> String {
        fs::read_to_string(self.path("state/local/state.lock.yaml")).expect("read machine lock")
    }

    pub fn events_text(&self) -> String {
        fs::read_to_string(self.path("state/local/events.jsonl")).expect("read event log")
    }

    /// The single run directory this apply created.
    pub fn run_dir(&self) -> PathBuf {
        let runs = self.path("state/local/runs");
        let mut dirs: Vec<PathBuf> = fs::read_dir(&runs)
            .expect("read runs dir")
            .map(|e| e.expect("run dir entry").path())
            .collect();
        dirs.sort();
        assert_eq!(dirs.len(), 1, "expected exactly one run dir in {runs:?}");
        dirs.pop().expect("one run dir")
    }

    pub fn meta_text(&self) -> String {
        fs::read_to_string(self.run_dir().join("meta.yaml")).expect("read run meta.yaml")
    }

    /// How many times each line appears in a counter file the fixture's own
    /// hooks/commands append to.
    pub fn counts(&self, rel: &str) -> BTreeMap<String, usize> {
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
pub fn scrub_run_ids(text: &str) -> String {
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
pub fn scrub_yaml(text: &str) -> String {
    const VOLATILE: [&str; 6] = [
        "generated_at:",
        "applied_at:",
        "duration_seconds:",
        "duration_secs:",
        "started_at:",
        "finished_at:",
    ];
    let scrubbed = scrub_run_ids(text);
    let lines: Vec<String> = scrubbed
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            match VOLATILE.iter().find(|k| trimmed.starts_with(**k)) {
                Some(key) => format!("{}{key} <SCRUBBED>", &line[..line.len() - trimmed.len()]),
                None => line.to_string(),
            }
        })
        .collect();
    sort_details_blocks(&lines).join("\n")
}

/// Sort the entries of every `details:` map.
///
/// `ResourceLock::details` is a `HashMap`, so its serialisation order is
/// nondeterministic RUN TO RUN — it has nothing to do with which scheduler
/// wrote it, and asserting on it would make this test flap. Entries are sorted
/// with their continuation lines (a block scalar such as `error: |`) attached,
/// so a multi-line failure text stays intact.
pub fn sort_details_blocks(lines: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        out.push(lines[i].clone());
        if lines[i].trim() != "details:" {
            i += 1;
            continue;
        }
        let indent = lines[i].len() - lines[i].trim_start().len() + 2;
        i += 1;
        let mut entries: Vec<Vec<String>> = Vec::new();
        while i < lines.len() {
            let line = &lines[i];
            let this_indent = line.len() - line.trim_start().len();
            if line.trim().is_empty() {
                match entries.last_mut() {
                    Some(entry) => entry.push(line.clone()),
                    None => break,
                }
            } else if this_indent == indent {
                entries.push(vec![line.clone()]);
            } else if this_indent > indent && !entries.is_empty() {
                entries
                    .last_mut()
                    .expect("open details entry")
                    .push(line.clone());
            } else {
                break;
            }
            i += 1;
        }
        entries.sort();
        out.extend(entries.into_iter().flatten());
    }
    out
}

/// Remove one scalar JSON field, value included, from an object line.
pub fn strip_json_field(line: &str, key: &str) -> String {
    let needle = format!("\"{key}\":");
    let Some(start) = line.find(&needle) else {
        return line.to_string();
    };
    let rest = &line[start + needle.len()..];
    let end = match rest.strip_prefix('"') {
        Some(quoted) => quoted.find('"').map(|i| i + 2).unwrap_or(rest.len()),
        None => rest.find([',', '}']).unwrap_or(rest.len()),
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
pub fn scrub_events(text: &str) -> Vec<String> {
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
pub fn meta_resources(meta: &str) -> BTreeMap<String, Vec<String>> {
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

pub const HEADER: &str = "version: \"1.0\"\nname: e09\n\
     machines:\n  local: { hostname: localhost, addr: 127.0.0.1, user: root }\n";

/// Two INDEPENDENT resources, so `--parallel` builds one wave of width 2 —
/// the only shape in which the wave path's multi-resource branch runs at all.
pub fn hooked_pair() -> Fixture {
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
pub fn failing_middle() -> Fixture {
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
pub fn retryable_pair() -> Fixture {
    Fixture::new(&format!(
        "{HEADER}policy:\n  failure: continue_independent\nresources:\n\
         \x20 keeper:\n    type: file\n    machine: local\n    path: {{ROOT}}/work/keeper.txt\n\
         \x20   content: \"k\\n\"\n    mode: \"0644\"\n\
         \x20 flaky:\n    type: task\n    machine: local\n    working_dir: {{ROOT}}\n\
         \x20   command: |\n      echo attempt >> {{ROOT}}/attempts.log\n      exit 7\n"
    ))
}

// --- the tests -----------------------------------------------------------

/// A cached task whose declared input never changes, beside a plain file so the
/// wave is wider than one. `marker` distinguishes the two revisions of the
/// command: the plan must call the task Update, while its INPUT is untouched.
///
/// `working_dir` is the fixture ROOT deliberately. `check_task_input_cache`
/// hashes `task_inputs` relative to the state dir's parent while
/// `probe::record_io_hashes` hashes them relative to `probe_base_dir`
/// (= `working_dir`), so the cache can only ever hit when the two agree. That
/// mismatch is a separate defect; this fixture stays out of its way.
pub fn cached_task_config(marker: &str) -> String {
    format!(
        "{HEADER}resources:\n\
         \x20 keeper:\n    type: file\n    machine: local\n    path: {{ROOT}}/work/keeper.txt\n\
         \x20   content: \"k\\n\"\n    mode: \"0644\"\n\
         \x20 builder:\n    type: task\n    machine: local\n    cache: true\n\
         \x20   working_dir: {{ROOT}}\n    task_inputs: [\"in/seed.txt\"]\n\
         \x20   output_artifacts: [\"work/out.bin\"]\n\
         \x20   command: |\n      # revision {marker}\n\
         \x20     echo build >> {{ROOT}}/attempts.log\n      touch {{ROOT}}/work/out.bin\n"
    )
}

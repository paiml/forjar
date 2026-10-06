//! A failed resource must not print or store its resolved secrets.
//!
//! forjar#679. The executor substitutes `{{secrets.*}}` before codegen, so the
//! script handed to the transport carries the plaintext. When the I8 gate
//! rejected that script, its numbered dump (#281) went into the failure text,
//! and the failure text went to stderr, `state.lock.yaml`, `events.jsonl` and
//! `last-apply.yaml` unredacted — `sensitive: true` included. #406 had closed
//! the same leak for run transcripts only; the failure text never met that
//! policy.
//!
//! Every resource here runs against 127.0.0.1 inside a tempdir, with a fixture
//! secret from `FORJAR_SECRET_*`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

/// The fixture plaintext. Long and unique so a match cannot be coincidental.
const PLAINTEXT: &str = "f679-PLAINTEXT-Hq3v8Ns1Wd6Ky4Pc-DO-NOT-COMMIT";
const SECRET_ENV: &str = "FORJAR_SECRET_F679_TOKEN";

/// A genuine SC2* error (`if … do`), which `validate_script` does not filter,
/// so the task is refused by the real I8 path before anything runs.
const I8_REFUSED: &str = "if true; do\n\
                          \x20 printf '%s\\n' 'tok={{secrets.f679-token}}'\n\
                          fi\n";

/// Valid shell that runs and fails, echoing the secret on stderr first.
const EXEC_FAILS: &str = "printf '%s\\n' 'tok={{secrets.f679-token}}' >&2\n\
                          exit 3\n";

struct Sandbox {
    dir: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("tempdir"),
        }
    }

    fn state(&self) -> PathBuf {
        self.dir.path().join("state")
    }

    /// One task, `command` as given, indented under a YAML block scalar.
    fn write_config(&self, command: &str, sensitive: bool, parallel: bool) -> PathBuf {
        let cfg = self.dir.path().join("forjar.yaml");
        let body: String = command.lines().map(|l| format!("      {l}\n")).collect();
        let flag = if sensitive {
            "    sensitive: true\n"
        } else {
            ""
        };
        fs::write(
            &cfg,
            format!(
                "version: \"1.0\"\n\
                 name: f679\n\
                 policy: {{ parallel_resources: {parallel} }}\n\
                 machines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\n\
                 resources:\n\
                 \x20 register:\n\
                 \x20   type: task\n\
                 \x20   machine: local\n\
                 \x20   working_dir: {dir}\n\
                 \x20   command: |\n{body}\
                 \x20   completion_check: \"false\"\n{flag}",
                dir = self.dir.path().display(),
            ),
        )
        .unwrap();
        cfg
    }

    /// `forjar apply`; returns stdout and stderr together.
    fn apply(&self, cfg: &Path) -> String {
        let out = Command::new(forjar())
            .env(SECRET_ENV, PLAINTEXT)
            .args([
                "apply",
                "-f",
                cfg.to_str().unwrap(),
                "--state-dir",
                self.state().to_str().unwrap(),
                "--yes",
            ])
            .output()
            .expect("forjar failed to start");
        let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&out.stderr));
        s
    }

    /// Every state file that holds the plaintext.
    fn leaking_files(&self) -> Vec<PathBuf> {
        walk(&self.state())
            .into_iter()
            .filter(|p| fs::read(p).is_ok_and(|b| String::from_utf8_lossy(&b).contains(PLAINTEXT)))
            .collect()
    }

    /// The text of the `last-apply.yaml` this apply wrote.
    fn last_apply(&self) -> String {
        let p = walk(&self.state())
            .into_iter()
            .find(|p| p.file_name().is_some_and(|n| n == "last-apply.yaml"))
            .expect("apply wrote a last-apply.yaml");
        fs::read_to_string(p).unwrap()
    }
}

fn walk(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            found.extend(walk(&p));
        } else {
            found.push(p);
        }
    }
    found
}

/// The run must really have hit the path under test, or "no leak" is vacuous.
fn assert_rejected_by_i8(output: &str, last_apply: &str) {
    assert!(
        output.contains("I8 violation") && last_apply.contains("I8 violation"),
        "the task was not refused by I8, so this test measured nothing:\n{output}"
    );
}

fn sensitive_i8_rejection_leaks_nothing(parallel: bool) {
    let sb = Sandbox::new();
    let output = sb.apply(&sb.write_config(I8_REFUSED, true, parallel));
    let last = sb.last_apply();
    assert_rejected_by_i8(&output, &last);
    assert!(
        !output.contains(PLAINTEXT),
        "the secret was printed (#679):\n{output}"
    );
    assert_eq!(
        sb.leaking_files(),
        Vec::<PathBuf>::new(),
        "the secret was written to state (#679)"
    );
    assert!(
        !last.contains("the script bashrs judged"),
        "a sensitive resource's script was dumped into last-apply.yaml:\n{last}"
    );
    assert!(
        last.contains("withheld"),
        "the report does not say the script was withheld:\n{last}"
    );
}

#[test]
fn a_sensitive_task_refused_by_i8_prints_and_stores_no_secret() {
    sensitive_i8_rejection_leaks_nothing(false);
}

#[test]
fn a_sensitive_task_refused_by_i8_prints_and_stores_no_secret_under_parallel() {
    sensitive_i8_rejection_leaks_nothing(true);
}

#[test]
fn a_plain_task_refused_by_i8_keeps_its_script_dump_with_the_secret_struck() {
    // #281's diagnostic stays for a resource that is not sensitive; only the
    // value goes.
    let sb = Sandbox::new();
    let output = sb.apply(&sb.write_config(I8_REFUSED, false, false));
    let last = sb.last_apply();
    assert_rejected_by_i8(&output, &last);
    assert!(
        !output.contains(PLAINTEXT),
        "the secret was printed:\n{output}"
    );
    assert_eq!(sb.leaking_files(), Vec::<PathBuf>::new());
    assert!(
        last.contains("the script bashrs judged") && last.contains("tok=***"),
        "the dump is gone or the value is not struck:\n{last}"
    );
}

#[test]
fn a_task_that_runs_and_fails_stores_no_secret_from_its_stderr() {
    let sb = Sandbox::new();
    let output = sb.apply(&sb.write_config(EXEC_FAILS, false, false));
    let last = sb.last_apply();
    assert!(
        last.contains("exit code 3"),
        "the task did not run and fail, so this test measured nothing:\n{last}"
    );
    assert!(
        !output.contains(PLAINTEXT),
        "the secret was printed:\n{output}"
    );
    assert_eq!(
        sb.leaking_files(),
        Vec::<PathBuf>::new(),
        "the stderr excerpt in the failure text carried the secret"
    );
}

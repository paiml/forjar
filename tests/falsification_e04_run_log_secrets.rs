//! Refs #406 (CRUX audit E04): a resolved secret must never reach `state/`.
//!
//! THE FLAW THIS CLOSES.
//!
//! `apply_single_resource` resolves `{{secrets.*}}` INTO the resource before
//! codegen, so the generated script carries the plaintext. `capture_output`
//! then writes that script verbatim to three files under
//! `state/<machine>/runs/<run_id>/`:
//!
//!   <res>.<action>.log    — the `script:` section of the human transcript
//!   <res>.<action>.json   — the `script` field of the machine transcript
//!   <res>.script          — the raw script, alone in a file
//!
//! `git_commit_state` then runs `git add state`, so `--auto-commit` publishes
//! every one of them into the repository's history. `redact_secrets` has
//! existed in `resolver::template` since FJ-2300 with NO production caller —
//! only tests, docs and examples ever invoked it.
//!
//! WHY THIS TEST GREPS FOR BASE64 TOO. #406's success criterion says "grep the
//! state tree for the plaintext — zero matches". Measured against unfixed
//! `main`, that criterion PASSES VACUOUSLY: `codegen::file` emits
//! `echo '<base64>' | base64 -d > path`, so a secret in `content:` is never in
//! the transcript as literal bytes. It is in it as
//! `YXBpX3Rva2VuPWUwNC1QTEFJTlRFWFQt…` — one `base64 -d` from plaintext, in
//! three files, in git. A value-substring redactor (which is what #406
//! proposes) cannot see it either: the blob encodes `api_token=` + secret +
//! `\n`, and because `api_token=` is 10 bytes the secret does not start on a
//! 3-byte boundary, so `base64(secret)` is not a substring of it.
//!
//! So the assertions below grep for BOTH forms, and the fix has to redact
//! base64 blobs whose PLAINTEXT contains a secret, not just literal matches.
//!
//! WHY THE WHOLE TREE. `walk` recurses everything under `--state-dir`, because
//! the defect is precisely that three DIFFERENT files each hold their own copy;
//! a test naming one of them passes while the other two leak.
//!
//! The redaction case additionally asserts the transcript IS still there. A fix
//! that simply stopped writing transcripts would pass a "no plaintext"
//! assertion while destroying the diagnostics #390 was filed to restore.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

/// The plaintext. Long and unique so a match cannot be coincidental.
const PLAINTEXT: &str = "e04-PLAINTEXT-Zq7x4Kv9Lm2Rw8Tn-DO-NOT-COMMIT";
const SECRET_ENV: &str = "FORJAR_SECRET_E04_TOKEN";

/// Standard base64, hand-rolled: `base64` is a normal dependency of `forjar`,
/// not a dev-dependency, so an integration test cannot link it.
fn b64(bytes: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(A[((n >> (18 - 6 * i)) & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

struct Sandbox {
    dir: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("tempdir"),
        }
    }
    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }
    fn state(&self) -> PathBuf {
        self.path("state")
    }

    /// Two resources, covering both ways a secret reaches a transcript:
    ///
    /// `managed` — `type: file`, secret inside `content:`. Codegen base64-encodes
    ///             the whole content, so the leak is ENCODED.
    /// `announce` — `type: task`, secret spliced straight into `command:`. The
    ///             leak is LITERAL.
    ///
    /// Both write only inside the sandbox; the machine is local/127.0.0.1.
    fn write_config(&self, sensitive: bool) -> PathBuf {
        self.write_config_with(sensitive, false)
    }

    /// `parallel` selects `machine_wave.rs` instead of `resource_ops.rs`. Both
    /// call `capture_exec_output`, from two different modules — which is exactly
    /// how #390-A shipped a path that wrote no transcript at all — so the
    /// redaction policy has to be proven on both.
    fn write_config_with(&self, sensitive: bool, parallel: bool) -> PathBuf {
        let cfg = self.path("forjar.yaml");
        let flag = if sensitive {
            "    sensitive: true\n"
        } else {
            ""
        };
        let policy = format!("policy: {{ parallel_resources: {parallel} }}\n");
        fs::write(
            &cfg,
            format!(
                "version: \"1.0\"\n\
                 name: e04\n{policy}\
                 machines:\n  local:\n    hostname: localhost\n    addr: 127.0.0.1\n\
                 resources:\n\
                 \x20 managed:\n\
                 \x20   type: file\n\
                 \x20   machine: local\n\
                 \x20   path: {target}\n\
                 \x20   content: \"api_token={{{{secrets.e04-token}}}}\\n\"\n\
                 \x20   mode: \"0600\"\n{flag}\
                 \x20 announce:\n\
                 \x20   type: task\n\
                 \x20   machine: local\n\
                 \x20   working_dir: {dir}\n\
                 \x20   command: |\n\
                 \x20     printf '%s\\n' 'literal={{{{secrets.e04-token}}}}' > {task_out}\n\
                 \x20   completion_check: |\n\
                 \x20     test -f {task_out}\n{flag}",
                target = self.path("managed.conf").display(),
                dir = self.dir.path().display(),
                task_out = self.path("task-out.txt").display(),
            ),
        )
        .unwrap();
        cfg
    }

    fn apply(&self, cfg: &Path) -> (i32, String) {
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
        (out.status.code().unwrap_or(-1), s)
    }
}

/// Every regular file under `root`, recursively.
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

/// Both recoverable forms of the secret: the literal bytes, and the base64 of
/// the file content that embeds them.
fn leak_forms() -> Vec<String> {
    vec![
        PLAINTEXT.to_string(),
        b64(format!("api_token={PLAINTEXT}\n").as_bytes()),
    ]
}

/// `(path, which form)` for every state file from which the secret is
/// recoverable.
fn leaks(root: &Path) -> Vec<(PathBuf, String)> {
    let forms = leak_forms();
    let mut out = Vec::new();
    for p in walk(root) {
        let Ok(bytes) = fs::read(&p) else { continue };
        let text = String::from_utf8_lossy(&bytes);
        for form in &forms {
            if text.contains(form.as_str()) {
                out.push((p.clone(), form.clone()));
            }
        }
    }
    out
}

fn run_transcripts(state: &Path) -> Vec<PathBuf> {
    walk(state)
        .into_iter()
        .filter(|p| p.to_string_lossy().contains("/runs/"))
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e == "log" || e == "json" || e == "script")
        })
        .filter(|p| {
            !p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("meta."))
        })
        .collect()
}

/// E04's success criterion, strengthened: apply resources carrying a secret,
/// then grep the ENTIRE `state/` tree for every recoverable form of it.
#[test]
fn no_state_file_contains_the_resolved_secret() {
    let sb = Sandbox::new();
    let cfg = sb.write_config(false);
    let (code, out) = sb.apply(&cfg);
    assert_eq!(code, 0, "apply must succeed; got:\n{out}");

    // Both resources really did converge with the RESOLVED secret — otherwise
    // this test proves nothing about redaction, only about a failed apply.
    let written = fs::read_to_string(sb.path("managed.conf")).expect("target file");
    assert!(
        written.contains(PLAINTEXT),
        "file resource must hold the resolved secret, got {written:?}"
    );
    let task_out = fs::read_to_string(sb.path("task-out.txt")).expect("task output");
    assert!(
        task_out.contains(PLAINTEXT),
        "task must have run with the resolved secret, got {task_out:?}"
    );

    // A leak scan over ZERO transcripts is vacuous (E04 quorum, agy lane): the
    // apply must actually have written a run directory for this to prove anything.
    assert!(
        !run_transcripts(&sb.state()).is_empty(),
        "no run transcript was written, so the leak scan below checks nothing"
    );
    let found = leaks(&sb.state());
    assert!(
        found.is_empty(),
        "secret recoverable from {} state file(s): {:#?}",
        found.len(),
        found
    );
}

/// The SAME guarantee on the parallel scheduler. `machine_wave.rs` is a second,
/// independent call site; #390-A is the precedent for one of the two silently
/// doing nothing.
#[test]
fn no_state_file_contains_the_resolved_secret_under_parallel() {
    let sb = Sandbox::new();
    let cfg = sb.write_config_with(false, true);
    let (code, out) = sb.apply(&cfg);
    assert_eq!(code, 0, "parallel apply must succeed; got:\n{out}");

    // The parallel path really was taken, and it really did write transcripts.
    assert!(
        !run_transcripts(&sb.state()).is_empty(),
        "no transcript written under --parallel; the redaction claim would be \
         vacuous"
    );
    let written = fs::read_to_string(sb.path("managed.conf")).expect("target file");
    assert!(written.contains(PLAINTEXT), "got {written:?}");

    let found = leaks(&sb.state());
    assert!(
        found.is_empty(),
        "secret recoverable from {} state file(s) on the parallel path: {:#?}",
        found.len(),
        found
    );
}

/// A redacted transcript is still a transcript. #390 was filed because a failing
/// task's output was destroyed; the fix for E04 must not re-destroy it.
#[test]
fn the_transcript_survives_redaction() {
    let sb = Sandbox::new();
    let cfg = sb.write_config(false);
    let (code, out) = sb.apply(&cfg);
    assert_eq!(code, 0, "apply must succeed; got:\n{out}");

    let transcripts = run_transcripts(&sb.state());
    assert!(
        !transcripts.is_empty(),
        "expected run transcripts under {}, found none",
        sb.state().display()
    );

    let script = transcripts
        .iter()
        .find(|p| p.file_name().is_some_and(|n| n == "managed.script"))
        .expect("managed.script must still be written");
    let body = fs::read_to_string(script).unwrap();
    assert!(
        body.contains("***"),
        "redacted script must carry the `***` marker, got:\n{body}"
    );
    // The non-secret scaffolding must survive, or the transcript is useless.
    assert!(
        body.contains("base64 -d"),
        "redaction must not eat the surrounding script, got:\n{body}"
    );
}

/// `sensitive: true` — Chef's `sensitive`, Ansible's `no_log` — suppresses the
/// transcript entirely, while the resource still converges.
#[test]
fn sensitive_resource_writes_no_transcript() {
    let sb = Sandbox::new();
    let cfg = sb.write_config(true);
    let (code, out) = sb.apply(&cfg);
    assert_eq!(code, 0, "apply must succeed; got:\n{out}");

    let written = fs::read_to_string(sb.path("managed.conf")).expect("target file");
    assert!(
        written.contains(PLAINTEXT),
        "a sensitive resource must still converge, got {written:?}"
    );

    let transcripts = run_transcripts(&sb.state());
    assert!(
        transcripts.is_empty(),
        "sensitive: true must write no transcript, found: {transcripts:#?}"
    );

    let found = leaks(&sb.state());
    assert!(found.is_empty(), "secret recoverable from {found:#?}");
}

/// `forjar init` must generate a `.gitignore` that keeps run transcripts out of
/// the repository `--auto-commit` writes to.
#[test]
fn init_gitignores_run_transcripts() {
    let sb = Sandbox::new();
    let out = Command::new(forjar())
        .args(["init", sb.dir.path().to_str().unwrap()])
        .output()
        .expect("forjar failed to start");
    assert!(out.status.success(), "init failed: {out:?}");

    let ignore =
        fs::read_to_string(sb.path(".gitignore")).expect("forjar init must generate a .gitignore");
    assert!(
        ignore.contains("state/*/runs/"),
        ".gitignore must exclude run transcripts, got:\n{ignore}"
    );
}

// ── `--auto-commit` (Refs #406, fix item 2) ─────────────────────────────────

fn git(repo: &Path, args: &[&str]) -> String {
    // SCRUB THE INHERITED GIT ENVIRONMENT. Under `git push`, the pre-push hook
    // (and so the quorum gate, and so `cargo test`) runs with GIT_DIR pointing
    // at the developer's own repository. A `git init`/`add -A`/`commit` here
    // with that inherited GIT_DIR operated on THAT repository with this
    // tempdir as its work tree: one commit by "e04" deleted 2,556 tracked
    // files from a feature branch (2026-09-02, forjar#406 quorum). The test
    // must never be able to escape its sandbox, whatever spawned it.
    let out = Command::new("git")
        .current_dir(repo)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_COMMON_DIR")
        .args(args)
        .output()
        .expect("git failed to start");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// `--auto-commit` stages `state` and must never carry a run transcript in with
/// it — INCLUDING in the repositories this exclusion exists for: the ones that
/// have been committing transcripts since before the fix. `.gitignore` cannot
/// help there; git ignores it for tracked paths.
///
/// WHY THE SEEDED, ALREADY-TRACKED TRANSCRIPT. git honours `:(exclude)` on a
/// DIRECTORY (`state/*/runs/`) only while that directory is entirely untracked,
/// where it can skip it without descending. Once one file under it is tracked,
/// matching is per PATH — `state/*/runs/` does not wildmatch
/// `state/local/runs/r-legacy/legacy.script` — and the exclusion silently stops
/// excluding, for the tracked files AND for the new ones beside them. Measured
/// on this branch before the pathspec grew its trailing `*`: this test failed
/// with the seeded transcript and both fresh run directories in the commit.
#[test]
fn auto_commit_never_stages_a_run_transcript() {
    let sb = Sandbox::new();
    let repo = sb.dir.path();
    git(repo, &["init", "-q", "."]);
    git(repo, &["config", "user.email", "e04@example.invalid"]);
    git(repo, &["config", "user.name", "e04"]);
    git(repo, &["config", "commit.gpgsign", "false"]);

    // A repository mid-migration: a transcript from before the fix is tracked,
    // and forjar is about to write over that same tree.
    let legacy = sb.state().join("local/runs/r-legacy");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join("legacy.script"), "echo seeded\n").unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "--no-verify", "-m", "legacy"]);
    fs::write(legacy.join("legacy.script"), format!("echo {PLAINTEXT}\n")).unwrap();

    let cfg = sb.write_config(false);
    let out = Command::new(forjar())
        .env(SECRET_ENV, PLAINTEXT)
        .args([
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            sb.state().to_str().unwrap(),
            "--yes",
            "--auto-commit",
        ])
        .output()
        .expect("forjar failed to start");
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.status.code(), Some(0), "apply must succeed:\n{log}");

    // CONTROL: forjar really did commit, and really did stage state. Without
    // this, "no transcript in the commit" would also pass if nothing committed.
    let head = git(repo, &["log", "-1", "--format=%s"]);
    assert!(
        head.contains("forjar:"),
        "no auto-commit happened: {head:?}"
    );
    let committed = git(repo, &["show", "--name-only", "--format=", "HEAD"]);
    assert!(
        committed.lines().any(|l| l.starts_with("state/")),
        "the auto-commit staged nothing under state/:\n{committed}"
    );

    // Transcripts — the pre-existing tracked one and every one written by this
    // apply — stay out of it.
    let leaked: Vec<&str> = committed.lines().filter(|l| l.contains("/runs/")).collect();
    assert!(
        leaked.is_empty(),
        "--auto-commit put run transcripts in git: {leaked:#?}"
    );
}

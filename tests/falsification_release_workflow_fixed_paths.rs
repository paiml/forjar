//! PMAT-230: NOTHING ON THESE RUNNERS IS EPHEMERAL, SO NOTHING MAY ASSUME IT IS.
//!
//! The clean-room runners are long-lived by design — that is what makes them a
//! clean room for this repository's binaries — and `/tmp` survives between jobs
//! on them. `release.yml`'s `checksums` job already knows: it clears its
//! staging directory first, under a comment recalling the v1.18.0 release whose
//! `SHA256SUMS` carried ten lines, four of them belonging to 1.17.0. The jobs
//! either side of it did not, and the v1.28.0 release stopped there:
//! ``/tmp/SHA256SUMS already exists (use `--clobber` to overwrite file or
//! `--skip-existing` to skip file)``, leaving a draft release with thirteen
//! assets and no installer, and `homebrew`'s `git clone` into `/tmp/tap` — two
//! steps further on, and therefore never yet reached — waiting behind it.
//!
//! Two rules, both read from the workflow YAML at `env!("CARGO_MANIFEST_DIR")`,
//! the same bytes GitHub Actions parses. The sibling binary
//! `falsification_release_workflow_shape.rs` holds PMAT-166's rules 1-8 about
//! the shape of the release itself; the small helpers below are duplicated from
//! it rather than lifted into `tests/common/`, for the reason that file states.

use std::fs;
use std::path::{Path, PathBuf};

fn workflows_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows")
}

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}"))
}

fn all_workflow_files() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(workflows_dir())
        .expect("read .github/workflows")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yml"))
        .collect();
    out.sort();
    out
}

/// Slice out one top-level job's block (2-space-indented `name:` key up to,
/// but not including, the next 2-space-indented key or EOF).
fn job_block<'a>(text: &'a str, job_name: &str) -> &'a str {
    let marker = format!("\n  {job_name}:\n");
    let start = text
        .find(&marker)
        .unwrap_or_else(|| panic!("no job `{job_name}` in the workflow"))
        + 1;
    let rest = &text[start..];
    let end = rest
        .match_indices('\n')
        .find(|(i, _)| {
            let line = &rest[i + 1..];
            line.starts_with("  ")
                && !line.starts_with("   ")
                && line.trim_end().ends_with(':')
                && !line[2..].starts_with('-')
        })
        .map(|(i, _)| i + 1)
        .unwrap_or(rest.len());
    &rest[..end]
}

fn non_comment_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter(|l| !l.trim_start().starts_with('#'))
}

// ---------------------------------------------------------------------
// Rule 1 (PMAT-230): every `gh release download` passes --clobber and none
// --skip-existing.
//
// PMAT-230. The clean-room runners are NOT ephemeral and `/tmp` persists
// between jobs. `checksums` already knows this — its staging directory is
// cleared first, with a comment recalling the v1.18.0 release whose
// SHA256SUMS carried ten lines, four of them belonging to 1.17.0 — but the
// two jobs that fetch a file with `gh release download` write into a fixed
// path with no guard at all. The v1.28.0 release died there:
// ``/tmp/SHA256SUMS already exists (use `--clobber` to overwrite file or
// `--skip-existing` to skip file)``, leaving a draft release with thirteen assets
// and no installer.
//
// `--skip-existing` is refused as well as absence, and it is the more
// dangerous of the two: it exits 0 leaving the PREVIOUS release's checksums
// in place, the following `test -s` guard passes, and `forjar dist
// --checksums-file` embeds them into `install.sh`. That is the v1.18.0
// failure again with a friendlier exit code, which is why this rule cannot
// be satisfied by making the error go away. `||` is refused for the same
// reason: suppressing the exit code leaves the stale file in place too.
//
// THIS IS A TEXT RATCHET AND HERE IS WHAT IT CANNOT CATCH. It reads the
// workflow's own text, so a download assembled from a variable (`$GH
// release download`, `eval "$cmd"`), one written inside a here-doc that
// this line-joiner does not follow, and one in a script the workflow calls
// rather than in the workflow itself all pass unseen. Six quorum lanes over
// two rounds shaped what it does catch: the subcommand is read as a token
// after `gh release` and not as the literal string `gh release download`,
// so global flags before the subcommand and a backslash anywhere inside it
// are caught; the flags are compared as whole tokens, so `--pattern
// "*--clobber*"` does not satisfy the check; and a trailing `#` comment is
// cut before any of that, so `… # --clobber` does not either. What it
// guarantees is that a call site written the way all three of today's are
// written cannot lose its --clobber without turning this test red.
// ---------------------------------------------------------------------
/// The shell text of one line, with any trailing `#` comment removed.
///
/// `non_comment_lines` drops a line that BEGINS with `#`; a comment at the
/// end of a command line survives it, and `gh release download … # --clobber`
/// would then satisfy a flag check while the command carries no such flag
/// (found by a quorum lane). Naive on purpose: a `#` inside a quoted string
/// is cut too. No call site in this repository has one, and a rule that
/// under-reads a command is safe here — it can only make the rule stricter.
fn strip_trailing_comment(line: &str) -> &str {
    match line.find('#') {
        Some(i) => &line[..i],
        None => line,
    }
}

/// One shell command, joined across the backslash continuations it is
/// written with, comments removed, and CUT at the first `&&`, `;` or `|`.
///
/// The cut is what keeps two commands on one continued line from lending
/// each other their flags — `gh release download … && \` followed by a
/// second `gh release` line would otherwise be read as one command carrying
/// the union of both flag sets (found by a quorum lane).
fn joined_command(lines: &[&str], start: usize) -> String {
    let mut cmd = String::new();
    for line in &lines[start..] {
        let text = strip_trailing_comment(line);
        cmd.push_str(text);
        cmd.push('\n');
        if !text.trim_end().ends_with('\\') {
            break;
        }
    }
    // `&&` and `;` only. Cutting at a bare `|` would also cut at `||` and
    // silently disarm the assertion below that refuses a suppressed failure —
    // measured, by the mutation battery, one edit after it was written.
    for sep in ["&&", ";"] {
        if let Some(i) = cmd.find(sep) {
            cmd.truncate(i);
        }
    }
    cmd
}

/// The `gh release` subcommand this command invokes, if it invokes one.
///
/// Tokens, never substrings, and the subcommand is the first token after
/// `release` that is not a flag — `gh` takes its global flags before the
/// subcommand (`gh release -R owner/repo download …`), and `-R`/`--repo`
/// take a value. That is the whole of `gh`'s grammar this needs to know,
/// and it is stated here rather than left implicit: three quorum lanes
/// walked past the earlier substring match, and two more showed that
/// matching the bare word `download` anywhere flags `gh release upload
/// --title download` as a download.
fn release_subcommand(cmd: &str) -> Option<&str> {
    let toks: Vec<&str> = cmd.split_whitespace().collect();
    let gh = toks.iter().position(|t| *t == "gh")?;
    // `release` must be THIS `gh`'s subcommand, not merely a later token: a
    // quorum lane showed `gh api --pattern release download` being read as a
    // release download because the word appeared somewhere after `gh`.
    let rel = next_word(&toks, gh + 1)?;
    if toks[rel] != "release" {
        return None;
    }
    next_word(&toks, rel + 1).map(|i| toks[i])
}

/// The index of the first token at or after `from` that is not a flag,
/// skipping the value of the two `gh` flags that take one.
fn next_word(toks: &[&str], from: usize) -> Option<usize> {
    let mut i = from;
    while i < toks.len() {
        let t = toks[i];
        if t == "-R" || t == "--repo" {
            i += 2;
        } else if t.starts_with('-') {
            i += 1;
        } else {
            return Some(i);
        }
    }
    None
}

/// The two halves of rule 9, asserted against one call site.
fn assert_download_overwrites(file_name: &str, cmd: &str) {
    let has = |flag: &str| cmd.split_whitespace().any(|t| t == flag);
    assert!(
        !has("--skip-existing"),
        "PMAT-230 rule 9: {file_name} passes --skip-existing to `gh release download`. \
         On these non-ephemeral runners that KEEPS the file the PREVIOUS release left \
         behind and exits 0, so the step's own `test -s` guard passes and the stale \
         checksums reach `install.sh`. Overwrite it with --clobber:\n{cmd}"
    );
    assert!(
        !cmd.contains("||"),
        "PMAT-230 rule 9: {file_name} guards a `gh release download` with `||`. A \
         suppressed failure leaves the file the previous release left exactly where it \
         was and continues, which is the outcome --clobber exists to prevent; this \
         repository's rule is that nothing swallows a measurement:\n{cmd}"
    );
    assert!(
        has("--clobber"),
        "PMAT-230 rule 9: {file_name} runs `gh release download` without --clobber. \
         `/tmp` persists between jobs on the clean-room runners, so the second release \
         to use this path dies with `already exists` — the v1.28.0 cut did, leaving a \
         draft release with no installer. The `checksums` job's staging-directory \
         comment is the same lesson one job over:\n{cmd}"
    );
}

/// Every `gh release download` call site in one workflow file.
fn download_sites(text: &str) -> Vec<String> {
    let lines: Vec<&str> = non_comment_lines(text).collect();
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| {
            strip_trailing_comment(l)
                .split_whitespace()
                .any(|t| t == "gh")
        })
        .map(|(i, _)| joined_command(&lines, i))
        .filter(|cmd| release_subcommand(cmd) == Some("download"))
        .collect()
}

#[test]
fn rule9_every_release_download_overwrites_what_a_previous_release_left() {
    let mut sites = 0usize;
    for path in all_workflow_files() {
        let file_name = path
            .file_name()
            .expect("workflow file has a name")
            .to_string_lossy()
            .to_string();
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        for cmd in download_sites(&text) {
            sites += 1;
            assert_download_overwrites(&file_name, &cmd);
        }
    }
    assert!(
        sites >= 3,
        "PMAT-230 rule 9: found only {sites} `gh release download` call site(s); the \
         rule is meant to sweep every workflow, and a rule that matches nothing passes \
         for the wrong reason"
    );
}

// ---------------------------------------------------------------------
// Rule 10: a job that writes into a FIXED /tmp directory clears it first.
//
// PMAT-230, found by a quorum lane reviewing rule 9's fix. The clean-room
// runners are not ephemeral, and release.yml already carries the lesson in
// its `checksums` job — "THE STAGING DIR IS A FIXED PATH ON A RUNNER THAT
// IS NOT EPHEMERAL", written after v1.18.0 shipped a SHA256SUMS with ten
// lines, four of them belonging to 1.17.0. Two more jobs write into fixed
// /tmp paths: `dist-artifacts` generates into /tmp/dist-output and uploads
// that whole directory as the release's artifact, and `homebrew` clones the
// tap into /tmp/tap, which `git clone` refuses when it exists. The second
// had never been reached, because the checksums download two steps above it
// died first (v1.27.0's homebrew job failed exactly there).
//
// Ordering, not merely presence: a clear that runs after the write is not a
// guard.
// ---------------------------------------------------------------------
#[test]
fn rule10_a_fixed_tmp_directory_is_cleared_before_it_is_written() {
    let text = read(".github/workflows/release.yml");
    for (job, clear, write) in [
        (
            "dist-artifacts",
            "rm -rf /tmp/dist-output",
            "--output-dir /tmp/dist-output",
        ),
        ("homebrew", "rm -rf /tmp/tap", "git clone"),
    ] {
        // Comment lines are dropped first: this rule's own explanation quotes
        // `git clone`, and a marker found inside a comment is not the step.
        let block: String = non_comment_lines(job_block(&text, job))
            .collect::<Vec<_>>()
            .join("\n");
        let c = block.find(clear).unwrap_or_else(|| {
            panic!(
                "PMAT-230 rule 10: release.yml's {job} job writes into a fixed /tmp \
                 directory on a runner that is not ephemeral and never clears it. \
                 `{clear}` is missing, so the previous release's files are still there \
                 when this one runs — the shape the `checksums` job's own staging-\
                 directory comment records from v1.18.0."
            )
        });
        let w = block
            .find(write)
            .unwrap_or_else(|| panic!("PMAT-230 rule 10: {job} no longer contains `{write}`"));
        assert!(
            c < w,
            "PMAT-230 rule 10: release.yml's {job} job clears its fixed directory AFTER \
             it writes into it (`{clear}` appears after `{write}`); a clear that runs \
             after the write is not a guard"
        );
    }
}

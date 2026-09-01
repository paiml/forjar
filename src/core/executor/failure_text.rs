//! Refs #390: the failure text an operator actually reads.
//!
//! # The incident
//!
//! An operator building llama.cpp with CUDA on `gx10` ran `forjar apply` six
//! times against the same task, editing `command:` between runs to add `echo`,
//! `nvcc --version` and `grep GGML_CUDA` diagnostics. Every run printed, byte
//! for byte:
//!
//! ```text
//!   JIDOKA: gx10/llama-cpp-build failed — dependents will be skipped:
//!   exit code 1: CMake Warning:
//!     Manually-specified variables were not used by the project:
//!       CMAKE_BUILD_TYPE=Release
//!   CMake Deprecation Warning: compat
//!   task=not-converged: command exited 0 but completion_check still fails
//!   task=not-converged: the declared state was not reached
//! ```
//!
//! Not one of the added diagnostics ever appeared. The two cmake lines appeared
//! every time, unchanged. From that they concluded — reasonably, and wrongly —
//! that forjar was replaying a cached transcript instead of running the edited
//! script, and filed it as a caching defect.
//!
//! Seven independent reproduction lanes, one of them over a real SSH transport,
//! later proved with append-only counter files that the command re-ran on every
//! single apply. Nothing was cached. The whole symptom was stream routing:
//!
//!   * `echo`, `nvcc --version` and `grep` write to STDOUT.
//!   * cmake's `CMake Warning` lines, and llama.cpp's own bare
//!     `message("CMAKE_BUILD_TYPE=...")` (CMake NOTICE mode), write to STDERR.
//!   * The operator's only failure line was built as
//!     `format!("exit code {}: {}", out.exit_code, out.stderr.trim())` — in
//!     `resource_ops.rs` and, duplicated, in `machine_wave.rs`. `out.stdout`
//!     was structurally absent from it.
//!
//! "Identical output across six runs" was forced by construction: the message
//! is a pure function of the exit code and stderr, and the operator's edits
//! changed neither.
//!
//! The headline also named the wrong failure. The command had exited 0 every
//! time; what exited 1 was the `completion_check` that GH-254 re-asserts at the
//! end of the generated script (`resources::task::batch_script`). Six builds
//! were spent hunting a compiler error that never existed, under a line reading
//! `exit code 1`.
//!
//! # What this module is
//!
//! The one place an `ExecOutput` becomes text a human reads. Before it there
//! were five constructors on the apply path and they disagreed about which
//! stream mattered. `resource_ops.rs` and `machine_wave.rs` reported stderr and
//! destroyed stdout. `output_verify::verify_against_host` reported stdout and
//! destroyed stderr — the exact mirror image, on the branch every `type: task`
//! without a `completion_check` lands in, because
//! `resources::task::check_script` falls through to
//! `verdict::always_diverged("task=pending")`. Whichever half of a failure
//! mattered, some path was built to throw it away.
//!
//! # What the text is allowed to cost
//!
//! It is not console-only. `record_failure` writes it verbatim into
//! `ProvenanceEvent::ResourceFailed`, i.e. into `state/<machine>/events.jsonl`,
//! which is append-only and which `forjar history --resource <id>` replays. A
//! cmake build emits megabytes on stdout, so each stream is excerpted
//! head-AND-tail with the middle elided and the true byte count stated — never
//! more than `HEAD_BYTES + TAIL_BYTES` per stream.
//!
//! Head as well as tail, and that is the load-bearing half. #390's missing
//! diagnostics ran BEFORE the build, so the tail-only excerpt a log viewer
//! would keep is exactly the one that would still have elided the lines the
//! operator spent six runs looking for.
//!
//! Note which direction this moves: today's `out.stderr.trim()` has no bound at
//! all, so a 3 MB stderr goes verbatim into an append-only log on every failed
//! apply. This is the first ceiling that string has ever had.
//!
//! # The pointer is conditional on purpose
//!
//! The full, unelided transcript is at
//! `state/<machine>/runs/<run_id>/<resource>.<action>.log`, and nothing at the
//! failure site had ever named it. It does now — but the path is handed in by
//! the code that WROTE it (`run_capture::capture_exec_output` returns it), so
//! the message can only name a file that exists. Under `--parallel` no run log
//! is written at all today; there the pointer is absent and the excerpt is the
//! only surviving copy. That gap is tracked as #390-A; this module will not
//! paper over it with a path that lies.

use super::*;
use crate::core::strutil::truncate_at_boundary;
use crate::resources::task::NOT_CONVERGED_MARKER;
use std::path::Path;

/// Bytes kept from the FRONT of an over-long stream.
const HEAD_BYTES: usize = 800;

/// Bytes kept from the END of an over-long stream.
const TAIL_BYTES: usize = 1200;

/// Head-only ceiling for an error that is prose rather than a stream.
const PROSE_BYTES: usize = 4096;

/// Where an execution happened, and where its transcript landed.
pub(super) struct Site<'a> {
    /// Resource id, for the pointer line.
    pub resource_id: &'a str,
    /// `--state-dir`. Relative by default, which is half of why #390's
    /// evidence was gone by the time anyone looked for it.
    pub state_dir: &'a Path,
    /// This apply's run id, when it has one.
    pub run_id: Option<&'a str>,
    /// The run log THIS execution wrote, handed in by the writer so a message
    /// can never name a file that was not created.
    pub log: Option<&'a Path>,
    /// The TEMPLATE-RESOLVED resource, so a printed `completion_check` is the
    /// text that actually ran and not one with `{{params.*}}` still in it.
    pub resolved: &'a Resource,
}

/// The report for a command that RAN and exited non-zero.
///
/// Refs #390: replaces `format!("exit code {}: {}", out.exit_code,
/// out.stderr.trim())` at `resource_ops.rs` and its duplicate in
/// `machine_wave.rs`.
pub(super) fn exec_failure(site: &Site, out: &transport::ExecOutput) -> String {
    let mut msg = exec_headline(site, out);
    msg.push_str(&streams(out));
    msg.push_str(&pointer(site));
    msg
}

/// FAILED vs NOT CONVERGED — different diagnoses, different next actions.
///
/// Both used to print as `exit code N:`. #390 is entirely the second kind and
/// its reporter read it as the first for six runs.
fn exec_headline(site: &Site, out: &transport::ExecOutput) -> String {
    if !out.stderr.contains(NOT_CONVERGED_MARKER) {
        return format!(
            "exit code {} — the command FAILED. Under `set -euo pipefail` the \
             script\nstops at the first failing line, so anything below it did \
             not run.\n",
            out.exit_code
        );
    }
    let mut head = format!(
        "NOT CONVERGED (script exit {}) — the command itself exited 0. What \
         failed is\nthe completion_check GH-254 re-asserts after it, so the \
         command is not what\nwent wrong; read STDOUT below.\n",
        out.exit_code
    );
    if let Some(check) = site.resolved.completion_check.as_deref() {
        head.push_str("  completion_check, re-run after the command:\n");
        for line in check.trim_end().lines() {
            head.push_str("    > ");
            head.push_str(line);
            head.push('\n');
        }
    }
    head.push_str(nested_shell_caveat(site.resolved));
    head
}

/// Refs #390-E: `timeout:` and `sudo: true` run the command in a NESTED `bash`
/// that does not inherit the outer `set -euo pipefail` — see
/// `resources::task::batch_script`. There an early failing line neither aborts
/// the script nor changes its exit status, so "the command itself exited 0" is
/// a claim this module cannot honestly make.
///
/// Say so rather than assert a diagnosis known to be wrong under a documented,
/// still-open defect. Trading one confidently wrong label for a more
/// authoritative one is the failure mode this whole module exists to end.
fn nested_shell_caveat(resolved: &Resource) -> &'static str {
    if resolved.timeout.is_none() && !resolved.sudo {
        return "";
    }
    "  NOTE this task declares `timeout:` or `sudo:`, so its command runs in a nested\n       \
     `bash` that does NOT inherit `set -euo pipefail` (#390-E). An earlier line\n       \
     may have failed without stopping the script — do not read the verdict above\n       \
     as proof that every command in it succeeded.\n"
}

/// The report for a command that exited 0 whose POST-APPLY verification then
/// said no: a `post_apply` hook, missing `output_artifacts` (FJ-2731), or the
/// host itself (FJ-2732).
///
/// Its own entry point because the diagnosis differs from a command failure —
/// nothing the operator wrote returned an error, forjar asked a second question
/// afterwards and the host answered no. Rendering that as `exit code 1:` is how
/// one sentence came to mean three things.
pub(super) fn verify_failure(site: &Site, out: &transport::ExecOutput, verdict: &str) -> String {
    let mut msg = String::from(
        "NOT CONVERGED — the command exited 0, but forjar asked the host \
         afterwards\nwhether the declared state is present and the answer was \
         no:\n",
    );
    for line in verdict.lines() {
        msg.push_str("  ! ");
        msg.push_str(line);
        msg.push('\n');
    }
    msg.push_str(nested_shell_caveat(site.resolved));
    msg.push_str("  what the apply that \"succeeded\" printed:\n");
    msg.push_str(&streams(out));
    msg.push_str(&pointer(site));
    msg
}

/// The report for an execution that never produced an exit code: a timeout, a
/// spawn failure, or an I8 (bashrs) rejection.
///
/// The `transport error: ` prefix is byte-identical to the string it replaces
/// and the error's own text is clipped from the FRONT, both deliberately.
/// `core::error::DECLARED_MARKERS` classifies an I8 rejection by a marker that
/// sits at the head of `e`, and forjar#281's numbered-script diagnostic is the
/// next thing after it. A tail cut would silently re-code a deterministic
/// validation failure as a retryable connection failure.
///
/// No streams and no log pointer: on this path forjar has neither. Saying so is
/// the point — #390's reporter went hunting for a file that was never written.
pub(super) fn transport_failure(e: &str) -> String {
    format!(
        "transport error: {}\n--- no exit code, no output and no run log exist \
         for this resource (#390-D)\n",
        clip_head(e, PROSE_BYTES)
    )
}

/// The report for a `pre_apply` / `post_apply` hook that exited non-zero.
///
/// Refs #390: three byte-identical copies of this existed —
/// `output_verify::run_pre_apply_hook`, `output_verify::check_post_hook` and
/// `machine_wave::exec_validated_hook` — every one of them stderr-only. A hook
/// that explains itself with `echo "nginx config invalid: line 42"` and no
/// `>&2`, which is what people actually write, lost its diagnostic in exactly
/// the way the reporter's task did.
///
/// No log pointer: hooks run through `exec_script_timeout` and are not
/// captured, so naming a run log here would name one that does not exist.
pub(super) fn hook_failure(label: &str, out: &transport::ExecOutput) -> String {
    format!(
        "{label} hook failed (exit {}){}",
        out.exit_code,
        streams(out)
    )
}

/// The report for a hook that could not be executed at all.
pub(super) fn hook_error(label: &str, e: &str) -> String {
    format!("{label} hook error: {}", clip_head(e, PROSE_BYTES))
}

/// FJ-2732's verdict, with BOTH of the check script's streams.
///
/// Refs #390: `verify_against_host` reported `out.stdout.trim()` and destroyed
/// `out.stderr` — the mirror image of the defect this module exists for, on the
/// branch every `type: task` without a `completion_check` reaches. A check that
/// explains itself on stderr (`test: /opt/x: No such file or directory`)
/// reported only `task=pending`.
pub(super) fn host_verdict(out: &transport::ExecOutput) -> String {
    format!(
        "apply exited 0 but the host does not report the declared state \
         (check exit {}){}",
        out.exit_code,
        streams(out)
    )
}

/// Both streams, always both, labelled and excerpted — and never silent.
///
/// An empty stream still gets a line. "(empty)" is itself a diagnosis: its
/// absence is what left #390's reporter unable to tell "my echoes produced
/// nothing" from "forjar is hiding my echoes" across six builds.
///
/// `pub(super)` because `helpers::copia_apply_file` reports a signature-phase
/// failure that is neither a resource command nor a hook, and it was the fifth
/// stderr-only constructor in this module tree.
pub(super) fn streams(out: &transport::ExecOutput) -> String {
    let mut s = String::new();
    s.push_str(&stream_block("stderr", &out.stderr));
    s.push_str(&stream_block("stdout", &out.stdout));
    s
}

/// One labelled stream, stating the TRUE size it was excerpted from.
fn stream_block(label: &str, raw: &str) -> String {
    let body = raw.trim();
    if body.is_empty() {
        return format!("\n--- {label}: (empty) ---");
    }
    format!(
        "\n--- {label} ({} bytes) ---\n{}",
        body.len(),
        excerpt(body)
    )
}

/// Head AND tail of one stream, with the middle elided and the drop stated.
///
/// Byte indices are walked to a char boundary in both directions: build output
/// is arbitrary UTF-8 and slicing it at a fixed offset panics mid-codepoint. A
/// diagnostic that panics is worse than the bug it was printing, and this code
/// runs only when something has already gone wrong.
fn excerpt(body: &str) -> String {
    if body.len() <= HEAD_BYTES + TAIL_BYTES {
        return body.to_string();
    }
    let head = truncate_at_boundary(body, HEAD_BYTES);
    let mut start = body.len() - TAIL_BYTES;
    while !body.is_char_boundary(start) {
        start += 1;
    }
    let elided = start - head.len();
    format!(
        "{head}\n[… {elided} bytes elided from the middle; the whole stream is \
         in the run log …]\n{}",
        &body[start..]
    )
}

/// Keep the FIRST `max` bytes on a char boundary, and state what was dropped.
fn clip_head(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let head = truncate_at_boundary(s, max);
    format!("{head}\n[… {} bytes elided …]", s.len() - head.len())
}

/// Name the transcript and the command that renders it — but only when one was
/// written, and always as an ABSOLUTE path.
///
/// `--state-dir` defaults to the RELATIVE path `state`, so a relative path
/// printed here resolves only from the directory the apply ran in, and a
/// stateless CI runner deletes it with the checkout. That is how #390's
/// reporter lost evidence which had been on disk the whole time, so when the
/// state dir is relative the note says so.
fn pointer(site: &Site) -> String {
    let Some(log) = site.log else {
        return "\n--- no run log was written for this resource\n".to_string();
    };
    let abs = std::fs::canonicalize(log).unwrap_or_else(|_| log.to_path_buf());
    let mut s = format!("\n--- full output: {}\n", abs.display());
    if let Some(rid) = site.run_id {
        s.push_str(&format!(
            "    forjar logs --state-dir {} --run {rid} --resource {}\n",
            site.state_dir.display(),
            site.resource_id
        ));
    }
    if site.state_dir.is_relative() {
        s.push_str(
            "    NOTE --state-dir is relative: it resolves only from the \
             directory this apply\n         ran in, and a stateless CI runner \
             deletes it with the checkout.\n",
        );
    }
    s
}

//! FJ-2732 (PMAT-200): the script must not be readable as its own stdin.
//!
//! # The defect
//!
//! Every transport hands the script to `bash` by writing it to the process's
//! STDIN (`transport/local.rs`, `ssh.rs`, `container.rs`, `pepita.rs` all do
//! `Command::new("bash")` + write). bash reads that stream lazily, one command
//! at a time, so a command that itself reads stdin consumes the REST OF THE
//! SCRIPT as its input.
//!
//! Measured on the published 1.12.1 binary:
//!
//! ```text
//!   command: |
//!     cat > eaten.txt
//!     echo SECOND-LINE-RAN > second.txt
//!
//!   eaten.txt  contains: echo SECOND-LINE-RAN > second.txt
//!   second.txt does not exist
//!   apply reported: 1 converged
//! ```
//!
//! Line 2 was consumed as line 1's input, never executed, and the run was
//! recorded as converged. v1.12 makes that worse rather than better: the
//! staleness probe hashes whatever the half-run produced and pins it, so the
//! corrupted artifact is remembered as correct.
//!
//! # The fix
//!
//! Wrap the script in a brace group with stdin redirected from `/dev/null`:
//!
//! ```sh
//! { <script>
//! } < /dev/null
//! ```
//!
//! bash must parse the whole compound command before executing any of it, so
//! the script text is consumed by the PARSER, and the redirection gives every
//! command inside a stdin that is not the script. A task that genuinely wants
//! input must now say so (`< file`, a heredoc, a pipe) — which is the honest
//! interface, because reading the controller's stdin over SSH never worked.
//!
//! # Why a brace group and not `bash -c`
//!
//! `bash -c` puts the script in argv, which is visible in `ps` on the target
//! (scripts can carry secrets), and hits ARG_MAX for large generated scripts.
//! Keeping the script on stdin preserves both properties; only its
//! *interpretation* changes.
//!
//! # Line numbers
//!
//! The wrapper adds exactly one leading line, so a bash error reporting
//! `line N` maps to script line `N - 1`. That is stable and documented rather
//! than variable.

/// Number of lines the wrapper prepends, for mapping bash line numbers back.
pub const WRAPPER_PREFIX_LINES: usize = 1;

/// Wrap a script so it cannot be consumed as its own stdin.
///
/// Idempotent in effect: wrapping an already-wrapped script is harmless, but
/// callers should wrap exactly once, at the transport funnel.
pub fn wrap_script_stdin_isolated(script: &str) -> String {
    // A brace group must contain at least one command — `{ }` is a bash syntax
    // error — so an empty script gets the `:` no-op, which exits 0 exactly as
    // an empty script did before.
    let body = if script.trim().is_empty() {
        ":"
    } else {
        script.trim_end()
    };
    // The closing brace must start a new line and be followed by the
    // redirection; `}` is a reserved word and needs to be command-position.
    format!("{{\n{body}\n}} < /dev/null\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};

    /// Run a script the way the transport does: bash, script on stdin.
    fn run_via_stdin(script: &str) -> (String, Option<i32>) {
        let mut child = Command::new("bash")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("bash");
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(script.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        (
            String::from_utf8_lossy(&out.stdout).to_string(),
            out.status.code(),
        )
    }

    fn tmp(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("forjar-stdin-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn unwrapped_script_is_eaten_by_a_stdin_reading_command() {
        // Pins the DEFECT, so the fix below is demonstrably a fix and not a
        // no-op. If bash ever stops behaving this way, this test tells us.
        let d = tmp("defect");
        let script = format!(
            "cd {}\ncat > eaten.txt\necho SECOND > second.txt\n",
            d.display()
        );
        run_via_stdin(&script);
        assert!(
            !d.join("second.txt").exists(),
            "the defect requires line 2 to be swallowed"
        );
        let eaten = std::fs::read_to_string(d.join("eaten.txt")).unwrap_or_default();
        assert!(
            eaten.contains("echo SECOND"),
            "line 2 became line 1's input"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn wrapped_script_runs_every_line() {
        let d = tmp("fixed");
        let script = format!(
            "cd {}\ncat > eaten.txt\necho SECOND > second.txt\n",
            d.display()
        );
        run_via_stdin(&wrap_script_stdin_isolated(&script));

        assert!(
            d.join("second.txt").exists(),
            "line 2 must run once the script is not its own stdin"
        );
        assert_eq!(
            std::fs::read_to_string(d.join("second.txt"))
                .unwrap()
                .trim(),
            "SECOND"
        );
        assert_eq!(
            std::fs::read_to_string(d.join("eaten.txt")).unwrap(),
            "",
            "the stdin-reading command gets /dev/null, not the script"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn exit_status_still_propagates() {
        // The wrapper must not swallow failure — `set -e` inside still aborts
        // and the group's status is the script's status.
        assert_eq!(
            run_via_stdin(&wrap_script_stdin_isolated("exit 7")).1,
            Some(7)
        );
        assert_eq!(
            run_via_stdin(&wrap_script_stdin_isolated("set -e\nfalse\necho NOPE")).1,
            Some(1)
        );
        assert_eq!(
            run_via_stdin(&wrap_script_stdin_isolated("true")).1,
            Some(0)
        );
    }

    #[test]
    fn stdout_is_unchanged() {
        let (out, code) = run_via_stdin(&wrap_script_stdin_isolated("echo one\necho two"));
        assert_eq!(out, "one\ntwo\n");
        assert_eq!(code, Some(0));
    }

    #[test]
    fn a_heredoc_inside_the_script_still_works() {
        // Heredocs are how forjar's own file resources write content, and they
        // consume from the script stream. They must survive the wrapper.
        let d = tmp("heredoc");
        let script = format!(
            "cat > {}/h.txt <<'EOF'\nline one\nline two\nEOF\n",
            d.display()
        );
        let (_, code) = run_via_stdin(&wrap_script_stdin_isolated(&script));
        assert_eq!(code, Some(0));
        assert_eq!(
            std::fs::read_to_string(d.join("h.txt")).unwrap(),
            "line one\nline two\n"
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn set_euo_pipefail_preamble_survives() {
        // Every generated apply script starts with this; the wrapper must not
        // change its meaning.
        let s = wrap_script_stdin_isolated("set -euo pipefail\necho ok");
        assert_eq!(run_via_stdin(&s).0, "ok\n");
        let bad = wrap_script_stdin_isolated("set -euo pipefail\nfalse\necho NOPE");
        assert_eq!(run_via_stdin(&bad).1, Some(1));
    }

    #[test]
    fn an_explicit_input_redirection_still_reaches_the_command() {
        // The honest interface: a task that wants input says so.
        let d = tmp("explicit");
        std::fs::write(d.join("in.txt"), "REAL INPUT\n").unwrap();
        let script = format!(
            "cd {}\ncat > got.txt < in.txt\necho done > done.txt\n",
            d.display()
        );
        run_via_stdin(&wrap_script_stdin_isolated(&script));
        assert_eq!(
            std::fs::read_to_string(d.join("got.txt")).unwrap(),
            "REAL INPUT\n"
        );
        assert!(d.join("done.txt").exists(), "the next line still runs");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn an_empty_script_still_succeeds() {
        // `{ }` is a bash syntax error. An empty script exited 0 before the
        // wrapper and must still do so — caught by an existing transport test,
        // not by anything I wrote.
        for s in ["", "   ", "\n\n"] {
            assert_eq!(
                run_via_stdin(&wrap_script_stdin_isolated(s)).1,
                Some(0),
                "empty script {s:?} must exit 0"
            );
        }
    }

    #[test]
    fn the_documented_line_offset_is_correct() {
        let wrapped = wrap_script_stdin_isolated("echo a\necho b");
        assert_eq!(
            wrapped.lines().position(|l| l == "echo a").unwrap(),
            WRAPPER_PREFIX_LINES,
            "a bash `line N` maps to script line N - WRAPPER_PREFIX_LINES"
        );
    }
}

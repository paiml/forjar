//! PMAT-204: a file resource whose PATH contains `666` or `777` could not be
//! applied — forjar refused its own correct script.
//!
//! WHAT HAPPENED. For every file resource carrying a `mode:`, codegen emits
//!
//! ```text
//! chmod '0644' '/opt/app666/config'
//! ```
//!
//! and `transport::validate_before_exec` hands that script to
//! `core::purifier::validate_script`, the I8 gate, which refuses on any
//! Error-severity bashrs diagnostic. bashrs SEC017 ("unsafe file permissions")
//! scans the LITERAL TEXT of any line containing the word `chmod` for the
//! substrings `777`/`666`, with a digit-boundary check but no idea which token
//! is the mode argument. A path that merely CONTAINS `666` therefore reads as a
//! world-writable chmod, and apply fails on a script that is correct.
//!
//! MEASURED, one line per script, `lint_shell` from the bashrs pinned in
//! Cargo.lock (6.68.0), 2026-09-07:
//!
//! ```text
//! | script line                                | SEC017                          |
//! |--------------------------------------------|---------------------------------|
//! | chmod '0644' '/tmp/.tmp5k666T/target.txt'   | ERROR (false: 666 is in PATH)   |
//! | chmod '0644' '/opt/app666/t'                | ERROR (false positive)          |
//! | chmod '0644' '/tmp/x777y/target.txt'        | ERROR (false positive)          |
//! | chmod '0644' '/tmp/plain/target.txt'        | clean                           |
//! | chmod '0666' '/tmp/plain/t'                 | NO FINDING AT ALL               |
//! | chmod 666 /tmp/real                         | ERROR (true positive)           |
//! ```
//!
//! Row 5 is the reason this ticket is not "loosen the gate". SEC017's boundary
//! check rejects `666` inside `'0666'` because the character before it is the
//! digit `0`, so the ONE shape forjar actually emits for a world-writable mode
//! is the one shape bashrs cannot see. The instrument was pointed at the wrong
//! token in both directions: it fired on paths and stayed silent on modes.
//!
//! WHY THE ASSERTIONS LOOK LIKE THIS. The convergence cells assert on the BYTES
//! AT THE PATH after `forjar apply`, not on the summary line and not on the
//! generated script text — the defect was visible only as a failure to write,
//! and a test that read the script would have passed against a gate that
//! rejected it. The refusal cells assert on forjar's OWN message (it names the
//! resource and the mode) and additionally assert that bashrs is not what
//! rejected them: SEC017 must not appear, because if it did, this file would go
//! green for the wrong reason the day bashrs learns to parse quotes.
//!
//! WHAT A GREEN RUN PROVES. (1) A path containing `666` or `777` converges.
//! (2) A declared world-writable mode is refused by forjar's own rule at the
//! I8 gate, before the script runs — a property nothing had before, because
//! the existing world-writable rule (`validate --check-security`,
//! `core::security_scanner`) only ever REPORTED it. (3) A genuine bare
//! `chmod 666` is still refused through SEC017: the true positive survives.
//! (4) The gate is not vacuous — it still refuses an unrelated Error-severity
//! class (SC1078, unterminated string).
//!
//! HOW TO FALSIFY. Make `core::purifier_sec017::chmod_mode_verdict` return the
//! exempt verdict unconditionally: the true-positive cells go red. Delete
//! `core::purifier::world_writable_chmod_errors` and the declared-mode cells go
//! red. Suppress SEC017 wholesale and the bare `chmod 666` cells go red.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use forjar::core::purifier::validate_script;

fn forjar_bin() -> &'static str {
    env!("CARGO_BIN_EXE_forjar")
}

const DECLARED: &str = "DECLARED\n";

/// Strip ANSI so assertions do not depend on colour.
fn plain(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c2 in chars.by_ref() {
                if c2.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
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

    /// Build the sub-directory ourselves. `mktemp` produced `/tmp/.tmp5k666T`
    /// by chance once (PR #476) and that is how this was found; a test that
    /// waited for the same luck would pass on almost every run.
    fn subdir(&self, name: &str) -> PathBuf {
        let p = self.dir.path().join(name);
        fs::create_dir_all(&p).expect("subdir");
        p
    }

    fn state(&self) -> PathBuf {
        self.dir.path().join("state")
    }

    fn write_config(&self, target: &Path, mode: &str) -> PathBuf {
        let cfg = self.dir.path().join("forjar.yaml");
        fs::write(
            &cfg,
            format!(
                "version: \"1.0\"\nname: chmod-path\n\
                 machines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\n\
                 resources:\n  managed: {{ type: file, machine: local, path: {}, content: \"{}\", mode: \"{mode}\" }}\n",
                target.display(),
                DECLARED.replace('\n', "\\n"),
            ),
        )
        .expect("config written");
        cfg
    }

    fn apply(&self, cfg: &Path) -> (i32, String) {
        let out = Command::new(forjar_bin())
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
        (out.status.code().unwrap_or(-1), plain(&s))
    }
}

/// One convergence cell: a file at `<tmp>/<dirname>/target.txt` must end up
/// holding the declared bytes.
fn converges_under(dirname: &str) {
    let sb = Sandbox::new();
    let target = sb.subdir(dirname).join("target.txt");
    let cfg = sb.write_config(&target, "0644");

    let (ec, out) = sb.apply(&cfg);
    assert_eq!(
        ec, 0,
        "apply refused a file under a directory named '{dirname}'. It printed:\n{out}"
    );
    assert_eq!(
        fs::read_to_string(&target).expect("the declared file is not at the path"),
        DECLARED,
        "apply exited 0 without writing the declared bytes to {}",
        target.display()
    );
}

// ── 1/2: the path is not a mode ─────────────────────────────────────────────

#[test]
fn a_path_containing_666_converges() {
    converges_under("app666");
}

#[test]
fn a_path_containing_777_converges() {
    converges_under("x777y");
}

// ── 3/4: forjar refuses a world-writable DECLARED mode ──────────────────────
// New property. bashrs sees nothing here (measurement row 5), so this cannot
// be delegated: the assertion requires forjar's own message and explicitly
// requires that SEC017 is NOT what rejected it.

fn refuses_declared_mode(mode: &str) {
    let sb = Sandbox::new();
    let target = sb.subdir("plain").join("target.txt");
    let cfg = sb.write_config(&target, mode);

    let (ec, out) = sb.apply(&cfg);
    assert_ne!(
        ec, 0,
        "forjar accepted a world-writable declared mode '{mode}'. It printed:\n{out}"
    );
    assert!(
        out.contains("managed") && out.contains(mode) && out.contains("world-writable"),
        "forjar rejected mode '{mode}' without naming the resource, the mode and why. It printed:\n{out}"
    );
    assert!(
        !out.contains("SEC017"),
        "the refusal of mode '{mode}' came from bashrs, not from forjar — measurement row 5 says \
         bashrs produces no finding for a quoted world-writable mode, so this is a coincidence \
         waiting to break. It printed:\n{out}"
    );
    assert!(
        !target.exists(),
        "forjar refused the config and wrote the file anyway"
    );
}

#[test]
fn a_declared_mode_of_0666_is_refused_by_forjar() {
    refuses_declared_mode("0666");
}

#[test]
fn a_declared_mode_of_0777_is_refused_by_forjar() {
    refuses_declared_mode("0777");
}

// ── 5: the true positive must survive ───────────────────────────────────────

#[test]
fn a_bare_chmod_666_still_fails_the_i8_gate() {
    let err = validate_script("chmod 666 /srv/data\n")
        .expect_err("a bare `chmod 666` was accepted — the SEC017 true positive was lost");
    assert!(err.contains("SEC017"), "rejected, but not by SEC017: {err}");
}

#[test]
fn a_bare_chmod_777_still_fails_the_i8_gate() {
    let err = validate_script("chmod -R 777 /var/www\n")
        .expect_err("a bare `chmod -R 777` was accepted — the SEC017 true positive was lost");
    assert!(err.contains("SEC017"), "rejected, but not by SEC017: {err}");
}

/// SEC017 fires on any line carrying the word `chmod`, not only on a chmod
/// command. Such a line has no mode argument to parse, so it is not the
/// false-positive shape and must still be refused.
#[test]
fn sec017_on_a_line_that_is_not_a_chmod_command_still_fails() {
    let err = validate_script("echo chmod 777 /var/www\n").expect_err(
        "SEC017 on a non-chmod line was suppressed — the exemption is not restricted to a \
         parsed chmod mode argument",
    );
    assert!(err.contains("SEC017"), "rejected, but not by SEC017: {err}");
}

/// A line with two chmods, in every shape that hid the second one from the
/// first version of this gate. Each of these was measured ACCEPTED by that
/// version and REFUSED before PMAT-204 existed: they are the regression the
/// review caught, and they are what the redact-and-re-lint rule exists for.
///
/// `chmod 666` unquoted is included last as the easy case; the six above it
/// are the ones that a command-position parser gets wrong, so a rule that
/// passes only the last line proves nothing.
#[test]
fn a_second_chmod_on_the_same_line_is_not_hidden_by_the_first() {
    for script in [
        "chmod '0644' '/srv/a'; sudo -u root chmod 777 /srv/b\n",
        "chmod '0644' '/srv/a'; `chmod 777 /srv/b`\n",
        "chmod '0644' '/srv/a'; env chmod 777 /srv/b\n",
        "chmod '0644' '/srv/a'; (chmod 777 /srv/b)\n",
        "chmod '0644' '/srv/a' && chmod 777 /srv/b\n",
        "chmod '0644' '/opt/app666/t'; chmod '0666' '/srv/b'\n",
        "chmod '0644' '/srv/a'; chmod 666 /srv/b\n",
    ] {
        assert!(
            validate_script(script).is_err(),
            "a real chmod sharing a line with a safe chmod was ACCEPTED: {script}"
        );
    }
}

/// A world-writable mode wherever it sits on the line, in the widths and
/// spellings the first version of this gate could not read: five octal digits,
/// a symbolic `a+w`, and a mode inside `find -exec` where chmod is not the
/// command. bashrs reports none of these — forjar does.
#[test]
fn a_world_writable_mode_is_refused_wherever_it_sits() {
    for script in [
        "chmod '00666' '/srv/b'\n",
        "chmod 'a+w' '/srv/b'\n",
        "chmod o+w /srv/b\n",
        "find /srv -type f -exec chmod '0666' {} \\;\n",
        "chmod '0644' '/srv/a'; chmod '0662' '/srv/b'\n",
    ] {
        let err = validate_script(script)
            .expect_err(&format!("a world-writable mode was accepted: {script}"));
        assert!(
            err.contains("FJ-CHMOD-WW"),
            "refused, but not by forjar's own world-writable check: {err}"
        );
    }
}

/// The fourth round's counterexamples. Five of the seven are refused at the
/// pre-PMAT-204 baseline and here, so they refute nothing; the two the baseline
/// also accepted are below. This test pins the one that is now closed: an octal
/// mode wider than six digits.
#[test]
fn an_octal_mode_of_any_width_is_read() {
    for script in [
        "chmod 0000666 /foo\n",
        "chmod '0000777' '/foo'\n",
        "chmod '000000662' '/foo'\n",
    ] {
        assert!(
            validate_script(script).is_err(),
            "a wide octal world-writable mode was accepted: {script}"
        );
    }
    // And a wide SAFE mode is still fine.
    assert!(validate_script("chmod '0000644' '/opt/app666/t'\n").is_ok());
}

/// A second mode later in the same chmod's arguments. The merge review found
/// it: `chmod '0644' '/x' 0666 '/y'` was accepted at the pre-PMAT-204 baseline
/// too — SEC017 cannot see `0666` behind its leading zero and forjar's own
/// check stopped after the first argument. Both directions are read now.
#[test]
fn a_world_writable_mode_later_in_the_arguments_is_refused() {
    for script in [
        "chmod '0644' '/x' 0666 '/y'\n",
        "chmod '0644' '/x' 'a+w' '/y'\n",
    ] {
        assert!(
            validate_script(script).is_err(),
            "a world-writable mode after the first argument was accepted: {script}"
        );
    }
    // A path is never an octal literal, so reading every argument does not
    // start refusing correct scripts.
    assert!(validate_script("chmod '0644' '/opt/app666/t' '/srv/0666-notes'\n").is_ok());
}

/// The shapes NEITHER instrument can decide, recorded so the claim stays exact:
/// a mode in a variable and a mode taken from another file pass now exactly as
/// they passed before PMAT-204. This test fails the day one of them starts
/// being refused — at which point the CHANGELOG and the module header must say
/// so rather than this test being deleted.
#[test]
fn the_undecidable_shapes_are_documented_not_claimed() {
    for script in [
        "MODE=666\nchmod \"$MODE\" '/srv/b'\n",
        "chmod '0644' --reference=/elsewhere /srv/b\n",
    ] {
        assert!(
            validate_script(script).is_ok(),
            "this shape is now refused — update the module header and the CHANGELOG: {script}"
        );
    }
}

// ── 6: the exempted shape, at the gate ──────────────────────────────────────

#[test]
fn a_quoted_safe_mode_on_a_path_containing_666_passes_the_gate() {
    assert!(
        validate_script("chmod '0644' '/opt/app666/config'\n").is_ok(),
        "the I8 gate still refuses forjar's own chmod line for a path containing 666"
    );
    assert!(
        validate_script("chmod '0755' '/opt/x777y/bin'\n").is_ok(),
        "the I8 gate still refuses forjar's own chmod line for a path containing 777"
    );
}

/// The quoted world-writable mode bashrs cannot see (measurement row 5) must
/// not slip through the gate either — the exemption is for modes that are NOT
/// world-writable, and forjar decides that itself.
#[test]
fn a_quoted_world_writable_mode_is_refused_at_the_gate() {
    let err = validate_script("chmod '0666' '/tmp/plain/t'\n").expect_err(
        "a quoted world-writable chmod passed the I8 gate — bashrs misses this shape and \
         forjar must not",
    );
    assert!(
        err.contains("0666"),
        "rejected without naming the mode: {err}"
    );
}

// ── 7: control — the gate is not vacuous ────────────────────────────────────

/// If the fix had switched off Error-severity refusal in general, every cell
/// above would still pass. This one fails unless an unrelated Error class is
/// still refused: SC1078, an unterminated double-quoted string.
#[test]
fn the_gate_still_refuses_an_unrelated_error_class() {
    let err = validate_script("echo \"this string never closes\n")
        .expect_err("an unterminated string was accepted — validate_script no longer refuses");
    assert!(
        err.contains("SC1"),
        "rejected, but not by the SC1 syntax family: {err}"
    );
}

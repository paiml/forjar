//! PMAT-204, the adversarial half: every shape a review round claimed could
//! get past the I8 gate, pinned here after being re-run against the
//! pre-PMAT-204 baseline (0776fd82) and the fix.
//!
//! The sibling file `falsification_chmod_path_is_not_a_mode.rs` pins what the
//! ticket is FOR — a path containing 666 or 777 must not be read as a mode, and
//! a world-writable mode must be refused. This file pins what four review
//! rounds and a merge review tried to sneak past it. Each test names the round
//! that produced its shapes and what the baseline did with them, because a
//! shape both commits refuse proves nothing about the change and is marked as
//! such rather than counted as evidence.
//!
//! Fifty shapes were measured against both commits. Four go from refused to
//! accepted, and all four are the ticket's own bug. Fourteen go from accepted
//! to refused. Nothing else moves.

use forjar::core::purifier::validate_script;

/// The refuter round's own counterexamples, each measured against the
/// pre-PMAT-204 baseline first so the table says what it means:
///
/// ```text
/// | shape                                        | at 0776fd82 | required |
/// |----------------------------------------------|-------------|----------|
/// | echo \" ; chmod 777 /foo ; echo \"            | refused     | refused  |
/// | echo \" ; (chmod 777 /foo) ; echo \"          | refused     | refused  |  <- regression, fixed
/// | chmod 'u=rwx,o=w' '/srv/b'                    | ACCEPTED    | refused  |  <- pre-existing hole, closed
/// | chmod u=r,o=w /srv/b                          | ACCEPTED    | refused  |
/// | chmod 'u=rwx,a+w' '/srv/b'                    | ACCEPTED    | refused  |
/// | `chmod 777 /foo`                              | ACCEPTED    | refused  |
/// ```
///
/// The second row is the one that matters most: a backslash-escaped quote made
/// the redactor re-pair the quotes and swallow a real `chmod 777`. It is why
/// the scanner honours backslash escapes rather than searching for quote
/// characters.
#[test]
fn the_refuted_shapes_are_refused() {
    for script in [
        "echo \\\" ; chmod 777 /foo ; echo \\\"\n",
        "echo \\\" ; (chmod 777 /foo) ; echo \\\"\n",
        "chmod 'u=rwx,o=w' '/srv/b'\n",
        "chmod u=r,o=w /srv/b\n",
        "chmod 'u=rwx,a+w' '/srv/b'\n",
        "(chmod 777 /foo)\n",
        "`chmod 777 /foo`\n",
    ] {
        assert!(
            validate_script(script).is_err(),
            "a shape the refuters killed is accepted again: {script}"
        );
    }
}

/// The other half of the same round: escaping and quoting must not make the
/// gate refuse a correct script either, or the fix would trade one broken
/// direction for another.
#[test]
fn escaped_and_embedded_quotes_do_not_break_a_correct_script() {
    for script in [
        "echo \\\"x\\\" ; chmod '0644' '/opt/app666/t'\n",
        "chmod '0644' '/srv/it\"s/t'\n",
        "chmod 'u+x' '/srv/b'\n",
        "chmod 'g+w' '/srv/b'\n",
    ] {
        assert!(
            validate_script(script).is_ok(),
            "a correct script is refused: {script}"
        );
    }
}

/// The judge round's counterexamples: a quoted mode padded with whitespace.
///
/// `chmod " 777" /foo` does not PARSE as an octal mode, and the version of the
/// redactor that removed every quoted token which failed to parse removed this
/// one — taking the `777` out of bashrs's sight and turning a script the
/// pre-PMAT-204 gate refused into one it accepted. Measured against the
/// baseline, all four of these were refused there and must be refused here.
/// The redactor removes plain PATH LITERALS only, which is why they are.
#[test]
fn a_whitespace_padded_quoted_mode_is_still_refused() {
    for script in [
        "chmod \" 777\" /foo\n",
        "chmod ' 777' /foo\n",
        "chmod '777 ' /foo\n",
        "chmod '\t777' /foo\n",
        "chmod 0644 777 /foo\n",
    ] {
        assert!(
            validate_script(script).is_err(),
            "a whitespace-padded quoted mode was accepted: {script}"
        );
    }
}

/// A quoted argument that is not a plain path literal is never redacted, so
/// nothing executable can be hidden inside one. These carry a `$`, a backtick,
/// a `;` or the word `chmod`, and each is left on the line for bashrs.
#[test]
fn only_plain_path_literals_are_redacted() {
    assert!(validate_script("chmod '0644' '/srv/$app666/t'\n").is_err());
    assert!(validate_script("chmod '0644' '/srv/a; chmod 777 /srv/b\n").is_err());
    // The control: a plain path literal containing 666 IS redacted, which is
    // the whole point of the ticket.
    assert!(validate_script("chmod '0644' '/opt/app666/t'\n").is_ok());
}

/// The merge review's counterexamples, each first re-run against the
/// pre-PMAT-204 baseline. Three were real and are closed here; the rest did not
/// reproduce.
///
/// ```text
/// | shape                                        | at 0776fd82 | required |
/// |----------------------------------------------|-------------|----------|
/// | chmod 0644 /foo ; bash -c 'c\hmod 777 /bar'   | refused     | refused  |  <- regression, fixed
/// | chmod 0644 file1;chmod '0666' file2           | ACCEPTED    | refused  |
/// | chmod 0000000000666 /foo                      | ACCEPTED    | refused  |
/// | chmod o=r-w /foo                              | ACCEPTED    | ACCEPTED |  <- false refusal, fixed
/// | chmod o+x-w /foo                              | ACCEPTED    | ACCEPTED |
/// ```
///
/// The first row is why a redacted argument may not contain whitespace: the
/// quoted payload has a slash, no metacharacter and — because of the backslash
/// — not the literal word `chmod`, so a rule that allowed spaces redacted it
/// whole and hid the `777`.
#[test]
fn a_quoted_payload_carrying_a_command_is_never_redacted() {
    for script in [
        "chmod 0644 /foo ; bash -c 'c\\hmod 777 /bar'\n",
        "chmod 0644 /foo ; bash -c \"c\\hmod 777 /bar\"\n",
        "chmod 644 'a\\' /foo 777 'b'\n",
        "chmod '0644' '/opt/app666/t' <(chmod 777 /b)\n",
    ] {
        assert!(
            validate_script(script).is_err(),
            "a quoted payload carrying a command was redacted away: {script}"
        );
    }
}

/// Command separators without spaces around them, and octal padding of any
/// length: both were accepted at the baseline and are refused here.
#[test]
fn separators_without_spaces_and_padded_octals_are_read() {
    assert!(validate_script("chmod 0644 file1;chmod '0666' file2\n").is_err());
    assert!(validate_script("chmod 0000000000666 /foo\n").is_err());
    assert!(validate_script("chmod '0000644' '/opt/app666/t'\n").is_ok());
}

/// A symbolic clause is read the way chmod applies it, left to right: `-w`
/// after a `+w` or an `=w` takes the bit away again. An earlier version asked
/// only whether a `w` appeared anywhere after the first operator and refused
/// these, which the baseline did not — a false refusal, and refusing a correct
/// script is a defect even though it fails safe.
#[test]
fn a_symbolic_clause_that_removes_world_write_is_not_refused() {
    for script in [
        "chmod o=r-w /foo\n",
        "chmod o+x-w /foo\n",
        "chmod 'a+w-w' '/srv/b'\n",
    ] {
        assert!(
            validate_script(script).is_ok(),
            "a mode that removes world write was refused: {script}"
        );
    }
    // The grants still are.
    assert!(validate_script("chmod o=w /foo\n").is_err());
    assert!(validate_script("chmod 'u=rwx,o=w' '/srv/b'\n").is_err());
}

/// The fourth merge review: comma clauses apply in order to one permission set.
/// `a=rwx,o-w` grants world write and then takes it back, and testing the
/// clauses independently refused it — a false refusal the pre-PMAT-204 gate did
/// not make (measured at both commits).
#[test]
fn a_later_clause_that_revokes_world_write_is_honoured() {
    for script in ["chmod 'a=rwx,o-w' '/srv/b'\n", "chmod 'o+w,o-w' '/srv/b'\n"] {
        assert!(
            validate_script(script).is_ok(),
            "a mode whose later clause removes world write was refused: {script}"
        );
    }
    // And the grant still stands when nothing takes it back.
    assert!(validate_script("chmod 'a=rwx' '/srv/b'\n").is_err());
    assert!(validate_script("chmod 'u=rwx,o=w' '/srv/b'\n").is_err());
}

/// The fifth merge review: a symbolic mode may start with a `-`. `-w,o+w`
/// removes write for everyone and then grants it to others; skipping it as a
/// command flag left the grant unread (accepted at the pre-PMAT-204 baseline
/// too, so a hole rather than a regression). Flags that carry no `w` stay
/// flags, so a bare mode after `-R` is still reached.
#[test]
fn a_symbolic_mode_starting_with_a_minus_is_still_a_mode() {
    assert!(validate_script("chmod -w,o+w /srv/b\n").is_err());
    assert!(validate_script("chmod -w /srv/b\n").is_ok());
    assert!(validate_script("chmod -R 777 /var/www\n").is_err());
    assert!(validate_script("chmod -R '0644' '/opt/app666/t'\n").is_ok());
}

//! Unit tests for `purifier_sec017`, split out to keep that file under the
//! 500-line health gate. Behavioural pins live in
//! `tests/falsification_chmod_path_is_not_a_mode.rs` and its review sibling.

use super::*;

/// The shape forjar generates, on the paths that broke.
#[test]
fn forjar_generated_lines_are_exempt() {
    for line in [
        "chmod '0644' '/tmp/.tmp5k666T/target.txt'",
        "chmod '0644' '/opt/app666/t'",
        "chmod '0755' '/tmp/x777y/bin'",
        "chmod -R '0750' '/srv/app666'",
        "chmod \"0644\" \"/opt/app666/t\"",
    ] {
        assert!(sec017_is_path_only(line), "not exempted: {line}");
    }
}

/// The refuted shapes: a second chmod that the first version of this module
/// did not see. Each was ACCEPTED by that version and is refused here.
#[test]
fn a_real_chmod_elsewhere_on_the_line_is_never_exempt() {
    for line in [
        "chmod '0644' '/srv/a'; sudo -u root chmod 777 /srv/b",
        "chmod '0644' '/srv/a'; `chmod 777 /srv/b`",
        "chmod '0644' '/srv/a'; env chmod 777 /srv/b",
        "chmod '0644' '/srv/a'; chmod 666 /srv/b",
        "chmod '0644' '/srv/a' && chmod 777 /srv/b",
        "chmod '0644' '/opt/app666/t'; chmod 777 /srv/b",
    ] {
        assert!(!sec017_is_path_only(line), "wrongly exempted: {line}");
    }
}

/// A line with no quoting has nothing to redact, so nothing to exempt.
#[test]
fn an_unquoted_line_is_never_exempt() {
    assert!(!sec017_is_path_only("chmod 666 /srv/data"));
    assert!(!sec017_is_path_only("echo chmod 777 /var/www"));
}

/// Row 5 of the measurement table: bashrs sees nothing, forjar must.
#[test]
fn world_writable_modes_are_named_wherever_they_sit() {
    assert_eq!(
        world_writable_modes("chmod '0666' '/tmp/plain/t'"),
        ["0666"]
    );
    assert_eq!(world_writable_modes("chmod 0777 /tmp/plain/t"), ["0777"]);
    assert_eq!(
        world_writable_modes("chmod '0662' '/tmp/plain/t'"),
        ["0662"]
    );
    // 5 digits: the width the first version read as "not a mode".
    assert_eq!(world_writable_modes("chmod '00666' '/srv/b'"), ["00666"]);
    // Second chmod on the line, quoted: invisible to SEC017 either way.
    assert_eq!(
        world_writable_modes("chmod '0644' '/srv/a'; chmod '0666' '/srv/b'"),
        ["0666"]
    );
    // Not in command position, still a world-writable mode.
    assert_eq!(
        world_writable_modes("find /srv -type f -exec chmod '0666' {} \\;"),
        ["0666"]
    );
    // Symbolic.
    assert_eq!(world_writable_modes("chmod 'a+w' '/srv/b'"), ["a+w"]);
    assert_eq!(world_writable_modes("chmod o+w /srv/b"), ["o+w"]);
    // Flags are skipped, the mode after them is read.
    assert_eq!(world_writable_modes("chmod -R '0666' '/srv/b'"), ["0666"]);
}

#[test]
fn safe_and_undecidable_modes_are_not_named() {
    for line in [
        "chmod '0644' '/srv/a'",
        "chmod 0755 /srv/a",
        "chmod u+x '/srv/a'",
        "chmod g+w '/srv/a'",
        "chmod --reference='/srv/a' '/srv/b'",
        "chmod \"$MODE\" '/srv/b'",
        "echo '/opt/app666/t'",
        "chmod",
    ] {
        assert!(
            world_writable_modes(line).is_empty(),
            "wrongly named world-writable: {line}"
        );
    }
}

#[test]
fn octal_parsing_reads_the_widths_chmod_accepts() {
    assert_eq!(parse_octal_mode("0644"), Some(0o644));
    assert_eq!(parse_octal_mode("644"), Some(0o644));
    assert_eq!(parse_octal_mode("1777"), Some(0o1777));
    assert_eq!(parse_octal_mode("00666"), Some(0o666));
    assert_eq!(parse_octal_mode("68"), None);
    assert_eq!(parse_octal_mode("0888"), None);
    assert_eq!(parse_octal_mode(""), None);
    // Wider than four digits is still a mode: leading zeros are padding,
    // and only the low twelve bits are kept.
    assert_eq!(parse_octal_mode("0000666"), Some(0o666));
    assert_eq!(parse_octal_mode("000000662"), Some(0o662));
    assert_eq!(parse_octal_mode("0000000"), Some(0));
    assert_eq!(parse_octal_mode("0123456789012"), None);
}

#[test]
fn redaction_keeps_the_mode_and_replaces_the_path() {
    assert_eq!(
        redact_quoted_paths("chmod '0644' '/opt/app666/t'"),
        "chmod '0644' '/x'"
    );
    assert_eq!(
        redact_quoted_paths("chmod '0644' '/srv/a'; chmod 777 /srv/b"),
        "chmod '0644' '/x'; chmod 777 /srv/b"
    );
    // An unbalanced quote is left exactly as written.
    assert_eq!(redact_quoted_paths("chmod '0644"), "chmod '0644");
}

//! forjar#848 and forjar#445: the check guard and the intact-block guard,
//! executed. Split out of `tests_cron_exec.rs` (500-line file-health limit);
//! the fake crontab, sudo and id it runs against are that module's harness.

use super::cron::*;
use super::tests_cron_exec::{absent, cron, write_exe, FakeCrontab, CMD};

// ── forjar#848: the check guard probes the read, not `sudo -n true` ──

/// A crontab-only sudoers rule (the fake refuses `true`) must still be observed.
#[test]
fn fj848_check_runs_under_a_crontab_only_sudoers_rule() {
    let fx = FakeCrontab::new("t848a");
    fx.seed("# forjar:backup\n# forjar-cmd:backup\n0 3 * * * /usr/local/bin/backup.sh\n");
    let out = fx.run(&check_script(&cron("backup", "0 3 * * *", CMD)));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{stdout} / {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("exists:backup"), "{stdout:?}");
    assert!(!stdout.contains("FORJAR_SKIP"), "{stdout:?}");
}

/// When sudo refuses even crontab, check SKIPs (exit 2), never reports missing.
/// Covers sudo's own refusals and the "Sorry, user" form; a host with no sudo
/// is covered below with the real shells' own messages.
#[test]
fn fj848_check_skips_when_sudo_refuses_the_read() {
    let msgs = [
        "sudo: a password is required",
        "Sorry, user bob is not allowed to execute '/usr/bin/crontab' as root",
    ];
    for (i, msg) in msgs.iter().enumerate() {
        let fx = FakeCrontab::new(&format!("t848b{i}"));
        write_exe(
            &fx.bin().join("sudo"),
            &format!("#!/usr/bin/env bash\necho '{msg}' >&2\nexit 1\n"),
        );
        let out = fx.run(&check_script(&cron("backup", "0 3 * * *", CMD)));
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{msg}: {err}");
        assert!(String::from_utf8_lossy(&out.stdout).contains("FORJAR_SKIP"));
    }
}

/// `no crontab for <user>` is not a sudo refusal: it is the benign missing case.
#[test]
fn fj848_no_crontab_is_missing_not_skip() {
    let fx = FakeCrontab::new("t848c");
    let out = fx.run(&check_script(&cron("backup", "0 3 * * *", CMD)));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("missing:backup"), "{stdout:?}");
}

// ── forjar#445: a hand-edited block is refused by name ──

#[test]
fn fj445_hand_edited_entry_under_a_marker_is_refused_by_name() {
    let fx = FakeCrontab::new("t445a");
    let seeded = "# forjar:backup\n0 9 * * * /bin/hand-written\n";
    fx.seed(seeded);
    let out = fx.run(&apply_script(&cron("backup", "0 3 * * *", CMD)));
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("cron 'backup'") && err.contains("edited by hand"),
        "{err:?}"
    );
    assert_eq!(fx.read(), seeded, "the crontab must be unchanged");
}

#[test]
fn fj445_absent_also_refuses_a_non_intact_block() {
    let fx = FakeCrontab::new("t445b");
    let seeded = "# forjar:backup\n";
    fx.seed(seeded);
    let out = fx.run(&apply_script(&absent("backup")));
    assert_eq!(
        out.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(fx.read(), seeded);
}

#[test]
fn fj445_an_intact_block_and_a_prefix_sibling_still_apply() {
    let fx = FakeCrontab::new("t445c");
    fx.apply(&cron("backup", "0 3 * * *", CMD));
    fx.apply(&cron("backup-db", "0 5 * * *", CMD));
    fx.apply(&cron("backup", "0 4 * * *", CMD));
    let store = fx.read();
    assert_eq!(store.matches(CMD).count(), 2, "{store:?}");
    assert!(
        store.contains("0 4 * * *") && store.contains("0 5 * * *"),
        "{store:?}"
    );
}

// ── forjar#445, review round 1: a block whose entry line was deleted ──
//
// `# forjar:backup` + `# forjar-cmd:backup` and then NOT an entry. Whatever
// line follows is another block's, a comment or a blank, and `delete_block`
// would drop it as if it were forjar's entry. The guard must refuse by name
// and leave the crontab byte-for-byte unchanged.

/// Seed a truncated `backup` block followed by `after`, apply `backup`, and
/// require the by-name refusal with the crontab untouched.
fn assert_truncated_block_refused(tag: &str, after: &str) {
    let fx = FakeCrontab::new(tag);
    let seeded = format!("# forjar:backup\n# forjar-cmd:backup\n{after}");
    fx.seed(&seeded);
    let out = fx.run(&apply_script(&cron("backup", "0 3 * * *", CMD)));
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{after:?}: {err}");
    assert!(
        err.contains("cron 'backup'") && err.contains("edited by hand"),
        "{after:?}: {err:?}"
    );
    assert_eq!(
        fx.read(),
        seeded,
        "{after:?}: the crontab must be unchanged"
    );
}

#[test]
fn fj445_truncated_block_before_a_sibling_marker_is_refused() {
    assert_truncated_block_refused(
        "t445d",
        "# forjar:other\n# forjar-cmd:other\n0 * * * * /usr/bin/other\n",
    );
}

#[test]
fn fj445_truncated_block_before_a_comment_is_refused() {
    assert_truncated_block_refused("t445e", "# a human note\n0 * * * * /usr/bin/other\n");
}

#[test]
fn fj445_truncated_block_before_a_blank_line_is_refused() {
    assert_truncated_block_refused("t445f", "\n0 * * * * /usr/bin/other\n");
}

#[test]
fn fj445_truncated_block_before_an_indented_comment_is_refused() {
    assert_truncated_block_refused("t445g", "  # indented note\n0 * * * * /usr/bin/other\n");
}

#[test]
fn fj445_absent_refuses_a_truncated_block_before_a_sibling_marker() {
    let fx = FakeCrontab::new("t445h");
    let seeded = "# forjar:backup\n# forjar-cmd:backup\n# forjar:other\n# forjar-cmd:other\n0 * * * * /usr/bin/other\n";
    fx.seed(seeded);
    let out = fx.run(&apply_script(&absent("backup")));
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(fx.read(), seeded);
}

/// The refusal must not widen: an entry line of any ordinary shape stays intact.
#[test]
fn fj445_entry_lines_of_every_ordinary_shape_stay_intact() {
    for entry in [
        "0 3 * * * /usr/local/bin/backup.sh",
        "@reboot /usr/local/bin/backup.sh",
        "  0 3 * * * /usr/local/bin/backup.sh",
        "MAILTO=ops@example.com",
    ] {
        let fx = FakeCrontab::new("t445i");
        fx.seed(&format!(
            "# forjar:backup\n# forjar-cmd:backup\n{entry}\n# forjar:other\n# forjar-cmd:other\n0 * * * * /usr/bin/other\n"
        ));
        fx.apply(&cron("backup", "0 4 * * *", CMD));
        let store = fx.read();
        assert!(
            store.contains("0 * * * * /usr/bin/other"),
            "{entry}: {store:?}"
        );
        assert!(
            store.contains("# forjar:other\n# forjar-cmd:other\n"),
            "{entry}: {store:?}"
        );
        assert_eq!(store.matches(CMD).count(), 1, "{entry}: {store:?}");
    }
}

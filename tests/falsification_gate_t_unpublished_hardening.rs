//! PMAT-1342 / T8b: gate T and a release never published to crates.io.

#[path = "release_goal_fixture/mod.rs"]
#[allow(dead_code)]
mod fx;
#[path = "t8b_fixture/mod.rs"]
mod t8b;
#[allow(unused_imports)]
use fx::*;
use t8b::*;

const BAD_VALUES: [&str; 9] = [
    "published",
    "false",
    "\"\"",
    "\" unpublished\"",
    "\"unpublished \"",
    "Unpublished",
    "yes",
    "null",
    "bogus",
];

/// The declaration is validated on EVERY row, below `cookbook_floor` too: the
/// floor decides whether the lock is read, not whether a value is meaningful.
#[test]
fn a_bogus_crates_io_below_the_cookbook_floor_is_red() {
    for v in BAD_VALUES {
        let fx = fixture(Case {
            cookbook_floor: "v9.9.9",
            crates_io: v,
            ..Case::default()
        });
        let r = run(&fx, AN_HOUR);
        r.assert_red(&format!("crates_io {v} below the floor is no declaration"));
        r.assert_says("not a value this gate knows");
    }
    // control: the exact word below the floor is merely not asked for anything
    let fx = fixture(Case {
        cookbook_floor: "v9.9.9",
        crates_io: "unpublished",
        ..Case::default()
    });
    run(&fx, AN_HOUR).assert_green("the exact word is a known value on any row");

    // a row below `floor:` (v0.0.0 < FLOOR v0.0.1) is still validated
    let fx = fixture(Case {
        extra_row: "  - tag: v0.0.0\n    cut: 2026-01-01T00:00:00Z\n    prs: []\n    tickets: []\n    crates_io: bogus\n",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("a bogus crates_io below floor: must still be rejected");
    r.assert_says("v0.0.0");
    r.assert_says("not a value this gate knows");
}

/// A key written twice (list item, top-level, or map) would let a reader see
/// one value while jq keeps the last.
#[test]
fn a_duplicate_crates_io_key_is_red() {
    for dup in [
        "bogus\n    crates_io: unpublished",
        "unpublished\n    crates_io: bogus",
        // the item's FIRST key is a key too
        "unpublished\n    tag: v0.0.1",
    ] {
        let fx = unpublished_fx(dup, "0.0.0");
        let r = run_index(&fx, Some(&idx(&[("0.0.0", false)])));
        r.assert_red("a duplicated key is not one declaration");
        r.assert_says("duplicate key");
    }
    // control: two distinct items sharing the same key names are valid
    let two_rows = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: SHA2,
        crates_io: "unpublished",
        extra_row: "  - tag: v0.0.0\n    cut: 2026-01-01T00:00:00Z\n    prs: []\n    tickets: []\n    crates_io: unpublished\n",
        ..Case::default()
    });
    // v0.0.0 is below floor and has no cookbook/crates_io lock check, only declaration check
    let mut two_rows = two_rows;
    two_rows.cookbook_files(TOML2, &lock_of("0.0.0"));
    run_index(&two_rows, Some(&idx(&[("0.0.0", false)])))
        .assert_green("distinct rows may share keys");

    // duplicate top-level key and duplicate map key in next:
    for bad in [
        "cadence_days: 2\nfloor: v0.0.1\nharness_floor: v0.0.1\ndogfood_floor: v0.0.1\ncookbook_floor: v0.0.1\ncookbook_floor:\nreleases: []\nnext:\n  tag: v0.0.2\n  due: 2026-01-03T00:00:00Z\n",
        "cadence_days: 2\nfloor: v0.0.1\nharness_floor: v0.0.1\ndogfood_floor: v0.0.1\nreleases: []\nreleases: []\nnext:\n  tag: v0.0.2\n  due: 2026-01-03T00:00:00Z\n",
        "cadence_days: 2\nfloor: v0.0.1\nharness_floor: v0.0.1\ndogfood_floor: v0.0.1\nreleases: []\nnext:\n  tag: v0.0.2\n  tag: v0.0.3\n  due: 2026-01-03T00:00:00Z\n",
    ] {
        let mut child = std::process::Command::new("awk")
            .args(["-f", &format!("{}/scripts/dogfood/lib/releases.awk", env!("CARGO_MANIFEST_DIR"))])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("awk must run");
        use std::io::Write;
        child.stdin.take().expect("stdin").write_all(bad.as_bytes()).expect("write");
        let out = child.wait_with_output().expect("wait");
        assert!(!out.status.success(), "awk accepted duplicate key:\n{bad}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("duplicate key"), "unexpected stderr: {err}");
    }
}

/// A num with a trailing newline or oversized component is not a semver num:
/// `$` would match before it, `\z` does not. Must be UNMEASURED, never reach `tonumber`.
#[test]
fn a_num_with_a_trailing_newline_is_unmeasured() {
    let fx = unpublished_fx("unpublished", "0.0.0");
    for num in [
        "1.34.0\\n",
        "0.0.0\\n",
        "0.0.1-rc.1\\n",
        // a leading line is not a semver num either
        "x\\n1.0.0",
        "x\\n0.0.0",
    ] {
        for y in [true, false] {
            let r = run_index(&fx, Some(&idx(&[("0.0.0", false), (num, y)])));
            r.assert_red(&format!("num {num:?} yanked {y}"));
            r.assert_says("UNMEASURED");
            r.assert_says("well-formed forjar version list");
        }
    }
    // oversized (10-digit) components on major, minor, or patch would lose float precision
    for big in ["1234567890.0.0", "0.1234567890.0", "0.0.1234567890"] {
        let r = run_index(&fx, Some(&idx(&[("0.0.0", false), (big, false)])));
        r.assert_red(&format!("ten-digit component must fail: {big}"));
        r.assert_says("UNMEASURED");
        r.assert_says("well-formed forjar version list");
    }
}

/// The selection fails closed on malformed inputs, leading zeros, 10-digit
/// components, non-boolean yanked, NDJSON, and empty stdin.
#[test]
fn the_selection_never_reads_a_newline_terminated_num() {
    for bad in [
        idx(&[("1.9.0", false), ("1.33.0\\n", false)]),
        idx(&[("1.9.0", false), ("x\\n1.33.0", false)]),
        idx(&[("1.33.0", false), ("01.33.0", false)]),
        idx(&[("1.33.0", false), ("0.01.0", false)]),
        idx(&[("1.33.0", false), ("0.0.01", false)]),
        idx(&[("1.33.0", false), ("1234567890.0.0", false)]),
        idx(&[("1.33.0", false), ("0.1234567890.0", false)]),
        idx(&[("1.33.0", false), ("0.0.1234567890", false)]),
        idx(&[("1.33.0", false), ("1.2.3.4 garbage!", false)]),
        r#"{"versions":[{"num":"1.33.0","yanked":false},{"num":"1.33.9","yanked":"false"}]}"#
            .to_string(),
        r#"{"versions":[{"num":"1.33.0","yanked":false},null]}"#.to_string(),
        r#"{"versions":[{"num":"1.33.0","yanked":false},{"num":123,"yanked":true}]}"#.to_string(),
        r#"{"versions":{"a":{"num":"1.33.0","yanked":false}}}"#.to_string(),
        format!(
            "{}\n{}",
            idx(&[("1.33.0", false)]),
            idx(&[("1.33.0", false)])
        ),
        String::new(),
    ] {
        let (code, out) = run_cratesio_below("1.34.1", &bad);
        assert_ne!(code, 0, "selection accepted malformed input {bad:?}: {out}");
    }
    // The TARGET version is held to the same strict semver: leading zeros, a
    // 10-digit component, a pre-release, a trailing newline or junk are refused.
    let ok = idx(&[("1.9.0", false)]);
    for bad_target in [
        "01.0.0",
        "1.01.0",
        "1.0.01",
        "1234567890.0.0",
        "1.1234567890.0",
        "1.0.1234567890",
        "1.34.1-rc1",
        "1.34.1\n",
        "1.34",
        "",
    ] {
        let (code, out) = run_cratesio_below(bad_target, &ok);
        assert_ne!(code, 0, "target {bad_target:?} accepted: {out}");
    }
    let (code, out) = run_cratesio_below("01.0.0", &idx(&[("0.0.0", false)]));
    assert_ne!(code, 0, "invalid target version must fail: {out}");
}

/// End-to-end predecessor wiring through tagged.sh with multiple published
/// versions below the release: locking an older version instead of the newest
/// predecessor stays red.
#[test]
fn tagged_end_to_end_locks_newest_predecessor_not_older_release() {
    let mut fx = fixture(Case {
        cookbook_floor: "v0.0.2",
        version: "0.0.2",
        ..Case::default()
    });
    let shipped_oid = stdout_of(&git(&fx.root, &["rev-parse", "v0.0.1^{commit}"]));
    let open_oid = stdout_of(&git(&fx.root, &["rev-parse", "HEAD~1"]));
    git(&fx.root, &["tag", "-a", "-m", "v0.0.2", "v0.0.2"]);
    let cut2: i64 = stdout_of(&git(
        &fx.root,
        &[
            "for-each-ref",
            "--format=%(creatordate:unix)",
            "refs/tags/v0.0.2",
        ],
    ))
    .parse()
    .expect("cut2");
    write(&fx.root, "open2.txt", "open after v0.0.2\n");
    git(&fx.root, &["add", "open2.txt"]);
    git(&fx.root, &["commit", "-qm", "more open work (#12)"]);
    let open2_oid = stdout_of(&git(&fx.root, &["rev-parse", "HEAD"]));
    write(
        &fx.root,
        "docs/audits/dogfood-0.0.2-receipt.md",
        "# dogfood 0.0.2\n\nverdict: GO\n\nDOGFOOD-0.0.2-RECEIPT-END\n",
    );
    write(&fx.root, "docs/audits/crux-0.0.2.md", "# crux 0.0.2\n");
    write(
        &fx.root,
        "docs/roadmaps/roadmap.yaml",
        &roadmap(&[
            (SHIPPED, &["release:v0.0.1"]),
            (OPEN, &["release:v0.0.2"]),
            ("PMAT-903", &["release:v0.0.3"]),
        ]),
    );
    let cut1_iso = iso(fx.cut);
    let cut2_iso = iso(cut2);
    let due = iso(cut2 + 2 * 86400);
    write(
        &fx.root,
        "docs/roadmaps/releases.yaml",
        &format!(
            "cadence_days: 2\nfloor: {FLOOR}\nharness_floor: {FLOOR}\ndogfood_floor: {FLOOR}\ncookbook_floor: v0.0.2\n\
             releases:\n\
               \x20\x20- tag: {FLOOR}\n    cut: {cut1_iso}\n    prs: [10]\n    tickets: [{SHIPPED}]\n    dogfood: docs/audits/dogfood-0.0.1-receipt.md\n    crux: docs/audits/crux-0.0.1.md\n\
               \x20\x20- tag: v0.0.2\n    cut: {cut2_iso}\n    prs: [11]\n    tickets: [{OPEN}]\n    dogfood: docs/audits/dogfood-0.0.2-receipt.md\n    crux: docs/audits/crux-0.0.2.md\n    cookbook: {SHA2}\n    crates_io: unpublished\n\
             next:\n  tag: v0.0.3\n  due: {due}\n"
        ),
    );
    git(&fx.root, &["add", "-A"]);
    git(&fx.root, &["commit", "-qm", "declare v0.0.2"]);
    git(&fx.root, &["push", "-q", "origin", "main", "--tags"]);
    fx.prs = format!(
        r#"[{{"number":10,"mergedAt":"2026-01-02T00:00:00Z","mergeCommit":{{"oid":"{shipped_oid}"}},"headRefName":"PMAT-901-the-shipped-work","title":"the shipped work","body":""}},{{"number":11,"mergedAt":"2026-01-03T00:00:00Z","mergeCommit":{{"oid":"{open_oid}"}},"headRefName":"PMAT-902-the-open-work","title":"the open work","body":""}},{{"number":12,"mergedAt":"2026-01-04T00:00:00Z","mergeCommit":{{"oid":"{open2_oid}"}},"headRefName":"PMAT-903-the-next-work","title":"more open work","body":""}}]"#
    );
    let two = idx(&[("0.0.0", false), ("0.0.1", false)]);
    fx.cookbook_files(TOML2, &lock_of("0.0.1"));
    run_index(&fx, Some(&two))
        .assert_green("0.0.1 is the newest published predecessor below 0.0.2");
    fx.cookbook_files(TOML2, &lock_of("0.0.0"));
    let r = run_index(&fx, Some(&two));
    r.assert_red("0.0.0 is older than 0.0.1");
    r.assert_says("0.0.1");
    r.assert_says("LOCKS forjar 0.0.0");
}

/// The real committed ledger parses and every row's crates_io declaration is known.
#[test]
fn the_committed_ledger_parses_and_every_crates_io_declaration_is_known() {
    let script = r#"set -euo pipefail
fail() { echo "FAIL: $1" >&2; exit 1; }
. scripts/dogfood/lib/window.sh
. scripts/dogfood/lib/releases.sh
. scripts/dogfood/lib/cratesio.sh
DOGFOOD_RELEASES_REF=worktree dogfood_load_releases
tags="$(jq -r '.releases[].tag' <<<"$DOGFOOD_RELEASES")"
for t in $tags; do
  dogfood_release_row "$t"
  crates_io_declaration_is_known "$DOGFOOD_RELEASE_ROW" "$t" || :
done
jq -e '
  (.releases[] | select(.tag == "v1.34.0") | .crates_io == "unpublished")
  and (.releases[] | select(.tag == "v1.34.1") | .crates_io == "unpublished")
  and (.releases[] | select(.tag == "v1.33.0") | has("crates_io") | not)
' <<<"$DOGFOOD_RELEASES" >/dev/null
"#;
    let out = std::process::Command::new("bash")
        .args(["-c", script])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("bash must run");
    assert!(
        out.status.success(),
        "committed docs/roadmaps/releases.yaml failed:\n{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A jq that cannot answer is UNMEASURED at every call in the exemption path,
/// never "absent", "not listed" or "no predecessor".
#[test]
fn a_jq_that_cannot_answer_is_unmeasured_at_every_call() {
    let index = idx(&[("0.0.0", false)]);
    // asking whether the row declares anything: a published row (lock = its
    // own version) must not read an unanswered question as an absent field
    let fx = unpublished_fx("", "0.0.1");
    let (r, _) = run_full(&fx, Some(&index), true, Some("has(\"crates_io\")"));
    r.assert_red("jq failed asking for crates_io");
    r.assert_says("UNMEASURED");
    r.assert_says("asking whether the row declares crates_io");
    // reading the value
    let fx = unpublished_fx("unpublished", "0.0.0");
    let (r, _) = run_full(&fx, Some(&index), true, Some(".crates_io == "));
    r.assert_red("jq failed reading crates_io");
    r.assert_says("reading the row");
    // the shape check
    let (r, _) = run_full(&fx, Some(&index), true, Some(".crate.id"));
    r.assert_red("jq failed on the shape check");
    r.assert_says("UNMEASURED");
    // counting
    let (r, _) = run_full(&fx, Some(&index), true, Some("select(.num == $v)"));
    r.assert_red("jq failed counting");
    r.assert_says("could not count");
    // selecting
    let (r, _) = run_full(&fx, Some(&index), true, Some("tonumber"));
    r.assert_red("jq failed selecting");
    r.assert_says("could not select the predecessor");
}

/// The declaration check reads exactly ONE JSON object: null, a scalar, an
/// array, empty input and a multi-document reply are UNMEASURED (rc 5 from the
/// `fail` stub), never "absent" (rc 1) or "unpublished" (rc 0).
#[test]
fn the_declaration_check_reads_exactly_one_object() {
    let script = r#"set -u
fail() { echo "FAIL: $1" >&2; exit 5; }
. scripts/dogfood/lib/cratesio.sh
rc=0; crates_io_declaration_is_known "$ROW" T || rc=$?; exit "$rc""#;
    let run = |row: &str| {
        std::process::Command::new("bash")
            .args(["-c", &format!("{script}\n")])
            .env("ROW", row)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("bash must run")
            .status
            .code()
    };
    assert_eq!(run(r#"{"tag":"v1.0.0"}"#), Some(1), "absent is rc 1");
    assert_eq!(
        run(r#"{"tag":"v1.0.0","crates_io":"unpublished"}"#),
        Some(0)
    );
    for bad in [
        "null",
        "5",
        "[]",
        "",
        r#"{"tag":"v1.0.0","crates_io":"bogus"}"#,
        "{\"tag\":\"v1.0.0\",\"crates_io\":\"bogus\"}\n{\"tag\":\"v1.0.0\"}",
        "{\"tag\":\"v1.0.0\"}\n{\"tag\":\"v1.0.0\",\"crates_io\":\"unpublished\"}",
        "{\"tag\":\"v1.0.0\"}\n{\"tag\":\"v1.0.0\"}",
        "{\"tag\":\"v1.0.0\",\"crates_io\":\"bogus\"}\n{\"tag\":\"v1.0.0\",\"crates_io\":\"unpublished\"}",
    ] {
        assert_eq!(run(bad), Some(5), "must be UNMEASURED/red, row: {bad:?}");
    }
}

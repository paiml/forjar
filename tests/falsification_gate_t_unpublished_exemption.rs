//! PMAT-1342 / T8b: gate T and a release never published to crates.io.

#[path = "release_goal_fixture/mod.rs"]
mod fx;
#[path = "t8b_fixture/mod.rs"]
mod t8b;
#[allow(unused_imports)]
use fx::*;
use t8b::*;

// ---------------------------------------------------------------------------
// PMAT-1342 / T8b: a release that was never published to crates.io.
//
// forjar 1.34.0 and 1.34.1 were tagged but never published, so no cookbook
// commit can lock them and gate T was red on main. The narrow exemption: a row
// that DECLARES `crates_io: unpublished` is not asked to lock its own version
// IF the crates.io index, read live, does not list that version, AND the lock
// pins the newest non-yanked stable release the index does list below it. Every
// other row, the newest included, keeps the full check; absent, unreadable or
// contradicting evidence stays red.
// ---------------------------------------------------------------------------

/// The positive case: declared, absent from the index, lock = newest published
/// predecessor.
#[test]
fn an_unpublished_release_may_lock_its_published_predecessor() {
    let fx = unpublished_fx("unpublished", "0.0.0");
    let r = run_index(&fx, Some(&idx(&[("0.0.0", false)])));
    r.assert_green("0.0.1 is declared unpublished, absent from crates.io, lock is 0.0.0");
    r.assert_says("never published");
}

/// MUTANT 1: the loosened rule must not exempt a PUBLISHED row. The row says
/// unpublished but crates.io lists 0.0.1 (yanked or not).
#[test]
fn a_row_declared_unpublished_that_crates_io_serves_stays_red() {
    let fx = unpublished_fx("unpublished", "0.0.0");
    let r = run_index(&fx, Some(&idx(&[("0.0.1", false), ("0.0.0", false)])));
    r.assert_red("crates.io lists 0.0.1, so the declaration is false");
    r.assert_says("crates.io lists");
    let r = run_index(&fx, Some(&idx(&[("0.0.1", true), ("0.0.0", false)])));
    r.assert_red("a yanked 0.0.1 was still published");
}

/// MUTANT 2: no declaration, no exemption, even when crates.io lacks the
/// version and the lock is the predecessor.
#[test]
fn an_undeclared_row_is_not_exempt_whatever_crates_io_says() {
    let fx = unpublished_fx("", "0.0.0");
    let r = run_index(&fx, Some(&idx(&[("0.0.0", false)])));
    r.assert_red("no crates_io: field, so the lock must pin the release");
    r.assert_says("LOCKS forjar 0.0.0");
}

/// MUTANT 3: an unreadable, empty, foreign or malformed index is UNMEASURED,
/// never a pass. Each malformed shape below made an earlier version of the
/// script print ok, because a jq error inside an `if` is not an error.
#[test]
fn an_unreadable_or_malformed_crates_io_index_is_unmeasured_and_red() {
    let fx = unpublished_fx("unpublished", "0.0.0");
    let r = run_index(&fx, None);
    r.assert_red("the index cannot be read");
    r.assert_says("UNMEASURED");
    for body in [
        "not json",
        r#"{"crate":{"id":"forjar"},"versions":[]}"#,
        // another crate's index
        r#"{"crate":{"id":"serde"},"versions":[{"num":"0.0.0","yanked":false}]}"#,
        // no crate id at all
        r#"{"versions":[{"num":"0.0.0","yanked":false}]}"#,
        // malformed elements next to a valid predecessor
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},"x"]}"#,
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},{"num":5,"yanked":false}]}"#,
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},{"num":"abc","yanked":false}]}"#,
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},{"num":"0.0.0"}]}"#,
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},null]}"#,
        // non-array versions object, non-boolean yanked, malformed pre-release, NDJSON
        r#"{"crate":{"id":"forjar"},"versions":{"a":{"num":"0.0.0","yanked":false}}}"#,
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},{"num":"0.0.2","yanked":null}]}"#,
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},{"num":"0.0.2","yanked":"false"}]}"#,
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},{"num":"0.0.2","yanked":0}]}"#,
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},{"num":"1.2.3.4 garbage!","yanked":false}]}"#,
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},{"num":"1.2.3-","yanked":false}]}"#,
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},{"num":"1.2.3+build","yanked":false}]}"#,
        concat!(
            r#"{"crate":{"id":"wrong"},"versions":[]}"#,
            "\n",
            r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false}]}"#
        ),
        concat!(
            r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false}]}"#,
            "\n",
            r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.9","yanked":false}]}"#
        ),
        // leading zeros on major, minor, or patch are not semver nums
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},{"num":"00.0.1","yanked":false}]}"#,
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},{"num":"0.00.1","yanked":false}]}"#,
        r#"{"crate":{"id":"forjar"},"versions":[{"num":"0.0.0","yanked":false},{"num":"0.0.01","yanked":false}]}"#,
    ] {
        let r = run_index(&fx, Some(body));
        r.assert_red(&format!("malformed index must be red: {body}"));
        r.assert_says("UNMEASURED");
        r.assert_says("well-formed forjar version list");
    }
    // Non-zero curl exit with valid JSON stdout must still fail closed as UNMEASURED.
    let (r, _) = run_full_opts(
        &fx,
        Some(&idx(&[("0.0.0", false)])),
        true,
        None,
        None,
        Some(22),
    );
    r.assert_red("non-zero curl exit with valid JSON stdout must fail");
    r.assert_says("UNMEASURED");
    r.assert_says("curl exit 22");
}

/// MUTANT 4: any value but the exact word `unpublished` is refused, and the
/// declaration is validated whatever the lock says (a published row whose lock
/// pins its own version cannot carry a bogus or false declaration either).
#[test]
fn an_unknown_crates_io_value_is_red() {
    for v in [
        "published",
        "false",
        "\"\"",
        "Unpublished",
        "yes",
        "null",
        "-absent-",
    ] {
        for lock in ["0.0.0", "0.0.1"] {
            let fx = unpublished_fx(v, lock);
            let r = run_index(&fx, Some(&idx(&[("0.0.0", false)])));
            r.assert_red("only the exact word unpublished declares anything");
            r.assert_says("crates_io");
        }
    }
}

/// MUTANT 5: a declared row whose lock already pins its own version is red:
/// the declaration says the release does not exist on crates.io, so a lock on
/// it contradicts it.
#[test]
fn a_declared_row_whose_lock_pins_its_own_version_is_red() {
    let fx = unpublished_fx("unpublished", "0.0.1");
    let r = run_index(&fx, Some(&idx(&[("0.0.0", false)])));
    r.assert_red("declared unpublished but locking the release itself");
    r.assert_says("must lock the newest published release below it");
}

/// MUTANT 6: the lock must be the newest published stable predecessor: not
/// ahead, not an older release, not a pre-release, not a yanked one, and
/// versions ABOVE the release never count.
#[test]
fn an_exempt_row_must_lock_the_newest_published_predecessor() {
    let fx = unpublished_fx("unpublished", "0.0.2");
    run_index(&fx, Some(&idx(&[("0.0.0", false)]))).assert_red("a lock ahead is no predecessor");

    // Only a pre-release below: there is no stable predecessor at all.
    let fx = unpublished_fx("unpublished", "0.0.0");
    let r = run_index(&fx, Some(&idx(&[("0.0.0-rc.1", false)])));
    r.assert_red("a pre-release is not a published predecessor");
    r.assert_says("no non-yanked stable release below");

    // A pre-release that sorts above the stable one must not be chosen: lock
    // 0.0.0 is the stable predecessor even though 0.0.1-rc.1 is newer.
    let r = run_index(&fx, Some(&idx(&[("0.0.0", false), ("0.0.1-rc.1", false)])));
    r.assert_green("0.0.0 is the newest stable below 0.0.1 whatever pre-releases exist");

    // Only yanked below: no predecessor.
    let r = run_index(&fx, Some(&idx(&[("0.0.0", true)])));
    r.assert_red("a yanked predecessor does not count");
    r.assert_says("no non-yanked stable release below");

    // The release is above: a newer published version never becomes the
    // predecessor (publishing 0.0.2 later must not break this row).
    let r = run_index(&fx, Some(&idx(&[("0.0.0", false), ("0.0.2", false)])));
    r.assert_green("0.0.2 is above 0.0.1 and is ignored");
    let fx2 = unpublished_fx("unpublished", "0.0.2");
    run_index(&fx2, Some(&idx(&[("0.0.0", false), ("0.0.2", false)])))
        .assert_red("locking a version above the release is not a predecessor lock");
}

/// MUTANT 7: the selection takes the NEWEST (not oldest, not first listed),
/// compares as numbers (1.10.0 > 1.9.0), stays strictly BELOW the release, and
/// skips pre-releases and yanked versions.
#[test]
fn the_predecessor_is_the_newest_stable_unyanked_version_strictly_below() {
    let many = idx(&[
        ("1.9.0", false),
        ("1.33.0", false),
        ("1.10.0", false),
        ("1.33.1-rc.1", false),
        ("1.33.1", true),
        ("1.34.2", false),
        ("1.35.0", false),
    ]);
    assert_eq!(newest_stable_below("1.34.1", &many), "1.33.0");
    assert_eq!(
        newest_stable_below("1.34.2", &many),
        "1.33.0",
        "strictly below"
    );
    assert_eq!(newest_stable_below("1.35.1", &many), "1.35.0");
    assert_eq!(
        newest_stable_below("1.10.1", &many),
        "1.10.0",
        "numeric order"
    );
    assert_eq!(
        newest_stable_below("1.9.0", &idx(&[("1.2.0", false), ("1.10.0", false)])),
        "1.2.0",
        "1.10.0 is not below 1.9.0"
    );
    assert_eq!(
        newest_stable_below("1.9.0", &many),
        "",
        "nothing below the oldest"
    );
    assert_eq!(
        newest_stable_below("1.33.1", &idx(&[("1.33.1-rc.1", false)])),
        ""
    );
}

/// The evidence cannot be forged from the environment: exporting CURL,
/// CRATES_IO_API, *_OVERRIDE, or FORJAR_GATE_T_TEST=1 never redirects the gate,
/// and `curl -q` ignores `.curlrc` under CURL_HOME.
#[test]
fn the_index_cannot_be_redirected_without_the_test_marker() {
    let fx = unpublished_fx("unpublished", "0.0.0");
    // control: with the PATH curl shim the forged index is served
    let (r, seen) = run_env_seen(&fx, Some(&idx(&[("0.0.0", false)])), true);
    r.assert_green("shim: honoured");
    assert!(!seen, "stub was not called");
    // curl -q must disable .curlrc so CURL_HOME cannot corrupt or redirect curl
    let ch = tempfile::tempdir().expect("curl_home");
    std::fs::write(ch.path().join(".curlrc"), "write-out = \"CORRUPTED\"\n").expect("curlrc");
    let (r, _) = run_full_opts(
        &fx,
        Some(&idx(&[("0.0.0", false)])),
        true,
        None,
        Some(ch.path()),
        None,
    );
    r.assert_green("curl -q ignores .curlrc");
    // env vars alone (including FORJAR_GATE_T_TEST=1) must never redirect curl or URL
    let (r, seen) = run_env_seen(&fx, Some(&idx(&[("0.0.0", false)])), false);
    assert!(
        !seen,
        "env: CURL / CURL_OVERRIDE exported in environment must not be honoured"
    );
    r.assert_red("env: CRATES_IO_API / CRATES_IO_API_OVERRIDE must not redirect the index");
    r.assert_says("UNMEASURED");
    r.assert_says("well-formed forjar version list");
}

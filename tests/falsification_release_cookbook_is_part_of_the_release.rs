//! PMAT-241: the cookbook is part of the release, not a downstream of it.
//!
//! paiml/forjar-cookbook is where forjar is USED rather than described: gate D
//! validates its configs against the built artifact and `make
//! dogfood-published VERSION=x.y.z` does it against what crates.io serves.
//! Nothing recorded WHICH cookbook that was, and the cookbook's master had not
//! moved since 2026-08-29T15:05:21Z while ten tags went out claiming to have
//! been dogfooded against it.
//!
//! So from `cookbook_floor` every ledger row names the cookbook commit the
//! release was qualified against, and gate T refuses a row that names none,
//! names a branch instead of a commit, names a commit the cookbook does not
//! carry, or names one whose `Cargo.toml` cannot admit the version that
//! shipped. Each case below drives the REAL script over a temp repository,
//! changes ONE thing, and asserts the gate is red for that reason by name.

#[path = "release_goal_fixture/mod.rs"]
mod fx;
use fx::*;

/// PMAT-241: a release at or above `cookbook_floor` names the cookbook it was
/// qualified against.
///
/// The cookbook is where forjar is USED rather than described — gate D
/// validates every one of its configs against the built artifact, and
/// `make dogfood-published VERSION=x.y.z` does it against what crates.io
/// actually serves. Nothing recorded WHICH cookbook that was, and
/// paiml/forjar-cookbook's master had not moved since 2026-08-29 while four
/// tags went out claiming to be dogfooded against it.
#[test]
fn a_release_at_the_cookbook_floor_without_its_cookbook_commit_is_red() {
    let fx = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: "",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("v0.0.1 is at the cookbook floor and its row names no cookbook commit");
    r.assert_says("cookbook");
    r.assert_says("ls-remote");
}

/// A branch name is not a commit: a branch moves, and a record of what a
/// release was qualified against must not.
#[test]
fn a_cookbook_field_that_is_not_a_commit_sha_is_red() {
    let fx = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: "master",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("`master` is a branch, not the commit the release was qualified against");
    r.assert_says("not a commit sha");
}

/// A commit the cookbook does not carry is UNMEASURED, not a pass: the
/// fixture's stubbed `gh` cannot answer a contents request, which is exactly
/// what an unreachable GitHub looks like.
#[test]
fn a_cookbook_commit_that_cannot_be_read_is_unmeasured_and_red() {
    let fx = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: "0123456789abcdef0123456789abcdef01234567",
        ..Case::default()
    });
    let r = run(&fx, AN_HOUR);
    r.assert_red("the cookbook commit cannot be read, so the claim is unmeasured");
    r.assert_says("UNMEASURED");
}

/// And nothing fires below the floor: the ten releases whose cookbook commit
/// nobody recorded are left alone rather than retrofitted with a record that
/// was never taken.
#[test]
fn a_release_below_the_cookbook_floor_is_not_asked_for_one() {
    let fx = fixture(Case {
        cookbook_floor: "v9.9.9",
        cookbook: "",
        ..Case::default()
    });
    run(&fx, AN_HOUR).assert_green("v0.0.1 is below the cookbook floor");
}

/// PMAT-241: the admission test is Cargo's, not `>=`.
///
/// `forjar = "1.2"` is a CARET requirement — `>=1.2.0, <2.0.0`. A plain `>=`
/// passes 2.0.0 against it, and it would do so at exactly the release where
/// this arm matters most: the one that breaks the cookbook. Measured wrong on
/// this branch before a lane could say it (`v2.0.0 >= v1.2 : yes`), and three
/// lanes said it too.
#[test]
fn a_cookbook_that_cannot_build_against_the_release_is_red() {
    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    let mut fx = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: SHA,
        ..Case::default()
    });

    // v0.0.1 against a 0.0.1 requirement: the cookbook can use what shipped.
    fx.cookbook_manifest("[dependencies]\nforjar = { version = \"0.0.1\" }\n");
    run(&fx, AN_HOUR).assert_green("the cookbook requires exactly what shipped");

    // Below 1.0 a caret admits only its own minor, so ^0.1 does not admit
    // 0.0.1 — the same arithmetic that makes ^1.2 refuse 2.0.0.
    fx.cookbook_manifest("[dependencies]\nforjar = { version = \"0.1\" }\n");
    let r = run(&fx, AN_HOUR);
    r.assert_red("v0.0.1 cannot satisfy a ^0.1 requirement");
    r.assert_says("is older than");

    // The ceiling is the next increment of the leftmost NON-ZERO component
    // Cargo was given, so `^0.0.0` stops at 0.0.1 and does not admit the
    // release. The rule this gate applies is Cargo's, not a sketch of it.
    fx.cookbook_manifest("[dependencies]\nforjar = { version = \"0.0.0\" }\n");
    let r = run(&fx, AN_HOUR);
    r.assert_red("^0.0.0 admits >=0.0.0, <0.0.1, and the release is 0.0.1");
    r.assert_says("stops at 0.0.1");

    // And the bare spelling of the same requirement reads the same way.
    fx.cookbook_manifest("[dependencies]\nforjar = \"0.0.1\"\n");
    run(&fx, AN_HOUR).assert_green("^0.0.1 admits exactly the release that shipped");
}

/// The requirement is read the way Cargo writes it: a caret is the default
/// spelling, a dev-dependency is not the dependency, and a requirement this
/// gate cannot evaluate is refused BY NAME rather than guessed at.
#[test]
fn the_cookbook_requirement_is_read_the_way_cargo_writes_it() {
    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    let mut fx = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: SHA,
        ..Case::default()
    });

    // A caret, which the first parser could not read at all: it reported
    // "declares no forjar version requirement", which is a false statement.
    fx.cookbook_manifest("[dependencies]\nforjar = \"^0.0.1\"\n");
    run(&fx, AN_HOUR)
        .assert_green("^0.0.1 is the same requirement written the way Cargo writes it");

    // A dev-dependency is not the dependency.
    fx.cookbook_manifest(
        "[dev-dependencies]\nforjar = \"9.9\"\n\n[dependencies]\nforjar = \"0.0.1\"\n",
    );
    run(&fx, AN_HOUR).assert_green("the requirement is the one under [dependencies]");

    // A multi-clause requirement is not evaluated, and says so.
    fx.cookbook_manifest("[dependencies]\nforjar = { version = \">=0.0.1, <1\" }\n");
    let r = run(&fx, AN_HOUR);
    r.assert_red("a multi-clause requirement is not evaluated");
    r.assert_says("multi-clause");
}

/// The rule itself, over the versions the real repository is in.
///
/// Every case above is pinned to the fixture's 0.0.x line, so the range that
/// actually matters — `forjar = "1.2"` admitting `>=1.2.0, <2.0.0` — is not
/// exercised by any of them. This drives `caret_admits` as it is written in
/// `scripts/dogfood/tagged.sh` over a table, so the arithmetic is measured
/// rather than inferred from one 0.0.x case. `2` is below the requirement,
/// `3` is past its ceiling, `0` admits.
#[test]
fn the_caret_rule_is_cargos_rule() {
    // (version that shipped, what the cookbook requires, expected exit)
    const TABLE: &[(&str, &str, i32)] = &[
        ("1.29.0", "1.2", 0),
        ("1.2.0", "1.2", 0),
        ("1.99.99", "1.2", 0),
        ("2.0.0", "1.2", 3),   // the defect: `>=` passes this one
        ("2.0.0", "1.2.3", 3), // and this one
        ("1.1.0", "1.2", 2),
        ("1.0.0", "1", 0),
        ("2.0.0", "1", 3),
        ("0.2.5", "0.2", 0),
        ("0.3.0", "0.2", 3), // below 1.0 the caret stops at the minor
        ("0.0.3", "0.0.3", 0),
        ("0.0.4", "0.0.3", 3), // and at 0.0.z it stops at the patch
        ("0.9.0", "0", 0),
        ("1.0.0", "0", 3),
        ("0.0.9", "0.0", 0),
        ("0.1.0", "0.0", 3),
    ];
    for (ver, req, want) in TABLE {
        let script = format!(
            r#"set -euo pipefail
               . scripts/dogfood/lib/window.sh
               . scripts/dogfood/lib/releases.sh
               eval "$(sed -n '/^caret_admits()/,/^}}/p' scripts/dogfood/tagged.sh)"
               # A function that is not there would come back 127, and `|| return 2`
               # inside the rule would read that as "below the requirement" — a
               # measurement of nothing, dressed as a verdict. Ask first.
               command -v caret_admits >/dev/null || exit 99
               command -v dogfood_semver_ge >/dev/null || exit 99
               caret_admits '{ver}' '{req}'"#
        );
        let out = std::process::Command::new("bash")
            .arg("-c")
            .arg(&script)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("bash must run");
        let got = out.status.code().unwrap_or(-1);
        assert_eq!(
            got,
            *want,
            "caret_admits {ver} {req} exited {got}, wanted {want}:\n{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

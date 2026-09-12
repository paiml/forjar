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

/// The rule itself, over every shape three review lanes broke it with.
///
/// Every fixture case above is pinned to the fixture's 0.0.x line, so the
/// range that actually matters — `forjar = "1.2"` admitting `>=1.2.0, <2.0.0`
/// — is exercised by none of them. This drives `dogfood_req_admits` as it is
/// written in `scripts/dogfood/lib/releases.sh`, over a table.
///
/// It SOURCES the library rather than slicing the function out of a script
/// with `sed`, which is what the first version of this test did: a lane showed
/// that writing `caret_admits () {` with one extra space made the slice empty
/// and the test red while the gate was fine, and that a function inside a
/// string literal would be extracted and pass while the gate was broken. A
/// test whose subject depends on the formatting of the file it reads is
/// measuring the formatting.
///
/// `2` is below the requirement, `3` is at or past its ceiling, `4` is a
/// version this rule refuses to evaluate, `0` admits.
#[test]
fn the_requirement_rule_is_cargos_rule() {
    // (version that shipped, operator, requirement, expected exit)
    const TABLE: &[(&str, &str, &str, i32)] = &[
        // A caret runs to the next major, which is the requirement the
        // cookbook actually carries (`forjar = { version = "1.2" }`).
        ("1.29.0", "^", "1.2", 0),
        ("1.2.0", "^", "1.2", 0),
        ("1.99.99", "^", "1.2", 0),
        ("2.0.0", "^", "1.2", 3), // a plain `>=` passes this one
        ("2.0.0", "^", "1.2.3", 3),
        ("1.1.0", "^", "1.2", 2),
        ("1.0.0", "^", "1", 0),
        ("2.0.0", "^", "1", 3),
        // Below 1.0 the caret narrows to the leftmost non-zero component.
        ("0.2.5", "^", "0.2", 0),
        ("0.3.0", "^", "0.2", 3),
        ("0.0.3", "^", "0.0.3", 0),
        ("0.0.4", "^", "0.0.3", 3),
        ("0.9.0", "^", "0", 0),
        ("1.0.0", "^", "0", 3),
        ("0.0.9", "^", "0.0", 0),
        ("0.1.0", "^", "0.0", 3),
        // A TILDE stops at the minor it was given, where a caret runs on.
        // Reading `~1.2` as `^1.2` admits 1.99.99 and 2.0.0, neither of which
        // Cargo admits: wider is the direction that produces a false green.
        ("1.2.9", "~", "1.2", 0),
        ("1.3.0", "~", "1.2", 3),
        ("1.99.99", "~", "1.2", 3),
        ("1.2.9", "~", "1.2.3", 0),
        ("1.3.0", "~", "1.2.3", 3),
        ("1.9.0", "~", "1", 0),
        ("2.0.0", "~", "1", 3),
        // An EXACT requirement admits one version when it was given three
        // components, and the last one it was given otherwise.
        ("1.2.3", "=", "1.2.3", 0),
        ("1.2.4", "=", "1.2.3", 3),
        ("1.2.9", "=", "1.2", 0),
        ("1.3.0", "=", "1.2", 3),
        // And what this rule REFUSES rather than measures wrong. `sort -V`
        // puts 1.2.4-alpha ABOVE 1.2.4 where Cargo puts it below; bash reads
        // a component of `3-9` as a subtraction and lands on an upper bound
        // of `0.0.-5`; `0.0.3.4` is not a version and was read as `0.0.3`;
        // `01.0.0` is not one either.
        ("1.2.4-alpha", "^", "1.2.3", 4),
        ("1.2.4", "^", "1.2.3-9", 4),
        ("0.0.4", "^", "0.0.3-9", 4),
        ("0.0.3", "^", "0.0.3.4", 4),
        ("1.0.0", "^", "01.0.0", 4),
        ("1.0.0+build", "^", "1", 4),
        ("1.0.0", "^", "1.*", 4),
        ("1.0.0", "^", "", 4),
    ];
    for (ver, op, req, want) in TABLE {
        let script = format!(
            r#"set -euo pipefail
               . scripts/dogfood/lib/window.sh
               . scripts/dogfood/lib/releases.sh
               # A function that is not there comes back 127, and `|| return 2`
               # inside the rule would read that as "below the requirement" — a
               # measurement of nothing, dressed as a verdict. Ask first.
               command -v dogfood_req_admits >/dev/null || exit 99
               command -v dogfood_semver_ge >/dev/null || exit 99
               dogfood_req_admits '{ver}' '{op}' '{req}'"#
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
            "dogfood_req_admits {ver} {op}{req} exited {got}, wanted {want}:\n{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// The operator reaches the rule, and a comment does not.
///
/// Two lanes found the gate stripping `^`, `~` and `=` and evaluating all
/// three as a caret — `=1.2.3` admitting everything below 2.0.0 — and all
/// three found `forjar = "1.2" # version = "2.0"` measuring 2.0. Both are
/// wrong in the FALSE GREEN direction: they admit a release the cookbook
/// cannot use. These drive the whole arm, not the rule underneath it.
#[test]
fn the_operator_reaches_the_rule_and_a_comment_does_not() {
    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    let mut fx = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: SHA,
        ..Case::default()
    });

    // `~0.0` admits >=0.0.0, <0.1.0, so the release is fine.
    fx.cookbook_manifest("[dependencies]\nforjar = \"~0.0\"\n");
    run(&fx, AN_HOUR).assert_green("~0.0 admits the 0.0.1 that shipped");

    // `=0.0.0` admits exactly 0.0.0. Read as a caret it would stop at 0.0.1
    // too, so the case that separates them is a tilde one major up: see the
    // table above. Here the point is that `=` is not silently widened.
    fx.cookbook_manifest("[dependencies]\nforjar = \"=0.0.0\"\n");
    let r = run(&fx, AN_HOUR);
    r.assert_red("=0.0.0 admits 0.0.0 and nothing else");
    r.assert_says("stops at 0.0.1");

    // The trailing comment is not the requirement.
    fx.cookbook_manifest("[dependencies]\nforjar = \"0.0.1\" # version = \"9.9\"\n");
    run(&fx, AN_HOUR).assert_green("the requirement is 0.0.1, not the 9.9 in the comment");

    // A pre-release tag would sort the wrong way, so it is refused by name
    // rather than measured. The RELEASE side is validated too, not just the
    // requirement: `ver` comes from the tag and nothing checked it.
    fx.cookbook_manifest("[dependencies]\nforjar = \"0.0.1-rc1\"\n");
    let r = run(&fx, AN_HOUR);
    r.assert_red("a pre-release requirement is not evaluated by this rule");
    r.assert_says("numeric components");
}

/// PMAT-537: the requirement is a range and the LOCK is what cargo builds.
///
/// Everything above reads `Cargo.toml`. Measured on `7c100454` — the exact
/// commit v1.29.0's ledger row named — the manifest says
/// `forjar = { version = "1.2" }` and the lock says `version = "1.2.1"`.
/// `^1.2` admits 1.29.0, so this gate printed `requires forjar 1.2 ok` about a
/// cookbook that compiles a version **twenty-seven minors older** than the
/// release it was recorded as qualifying.
///
/// The link was there and pointed at something never built against the
/// release, which is the half-true record PMAT-235 and PMAT-236 exist to
/// refuse.
///
/// Of the three readings of "every tagged release updates forjar-cookbook" —
/// the lock pins the released version, the lock is merely not older than the
/// last release, or the requirement admits the release — this gate takes the
/// FIRST. It is the only one under which "was qualified against" is true of
/// the thing that was actually compiled, and it puts a real obligation on a
/// second repository, which is why the arm says so in its own text.
#[test]
fn a_cookbook_whose_lock_is_not_the_release_is_named_and_red() {
    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    let mut fx = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: SHA,
        ..Case::default()
    });
    let toml = "[dependencies]\nforjar = \"0.0.1\"\n";

    // The lock pins the release: qualified.
    fx.cookbook_files(
        toml,
        "[[package]]\nname = \"forjar\"\nversion = \"0.0.1\"\n",
    );
    run(&fx, AN_HOUR).assert_green("the cookbook builds exactly what shipped");

    // The exact shape 7c100454 was in: the requirement admits the release and
    // the lock is older. Green before this arm existed.
    fx.cookbook_files(
        "[dependencies]\nforjar = \"0.0\"\n",
        "[[package]]\nname = \"forjar\"\nversion = \"0.0.0\"\n",
    );
    let r = run(&fx, AN_HOUR);
    r.assert_red(
        "^0.0 admits 0.0.1 and the lock pins 0.0.0, so nothing was built against the release",
    );
    r.assert_says("LOCKS forjar 0.0.0");
    r.assert_says("a requirement is a range and the lock is what cargo builds");

    // A lock NEWER than the release is refused too. The claim is "this cookbook
    // was qualified against this release", and a cookbook built against a later
    // one was not — being ahead is a different error, not a lesser one.
    fx.cookbook_files(
        toml,
        "[[package]]\nname = \"forjar\"\nversion = \"0.0.2\"\n",
    );
    run(&fx, AN_HOUR).assert_red("a lock ahead of the release did not qualify it either");
}

/// A lock that cannot be read, or carries no forjar, is UNMEASURED and red.
///
/// The whole arm exists because a range was mistaken for a measurement. A
/// missing lock would put it straight back: nothing is known about what that
/// cookbook compiles, and nothing is not a pass.
#[test]
fn a_cookbook_lock_that_says_nothing_is_unmeasured_and_red() {
    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";
    let mut fx = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: SHA,
        ..Case::default()
    });
    let toml = "[dependencies]\nforjar = \"0.0.1\"\n";

    // No lock at that commit at all.
    fx.cookbook_files(toml, "");
    let r = run(&fx, AN_HOUR);
    r.assert_red("a cookbook with no lockfile measures nothing");
    r.assert_says("no readable Cargo.lock");
    r.assert_says("UNMEASURED");

    // A lock that exists and pins every crate BUT forjar.
    fx.cookbook_files(
        toml,
        "[[package]]\nname = \"serde\"\nversion = \"1.0.0\"\n\n[[package]]\nname = \"tokio\"\nversion = \"1.0.0\"\n",
    );
    let r = run(&fx, AN_HOUR);
    r.assert_red("a lock with no forjar entry measures nothing about forjar");
    r.assert_says("pins no forjar version");

    // And the entry must be forjar's OWN version, not the one that happens to
    // follow another package's name. A lock is a list and the arm walks it.
    fx.cookbook_files(
        toml,
        "[[package]]\nname = \"serde\"\nversion = \"9.9.9\"\n\n[[package]]\nname = \"forjar\"\nversion = \"0.0.1\"\n",
    );
    run(&fx, AN_HOUR).assert_green("forjar's entry is forjar's, whatever precedes it");
}

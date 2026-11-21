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

const SHA2: &str = "0123456789abcdef0123456789abcdef01234567";
const TOML2: &str = "[dependencies]\nforjar = \"0.0\"\n";

fn lock_of(v: &str) -> String {
    format!("[[package]]\nname = \"forjar\"\nversion = \"{v}\"\n")
}

/// Run the gate with the index redirected to a document (or to a path that does
/// not exist when `index` is None). `marked` sets the test marker the script
/// requires before it honours any redirect.
fn run_env(fx: &Fixture, index: Option<&str>, marked: bool) -> Run {
    run_env_seen(fx, index, marked).0
}

/// As `run_env`, and whether the redirected curl was ever invoked.
fn run_env_seen(fx: &Fixture, index: Option<&str>, marked: bool) -> (Run, bool) {
    run_full(fx, index, marked, None)
}

/// Every jq the gate runs whose program contains `jq_fail` exits 5 (a jq that
/// cannot answer); all others are the real jq. When `serve_index` is true, a
/// `curl` shim on PATH rewrites crates.io requests to the fixture's local file.
fn run_full(
    fx: &Fixture,
    index: Option<&str>,
    serve_index: bool,
    jq_fail: Option<&str>,
) -> (Run, bool) {
    run_full_opts(fx, index, serve_index, jq_fail, None, None)
}

fn run_full_opts(
    fx: &Fixture,
    index: Option<&str>,
    serve_index: bool,
    jq_fail: Option<&str>,
    curl_home: Option<&std::path::Path>,
    curl_exit: Option<i32>,
) -> (Run, bool) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("forjar.json");
    if let Some(body) = index {
        std::fs::write(&path, body).expect("write index");
    }
    let seen = dir.path().join("seen");
    let stub = dir.path().join("curl-stub.sh");
    std::fs::write(
        &stub,
        format!("#!/bin/sh\ntouch {}\nexec curl \"$@\"\n", seen.display()),
    )
    .expect("stub");
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    let shim = dir.path().join("shim");
    std::fs::create_dir_all(&shim).expect("shim dir");
    if serve_index {
        std::fs::write(
            shim.join("curl"),
            "#!/bin/bash\nset -euo pipefail\nif [ -n \"${CURL_SHIM_EXIT:-}\" ]; then\n  cat \"$CURL_SHIM_INDEX\"\n  exit \"$CURL_SHIM_EXIT\"\nfi\nargs=()\nfor a in \"$@\"; do\n  if [ \"$a\" = \"https://crates.io/api/v1/crates/forjar\" ]; then\n    args+=(\"file://${CURL_SHIM_INDEX}\")\n  else\n    args+=(\"$a\")\n  fi\ndone\nPATH=\"$ORIG_PATH\" exec curl \"${args[@]}\"\n",
        )
        .expect("curl shim");
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(shim.join("curl"), std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
    }
    if !serve_index {
        // No live network, ever: a curl that always fails (exit 6), so a gate that
        // honoured an exported CURL / CRATES_IO_API would be the only way to green.
        std::fs::write(shim.join("curl"), "#!/bin/sh\nexit 6\n").expect("failing curl shim");
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(shim.join("curl"), std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
    }
    if jq_fail.is_some() {
        std::fs::write(
            shim.join("jq"),
            "#!/bin/bash\nfor a in \"$@\"; do case \"$a\" in *\"$JQ_FAIL_MARK\"*) exit 5;; esac; done\nPATH=\"$ORIG_PATH\" exec jq \"$@\"\n",
        )
        .expect("jq shim");
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(shim.join("jq"), std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
    }
    let orig = std::env::var("PATH").unwrap_or_default();
    let mut cmd = std::process::Command::new("bash");
    cmd.arg(fx.root.join("scripts/dogfood/tagged.sh"))
        .current_dir(&fx.root)
        .env("GH", &fx.gh)
        .env("DOGFOOD_NOW", (fx.cut + AN_HOUR).to_string())
        .env_remove("FORJAR_GATE_T_TEST")
        .env_remove("CURL_OVERRIDE")
        .env_remove("CRATES_IO_API_OVERRIDE")
        .env_remove("CURL_SHIM_EXIT")
        .env_remove("CURL_SHIM_INDEX");
    {
        cmd.env("PATH", format!("{}:{orig}", shim.display()))
            .env("ORIG_PATH", &orig);
    }
    if let Some(m) = jq_fail {
        cmd.env("JQ_FAIL_MARK", m);
    }
    if let Some(home) = curl_home {
        cmd.env("CURL_HOME", home);
    }
    let url = format!("file://{}", path.display());
    if serve_index {
        cmd.env("CURL_SHIM_INDEX", &path);
        if let Some(rc) = curl_exit {
            cmd.env("CURL_SHIM_EXIT", rc.to_string());
        }
    } else {
        // the names an operator or a CI job might plausibly export
        cmd.env("FORJAR_GATE_T_TEST", "1")
            .env("CRATES_IO_API", &url)
            .env("CRATES_IO_API_OVERRIDE", &url)
            .env("CURL", &stub)
            .env("CURL_OVERRIDE", &stub);
    }
    let out = cmd.output().expect("bash must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (
        Run {
            code: out.status.code().unwrap_or(-1),
            text,
        },
        seen.exists(),
    )
}

fn run_index(fx: &Fixture, index: Option<&str>) -> Run {
    run_env(fx, index, true)
}

fn unpublished_fx(crates_io: &'static str, lock: &str) -> Fixture {
    let mut fx = fixture(Case {
        cookbook_floor: FLOOR,
        cookbook: SHA2,
        crates_io,
        ..Case::default()
    });
    fx.cookbook_files(TOML2, &lock_of(lock));
    fx
}

/// An index document for crate `forjar` from (num, yanked) pairs.
fn idx(vs: &[(&str, bool)]) -> String {
    let items: Vec<String> = vs
        .iter()
        .map(|(n, y)| format!(r#"{{"num":"{n}","yanked":{y}}}"#))
        .collect();
    format!(
        r#"{{"crate":{{"id":"forjar"}},"versions":[{}]}}"#,
        items.join(",")
    )
}

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

/// Run the predecessor selection directly against an index with MANY versions,
/// which a fixture whose only tag is v0.0.1 cannot hold.
fn run_cratesio_below(ver: &str, index: &str) -> (i32, String) {
    use std::io::Write;
    let lib = format!(
        "{}/scripts/dogfood/lib/cratesio.sh",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut child = std::process::Command::new("bash")
        .args([
            "-c",
            &format!("fail() {{ echo \"$1\" >&2; exit 1; }}; . \"{lib}\" && cratesio_newest_stable_below \"$1\""),
            "bash",
            ver,
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("bash must run");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(index.as_bytes())
        .expect("write");
    let out = child.wait_with_output().expect("wait");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    )
}

fn newest_stable_below(ver: &str, index: &str) -> String {
    let (code, out) = run_cratesio_below(ver, index);
    assert_eq!(
        code, 0,
        "cratesio_newest_stable_below {ver} exited {code}: {out}"
    );
    out
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

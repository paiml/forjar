//! PMAT-607 (forjar#607): the release path runs no Python, and a pre-release
//! is not a release.
//!
//! v1.33.0 was tagged and never booked. Two things stood between it and the
//! tool gate T names (`scripts/release-goal.sh cut`):
//!
//! - The ledger loader and the three row and ledger edits were python3
//!   heredocs. They are now awk over the file text. Every case here except the
//!   pre-release ones runs with a `python3` on PATH that refuses to run, so the
//!   old code goes red on any of them. The cases are one per FORM the port
//!   reads or writes.
//! - `git tag --sort=-v:refname` and `sort -V` both rank v1.33.0-rc.1 ABOVE
//!   v1.33.0. So gate T asked for a row for the rc, and v1.33.0's window came
//!   out as the one PR merged after the rc. A release is a vX.Y.Z tag, the one
//!   shape the ledger admits for `tag:` (scripts/dogfood/lib/tags.sh).

#[path = "release_goal_fixture/mod.rs"]
mod fx;
use fx::*;
use std::process::Command;

/// A PATH whose `python3` and `python` refuse to run and say so.
fn no_python(fx: &Fixture) -> String {
    let bin = fx._dir.path().join("no-python");
    for name in ["python3", "python"] {
        let p = bin.join(name);
        std::fs::create_dir_all(&bin).expect("mkdir");
        std::fs::write(
            &p,
            "#!/bin/sh\necho \"PMAT-607: python was called on the release path: $*\" >&2\nexit 97\n",
        )
        .expect("write the refusing python");
        let mut perm = std::fs::metadata(&p).expect("stat").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perm, 0o755);
        std::fs::set_permissions(&p, perm).expect("chmod");
    }
    format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

fn bash(fx: &Fixture, path: &str, args: &[&str]) -> Run {
    let out = Command::new("bash")
        .args(args)
        .current_dir(&fx.root)
        .env("PATH", path)
        .env("GH", &fx.gh)
        .env("DOGFOOD_NOW", (fx.cut + AN_HOUR).to_string())
        .output()
        .expect("bash must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        code: out.status.code().unwrap_or(-1),
        text,
    }
}

fn goal(fx: &Fixture, args: &[&str]) -> Run {
    let mut all = vec!["scripts/release-goal.sh"];
    all.extend_from_slice(args);
    bash(fx, &no_python(fx), &all)
}

/// The block of row `id` in the fixture's roadmap.
fn row(fx: &Fixture, id: &str) -> String {
    let roadmap = read(fx, "docs/roadmaps/roadmap.yaml");
    let tail = roadmap
        .split(&format!("- id: {id}\n"))
        .nth(1)
        .unwrap_or_else(|| panic!("{id} is a row:\n{roadmap}"));
    tail.split("- id: ").next().unwrap_or_default().to_string()
}

/// Forms: `releases: []` opened; a label list emptied to `labels: []`; a label
/// added to `labels: []`; a label appended to a list; a label already there.
#[test]
fn the_cut_books_the_tag_with_no_python_on_the_path() {
    let fx = fixture(Case {
        pre_cut: true,
        rows: vec![
            (SHIPPED, vec!["release:v0.0.1"]),
            (OPEN, vec![]),
            ("PMAT-903", vec!["release:v0.0.1"]),
            ("PMAT-904", vec!["kind:code", "release:v0.0.1"]),
        ],
        ..Case::default()
    });
    let t = goal(&fx, &["cut", FLOOR, "--next", NEXT]);
    assert_eq!(t.code, 0, "cut must succeed without python:\n{}", t.text);
    t.assert_says("declared v0.0.1 (cut ");
    t.assert_says("already  PMAT-901 release:v0.0.1");
    t.assert_says("removed  PMAT-903 release:v0.0.1");
    t.assert_says("labelled PMAT-903 release:v0.0.2");
    t.assert_says("removed  PMAT-904 release:v0.0.1");

    let ledger = read(&fx, "docs/roadmaps/releases.yaml");
    assert!(!ledger.contains("releases: []"), "{ledger}");
    assert!(ledger.contains("releases:\n  - tag: v0.0.1\n"), "{ledger}");
    let due = iso(fx.cut + 2 * 86400);
    assert!(
        ledger.ends_with(&format!("    tickets: [PMAT-901]\n    dogfood: docs/audits/dogfood-0.0.1-receipt.md\n    crux: docs/audits/crux-0.0.1.md\nnext:\n  tag: v0.0.2\n  due: {due}\n")),
        "the row, then the next goal, and nothing else:\n{ledger}"
    );
    let p903 = row(&fx, "PMAT-903");
    assert!(
        p903.contains("  labels:\n  - release:v0.0.2\n  notes: null\n"),
        "emptied to [] and then given the next goal:\n{p903}"
    );
    let p904 = row(&fx, "PMAT-904");
    assert!(
        p904.contains("  labels:\n  - kind:code\n  - release:v0.0.2\n  notes: null\n"),
        "one label out of a list of two, the next one appended:\n{p904}"
    );
    assert!(
        !p904.contains("updated: 2026-01-01T00:00:00Z"),
        "an edited row has its updated: bumped:\n{p904}"
    );

    let s = goal(&fx, &["sync"]);
    assert_eq!(s.code, 0, "{}", s.text);
    s.assert_says("labelled PMAT-902 release:v0.0.2");
    assert!(
        row(&fx, OPEN).contains("  labels:\n  - release:v0.0.2\n"),
        "labels: [] became a list:\n{}",
        row(&fx, OPEN)
    );

    git(&fx.root, &["add", "-A"]);
    git(&fx.root, &["commit", "-qm", "book v0.0.1"]);
    let r = bash(&fx, &no_python(&fx), &["scripts/dogfood/tagged.sh"]);
    r.assert_green("booked with no python, and gate T reads the ledger with no python");
}

#[test]
fn a_label_is_appended_once_and_an_unknown_row_is_refused() {
    let fx = fixture(Case {
        rows: vec![(SHIPPED, vec!["release:v0.0.1"]), (OPEN, vec!["kind:code"])],
        ..Case::default()
    });
    let t = goal(&fx, &["tag", OPEN, NEXT]);
    assert_eq!(t.code, 0, "{}", t.text);
    t.assert_says("labelled PMAT-902 release:v0.0.2");
    let t = goal(&fx, &["tag", OPEN, NEXT]);
    assert_eq!(t.code, 0, "{}", t.text);
    t.assert_says("already  PMAT-902 release:v0.0.2");
    assert_eq!(row(&fx, OPEN).matches("release:v0.0.2").count(), 1);
    let t = goal(&fx, &["tag", "PMAT-999", NEXT]);
    assert_eq!(t.code, 3, "{}", t.text);
    t.assert_says("PMAT-999 is not a row of docs/roadmaps/roadmap.yaml");
}

/// Load `ledger` through lib/releases.sh as the worktree copy.
fn load(fx: &Fixture, ledger: &str) -> Run {
    write(&fx.root, "docs/roadmaps/releases.yaml", ledger);
    bash(
        fx,
        &no_python(fx),
        &[
            "-c",
            "set -euo pipefail; fail() { echo \"FAIL $1\"; exit 1; }; \
             . scripts/dogfood/lib/window.sh; . scripts/dogfood/lib/releases.sh; \
             DOGFOOD_RELEASES_REF=worktree dogfood_load_releases; echo \"$DOGFOOD_RELEASES\"",
        ],
    )
}

/// Forms: full-line, indented and trailing comments; flow lists with and
/// without spaces; an empty list; null; a sha with a leading 0; a comment line
/// inside a row (what `window` writes for strays).
#[test]
fn the_loader_reads_every_form_the_ledger_uses() {
    let fx = fixture(Case::default());
    let r = load(
        &fx,
        "# head\ncadence_days: 2   # days\nfloor: v0.0.1\nharness_floor: v0.0.1\ndogfood_floor: v0.0.1\n\
         releases:\n  # a row\n  - tag: v0.0.1\n    cut: 2026-01-01T00:00:00Z\n    prs: [10,11, 12]\n    tickets: []\n\
         \x20   # stray ids (no row, no alias): PMAT-9\n    cookbook: 0be3e1ec\nnext:\n  tag: v0.0.2\n  due: 2026-01-03T00:00:00Z\n",
    );
    assert_eq!(r.code, 0, "{}", r.text);
    assert_eq!(
        r.text.trim(),
        r#"{"cadence_days":2,"floor":"v0.0.1","harness_floor":"v0.0.1","dogfood_floor":"v0.0.1","releases":[{"tag":"v0.0.1","cut":"2026-01-01T00:00:00Z","prs":[10,11,12],"tickets":[],"cookbook":"0be3e1ec"}],"next":{"tag":"v0.0.2","due":"2026-01-03T00:00:00Z"}}"#
    );
    let r = load(
        &fx,
        "cadence_days: 2\nfloor: v0.0.1\nharness_floor: v0.0.1\ndogfood_floor: v0.0.1\nreleases: []\n\
         next:\n  tag: v0.0.1\n  due: 2026-01-03T00:00:00Z\n",
    );
    assert_eq!(r.code, 0, "{}", r.text);
    assert!(r.text.contains(r#""releases":[],"next""#), "{}", r.text);
}

/// A shape outside the ledger's subset is refused by line, never guessed at.
#[test]
fn the_loader_refuses_a_shape_it_does_not_know() {
    let fx = fixture(Case::default());
    let head = "cadence_days: 2\nfloor: v0.0.1\nharness_floor: v0.0.1\ndogfood_floor: v0.0.1\n";
    for (body, line) in [
        ("releases:\n- tag: v0.0.1\n", "line 6"),
        ("releases:\n  - tag: v0.0.1\n    prs:\n    - 10\n", "line 8"),
        ("releases: *rows\n", "line 5"),
        ("releases: [[10]]\n", "line 5"),
        ("releases: []\ncadence: 017\n", "line 6"),
    ] {
        let r = load(&fx, &format!("{head}{body}"));
        assert_eq!(r.code, 1, "{body:?} must be refused:\n{}", r.text);
        r.assert_says("does not parse");
        r.assert_says(line);
    }
}

/// An rc below its release (forjar's v1.33.0-rc.1) and an rc above the newest
/// release. Under the old rule gate T asked for a row for the first one. It
/// also measured the open window from the second.
#[test]
fn a_pre_release_is_not_a_release() {
    let fx = fixture(Case::default());
    git(
        &fx.root,
        &["tag", "-a", "-m", "rc", "v0.0.1-rc.1", "v0.0.0^{commit}"],
    );
    git(&fx.root, &["tag", "-a", "-m", "rc", "v0.0.2-rc.1", "HEAD"]);
    let path = std::env::var("PATH").unwrap_or_default();
    let r = bash(&fx, &path, &["scripts/dogfood/tagged.sh"]);
    r.assert_green("an rc tag has no row and opens no window");
    r.assert_says("merged since v0.0.1 carry release:v0.0.2");
    let w = bash(&fx, &path, &["scripts/release-goal.sh", "window", FLOOR]);
    assert_eq!(w.code, 0, "{}", w.text);
    w.assert_says("    prs: [10]\n");
}

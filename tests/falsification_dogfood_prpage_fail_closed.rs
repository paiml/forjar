//! PMAT-1179: every fail-closed path of scripts/dogfood/lib/prpage.sh is pinned by a case, and each case
//! is shown to go red on a mutant of the script that fails open at that path.
#![cfg(unix)]
use std::path::{Path, PathBuf};
use std::process::Command;

const DAY: i64 = 86_400;
const PRPAGE: &str = "scripts/dogfood/lib/prpage.sh";

struct Repo {
    _dir: tempfile::TempDir,
    root: PathBuf,
    base: i64,
}

fn sh(dir: &Path, prog: &str, args: &[&str], date: Option<i64>) -> String {
    let mut c = Command::new(prog);
    c.current_dir(dir).args(args);
    if let Some(t) = date {
        c.env("GIT_COMMITTER_DATE", format!("{t} +0000"));
        c.env("GIT_AUTHOR_DATE", format!("{t} +0000"));
    }
    let o = c.output().expect("spawn");
    assert!(o.status.success(), "{prog} {args:?}: {o:?}");
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

/// base <- ancestor (dated `ancestor_off` after base) <- head (dated 3600 s after base).
fn repo(ancestor_off: i64) -> Repo {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("r");
    std::fs::create_dir_all(&root).expect("mkdir");
    let base = 1_700_000_000;
    sh(&root, "git", &["init", "-q", "-b", "main"], None);
    for (cfg, v) in [
        ("user.email", "t@example.com"),
        ("user.name", "t"),
        ("commit.gpgsign", "false"),
        ("core.hooksPath", "/dev/null"),
    ] {
        sh(&root, "git", &["config", cfg, v], None);
    }
    for (i, off) in [(0, 0), (1, ancestor_off), (2, 3600)] {
        std::fs::write(root.join("f"), i.to_string()).expect("write");
        sh(&root, "git", &["add", "f"], None);
        sh(
            &root,
            "git",
            &["commit", "-qm", &format!("c{i}")],
            Some(base + off),
        );
        if i == 0 {
            sh(&root, "git", &["tag", "lo"], None);
        }
        if i == 1 {
            sh(&root, "git", &["tag", "mid"], None);
        }
    }
    Repo {
        _dir: dir,
        root,
        base,
    }
}

/// Runs `dogfood_pr_fetch LOWER UPPER` from `script` with a stub gh answering `body` (or failing when `None`).
/// Returns (exit code, output, the search bound the stub saw).
fn drive(
    r: &Repo,
    script: &Path,
    lower: &str,
    upper: &str,
    body: Option<&str>,
    shim: &str,
) -> (i32, String, String) {
    let log = r.root.join("../gh.args");
    let gh = r.root.join("../gh");
    let reply = body.map_or("echo boom >&2; exit 1".to_string(), |b| {
        format!("printf '%s' '{b}'")
    });
    std::fs::write(
        &gh,
        format!("#!/bin/sh\necho \"$*\" >> '{}'\n{reply}\n", log.display()),
    )
    .expect("gh");
    std::fs::set_permissions(&gh, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .expect("chmod");
    let drv = r.root.join("../drv.sh");
    let text = format!(
        "set -euo pipefail\nfail() {{ echo \"FAIL: $*\"; exit 7; }}\nGH='{gh}'\nREPO=o/r\nPR_PAGE_LIMIT=200\n{shim}\n. '{s}'\ndogfood_pr_fetch '{lower}' '{upper}' number\necho \"RAW=$DOGFOOD_PR_RAW\"\n",
        gh = gh.display(),
        s = script.display(),
    );
    std::fs::write(&drv, text).expect("drv");
    let _ = std::fs::remove_file(&log);
    let mut c = Command::new("bash");
    c.arg(&drv).current_dir(&r.root);
    let o = c.output().expect("bash");
    let seen = std::fs::read_to_string(&log).unwrap_or_default();
    let bound = seen
        .split("merged:")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .unwrap_or("")
        .to_string();
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        bound,
    )
}

fn iso(t: i64) -> String {
    sh(
        Path::new("/"),
        "date",
        &["-u", "-d", &format!("@{t}"), "+%Y-%m-%dT%H:%M:%SZ"],
        None,
    )
}

/// One case = one fail-closed path. Ok when the script behaves as documented, Err(why) when it does not.
type Case = fn(&Repo, &Path) -> Result<(), String>;

fn refuses(r: (i32, String, String), says: &str) -> Result<(), String> {
    if r.0 == 7 && r.1.contains(says) && !r.1.contains("RAW=") {
        return Ok(());
    }
    Err(format!(
        "expected a refusal saying {says:?}, got exit {} / {}",
        r.0, r.1
    ))
}

fn gh_failure(r: &Repo, s: &Path) -> Result<(), String> {
    refuses(drive(r, s, "lo", "main", None, ""), "cannot be enumerated")
}
fn non_array_answer(r: &Repo, s: &Path) -> Result<(), String> {
    for body in ["{}", "[1]", "", "[] []"] {
        refuses(
            drive(r, s, "lo", "main", Some(body), ""),
            "one JSON array of PRs",
        )
        .map_err(|e| format!("{body:?}: {e}"))?;
    }
    Ok(())
}
fn unreadable_commit_time(r: &Repo, s: &Path) -> Result<(), String> {
    refuses(
        drive(r, s, "no-such-ref", "main", Some("[]"), ""),
        "cannot read the commit time of no-such-ref",
    )?;
    refuses(
        drive(r, s, "lo", "no-such-ref", Some("[]"), ""),
        "cannot read the commit time of no-such-ref",
    )
}
fn failed_git_log_range(r: &Repo, s: &Path) -> Result<(), String> {
    let shim = "git() { case \"$*\" in *..*) return 128 ;; esac; command git \"$@\"; }";
    refuses(
        drive(r, s, "lo", "main", Some("[]"), shim),
        "cannot list the commits in lo..main",
    )
}
fn bound_format_failure(r: &Repo, s: &Path) -> Result<(), String> {
    refuses(
        drive(r, s, "lo", "main", Some("[]"), "date() { return 1; }"),
        "cannot format window bounds",
    )
}
fn empty_range_to(r: &Repo, s: &Path) -> Result<(), String> {
    let want = iso(r.base + 3600 + DAY);
    let (code, out, bound) = drive(r, s, "main", "main", Some("[]"), "");
    let got = bound.rsplit("..").next().unwrap_or("");
    if code == 0 && out.contains("RAW=[]") && got == want {
        return Ok(());
    }
    Err(format!("empty range must bound on UPPER's own commit time + 1 day ({want}), got exit {code} bound {bound:?}"))
}
fn to_is_max_commit_time_plus_a_day(r: &Repo, s: &Path) -> Result<(), String> {
    // the ancestor is dated 10 days after base, 10 days minus 1 h after HEAD: TO must cover it, plus one day
    let want = iso(r.base + 10 * DAY + DAY);
    let (code, _, bound) = drive(r, s, "lo", "main", Some("[]"), "");
    let got = bound.rsplit("..").next().unwrap_or("");
    if code == 0 && got == want {
        return Ok(());
    }
    Err(format!(
        "TO must be max commit time + 1 day ({want}), got exit {code} bound {bound:?}"
    ))
}

/// (name, case, repo ancestor offset, mutation: (from, to) applied to a scratch copy of prpage.sh)
const MUTANTS: [(&str, Case, i64, &str, &str); 9] = [
    (
        "gh failure swallowed",
        gh_failure,
        0,
        "--json \"$fields\" 2>&1)\" || rc=$?",
        "--json \"$fields\" 2>&1)\" || true",
    ),
    (
        "array check dropped",
        non_array_answer,
        0,
        "(.[0] | type == \"array\")",
        "true",
    ),
    (
        "commit-time check dropped",
        unreadable_commit_time,
        0,
        "if [ \"$rc\" -ne 0 ] || [ \"$e\" -le 0 ]; then",
        "if false; then",
    ),
    (
        "git log failure swallowed",
        failed_git_log_range,
        0,
        "2>/dev/null)\" || rc=$?\n  if [ \"$rc\" -ne 0 ]; then\n    fail \"cannot list",
        "2>/dev/null)\" || true\n  if [ \"$rc\" -ne 0 ]; then\n    fail \"cannot list",
    ),
    (
        "bound-format failure swallowed",
        bound_format_failure,
        0,
        "|| fail \"cannot format window bounds — UNMEASURED\"",
        "|| true",
    ),
    (
        "empty range loses UPPER time",
        empty_range_to,
        0,
        "printf '%s\\n%s\\n' \"$to\" \"$cts\"",
        "printf '%s\\n' \"$cts\"",
    ),
    (
        "commit times dropped, +1 day kept",
        to_is_max_commit_time_plus_a_day,
        10 * DAY,
        "printf '%s\\n%s\\n' \"$to\" \"$cts\"",
        "printf '%s\\n' \"$to\"",
    ),
    (
        "max never taken",
        to_is_max_commit_time_plus_a_day,
        10 * DAY,
        "m = $1+0 }",
        "m = m }",
    ),
    (
        "+1 day dropped",
        empty_range_to,
        0,
        "print m + 86400",
        "print m",
    ),
];

fn real() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(PRPAGE)
}

#[test]
fn every_fail_closed_path_of_prpage_refuses() {
    let cases: [(&str, Case, i64); 7] = [
        ("gh failure", gh_failure, 0),
        ("non-array answer", non_array_answer, 0),
        ("unreadable commit time", unreadable_commit_time, 0),
        ("failed git log LOWER..UPPER", failed_git_log_range, 0),
        ("bound-format failure", bound_format_failure, 0),
        ("empty-range TO", empty_range_to, 0),
        (
            "TO = max commit time + 1 day",
            to_is_max_commit_time_plus_a_day,
            10 * DAY,
        ),
    ];
    for (name, case, off) in cases {
        case(&repo(off), &real()).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[test]
fn each_case_goes_red_on_its_mutant() {
    let src = std::fs::read_to_string(real()).expect("prpage.sh");
    let scratch = tempfile::tempdir().expect("scratch");
    for (i, (name, case, off, from, to)) in MUTANTS.iter().enumerate() {
        assert_eq!(
            src.matches(from).count().min(2),
            if *name == "bound-format failure swallowed" {
                2
            } else {
                1
            },
            "{name}: the mutation target must exist in prpage.sh"
        );
        let path = scratch.path().join(format!("m{i}.sh"));
        std::fs::write(&path, src.replace(from, to)).expect("mutant");
        let red = case(&repo(*off), &path);
        eprintln!("mutant {name}: {red:?}");
        assert!(red.is_err(), "mutant survived: {name}");
    }
}

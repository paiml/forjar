//! Shared fixture for the gate T tests (PMAT-225, forjar#506): a temp
//! repository with a bare origin, two annotated tags, a stubbed `gh` and a
//! declared ledger, every knob of which one case can turn. Kept out of the
//! test files so the file-size gate holds; nothing here asserts anything.
//!
//! The gate joins a DECLARED side (`docs/roadmaps/releases.yaml` and the
//! `release:<tag>` labels on `docs/roadmaps/roadmap.yaml` rows) against a
//! MEASURED one (git's tags and their creation instants, the PRs a stubbed
//! `gh` reports merged, placed by ancestry, and a clock the test pins with
//! `DOGFOOD_NOW`). Every case below drives the REAL script over a temp
//! repository with a bare `origin`, changes ONE thing, and asserts the gate is
//! red for that reason by name — or green. A gate that exited 1 without its
//! `GATE T FAIL` line is a death, not a verdict, and is a failure here too.

#![allow(dead_code)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) const FLOOR: &str = "v0.0.1";
pub(crate) const NEXT: &str = "v0.0.2";
pub(crate) const SHIPPED: &str = "PMAT-901";
pub(crate) const OPEN: &str = "PMAT-902";

pub(crate) fn git(cwd: &Path, args: &[&str]) -> Output {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "tester")
        .env("GIT_AUTHOR_EMAIL", "tester@example.com")
        .env("GIT_COMMITTER_NAME", "tester")
        .env("GIT_COMMITTER_EMAIL", "tester@example.com")
        .output()
        .expect("git must run");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

pub(crate) fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

pub(crate) fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().expect("a parent")).expect("mkdir");
    std::fs::write(p, body).expect("write");
}

pub(crate) fn roadmap(rows: &[(&str, &[&str])]) -> String {
    let mut text = String::from("roadmap_version: '1.0'\nroadmap:\n");
    for (id, labels) in rows {
        text.push_str(&format!(
            "- id: {id}\n  title: fixture row\n  status: planned\n  updated: 2026-01-01T00:00:00Z\n"
        ));
        if labels.is_empty() {
            text.push_str("  labels: []\n");
        } else {
            text.push_str("  labels:\n");
            for l in *labels {
                text.push_str(&format!("  - {l}\n"));
            }
        }
        text.push_str("  notes: null\n");
    }
    text
}

/// What one case declares and stands on. Every field has the green default.
pub(crate) struct Case {
    /// The `prs:` list the ledger declares for the floor tag.
    pub(crate) declared_prs: &'static str,
    /// The rows of the roadmap: (id, labels).
    pub(crate) rows: Vec<(&'static str, Vec<&'static str>)>,
    /// A row appended to the ledger after the floor's, or none.
    pub(crate) extra_row: &'static str,
    /// Seconds added to the derived due instant before it is declared.
    pub(crate) due_skew: i64,
    /// The `dogfood_floor:` the ledger declares.
    pub(crate) dogfood_floor: &'static str,
    /// Whether the dogfood receipt and crux document exist at HEAD.
    pub(crate) receipts: bool,
    /// The version Cargo.toml carries at HEAD.
    pub(crate) version: &'static str,
    /// The head branch the stubbed `gh` reports for the floor window's PR.
    pub(crate) shipped_branch: &'static str,
    /// The world BEFORE the cut is booked: v0.0.1 exists and the ledger still
    /// says next is v0.0.1 (due = v0.0.0's cut + cadence) with no rows.
    pub(crate) pre_cut: bool,
}

impl Default for Case {
    fn default() -> Self {
        Case {
            declared_prs: "[10]",
            rows: vec![
                (SHIPPED, vec!["release:v0.0.1"]),
                (OPEN, vec!["release:v0.0.2"]),
            ],
            extra_row: "",
            due_skew: 0,
            dogfood_floor: FLOOR,
            receipts: true,
            version: "0.0.1",
            shipped_branch: "PMAT-901-the-shipped-work",
            pre_cut: false,
        }
    }
}

pub(crate) struct Fixture {
    pub(crate) _dir: tempfile::TempDir,
    pub(crate) root: PathBuf,
    pub(crate) gh: String,
    /// The floor tag's creation instant, as seconds.
    pub(crate) cut: i64,
}

pub(crate) fn stub_gh(dir: &Path, json: &str) -> String {
    let p = dir.join("gh");
    std::fs::write(
        &p,
        format!("#!/usr/bin/env bash\ncat <<'FIXTURE_JSON'\n{json}\nFIXTURE_JSON\n"),
    )
    .expect("write stub");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    p.to_string_lossy().into_owned()
}

pub(crate) fn iso(epoch: i64) -> String {
    let out = Command::new("date")
        .args(["-u", "-d", &format!("@{epoch}"), "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .expect("date must run");
    stdout_of(&out)
}

/// v0.0.0 (below the floor) -> PR #10 -> v0.0.1 (the floor) -> PR #11 (the
/// open window) -> the declaration commit. Tags are annotated and pushed to a
/// bare origin, so their creation instant is the tagger date, as forjar's are.
pub(crate) fn fixture(case: Case) -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("repo");
    let origin = dir.path().join("origin.git");
    std::fs::create_dir_all(&root).expect("mkdir repo");
    let nohooks = dir.path().join("nohooks");
    std::fs::create_dir_all(&nohooks).expect("mkdir nohooks");
    git(dir.path(), &["init", "-q", "--bare", "origin.git"]);
    git(&root, &["init", "-q", "-b", "main"]);
    git(
        &root,
        &["config", "core.hooksPath", &nohooks.to_string_lossy()],
    );
    git(&root, &["config", "commit.gpgsign", "false"]);
    git(
        &root,
        &["remote", "add", "origin", &origin.to_string_lossy()],
    );

    for rel in [
        "scripts/dogfood/tagged.sh",
        "scripts/dogfood/lib/window.sh",
        "scripts/dogfood/lib/releases.sh",
        "scripts/release-goal.sh",
    ] {
        let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
        let body = std::fs::read_to_string(&src)
            .unwrap_or_else(|e| panic!("the gate under test must exist at {}: {e}", src.display()));
        write(&root, rel, &body);
    }
    write(
        &root,
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\n",
    );
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "before the floor"]);
    git(&root, &["tag", "-a", "-m", "v0.0.0", "v0.0.0"]);

    write(
        &root,
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.0.1\"\n",
    );
    write(&root, "shipped.txt", "the work v0.0.1 shipped\n");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "the shipped work (#10)"]);
    let shipped_oid = stdout_of(&git(&root, &["rev-parse", "HEAD"]));
    git(&root, &["tag", "-a", "-m", FLOOR, FLOOR]);
    // The instant as seconds, rendered in UTC the way the gate renders it: a
    // fixture that formatted it in local time disagreed with the gate by the
    // host's offset, which is the gate catching a real disagreement.
    let cut: i64 = stdout_of(&git(
        &root,
        &[
            "for-each-ref",
            "--format=%(creatordate:unix)",
            &format!("refs/tags/{FLOOR}"),
        ],
    ))
    .parse()
    .expect("an epoch");
    let cut_iso = iso(cut);

    write(&root, "open.txt", "the work merged since the floor\n");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "the open work (#11)"]);
    let open_oid = stdout_of(&git(&root, &["rev-parse", "HEAD"]));

    // The declaration: the ledger, the registry, the version, the receipts.
    let base_cut: i64 = stdout_of(&git(
        &root,
        &[
            "for-each-ref",
            "--format=%(creatordate:unix)",
            "refs/tags/v0.0.0",
        ],
    ))
    .parse()
    .expect("an epoch");
    let due = iso(cut + 2 * 86400 + case.due_skew);
    let mut ledger = if case.pre_cut {
        format!(
            "cadence_days: 2\nfloor: {FLOOR}\nharness_floor: {FLOOR}\ndogfood_floor: {}\nreleases: []\nnext:\n  tag: {FLOOR}\n  due: {}\n",
            case.dogfood_floor,
            iso(base_cut + 2 * 86400)
        )
    } else {
        format!(
        "cadence_days: 2\nfloor: {FLOOR}\nharness_floor: {FLOOR}\ndogfood_floor: {}\nreleases:\n  - tag: {FLOOR}\n    cut: {cut_iso}\n    prs: {}\n    tickets: [{SHIPPED}]\n    dogfood: docs/audits/dogfood-0.0.1-receipt.md\n    crux: docs/audits/crux-0.0.1.md\n",
        case.dogfood_floor, case.declared_prs
    )
    };
    if !case.pre_cut {
        ledger.push_str(case.extra_row);
        ledger.push_str(&format!("next:\n  tag: {NEXT}\n  due: {due}\n"));
    }
    write(&root, "docs/roadmaps/releases.yaml", &ledger);
    let rows: Vec<(&str, &[&str])> = case
        .rows
        .iter()
        .map(|(id, l)| (*id, l.as_slice()))
        .collect();
    write(&root, "docs/roadmaps/roadmap.yaml", &roadmap(&rows));
    write(
        &root,
        "Cargo.toml",
        &format!(
            "[package]\nname = \"fixture\"\nversion = \"{}\"\n",
            case.version
        ),
    );
    if case.receipts {
        write(
            &root,
            "docs/audits/dogfood-0.0.1-receipt.md",
            "# dogfood 0.0.1\n\nverdict: GO\n\nDOGFOOD-0.0.1-RECEIPT-END\n",
        );
        write(&root, "docs/audits/crux-0.0.1.md", "# crux 0.0.1\n");
    }
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "declare the release goals"]);
    git(&root, &["push", "-q", "origin", "main", "--tags"]);

    let json = format!(
        r#"[{{"number":10,"mergedAt":"2026-01-02T00:00:00Z","mergeCommit":{{"oid":"{shipped_oid}"}},"headRefName":"{}","title":"the shipped work","body":""}},{{"number":11,"mergedAt":"2026-01-03T00:00:00Z","mergeCommit":{{"oid":"{open_oid}"}},"headRefName":"PMAT-902-the-open-work","title":"the open work","body":""}}]"#,
        case.shipped_branch
    );
    let gh = stub_gh(dir.path(), &json);
    Fixture {
        _dir: dir,
        root,
        gh,
        cut,
    }
}

pub(crate) struct Run {
    pub(crate) code: i32,
    pub(crate) text: String,
}

impl Run {
    pub(crate) fn assert_red(&self, why: &str) {
        assert_ne!(self.code, 0, "{why}; the gate exited 0:\n{}", self.text);
        assert!(
            !self.text.contains("GATE T PASS"),
            "{why}; the gate printed PASS:\n{}",
            self.text
        );
        assert!(
            self.text.contains("GATE T FAIL"),
            "{why}; exit {} with no GATE T FAIL line is a death, not a verdict:\n{}",
            self.code,
            self.text
        );
    }

    pub(crate) fn assert_green(&self, why: &str) {
        assert_eq!(
            self.code, 0,
            "{why}; the gate exited {}:\n{}",
            self.code, self.text
        );
        assert!(
            self.text.contains("GATE T PASS"),
            "{why}; exit 0 without a GATE T PASS line:\n{}",
            self.text
        );
    }

    pub(crate) fn assert_says(&self, needle: &str) {
        assert!(
            self.text.contains(needle),
            "the verdict does not name {needle:?}, so a reader cannot act on it:\n{}",
            self.text
        );
    }
}

/// Run the gate at instant `now` (seconds after the floor's cut).
pub(crate) fn run_at(fx: &Fixture, after_cut: i64, gh: &str) -> Run {
    let out = Command::new("bash")
        .arg(fx.root.join("scripts/dogfood/tagged.sh"))
        .current_dir(&fx.root)
        .env("GH", gh)
        .env("DOGFOOD_NOW", (fx.cut + after_cut).to_string())
        .output()
        .expect("bash must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        code: out.status.code().unwrap_or(-1),
        text,
    }
}

pub(crate) fn run(fx: &Fixture, after_cut: i64) -> Run {
    run_at(fx, after_cut, &fx.gh.clone())
}

pub(crate) const AN_HOUR: i64 = 3600;

impl Fixture {
    pub(crate) fn assert_committed(&self, rel: &str) {
        let out = git(&self.root, &["cat-file", "-e", &format!("HEAD:{rel}")]);
        assert!(out.status.success(), "{rel} must be at HEAD");
    }
}

/// Run `scripts/release-goal.sh` with the stubbed `gh` at `after_cut` seconds
/// past the floor's cut; stdout and stderr together, and the exit code.
pub(crate) fn tool(fx: &Fixture, after_cut: i64, args: &[&str]) -> Run {
    let out = Command::new("bash")
        .arg(fx.root.join("scripts/release-goal.sh"))
        .args(args)
        .current_dir(&fx.root)
        .env("GH", &fx.gh)
        .env("DOGFOOD_NOW", (fx.cut + after_cut).to_string())
        .output()
        .expect("bash must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        code: out.status.code().unwrap_or(-1),
        text,
    }
}

pub(crate) fn read(fx: &Fixture, rel: &str) -> String {
    std::fs::read_to_string(fx.root.join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

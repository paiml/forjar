//! PMAT-160 (#466 #467 #468): `--check`, `--dry-run` and the real apply must
//! select the SAME resources from the same selectors.
//!
//! Measured against 1.25.2 with the fixture below (`alpha` depends on `bravo`,
//! `charlie` unrelated and deliberately red):
//!
//! ```text
//!   apply --check  --subset alpha   FAIL charlie ... rc=1   (#467: no selection at all)
//!   apply --dry-run --subset alpha  error: resource 'alpha' depends on unknown 'bravo'
//!   apply          --subset alpha   error: resource 'alpha' depends on unknown 'bravo'
//! ```
//!
//! Three modes, three different answers, none of them the operator's. `--check`
//! returned before any selector ran, so a resource outside the scope failed the
//! run; `--subset` pruned the config BEFORE the DAG was validated, so a
//! `depends_on` the file declares correctly came back as "unknown".
//!
//! These tests go through the binary because that is where the three paths
//! diverge: each is a different call site in `dispatch_apply_b`, and a unit
//! test of any one of them cannot see the disagreement.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// `alpha` (depends on `bravo`, group `web`), `bravo`, and unrelated `charlie`.
///
/// Every resource is a `file`, so its apply script writes a marker that says it
/// ran and its check script exits 0 only when that marker holds the expected
/// content — one fixture that answers "what would run", "what ran" and "what is
/// converged" without a mock.
struct Project {
    dir: PathBuf,
    cfg: PathBuf,
    state: PathBuf,
}

const IDS: [&str; 3] = ["alpha", "bravo", "charlie"];

impl Project {
    fn new(dir: &Path) -> Self {
        let p = Self {
            dir: dir.to_path_buf(),
            cfg: dir.join("forjar.yaml"),
            state: dir.join("state"),
        };
        std::fs::write(&p.cfg, p.yaml()).expect("write config");
        p
    }

    fn yaml(&self) -> String {
        format!(
            "version: \"1.0\"\n\
             name: filter-pipeline\n\
             machines:\n\
             \x20 local:\n\
             \x20   hostname: localhost\n\
             \x20   addr: 127.0.0.1\n\
             resources:\n\
             \x20 alpha:\n\
             \x20   type: file\n\
             \x20   machine: local\n\
             \x20   path: {}\n\
             \x20   content: \"alpha\"\n\
             \x20   depends_on: [bravo]\n\
             \x20   resource_group: web\n\
             \x20 bravo:\n\
             \x20   type: file\n\
             \x20   machine: local\n\
             \x20   path: {}\n\
             \x20   content: \"bravo\"\n\
             \x20 charlie:\n\
             \x20   type: file\n\
             \x20   machine: local\n\
             \x20   path: {}\n\
             \x20   content: \"charlie\"\n",
            self.marker("alpha").display(),
            self.marker("bravo").display(),
            self.marker("charlie").display()
        )
    }

    fn marker(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.txt"))
    }

    /// Pre-converge `ids` so their check scripts pass; everything else stays red.
    fn converge(&self, ids: &[&str]) {
        for id in ids {
            std::fs::write(self.marker(id), id).expect("write marker");
        }
    }

    /// The ids whose marker exists — what an apply actually did.
    fn applied(&self) -> Vec<String> {
        IDS.iter()
            .filter(|id| self.marker(id).exists())
            .map(|id| (*id).to_string())
            .collect()
    }

    fn run(&self, extra: &[&str]) -> Output {
        forjar()
            .args(["apply", "-f"])
            .arg(&self.cfg)
            .arg("--state-dir")
            .arg(&self.state)
            .args(extra)
            .arg("--yes")
            .output()
            .expect("run apply")
    }
}

fn combined(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// The `results[].resource` of `--check --json`, deduplicated in report order.
fn checked_ids(out: &Output) -> Vec<String> {
    ids_at(out, "results", "resource")
}

/// The `changes[].resource` of `--dry-run --json`.
fn planned_ids(out: &Output) -> Vec<String> {
    ids_at(out, "changes", "resource")
}

fn ids_at(out: &Output, array: &str, key: &str) -> Vec<String> {
    let text = String::from_utf8_lossy(&out.stdout);
    let start = text
        .find('{')
        .unwrap_or_else(|| panic!("no JSON in:\n{text}"));
    let doc: serde_json::Value =
        serde_json::from_str(text[start..].trim()).unwrap_or_else(|e| panic!("{e}:\n{text}"));
    let mut ids: Vec<String> = Vec::new();
    for item in doc[array].as_array().expect("array present") {
        let id = item[key].as_str().expect("id present").to_string();
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

// ── #467: `--check` ran before any selector ─────────────────────────────────

/// THE #467 CASE. `charlie` is red and out of scope; the run must be green.
#[test]
fn check_under_subset_ignores_the_red_resource_out_of_scope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = Project::new(dir.path());
    p.converge(&["alpha", "bravo"]);

    let out = p.run(&["--check", "--subset", "alpha"]);
    let text = combined(&out);
    assert!(
        out.status.success(),
        "an out-of-scope red resource must not fail a scoped check:\n{text}"
    );
    assert!(text.contains("alpha"), "{text}");
    assert!(
        text.contains("bravo"),
        "the closure of --subset alpha includes bravo:\n{text}"
    );
    assert!(
        !text.contains("charlie"),
        "charlie is out of scope:\n{text}"
    );
}

#[test]
fn check_under_r_and_g_ignores_the_red_resource_out_of_scope() {
    for sel in [["-r", "alpha"], ["-g", "web"]] {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = Project::new(dir.path());
        p.converge(&["alpha", "bravo"]);

        let out = p.run(&["--check", sel[0], sel[1]]);
        let text = combined(&out);
        assert!(out.status.success(), "{sel:?} must exit 0:\n{text}");
        assert!(text.contains("bravo"), "{sel:?}:\n{text}");
        assert!(!text.contains("charlie"), "{sel:?}:\n{text}");
    }
}

#[test]
fn check_under_exclude_and_skip_never_runs_the_excluded_resource() {
    for sel in [
        vec!["--subset", "*", "--exclude", "charlie"],
        vec!["--skip", "charlie"],
    ] {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = Project::new(dir.path());
        p.converge(&["alpha", "bravo"]);

        let mut args = vec!["--check"];
        args.extend(sel.iter());
        let out = p.run(&args);
        let text = combined(&out);
        assert!(out.status.success(), "{sel:?} must exit 0:\n{text}");
        assert!(!text.contains("charlie"), "{sel:?}:\n{text}");
    }
}

/// `-m` selects the executor, not the resource set — it must survive the
/// rewiring alongside the resource selectors.
#[test]
fn check_still_honours_the_machine_selector() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = Project::new(dir.path());
    p.converge(&["alpha", "bravo"]);

    let out = p.run(&["--check", "--subset", "alpha", "-m", "local"]);
    let text = combined(&out);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("alpha"), "{text}");
}

// ── #466: `--dry-run` rendered the unscoped plan ────────────────────────────

#[test]
fn dry_run_lists_exactly_the_closure_and_its_summary_agrees() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = Project::new(dir.path());

    let out = p.run(&["--dry-run", "--subset", "alpha"]);
    let text = combined(&out);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("alpha"), "{text}");
    assert!(
        text.contains("bravo"),
        "the closure must be listed:\n{text}"
    );
    assert!(!text.contains("charlie"), "{text}");
    assert!(
        text.contains("2 to add, 0 to change"),
        "the summary must count the listed lines:\n{text}"
    );
    // bravo is alpha's prerequisite, so the body lists it first.
    let (b, a) = (text.find("bravo"), text.find("alpha on"));
    assert!(
        b < a,
        "the closure must be listed in execution order:\n{text}"
    );
}

#[test]
fn dry_run_json_reports_the_same_set_as_the_text_body() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = Project::new(dir.path());

    let out = p.run(&["--dry-run", "--json", "--subset", "alpha"]);
    assert!(out.status.success(), "{}", combined(&out));
    assert_eq!(sorted(planned_ids(&out)), vec!["alpha", "bravo"]);
}

#[test]
fn unscoped_dry_run_still_lists_every_resource() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = Project::new(dir.path());

    let out = p.run(&["--dry-run", "--json"]);
    assert!(out.status.success(), "{}", combined(&out));
    assert_eq!(sorted(planned_ids(&out)), vec!["alpha", "bravo", "charlie"]);
}

// ── #468: `--subset` pruned before the DAG was validated ────────────────────

#[test]
fn apply_under_subset_converges_the_dependency_too() {
    for sel in [["--subset", "alpha"], ["-r", "alpha"]] {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = Project::new(dir.path());

        let out = p.run(&[sel[0], sel[1]]);
        let text = combined(&out);
        assert!(
            !text.contains("depends on unknown"),
            "{sel:?} declared bravo; selection must not make it unknown:\n{text}"
        );
        assert!(out.status.success(), "{sel:?}:\n{text}");
        assert_eq!(p.applied(), vec!["alpha", "bravo"], "{sel:?}:\n{text}");
    }
}

/// The guard: a dependency the FILE never declares is still an error. The fix
/// is "validate the whole graph first", not "stop validating".
#[test]
fn an_undeclared_dependency_is_still_refused_under_subset() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = Project::new(dir.path());
    let broken = dir.path().join("broken.yaml");
    std::fs::write(
        &broken,
        p.yaml()
            .replace("depends_on: [bravo]", "depends_on: [ghost]"),
    )
    .expect("write broken config");

    let out = forjar()
        .args(["apply", "-f"])
        .arg(&broken)
        .arg("--state-dir")
        .arg(&p.state)
        .args(["--subset", "alpha", "--yes"])
        .output()
        .expect("run apply");
    let text = combined(&out);
    assert!(!out.status.success(), "{text}");
    assert!(
        text.contains("depends on unknown resource 'ghost'"),
        "today's message must survive:\n{text}"
    );
}

// ── the whole point: one selection, three modes ─────────────────────────────

/// For every selector form, `--check`, `--dry-run` and the real apply must
/// select the SAME ids. Run from an unconverged project so all three answer
/// about the same work: check reports every selected resource (red), the dry
/// run plans it, and the apply leaves exactly its markers.
#[test]
fn check_dry_run_and_apply_select_the_same_ids() {
    for sel in [
        vec!["--subset", "alpha"],
        vec!["-r", "alpha"],
        vec!["-g", "web"],
        vec!["--subset", "*", "--exclude", "charlie"],
        vec!["--skip", "charlie"],
    ] {
        let expected = vec!["alpha".to_string(), "bravo".to_string()];
        assert_eq!(sorted(selected_by_check(&sel)), expected, "check {sel:?}");
        assert_eq!(
            sorted(selected_by_dry_run(&sel)),
            expected,
            "dry run {sel:?}"
        );
        assert_eq!(sorted(selected_by_apply(&sel)), expected, "apply {sel:?}");
    }
}

fn selected_by_check(sel: &[&str]) -> Vec<String> {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = Project::new(dir.path());
    let mut args = vec!["--check", "--json"];
    args.extend(sel.iter());
    checked_ids(&p.run(&args))
}

fn selected_by_dry_run(sel: &[&str]) -> Vec<String> {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = Project::new(dir.path());
    let mut args = vec!["--dry-run", "--json"];
    args.extend(sel.iter());
    planned_ids(&p.run(&args))
}

fn selected_by_apply(sel: &[&str]) -> Vec<String> {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = Project::new(dir.path());
    let out = p.run(sel);
    assert!(out.status.success(), "apply {sel:?}:\n{}", combined(&out));
    p.applied()
}

/// PMAT-160 quorum finding: the standalone `check` command was routed through
/// the same resolver and this suite never drove it. `check -r` selects the
/// closure; `check -r <typo>` is refused rather than reported as `0 pass`.
#[test]
fn standalone_check_selects_the_closure_and_refuses_a_typo() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = Project::new(dir.path());
    p.converge(&["alpha", "bravo"]);
    let check = |extra: &[&str]| {
        forjar()
            .args(["check", "-f"])
            .arg(&p.cfg)
            .arg("--state-dir")
            .arg(&p.state)
            .args(extra)
            .output()
            .expect("run check")
    };
    let out = check(&["-r", "alpha", "--json"]);
    let mut ids = checked_ids(&out);
    ids.sort();
    assert_eq!(ids, ["alpha", "bravo"], "{}", combined(&out));
    let out = check(&["-r", "typo"]);
    assert!(
        !out.status.success(),
        "a typo must be refused:\n{}",
        combined(&out)
    );
    assert!(
        combined(&out).contains("matches no resource"),
        "{}",
        combined(&out)
    );
}

/// PMAT-160 quorum finding: a negative selector that empties the selection
/// used to converge nothing at exit 0 — the FJ-2723 silent-green shape.
#[test]
fn a_negative_that_empties_the_selection_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let p = Project::new(dir.path());
    let out = p.run(&["--dry-run", "--exclude", "*"]);
    assert!(!out.status.success(), "{}", combined(&out));
    assert!(
        combined(&out).contains("no resources remain"),
        "{}",
        combined(&out)
    );
    assert!(p.applied().is_empty());
}

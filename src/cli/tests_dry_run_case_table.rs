//! Case table: `apply -r <leaf> --dry-run` must show EVERY action
//! `apply -r <leaf>` then takes (forjar#704).
//!
//! Each row converges one config, plants one kind of drift, and then asserts
//! four things against the SAME state dir:
//!
//! 1. the dry run's plan lists exactly the row's expected `Update`s;
//! 2. a dry run, run twice (composed plan and the real `apply --dry-run`),
//!    leaves every lock file byte-identical and plans the same both times;
//! 3. the real `apply -r <leaf>` converges exactly the resources the dry run
//!    listed — read from the apply's own `resource_converged` events, not from
//!    the plan — so a preview that hides work goes red here;
//! 4. the observable effect: drifted files restored, the "upgrade" task's
//!    command run exactly once when (and only when) its check had failed.
//!
//! Row (b) is the incident shape: a package-upgrade style dependency whose
//! completion check fails. The dry run used to say "no changes" and the apply
//! then ran the upgrade. It is modelled with a `task` whose `completion_check`
//! is a command that fails: local transport, no network, no package manager.

#[cfg(test)]
mod tests {
    use crate::cli::apply::cmd_apply;
    use crate::cli::apply_drift::{check_pre_apply_drift, GateScope};
    use crate::cli::apply_dry_run::scoped_dry_run_plan;
    use crate::cli::apply_selection::{resolve_selection, Selectors};
    use crate::core::types::PlanAction;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    const LEAF: &str = "leaf-file";

    /// Paths of everything a row may tamper with or observe.
    struct Fixture {
        _dir: tempfile::TempDir,
        config: PathBuf,
        state: PathBuf,
        deep: PathBuf,
        base: PathBuf,
        leaf: PathBuf,
        other: PathBuf,
        marker: PathBuf,
        runlog: PathBuf,
    }

    /// leaf-file -> base-file -> deep-file   (deep-file is 2 levels deep)
    /// leaf-file -> pkg-upgrade              (task; check = marker exists)
    /// other-file                            (outside the selection)
    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let p = |n: &str| dir.path().join(n);
        let f = Fixture {
            config: p("forjar.yaml"),
            state: p("state"),
            deep: p("deep.txt"),
            base: p("base.txt"),
            leaf: p("leaf.txt"),
            other: p("other.txt"),
            marker: p("upgraded.marker"),
            runlog: p("upgrade.runs"),
            _dir: dir,
        };
        let yaml = format!(
            r#"
version: "1.0"
name: dry-run-case-table
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  deep-file:
    type: file
    machine: local
    path: {deep}
    content: "deep declared"
  base-file:
    type: file
    machine: local
    path: {base}
    content: "base declared"
    depends_on: [deep-file]
  pkg-upgrade:
    type: task
    machine: local
    command: "echo ran >> '{runlog}' && touch '{marker}'"
    completion_check: "[ -f '{marker}' ]"
  leaf-file:
    type: file
    machine: local
    path: {leaf}
    content: "leaf declared"
    depends_on: [base-file, pkg-upgrade]
  other-file:
    type: file
    machine: local
    path: {other}
    content: "other declared"
policy:
  tripwire: true
"#,
            deep = f.deep.display(),
            base = f.base.display(),
            leaf = f.leaf.display(),
            other = f.other.display(),
            runlog = f.runlog.display(),
            marker = f.marker.display(),
        );
        std::fs::write(&f.config, yaml).unwrap();
        f
    }

    /// `apply -f <config> --state-dir <state> [-r <resource>] [--dry-run] --yes`.
    fn apply(config: &Path, state: &Path, resource: Option<&str>, dry_run: bool) {
        cmd_apply(
            config, state, None, resource, None, None, false, dry_run, false, &[], false, None,
            false, false, None, None, false, false, None, false, false, 0, true, false, None,
            false, None, None, None, false, None, false, None, false, None, &[],
        )
        .unwrap();
    }

    /// The dry run's plan for `apply -r <resource> --dry-run`, composed as
    /// `cmd_apply_scoped` composes it: resolve the selection (closing over
    /// `depends_on`), run the drift gate as a dry run, plan.
    fn dry_run_plan(f: &Fixture, resource: &str) -> BTreeMap<String, PlanAction> {
        let mut config = crate::cli::helpers::parse_and_validate(&f.config).unwrap();
        let sel = Selectors {
            resource: Some(resource),
            ..Default::default()
        };
        resolve_selection(&mut config, &sel, false).unwrap();
        let scope = GateScope::default();
        let observed = check_pre_apply_drift(&config, &f.state, scope, false, true, false).unwrap();
        let plan = scoped_dry_run_plan(&config, &f.state, &scope, &observed, true).unwrap();
        plan.changes
            .into_iter()
            .map(|c| (c.resource_id, c.action))
            .collect()
    }

    /// Every lock file under the state dir, by path, with its bytes.
    fn lock_snapshot(state: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        let mut out = BTreeMap::new();
        let mut stack = vec![state.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.contains("lock"))
                {
                    out.insert(p.clone(), std::fs::read(&p).unwrap());
                }
            }
        }
        out
    }

    fn events(state: &Path) -> Vec<serde_json::Value> {
        let path = crate::tripwire::eventlog::event_log_path(state, "local");
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }

    /// The resources the apply actually converged, from its own event log.
    fn converged_since(state: &Path, skip: usize) -> BTreeSet<String> {
        events(state)
            .into_iter()
            .skip(skip)
            .filter(|e| e["event"] == "resource_converged")
            .filter_map(|e| e["resource"].as_str().map(str::to_string))
            .collect()
    }

    fn runs(f: &Fixture) -> usize {
        std::fs::read_to_string(&f.runlog)
            .map(|s| s.lines().count())
            .unwrap_or(0)
    }

    struct Row {
        name: &'static str,
        tamper: fn(&Fixture),
        /// What the dry run must list as `Update` — and, by invariant 3, what
        /// the apply must converge: the drifted resource plus every selected
        /// dependent `depends_on` propagation promotes. The rest are `NoOp`.
        updates: &'static [&'static str],
    }

    fn rows() -> Vec<Row> {
        vec![
            Row {
                name: "(a) drifted file dependency",
                tamper: |f| std::fs::write(&f.base, "tampered").unwrap(),
                updates: &["base-file", LEAF],
            },
            Row {
                name: "(b) upgrade-style task dependency whose check fails",
                tamper: |f| std::fs::remove_file(&f.marker).unwrap(),
                updates: &["pkg-upgrade", LEAF],
            },
            Row {
                name: "(c) converged dependencies",
                tamper: |_| {},
                updates: &[],
            },
            Row {
                name: "(d) drift on the selected leaf itself",
                tamper: |f| std::fs::write(&f.leaf, "tampered").unwrap(),
                updates: &[LEAF],
            },
            Row {
                name: "(e) two drifted dependencies, dry run repeated: no lock written",
                tamper: |f| {
                    std::fs::write(&f.base, "tampered").unwrap();
                    std::fs::remove_file(&f.marker).unwrap();
                },
                updates: &["base-file", "pkg-upgrade", LEAF],
            },
            Row {
                name: "(f) dependency two levels deep",
                tamper: |f| std::fs::write(&f.deep, "tampered").unwrap(),
                updates: &["deep-file", "base-file", LEAF],
            },
            Row {
                name: "(g) drift outside the selection",
                tamper: |f| std::fs::write(&f.other, "tampered").unwrap(),
                updates: &[],
            },
        ]
    }

    fn check_row(row: &Row) {
        let name = row.name;
        let f = fixture();
        apply(&f.config, &f.state, None, false);
        assert_eq!(runs(&f), 1, "{name}: the converge runs the upgrade once");
        (row.tamper)(&f);

        // 1. The dry run lists exactly the expected updates.
        let before = lock_snapshot(&f.state);
        assert!(!before.is_empty(), "{name}: no lock to compare against");
        let plan = dry_run_plan(&f, LEAF);
        let listed: BTreeSet<String> = plan
            .iter()
            .filter(|(_, a)| **a != PlanAction::NoOp)
            .map(|(id, _)| id.clone())
            .collect();
        let want: BTreeSet<String> = row.updates.iter().map(|s| s.to_string()).collect();
        assert_eq!(listed, want, "{name}: dry run listed {plan:?}");
        for id in &want {
            assert_eq!(plan[id], PlanAction::Update, "{name}: {id} must be Update");
        }
        assert!(
            !plan.contains_key("other-file"),
            "{name}: -r {LEAF} must not plan outside its selection: {plan:?}"
        );

        // 2. Dry run twice, through both paths: same plan, no lock written.
        apply(&f.config, &f.state, Some(LEAF), true);
        apply(&f.config, &f.state, Some(LEAF), true);
        assert_eq!(plan, dry_run_plan(&f, LEAF), "{name}: a dry run changed the next plan");
        assert_eq!(before, lock_snapshot(&f.state), "{name}: a dry run wrote a lock");
        assert_eq!(runs(&f), 1, "{name}: a dry run executed the upgrade");

        // 3. The apply converges exactly what the dry run listed.
        let seen = events(&f.state).len();
        apply(&f.config, &f.state, Some(LEAF), false);
        let acted = converged_since(&f.state, seen);
        assert_eq!(
            acted, listed,
            "{name}: apply converged {acted:?} but the dry run showed {listed:?}"
        );

        // 4. Observable effect.
        let upgraded = want.contains("pkg-upgrade");
        assert_eq!(runs(&f), 1 + usize::from(upgraded), "{name}: upgrade run count");
        assert!(f.marker.exists(), "{name}: the upgrade's check still fails");
        for (path, body) in [
            (&f.deep, "deep declared"),
            (&f.base, "base declared"),
            (&f.leaf, "leaf declared"),
        ] {
            assert_eq!(std::fs::read_to_string(path).unwrap(), body, "{name}: {path:?}");
        }
        let other_want = if name.starts_with("(g)") { "tampered" } else { "other declared" };
        assert_eq!(
            std::fs::read_to_string(&f.other).unwrap(),
            other_want,
            "{name}: -r {LEAF} must not touch other-file"
        );
    }

    #[test]
    fn dry_run_shows_every_action_apply_takes() {
        // Every row runs, and the verdict names every row that failed: a
        // table that stops at its first red hides which other rows agree.
        let failed: Vec<&str> = rows()
            .iter()
            .filter(|row| std::panic::catch_unwind(|| check_row(row)).is_err())
            .map(|row| row.name)
            .collect();
        assert!(failed.is_empty(), "case-table rows failed: {failed:#?}");
    }
}

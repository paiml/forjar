//! A scoped dry run must list the dependency drift the apply would repair.
//!
//! Measured on forjar 1.33.0 against a fleet manifest: `apply -r <leaf>
//! --dry-run` printed every `depends_on` prerequisite of the leaf as
//! `= … no changes`, and `apply -r <leaf> --yes` on the same state dir then
//! reported `5 converged (2 repaired drift)` and ran a prerequisite's command
//! (a full package upgrade) as a drift repair. The preview an operator reads
//! before an apply did not show work the apply then did.
//!
//! Why: both runs execute the SAME pre-apply drift gate and observe the SAME
//! drift. The real apply turns each finding into `status: drifted` in the lock
//! and the planner turns that into `Update`. The dry run (correctly) does not
//! write the lock (forjar#310) — and then planned from the unmodified lock on
//! disk, so the findings it had just printed never reached its plan. The fix
//! routes both through `apply_drift::overlay_drift`, one function from finding
//! to planner input.

#[cfg(test)]
mod tests {
    use crate::cli::apply::cmd_apply;
    use crate::cli::apply_drift::{check_pre_apply_drift, GateScope};
    use crate::cli::apply_dry_run::scoped_dry_run_plan;
    use crate::cli::apply_selection::{resolve_selection, Selectors};
    use crate::core::types::PlanAction;
    use std::path::Path;

    fn write_config(config_path: &Path, base: &Path, leaf: &Path) {
        std::fs::write(
            config_path,
            format!(
                r#"
version: "1.0"
name: dry-run-dep-drift
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  base-file:
    type: file
    machine: local
    path: {}
    content: "base declared"
  leaf-file:
    type: file
    machine: local
    path: {}
    content: "leaf declared"
    depends_on: [base-file]
policy:
  tripwire: true
"#,
                base.display(),
                leaf.display()
            ),
        )
        .unwrap();
    }

    /// `apply -f <config> --state-dir <state> [-r <resource>] --yes`.
    fn apply(config: &Path, state: &Path, resource: Option<&str>) {
        cmd_apply(
            config, state, None, resource, None, None, false, false, false, &[], false, None,
            false, false, None, None, false, false, None, false, false, 0, true, false, None,
            false, None, None, None, false, None, false, None, false, None, &[],
        )
        .unwrap();
    }

    /// The dry run's plan for `apply -r <resource> --dry-run`, composed exactly
    /// as `cmd_apply_scoped` composes it: resolve the selection (which closes
    /// over `depends_on`), run the drift gate as a dry run, plan.
    fn dry_run_plan(config_path: &Path, state: &Path, resource: &str) -> Vec<(String, PlanAction)> {
        let mut config = crate::cli::helpers::parse_and_validate(config_path).unwrap();
        let sel = Selectors {
            resource: Some(resource),
            ..Default::default()
        };
        resolve_selection(&mut config, &sel, false).unwrap();
        let scope = GateScope::default();
        let observed = check_pre_apply_drift(&config, state, scope, false, true, false).unwrap();
        let plan = scoped_dry_run_plan(&config, state, &scope, &observed).unwrap();
        plan.changes
            .into_iter()
            .map(|c| (c.resource_id, c.action))
            .collect()
    }

    #[test]
    fn scoped_dry_run_lists_dependency_drift_that_apply_repairs() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        let base = dir.path().join("base.txt");
        let leaf = dir.path().join("leaf.txt");
        write_config(&config_path, &base, &leaf);

        // Converge both, so the lock is real and says Converged for both.
        apply(&config_path, &state, None);
        assert_eq!(std::fs::read_to_string(&base).unwrap(), "base declared");

        // Drift the DEPENDENCY, not the selected resource.
        std::fs::write(&base, "tampered").unwrap();

        let changes = dry_run_plan(&config_path, &state, "leaf-file");
        let base_action = changes
            .iter()
            .find(|(id, _)| id == "base-file")
            .map(|(_, a)| a.clone());
        assert_eq!(
            base_action,
            Some(PlanAction::Update),
            "apply -r leaf-file --dry-run must show the drifted dependency as a change; got {changes:?}"
        );
        // The dry run must not have written the lock (forjar#310): a second
        // dry run sees the same drift, and the file is untouched.
        assert_eq!(std::fs::read_to_string(&base).unwrap(), "tampered");
        let again = dry_run_plan(&config_path, &state, "leaf-file");
        assert_eq!(changes, again, "a dry run changed what the next dry run plans");

        // And the dry run described what the apply then does: the real
        // `apply -r leaf-file` repairs the dependency.
        apply(&config_path, &state, Some("leaf-file"));
        assert_eq!(std::fs::read_to_string(&base).unwrap(), "base declared");
    }

    #[test]
    fn scoped_dry_run_without_drift_stays_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        let state = dir.path().join("state");
        let base = dir.path().join("base.txt");
        let leaf = dir.path().join("leaf.txt");
        write_config(&config_path, &base, &leaf);
        apply(&config_path, &state, None);

        let changes = dry_run_plan(&config_path, &state, "leaf-file");
        assert!(
            changes.iter().all(|(_, a)| *a == PlanAction::NoOp),
            "no drift, so the dry run must plan nothing: {changes:?}"
        );
    }
}

//! Tests for the apply dispatcher: the `--dry-run` family (GH-208) and the
//! flags `--plan-file` used to drop (Refs #358).
//!
//! Extracted from `dispatch_apply_b.rs`, which reached the repo's 500-line
//! ceiling once the plan-file wiring landed beside it.

use super::*;

mod tests_gh208_dry_run_family {
    use super::*;

    // GH-208: --dry-run-shell/-json/-summary/-diff performed a REAL apply on the
    // published 1.12.3 binary — files created, state written — because only
    // `args.dry_run` reached the execute guard. Asking for a preview and getting
    // a mutation is the most dangerous shape a flag can have.

    fn args_with(f: impl FnOnce(&mut ApplyArgs)) -> ApplyArgs {
        let mut a = ApplyArgs::default();
        f(&mut a);
        a
    }

    #[test]
    fn every_dry_run_flag_suppresses_execution() {
        // Asserted one flag at a time: a table of fn pointers trips the
        // very-complex-type lint and reads no better.
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run = true)),
            "--dry-run"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_shell = true)),
            "--dry-run-shell must suppress execution: a flag named dry-run must never mutate"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_json = true)),
            "--dry-run-json must suppress execution"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_summary = true)),
            "--dry-run-summary must suppress execution"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_diff = true)),
            "--dry-run-diff must suppress execution"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_cost = true)),
            "--dry-run-cost"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_graph = true)),
            "--dry-run-graph"
        );
        assert!(
            effective_dry_run(&args_with(|a| a.dry_run_verbose = true)),
            "--dry-run-verbose"
        );
    }

    #[test]
    fn a_plain_apply_is_not_dry_run() {
        // The guard against "fixed" meaning "never applies anything".
        assert!(!effective_dry_run(&ApplyArgs::default()));
    }
}

mod tests_358_plan_file_flags {
    use super::*;

    // Refs #358: `apply --plan-file` built an `ApplyConfig` in which one field
    // came from the invocation and fifteen were hard-coded, so every knob and
    // every selector but `-m` parsed, did nothing, and exited 0.

    /// Every field of [`ApplyKnobs`] must come from ITS OWN flag.
    ///
    /// The defect was fifteen `ApplyConfig` fields hard-coded off, so what has
    /// to be guarded is not "does an apply succeed" — it did, at exit 0, with
    /// every flag ignored — but "is this field wired to that flag". Each knob is
    /// set to a value distinct from the default AND from every other knob's, so
    /// a copy-paste that reads the wrong arg fails here rather than in a
    /// behavioural test nobody wrote.
    #[test]
    fn every_knob_is_read_from_its_own_flag() {
        let args = ApplyArgs {
            force_unlock: true,
            progress: true,
            timeout: Some(11),
            retry: 7,
            parallel: true,
            max_parallel: Some(13),
            resource_timeout: Some(17),
            rollback_on_failure: true,
            ..Default::default()
        };
        let k = knobs_from(&args);
        assert!(k.force_unlock, "--force-unlock");
        assert!(k.progress, "--progress");
        assert_eq!(k.timeout_secs, Some(11), "--timeout");
        assert_eq!(k.retry, 7, "--retry");
        assert!(k.parallel, "--parallel");
        assert_eq!(k.max_parallel, Some(13), "--max-parallel");
        assert_eq!(k.resource_timeout, Some(17), "--resource-timeout");
        assert!(k.rollback_on_failure, "--rollback-on-failure");
    }

    /// The other half of the guard: a bare apply must not silently ARM
    /// anything. A wiring bug that hard-codes `true` is as wrong as one that
    /// hard-codes `false`.
    #[test]
    fn a_bare_apply_arms_no_knob() {
        let k = knobs_from(&ApplyArgs::default());
        assert!(!k.force_unlock && !k.progress && !k.parallel && !k.rollback_on_failure);
        assert_eq!(k.retry, 0);
        assert_eq!(
            (k.timeout_secs, k.max_parallel, k.resource_timeout),
            (None, None, None)
        );
    }

    /// The `ApplyConfig` the plan path builds may hard-code a field ONLY when
    /// that field is refused earlier or decided earlier. Four are: `--force`,
    /// `--refresh` and `--force-tag` are rejected by
    /// `reject_replanning_flags`, and `dry_run` has already returned through
    /// the preview.
    ///
    /// This reads the source because that is where the defect lived: every
    /// other field looked fine at a glance and was `false`.
    #[test]
    fn the_plan_paths_apply_config_hard_codes_only_what_is_refused() {
        let src = include_str!("apply_from_plan.rs");
        let literal = src
            .split_once("let cfg = executor::ApplyConfig {")
            .expect("the ApplyConfig literal")
            .1
            .split_once("\n    };")
            .expect("the end of the literal")
            .0;
        let allowed = ["force", "refresh", "force_tag", "dry_run"];
        for line in literal.lines() {
            let line = line.trim();
            let Some((field, value)) = line.split_once(": ") else {
                continue;
            };
            let value = value.trim_end_matches(',');
            if !matches!(value, "false" | "true" | "None" | "0") {
                continue;
            }
            assert!(
                allowed.contains(&field),
                "`{field}: {value}` is hard-coded in the plan path's ApplyConfig. \
                 Refs #358: a flag that is neither fed from the request nor refused \
                 by name is a flag the operator passes and forjar ignores."
            );
        }
    }
}

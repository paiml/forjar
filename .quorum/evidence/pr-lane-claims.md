# Quorum evidence — PMAT-237 — the claims as put to the lanes

# PMAT-237 — claims for the quorum lanes

Branch PMAT-237-pr-lane-pareto, one commit (479d8edd) on main (0c1277c6).
Judge the diff `main...479d8edd`. This changes what CI runs, so judge it as
a change to a gate, not to a script.

1. The measurement is real: of the ten most recently merged PRs, eight
   touched no `src/` at all (`gh pr view <n> --json files`), and one PR's
   jobs cost about 131 job-minutes — ledger-replay 34, dogfood 26, bench
   18, the workspace suite 14, coverage 12, ci/lint 7, ci/coverage 7,
   examples-validate 6.
2. `scripts/ci/changed-class.sh` is an ALLOW-LIST of the harmless: only
   `docs/*`, `.quorum/*` and `CHANGELOG.md` are harmless, everything else
   is code, and an unreadable or empty file list is code.
3. Three paths under those prefixes are explicitly NOT harmless and the
   script says why: `README.md` (gate D runs its fenced blocks),
   `docs/audits/surface_audit.csv` (gate C diffs the live surface against
   it), `contracts/**` (gate G validates it).
4. Nine cases in
   tests/falsification_pr_lane_runs_what_the_change_can_break.rs drive the
   script with fabricated file lists in both directions, including the
   three above, an unclassified path, and an empty list.
5. The registered mutation — the unclassified arm made harmless — turns
   three of those cases red, and two wiring mutations (a heavy job
   ungated, the gate's refusal text removed) each turn their rule red.
6. Five heavy jobs in ci.yml plus `ledger-replay`, `coverage` and
   `benchmark` carry `if: needs.classify.outputs.code == 'true'`, and a
   rule asserts it for each by reading the workflow YAML.
7. The saving cannot become a hole: ci.yml's `gate` and proofs.yml's
   `proofs` aggregator each re-read the class and FAIL when a heavy job
   was skipped on a change the classifier called code. Neither trusts the
   skip.
8. The class is computed by one composite action that calls the one
   script, and a rule asserts every gated workflow uses that action rather
   than its own copy.
9. `make dogfood-release` and `scripts/dogfood/*` are untouched: the
   release gate still runs everything, so this redistributes cost and
   lowers no floor.
10. All four edited workflows parse as YAML and actionlint reports the same
    22 findings as main.
11. This very PR touches `.github/**`, `scripts/**` and `tests/**`, so the
    classifier calls it code and every heavy job runs on it — the change
    does not skip its own review.

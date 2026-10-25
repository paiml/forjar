# Quorum evidence — #791 round 2 — lane summaries

Author model on the commits: Claude Sonnet 5.5 (Co-Authored-By trailer). No lane used it. All lanes and judges ran claude-opus-5-5 through Claude Code with read-only tools; the lanes could not run cargo and read the code, the judges 2 and 3 ran the falsification target.

## claim lanes (three roles over the same 409-line diff, PR base epic/forjar-img-001)
- lane 1, claude-opus-5-5, code and refactor: PASS. C1-C3 confirmed, C4 and C5 refuted. The helper split at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:149 and :175 keeps behaviour. Finding: stale PMAT-756 doc comment at main.rs:132-141 above the new EMPTY paragraph.
- lane 2, claude-opus-5-5, workflow side: PASS. Enumerated every runs-on in .github/workflows itself and found no declared self-hosted job without an arch label. Finding: runner_labels flattens matrix legs (scan.rs:127), pre-existing.
- lane 3, claude-opus-5-5, gate weakening: PASS. No assert removed or loosened; the guard is stricter because ARCH_NEUTRAL is empty. Finding: no fixture control drives arch_scan or stale_arch_neutral.

## falsification probe
Judge 2 and judge 3 each ran cargo test --test falsification_every_ci_job_runs_on_the_fleet on the PR head: 13 passed, 0 failed. The revert probe of round 1 (workflows restored to 2cf561b0 gives 12 passed, 1 failed) is unchanged because no workflow or test file changed since.

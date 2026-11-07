# Quorum evidence — #791 (fix/restore-x64-runs-on) — lane summaries

Author model on the commits: Claude Sonnet 5.5 (Co-Authored-By trailer). No lane used it. All lanes ran claude-opus-5-5 through Claude Code (read-only tools), agy being out of scope for this round.

## claim lanes (three distinct review roles, each over the same 409-line diff)
- lane 1, claude-opus-5-5, code and refactor: PASS. The helper split in tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:149 and :175 changes no behaviour; both assert! messages at :204 and :215 are unchanged.
- lane 2, claude-opus-5-5, workflow side: PASS. Every changed runs-on line keeps self-hosted and clean-room and adds X64 (18 runs-on lines plus 2 lint.yml runner matrix lines).
- lane 3, claude-opus-5-5, gate weakening: PASS. ARCH_NEUTRAL at main.rs:145 is empty, so the guard admits nothing; the check got stricter, not weaker.

## refuters and judges
Three Opus agents refuted each of five claims and three judged. C4 (planted) and C5 were refuted 3 of 3.

## falsification probe (measured by the author on a build host)
Green on the PR head: cargo test --test falsification_every_ci_job_runs_on_the_fleet gives 13 passed, 0 failed.
With .github/workflows reverted to the epic base (2cf561b0): 12 passed, 1 failed (a_fleet_job_names_the_architecture_it_needs). Restored: 13 passed.

## sandbox notes
Opus subagents could not run cargo (sandbox refused the .cargo-lock write) and read the code instead; the author ran the probe.

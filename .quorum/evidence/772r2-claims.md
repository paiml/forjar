# Quorum evidence — PMAT-770 / r2 base question (ci/img001-chip-independent-placement-r2) — adjudicated claims

Round design: 3 lanes (agy gemini-4-argon-high, agy gemini-3.8-flash-high, claude-haiku-4-5) judged 7 claims against an inline git evidence bundle (no tools). Authors are Sonnet 5.5 and Opus 5.5, so no lane used either id. Two planted controls (C6, C7) were injected and killed 3 of 3. The branch touches no Rust file, so citations point into the base tree.

## CONFIRMED — 4 claims survived refutation

1. [design] (C1) PR 771 is not on origin/main: it merged into epic/forjar-img-001 as fd70436f, which is not an ancestor of origin/main 1a2fef64, and the epic PR 768 (base main) is still OPEN with no merge commit.
   - evidence: gh pr view 771 gives baseRefName epic/forjar-img-001; git merge-base --is-ancestor returned no; the guard 771 changed is tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:171 in the base tree.

2. [design] (C2) The epic's workflow edits make r2's label edits mostly redundant: against origin/epic/forjar-img-001 the r2 workflow diff shrinks from 21 sites in 10 files to 3 files (ci.yml, nightly.yml, proofs.yml), 3 lines each way.
   - evidence: git diff origin/epic/forjar-img-001 HEAD --stat -- .github read by all three lanes; the architecture guard those labels answer to is tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:171 in the base tree.

3. [design] (C3) Option (c), porting the ARCH_NEUTRAL list onto main inside r2, edits the same test file the epic already changed (tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:171 region), so it conflicts when PR 768 merges into main.
   - evidence: git diff origin/main origin/epic/forjar-img-001 -- tests/falsification_every_ci_job_runs_on_the_fleet/main.rs shows the ARCH_NEUTRAL addition; r2 would add the same hunk against the base copy at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:171.

4. [design] (C4) Option (a), rebase r2 onto the epic and retarget the PR to it, is the best base: it inherits the merged ARCH_NEUTRAL guard, does not block on 768 (option b) and does not duplicate 771 (option c); the remaining work is the runner.arch cache and matrix-name fixes and two guard holes.
   - evidence: unanimous pick (a) in all three lane verdicts; the guard to extend is tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:171, present only on the epic.

## REFUTED — 3 claims killed

1. [probe] refuted 3/3 — (C5) "The current r2 branch can satisfy quorum-gate.sh as kind:code without touching a test file."
   - corrected: it cannot. The gate requires falsification.test_file to be touched by the branch, and r2 touches ten workflow files and the roadmap only; the guard test at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:171 is untouched. A kind:triage receipt is refused for workflow edits.

2. [probe] refuted 3/3 — (C6, PLANTED CONTROL) "r2 adds an ARCH_NEUTRAL allow-list to the fleet test."
   - corrected: false by construction. r2 touches no file under tests/ and ARCH_NEUTRAL is absent from the base copy of tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:171; the allow-list is the content of PR 771 on the epic.

3. [probe] refuted 3/3 — (C7, PLANTED CONTROL) "r2 removes clean-room from the runs-on labels as well as X64."
   - corrected: false. Every hunk turns [self-hosted, clean-room, X64] into [self-hosted, clean-room]; clean-room stays. The label set is the one the guard at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:171 inspects.

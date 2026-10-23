# Quorum evidence - ci/img001-chip-independent-placement-r3 (#772) - adjudicated claims

Canonical format: `N. [lane] claim` with an indented `- evidence:` or `- corrected:` subline.
Measured against the merge-base with origin/main (the gate's fixed base); the lanes reviewed the r3 delta against origin/epic/forjar-img-001.

## CONFIRMED - 6 claims survived refutation

1. [argon,flash,sonnet5] Hole 1 is closed: the arch rule is judged per matrix leg, so one leg naming X64 can no longer vouch for a sibling leg that names nothing.
   - evidence: `unpinned_in` iterates `runner_legs(job)` and flags each leg that is self-hosted without an arch label, at tests/falsification_every_ci_job_runs_on_the_fleet/arch.rs:33. Mutating it to flatten the legs turned `a_matrix_with_one_pinned_leg_and_one_unpinned_leg_fails` (arch.rs:76) RED when run by the gate author.

2. [argon,flash,sonnet5] Hole 2 is closed: an ARCH_NEUTRAL job moved to a hosted runner is now reported stale instead of passing because it carries no arch label.
   - evidence: `stale_in` flags a listed job when no leg is on the fleet, tests/falsification_every_ci_job_runs_on_the_fleet/arch.rs:57 onward. Deleting that branch turned `a_neutral_job_moved_to_a_hosted_runner_is_stale` (arch.rs:117) RED; both mutations restored afterwards and 17 of 17 green.

3. [argon,flash,sonnet5] The ci.yml dogfood-surface and proofs.yml ledger-replay edits are load-bearing, not cosmetic: the real-workflow guard needs them.
   - evidence: reverting only those two workflow files to origin/epic/forjar-img-001 made `a_fleet_job_names_the_architecture_it_needs` FAILED (16 passed, 1 failed). The test feeds real workflows through `arch::unpinned_in` at tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:252 and the ARCH_NEUTRAL list at main.rs:143.

4. [argon,flash,sonnet5] Required check names are unchanged: the four matrix.check strings in lint.yml equal the clippy check-runs main reports today.
   - evidence: measured with gh api on the check-runs of origin/main, which listed exactly clippy (self-hosted, clean-room, X64), its encryption variant, clippy (macos-latest) and clippy (macos-latest, encryption). The unpinned-matrix names are hard-coded through name: matrix.check, so dropping X64 from runs-on cannot rename them again.

5. [sonnet5,argon] runner.arch was added to every cache key in lint, msrv, bench, stress and coverage, and to coverage's restore-keys, which is the only restore-keys in those five files.
   - evidence: grep of actions/cache in the five workflows shows one cache step per file and a restore-keys block only in coverage.yml, which the diff updates in lockstep with its key. The ARCH_NEUTRAL consumer of those keys is tests/falsification_every_ci_job_runs_on_the_fleet/main.rs:143.

6. [argon,flash,opus5] Latent limitation, accepted as a follow-up: runner_legs emits one leg per matrix key rather than the product, so a runs-on built from two matrix expressions would be mis-judged.
   - evidence: the loop at tests/falsification_every_ci_job_runs_on_the_fleet/scan.rs:228 builds legs per key. No workflow in the tree uses two matrix expressions in one runs-on (release.yml, nightly.yml and lint.yml use the single matrix.runner key), so no current job is mis-judged; severity low, ticket not required for merge.

## REFUTED - 3 claims killed

1. [opus5] High: the features-empty clippy legs would still be renamed by GitHub with an empty element, so required checks hang and #772 recurs.
   - corrected: false by measurement. The check-runs of origin/main list the four clippy names exactly as the matrix.check strings spell them, so the pinned names are transcribed from reality rather than reconstructed. See tests/falsification_every_ci_job_runs_on_the_fleet/arch.rs:117 for the guard that stays separate from naming.

2. [opus5] Medium: restore-keys is unchanged in four of the five cache files, so a cross-arch target restore could still happen.
   - corrected: false. Only coverage.yml has a restore-keys block, and it was changed; bench, msrv, stress and lint each have a single key line that gained runner.arch. A restore cannot match an arch-less key that no step writes. Related code is tests/falsification_every_ci_job_runs_on_the_fleet/scan.rs:202.

3. [opus5] Medium: runner_legs fails open when runs-on cannot be resolved, hiding an unpinned fleet job.
   - corrected: not true of this tree. The only templated runs-on values are matrix.runner in lint.yml, release.yml and nightly.yml, each backed by a matrix that defines runner, so none resolves to an empty leg today. Recorded as hardening, not a defect of this change; see tests/falsification_every_ci_job_runs_on_the_fleet/scan.rs:228.

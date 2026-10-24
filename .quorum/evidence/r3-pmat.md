# pmat lane - r3

analyze_vacuous_tests ran over the worktree. The touched paths are under tests/falsification_every_ci_job_runs_on_the_fleet and none appears in the vacuous list; the repo-wide list is the existing crates/forjar-contracts kernel tests, none touched here.

Falsification, run by the gate author on this tree: reverting ci.yml and proofs.yml to the epic made 1 of 17 FAILED; flattening unpinned_in and removing the hosted-runner branch of stale_in made 2 of 17 FAILED; with everything restored 17 of 17 pass.

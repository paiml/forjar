# pmat lane — #768 / PR 771

`pmat analyze vacuous-tests -p tests -f json` over the repository tests directory: 380 files parsed, 4944 tests examined,
84 cannot fail, 0 of them in tests/falsification_every_ci_job_runs_on_the_fleet. The new falsifier asserts on parsed
workflow files and on a table it compares them to, and it was shown red against the epic base and green against the PR head.

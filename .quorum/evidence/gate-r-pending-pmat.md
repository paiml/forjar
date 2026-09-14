# PMAT-234 — pmat measurements

## Vacuity

`pmat analyze vacuous-tests` over the whole tree: **400 of 19650 `#[test]` fns
cannot fail (2.0%) across 2164 parsed files; 3 more skip silently when a fixture
is missing.** None of the four cases in
`tests/falsification_release_check_says_what_is_pending.rs` appears in that
list, and neither does anything in
`tests/falsification_dogfood_release_check_pr_window.rs` or
`tests/release_check_fixture/mod.rs`.

The scan runs over the tree rather than over one path because a path argument
makes it refuse: it enumerates tracked files to get a denominator, and a single
file has none.

## bashrs

`bashrs lint scripts/dogfood/release-check.sh`: **0 errors**. The warnings are
the standing set this repository carries, none introduced here.

## The gate against the real repository

`bash scripts/dogfood/release-check.sh` exits 0 both before and after — the
defect was never the exit code, it was the sentence. What changed is what the
sentence says, and the before and after are both in
`docs/audits/logs/PMAT-234-pretag.log`, measured against the live repository
rather than only against a fixture.

## Quality gates at commit

Pre-commit format, complexity, clippy and SATD passed on both commits of this
branch. TDG baseline: 825 files analysed, average 93.7, 442 A+ and 276 A.

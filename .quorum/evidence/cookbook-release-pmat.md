# PMAT-241 — pmat measurements

## Vacuity

`pmat analyze vacuous-tests` over the whole tree: **400 of 19650 `#[test]` fns
cannot fail (2.0%) across 2164 parsed files; 3 more skip silently when a fixture
is missing.**

None of the eight cases in
`tests/falsification_release_cookbook_is_part_of_the_release.rs` appears in that
list, and neither does anything in
`tests/falsification_release_goals_are_measured.rs` or
`tests/release_goal_fixture/mod.rs`. The scan is run over the tree rather than
over one path because a path argument makes it refuse: it enumerates tracked
files to get a denominator, and a single file has none.

## Quality gates at commit

The pre-commit gates ran on both commits of this branch and passed: format,
complexity, clippy, SATD (2 comments, both pre-existing), with the documentation
check warning about a path this repository does not use. TDG baseline after the
change: 825 files analysed, average score 93.7, 442 A+ and 276 A.

## bashrs

`bashrs lint scripts/dogfood/tagged.sh` and
`bashrs lint scripts/dogfood/lib/releases.sh`: **0 errors** each. The warnings
are the standing set this repository carries (PERF002 on command substitution
in loops, REL003 on `read` without a timeout, SC2161 on the header `cd`), none
of them introduced here. An earlier revision did introduce one error — SC2168,
`local` outside a function, from a `local` declared mid-body — and it was fixed
by hoisting the declaration rather than suppressed.

## The gate against the real repository

`bash scripts/dogfood/tagged.sh` exits 0 on this branch: six tagged releases
since v1.25.0 reconcile with git and GitHub, 26 tickets carry their tag and say
they shipped, 11 tickets from 8 PRs merged since v1.28.0 carry
`release:v1.29.0`, and the next cut is due 2026-09-12T16:07:14Z with 27h left.
The cookbook arm does not fire, because no tag at or above `cookbook_floor`
exists yet — which is the floor doing what it says it does.

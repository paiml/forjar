# PMAT-534 — instruments, and what each said

    cargo test --no-fail-fast   4 suites at HEAD: 6 + 4 + 9 + 4 = 23 passed, 0 failed
    cargo clippy --tests        clean; the shared fixture is `#![allow(unused)]`
                                because each consumer uses a different subset
    cargo fmt --all             clean
    bash -n                     scripts/dogfood/release-check.sh parses
    shellcheck -x               scripts/dogfood/release-check.sh clean
    bashrs lint                 0 errors at HEAD and 0 on origin/main's copy;
                                38 warnings / 94 infos against 35 / 86
    yaml.safe_load              .github/workflows/release.yml and
                                docs/roadmaps/roadmap.yaml both parse
    release-check.sh            GATE R PASS on this repository, exit 0
    quorum-gate.sh PRINT_HASH   recorded in this round's receipt as diff_sha256;
                                the value cannot be quoted here, because this
                                file is one of the inputs it hashes

Three of bashrs's new findings are worth naming rather than counting. Two sit on
line 193, which is byte-identical to the line this branch shipped before the
review round: SC1079 and SC2086 on the nested quoting of
`latest_rel="$("$GH" api "repos/${REPO}/…" …)"`. shellcheck, which owns those
codes, reports nothing on the file. The third, BRS0023 "use read -r", fires on
the words "could not be read" inside a message string at line 215; there is no
`read` builtin on that line.

VACUOUS SCAN. `pmat analyze vacuous-tests` reports 432 of 19716 `#[test]` fns
cannot fail (2.2%) across 2178 parsed files, plus 3 that skip silently when a
fixture is missing. None of them is in a path this branch touches: neither
`falsification_release_check_latest_points_at_the_release` nor
`release_check_fixture` appears anywhere in its output.
That is necessary and not sufficient, and this ticket is the reason to say so
plainly: before the round, `a_pointer_that_answers_with_nothing_is_unmeasured_and_red`
did not exist and the guard it defends could be deleted with every case staying
green. A scan that asks whether a test CAN fail cannot see a branch that no test
reaches. The kill matrix in log section 8 is what settles it.

TOOL DEFECTS FOUND. The paiml-implement delegate hit its 30-turn cap before
writing a receipt, for the twelfth time this session; filed as
paiml-implement#141. An earlier dispatch of this same round died on an API
session limit with an empty out_dir, which is the failure mode `lane-reduce`'s
`--not-before` and width refusal exist to keep from turning into a false PASS.

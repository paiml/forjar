# PMAT-535 — the claims put to the round

The branch adds one arm to `scripts/quorum-gate.sh`: if the branch name contains
a `PMAT-<n>`, some commit being pushed must carry it on a `Pmat-Ticket:` line.

1. The rule is the right rule. It refuses the shape PR #532 was in, and it
   refuses nothing a contributor legitimately does — a release-cut branch
   carrying several tickets, a branch with no ticket id, a rebase, a force-push,
   a first push with no remote copy of the branch.
2. Reading the trailer with `sed` rather than `%(trailers:key=…)` is correct,
   and the stated reason for it is true.
3. The six falsification cases each fail if the arm is deleted.
4. The receipt and the log describe what the commands actually produced, and
   the PR #532 story in them is true of the real repository.
5. The bashrs section is honest: four new warnings, all artifacts of one
   multi-line `die` message.
6. The arm is placed correctly — after `PRINT_HASH`, before `receipt=`.

Each lane was given a different brief (the rule; the trailer reading and the
tests; the receipt and the log as documents) and all three ended with the
standing instruction: *quote any sentence a reader could check and find false,
say what is actually true, and cite where you measured it.*

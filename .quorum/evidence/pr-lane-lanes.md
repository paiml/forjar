# PMAT-237 — quorum lanes (agy, --sandbox, writes=false)

Three lanes dispatched on 479d8edd with the base pinned at 0c1277c6. **One returned.** The round is recorded as it happened rather than as three lanes wide.

## What happened to the other two

One lane created files inside the operator's working tree and COMMITTED them to the working branch while probing how the classifier handles a path with a space and a non-ASCII byte: commit `16c2af56 quote`, adding `docs/ä.md`, and later `docs/space file.md` and `docs/ünicode.md` staged plus an untracked `test`. The brief said not to edit the repository; the lane did anyway. Everything was reset and removed and the branch verified back at its own commit, and the remaining lanes were stopped rather than allowed to continue writing.

The round was not re-run. A second round carried the same risk, the findings that mattered were already in hand and fixed, and the evidence this change rests on is its own thirteen cases and three mutations. Those paths never needed files: the classifier reads a list on stdin, and a future brief says so.

## lane 2 — verdict FAIL (conv-49305086, 1 turn(s), 0s)

C1 CONFIRMED, C2 CONFIRMED, C3 REFUTED, C4 REFUTED, C5 CONFIRMED, C6 CONFIRMED, C7 REFUTED, C8 CONFIRMED, C9 CONFIRMED, C10 CONFIRMED, C11 CONFIRMED
C3 is refuted because README.md and contracts/** are not under the harmless prefixes (docs/*, .quorum/*, CHANGELOG.md). C4 is refuted because only 6 cases drive the script, while 3 test the YAML wiring. C7 is refuted because if the classify job fails or is cancelled, $CODE is empty, which circumvents the safety checks (evaluates `[ "$CODE" != "true" ]` as true) and allows skipped jobs to pass the gate silently.
CAN A BREAKING CHANGE BE CLASSIFIED HARMLESS? Yes: a source file renamed to docs/ is only reported by its new path in `git diff --name-only`, yielding code=false and blinding all heavy gates, which is the worst path found. A malicious PR can also modify `changed-class.sh` to echo code=false, hijacking the classifier.

# PMAT-236 + PMAT-238 + PMAT-239 — quorum lanes (agy, --sandbox, writes=false)

Three lanes at 7faabf52 with `origin/main` pinned to cc339449. **All three returned, and the repository was intact afterwards** — the previous round's brief put the no-write rule at the end and a lane created branches in the working tree and left HEAD on one; this brief opened with that incident and the rule, and nothing was touched.

Each lane was told to attack in a fixed order: the ref the statuses are read at, then the SIGPIPE class across the whole of scripts/ rather than the one instance, then whether ten runs can distinguish the fix from luck, then whether a ticket can reach the window without passing the new arm.

### lane 1 — verdict FAIL (conv-660c1af8, 1 turn(s), 0s)

Claims 1, 2, 3, 5, 7, 8, 9, 10, and 11 are CONFIRMED. Claims 4 and 6 are REFUTED. The claim that the tests are RED on main and GREEN at HEAD (Claim 4) is backwards. The claim that no pipeline can return 141 under pipefail (Claim 6) is false, as 'git tag | head -1' and 13 other leaky pipelines were found to be present and capable of causing a SIGPIPE failure under pipefail.

### lane 2 — verdict FAIL (conv-3a06ac22, 1 turn(s), 0s)

The PR correctly implements the status check for PMAT-236, ensuring that shipped tickets report as completed. However, it fails on Claim 4 (the logic was stated backwards) and Claim 6. In fixing the `pipefail` SIGPIPE issue for `grep -q` (PMAT-239), the author introduced a new pipeline `grep -E ... | head -1` in `window.sh` line 301, which is susceptible to the exact same SIGPIPE (exit 141) failure if grep continues writing after head exits. The branch should fail and be sent back for correction.

### lane 3 — verdict FAIL (conv-919980f0, 1 turn(s), 0s)

Claims 1, 2, 3, 4, 5, 8, 9, 10, and 11 are CONFIRMED based on direct execution, diff analysis, and bash translation of the fixtures. However, Claims 6 and 7 are REFUTED. Claim 6 falsely claims no pipeline can return 141 under pipefail, but `git tag ... | head -1` can still return 141, and numerous `printf | grep -q` pipelines remain across `scripts/dogfood/`. Claim 7 relies on a 10-run loop which has a 1.7% chance to pass by luck (failing to be deterministic), missing the cheaper check of simply overflowing the pipe buffer with a massive fixture.


## reduction

3 FAIL. Nine of eleven claims confirmed by at least two lanes. Three refutations: the SIGPIPE class (3/3, right, and it found three more sites plus one this branch had just added), the ten-run loop (1/3, right, and it produced the deterministic rule that replaced it), and the red/green measurement (2/3, wrong — re-run by the orchestrator and recorded as a lane error). No second round: every finding was acted on and measured, and the two that reproduce are fixed in the tree.

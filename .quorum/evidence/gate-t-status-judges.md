# Quorum evidence — PMAT-236 + PMAT-238 + PMAT-239 — adjudicated claims

One round of three sandboxed lanes, base pinned at cc339449, the diff at 7faabf52: 3/3 FAIL. Nine of eleven claims confirmed by at least two lanes; three refutations, two of which reproduce and are fixed, one of which does not and is recorded as a lane error. **All three lanes returned and the repository was intact afterwards**, which the previous round's was not — that brief put the no-write rule at the end, this one opened with the incident.

Citations resolve at the merge base: `tests/falsification_release_goals_are_measured.rs:171` is the end of the suite as it stood, where every case this branch adds was appended.

## CONFIRMED

1. [the-arm] A TICKET A RELEASE NAMES MUST SAY IT SHIPPED — the new arm refuses when a ticket named by a tagged ledger row, or by a PR merged since the newest tag, has a status other than `completed` or `cancelled`, and refuses as UNMEASURED when the row has no status at all.
- evidence: all three lanes read the arm and ran the gate. The two cases that drive it were appended after `tests/falsification_release_goals_are_measured.rs:171` and are red against main's scripts.

2. [names-it] THE FAILURE NAMES THE TICKET, WHERE IT IS NAMED, ITS STATUS AND THE WAY OUT — including both `pmat work edit` calls, because the tool refuses `planned -> completed` directly.
- evidence: 3/3 from the failure text; the case at `tests/falsification_release_goals_are_measured.rs:171`'s file asserts the ticket id, the status and `pmat work edit` all appear.

3. [one-ref] THE STATUSES COME FROM THE SAME SINGLE READ AT THE SAME REF AS THE LABELS — `dogfood_roadmap_rows` reads `docs/roadmaps/roadmap.yaml` once at `${DOGFOOD_ROADMAP_REF:-HEAD}` and fills ids, labels and statuses together.
- evidence: 3/3, each lane tracing the loader. This was the load-bearing question of the brief: an arm that read the worktree while the rest read HEAD would judge two different trees. The suite that drives the gate over a fixture repository is `tests/falsification_release_goals_are_measured.rs:1`.

4. [wording] THE VERDICT LINE COUNTS TICKETS AS TICKETS — `N ticket(s) from M PR(s) merged since <tag> carry release:<next>`, and the two fixture assertions that quoted the old wording are updated.
- evidence: 3/3; the live gate now prints `8 ticket(s) from 7 PR(s)`, which is what PR #515 carrying two tickets actually means. The assertion that quoted the old wording is `tests/falsification_release_goals_are_measured.rs:23`.

5. [prefix] THE `planned:` PREFIX DOES NOT LEAK — it selects a row's status in the fixture and never reaches a label, a ledger row or an id the gate looks up.
- evidence: 3/3 reading `roadmap()` in the shared fixture module that `tests/falsification_release_goals_are_measured.rs:9` imports.

6. [drift-is-real] THE DRIFT RECURRED WITHIN A DAY — PMAT-232, PMAT-235 and PMAT-237 were merged and still read `planned` one day after PMAT-235 backfilled sixteen others. They are `completed` in this diff.
- evidence: 3/3 against the roadmap and the merged PRs; the arm fired on PMAT-232 the moment it existed, and the case that reproduces it sits after `tests/falsification_release_goals_are_measured.rs:171`.

7. [scope] NOTHING UNDER src/ CHANGES, SO THE BRANCH IS AS WIDE AS IT CLAIMS — two shell files, two test files, one shared fixture, the roadmap and two audit logs. Gate F's mutation arm therefore has nothing to mutate here and measures zero rather than being skipped, and the whole of the change is readable in one sitting.
- evidence: 3/3 from `git diff --name-only main...HEAD`; the test surface is `tests/falsification_release_goals_are_measured.rs:171`'s file and the cut-flow suite beside it.

## REFUTED

8. [sigpipe-class] THE CLAIM THAT NO PIPELINE HERE CAN RETURN 141 — refuted by all three lanes, and they were right three times over: `git tag … | head -1` in `dogfood_prev_tag`, and `git tag … | grep -v … | head -1` in both `tagged.sh` and `release-goal.sh`.
- evidence: each lane grepped the whole of `scripts/` rather than the one function the ticket named, which is what the brief asked for and what the ticket had not done. The rule that now refuses the pattern was appended after `tests/falsification_release_goals_are_measured.rs:171`.
- corrected: all three are one capture plus one `awk` or a parameter expansion. **The same grep found a nineteenth site that this branch had just introduced** — `grep … | head -1 | cut` in the new status reader, one line from the defect being fixed — now one `awk`. Eighteen further sites elsewhere are filed as PMAT-240 with the census committed.

9. [ten-runs] THE TEN-RUN LOOP AS EVIDENCE — refuted by one lane: at one flake in three, ten passes have a 1.7% chance of being luck, and a cheaper deterministic check exists.
- evidence: the arithmetic, and the lane's own suggestion of overflowing the pipe buffer.
- corrected: `the_release_goal_scripts_carry_no_pipeline_that_can_take_sigpipe` reads the three files this branch owns and refuses the PATTERN, ignoring comments. It is red when one leaky pipeline is appended to any of them, measured. The loop is kept as a weaker companion and the receipt no longer leans on it. The lane's pipe-buffer idea would test one instance; the rule tests the class, which is the better answer to the same objection.

10. [red-green] THE RED/GREEN MEASUREMENT, REPORTED "BACKWARDS" BY TWO LANES — it does not reproduce.
- evidence: re-run by the orchestrator with main's `tagged.sh` and `lib/window.sh` checked out over this branch's tests: `a_shipped_ticket_whose_row_says_planned_is_named_and_red`, `a_merged_ticket_whose_row_says_planned_is_named_and_red` and the pattern rule all FAIL (14 passed, 4 failed), and all pass at HEAD (18 passed). `docs/audits/logs/PMAT-236-gate-tests.log` records both halves. Recorded as a lane error rather than dropped: two lanes agreeing is not a measurement.

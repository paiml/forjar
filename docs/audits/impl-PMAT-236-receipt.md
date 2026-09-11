# Implementation receipt — PMAT-236 — a shipped ticket says it shipped, and gate T stops lying at random

verdict: PASS — three defects in one gate, all three found by running it. **PMAT-236**: T2 and T4 reconcile the ledger and the `release:<tag>` labels and neither looked at `status`, so the roadmap said none of the 1.28.0 work had started on the day 1.28.0 shipped; a new arm checks every ticket a tagged row names and every ticket merged since the newest tag, and it fired on PMAT-232 the moment it existed — the drift recurring within a day of PMAT-235's backfill. **PMAT-238**: the verdict line said `8 of 7 PR(s)` because one PR carried two tickets; it now says `N ticket(s) from M PR(s)`. **PMAT-239**: `printf … | grep -q` took SIGPIPE under `pipefail` and the gate called a readable registry UNMEASURED on one run in three; it is a here-string now. Three cases are red against main's scripts and green here.

orch_model: opus [A]   orch_class: code   orch_decision: admit   orch_basis: state
fable_binding: false   quota_age_h: absent   quota_mark: ?   k_measured_at_set: refused(R-5)

routes:
  ph0  class=orchestration  route=self  w=100.00  basis=absent
  ph1  class=impl  route=agy-goal  w=1.00  basis=absent  note=fable-binding  effort=1[U]  (executed by self: three arms of one shell gate and its fixture)
  ph2.quorum  class=review  route=agy-quorum  w=1.00  basis=absent  effort=1[U]  (delegate, three lanes)
  ph3  class=orchestration  route=self  w=100.00  basis=absent

verification:
  cmd="cargo test --no-fail-fast over the three suites, with main's scripts/dogfood checked out (RED: 3 failed) then at HEAD (GREEN: 19 + 2 + 17)"  claimed_exit=101(lanes)  rerun_exit=101/0  log_path=docs/audits/logs/PMAT-236-gate-tests.log  sha256=recorded-in-the-log

## Why one branch for three tickets

All three are arms of gate T, all three were found by running it on main within an hour, and all three are exercised by the same fixture. Three reviews of three one-line changes is the spend this repository's cadence brief calls waste; one branch, one round, three receipts.

## PMAT-236 — the status was never reconciled

The ledger says which tickets a release shipped and the labels say it from the other side, and gate T checked both. Nothing checked `status`, so sixteen tickets across five releases read `planned` or `inprogress` while their labels were correct, and the roadmap said the 1.28.0 work had not started on the day it shipped. PMAT-235 backfilled them by hand; **three more had already drifted by the next day** (PMAT-232, PMAT-235 itself and PMAT-237), which is what a rule nobody enforces does.

The arm covers both halves of the window: every ticket a tagged row names, and every ticket whose PR merged since the newest tag — the work has landed either way. It names the ticket, where it is named, its actual status, and the two `pmat work edit` calls that move it, because the tool refuses `planned -> completed` directly.

## PMAT-238 — tickets counted against PRs

`8 of 7 PR(s) merged since v1.28.0 carry release:v1.29.0`, because PR #515 carried two tickets and the arm counts tickets while the sentence said PRs. Harmless arithmetic, and the one line an operator is told to read said something impossible.

## PMAT-239 — a gate that failed at random

`GATE T FAIL grep exited 141 looking PMAT-225 up in the ticket registry — UNMEASURED` on one run, PASS on the next two. 141 is 128+13: `printf … | grep -q` has grep exit at the first match and close the pipe, printf takes the signal, and `pipefail` makes that the pipeline's status. **The gate was right to refuse an unmeasured read; the read should not have been unmeasurable.** A here-string has no second process to kill.

## Falsification

`docs/audits/logs/PMAT-236-gate-tests.log`: with main's `scripts/dogfood/tagged.sh` and `lib/window.sh` checked out over this branch's tests, three cases fail — the two status cases and the green fixture, whose assertion quotes the corrected sentence. With the scripts restored, 19 + 2 + 17 green.

- `a_shipped_ticket_whose_row_says_planned_is_named_and_red` and `a_merged_ticket_whose_row_says_planned_is_named_and_red` drive both halves of the arm.
- `the_same_tree_gives_the_same_verdict_every_time` runs the gate ten times over one fixture and refuses any 141. **It is green against main too, and says so**: the SIGPIPE was one run in three, so ten passes is a regression guard rather than a proof, and one flake in three is how it was found at all.
- The fixture now writes `status: completed` unless a row id carries a `planned:` prefix, because a fixture that wrote every row as `planned` would make every case red for a reason no case is about.

## What the review changed

Three lanes, all three returning, and the strengthened brief held: the repository was intact afterwards, which the previous round's was not.

- **C6 refuted 3/3, and they were right three times over.** The claim said no pipeline in these scripts can return 141; `git tag … | head -1` in `dogfood_prev_tag` and `git tag … | grep -v … | head -1` in both `tagged.sh` and `release-goal.sh` all can. All three are one capture plus one `awk` or a parameter expansion now. **Grepping for the pattern rather than the instance also found a nineteenth that this very branch had just added** — `grep … | head -1 | cut` in the new status reader, one line from the defect being fixed — and it is one `awk` now.
- **C7 refuted by one lane, correctly.** At one flake in three, ten passes have a 1.7% chance of being luck. The deterministic half is `the_release_goal_scripts_carry_no_pipeline_that_can_take_sigpipe`, which refuses the PATTERN in the three files this branch owns and is red when one leaky pipeline is appended to any of them. The ten-run loop stays as a weaker companion and the receipt no longer leans on it.
- **C4 refuted by two lanes, and it does not reproduce.** They reported the red/green measurement "backwards". Re-run by the orchestrator with main's two scripts checked out over this branch's tests: `a_shipped_ticket_whose_row_says_planned_is_named_and_red` and `a_merged_ticket_whose_row_says_planned_is_named_and_red` both FAIL (14 passed, 3 failed), and both pass at HEAD. Recorded as a lane error rather than dropped.

Eighteen further sites of the same class exist under `scripts/` — four of them `git tag … | head -1` in other gates. They are **PMAT-240**, with the census committed as `docs/audits/logs/PMAT-239-sigpipe-census.log`, and the new rule is deliberately scoped to the three files this branch fixed rather than made red on work it has not done.

## Gaps, named

- `the_same_tree_gives_the_same_verdict_every_time` cannot prove determinism — a lane put it at 1.7% luck — and it is kept only beside the pattern rule, which can.
- The pattern rule covers three files. Eighteen sites elsewhere are PMAT-240, and until that lands any other gate can still report UNMEASURED at random.
- The arm accepts `cancelled` as well as `completed`: a ticket that shipped and was then cancelled is not distinguished from one that was never done. No such row exists.
- PMAT-234 is still open — gate R's verdict line still says `pre-tag` after every successful release. It is the same class of defect as PMAT-238 in a different gate.

IMPL-PMAT-236-RECEIPT-END

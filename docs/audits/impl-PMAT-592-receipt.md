# Implementation receipt — PMAT-592 — forjar 1.32.0, the cut

verdict: PASS — merged as PR #593 (07f6d122). Version 1.31.0 to 1.32.0 in Cargo.toml and Cargo.lock, the CHANGELOG heading over the window, and docs/audits/crux-1.32.0.md for the one behaviour bullet; plus the three blockers the cut had to clear, none of them planned: gate B's CB-21xx ratchet was red ON MAIN (CB-2115 55/43, CB-2114 37/34), gate A refused the three #583 receipts, and the quorum receipt binding was not portable between clones (forjar#573). Nine dogfood gates exit 0 on the cut branch; quorum gate passed with 3 confirmed and 10 refuted claims.

## How this was implemented — stated, not inferred

Directly, in a forjar session, **not** through the paiml-implement harness. No
discover.json, route.sh verdict or dispatch ledger exists for this ticket, and
none is invented here. What exists is recorded below and in
`.quorum/PMAT-592-release-1.32.0.json`.

## What the cut had to clear

| blocker | measured | cleared by |
|---|---|---|
| gate T | five findings across three tickets: two missing `release:v1.32.0` labels (PMAT-565, PMAT-579) and three stale statuses (PMAT-565, PMAT-576, PMAT-579). The cut's own commit message said "four"; a review lane added them up | `release-goal.sh sync`; `pmat work edit` through inprogress |
| gate B | CB-2115 55/43 and CB-2114 37/34 **on origin/main**, measured in a detached worktree; the cut branch was three findings better | 12 DRIFT title syncs, PMAT-153 completed, rows minted — **no ceiling edited** |
| gate A | the three #583 receipts lacked both a verdict line and an END marker | both added to all three |
| quorum binding | CI refused the receipt as STALE on a head the local gate passed | `--full-index` (forjar#573): git sized index-line abbreviations by the clone's object count — 8 here, 9 on the runner |

## What the quorum found

Two agy lanes and one Claude lane, read-only over the recorded gate result. Every
structural claim about the diff held; every refutation was prose: "the fourth
signature" where the CHANGELOG says third; a 50-hour-late cut called "two days
after 1.31.0"; 55 and 53 quoted as one measurement from two unnamed instruments;
one reason given for holding back both #590 and #591. The 55/53 finding was
raised independently by all three lanes.

## Gaps

- The quorum reviewed the branch before #573's fix; that fix was added after,
  with its own falsification test and RED proof, and C8 ("changes no .rs") was
  withdrawn in the receipt as R10.
- Not tagged at merge. The tag waits on the tag candidate's clean-room gate and
  the dogfood receipt on it.

IMPL-PMAT-592-RECEIPT-END

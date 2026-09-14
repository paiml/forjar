# Quorum evidence — PMAT-236 + PMAT-238 + PMAT-239 — the claims as put to the lanes

# PMAT-236 + PMAT-238 + PMAT-239 — claims for the quorum lanes

Branch PMAT-236-gate-t-checks-status, two commits (ba046399, 7faabf52) on
main (cc339449). Judge the diff `main...7faabf52`. Three arms of one gate,
one branch, one review.

1. PMAT-236: `scripts/dogfood/tagged.sh` gains an arm that refuses when a
   ticket named by a tagged ledger row, OR named by a PR merged since the
   newest tag, has a roadmap status other than `completed` or `cancelled`,
   and refuses as UNMEASURED when the row has no status at all.
2. The failure names the ticket, where it is named (a tagged release, or
   merged since the newest tag), its actual status, and the two
   `pmat work edit` calls that move it.
3. The arm reads the roadmap at the same ref as the rest of gate T: the
   statuses come from `dogfood_roadmap_rows`, which reads
   `docs/roadmaps/roadmap.yaml` once at `${DOGFOOD_ROADMAP_REF:-HEAD}`.
4. `a_shipped_ticket_whose_row_says_planned_is_named_and_red` and
   `a_merged_ticket_whose_row_says_planned_is_named_and_red` are RED
   against main's scripts and GREEN at HEAD.
5. PMAT-238: the verdict line now reads `N ticket(s) from M PR(s) merged
   since <tag> carry release:<next>`, never one count of the other, and
   the two fixture assertions that quoted the old wording are updated.
6. PMAT-239: no pipeline in `scripts/dogfood/lib/window.sh` feeds a
   process that exits early from a `printf`. The registry lookup is a
   here-string, so no pipeline can return 141 under `set -o pipefail`.
7. `the_same_tree_gives_the_same_verdict_every_time` runs the gate ten
   times over one fixture, asserts the same exit code each time and no
   `exited 141`. It is GREEN against main too, because the flake was one
   run in three — it is a regression guard, not a falsifier, and the
   receipt says so.
8. The fixture writes `status: completed` unless a row id carries a
   `planned:` prefix, and that prefix is what the two new cases use.
9. Gate T passes on this branch against the live repository, and its line
   reads `26 ticket(s) carry their tag and say they shipped; 8 ticket(s)
   from 7 PR(s) merged since v1.28.0 carry release:v1.29.0`.
10. The drift is real and recurred: PMAT-232, PMAT-235 and PMAT-237 were
    merged and still read `planned` one day after PMAT-235 backfilled
    sixteen others. They are `completed` in this diff.
11. No file under src/ changes; the diff is two shell files, two test
    files, one fixture and the roadmap.

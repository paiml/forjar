# PMAT-557 — adjudicated claims

Two rounds of three sandboxed agy quorum lanes — the paiml-implement review
(1 FAIL, 2 PASS) and the merge rail's first round (2 FAIL, 1 PASS), neither
agreed. Five confirmations and five refutations, every GitHub-side claim re-run by
the orchestrator because no lane could reach GitHub. The row is what the tag
and the window say; the one sentence a lane called false was not.

## CONFIRMED

1. [cut] That the v1.30.0 row's `cut` is the tag's creatordate in UTC and its
   `prs` and `tickets` are exactly the window between v1.29.0 and v1.30.0
   (all three lanes against git; the orchestrator against GitHub).
   - evidence: `docs/roadmaps/releases.yaml:79` onward holds the row; the tag's
     creatordate is 22:52:01+02:00, which is 20:52:01Z;
     `release-goal.sh window v1.30.0` prints the same ten PRs and eleven
     tickets, #548 and #563 counted as merged after the tag and #527 as inside
     v1.29.0.

2. [cookbook] That the named cookbook commit locks the version that shipped
   (the orchestrator, through gate T's own reader of the cookbook's Cargo.lock).
   - evidence: gate T prints, for this branch,
     `v1.30.0 cookbook 60acf9c9 requires forjar 1.30 and locks 1.30.0 ok`.
     The row the cut script wrote named 0be3e1ec, which locks 1.29.0, and gate
     T refused it by name — that refusal is why paiml/forjar-cookbook#21
     exists.

3. [marker] That appending `DOGFOOD-1.30.0-RECEIPT-END` is honest: the receipt
   ends on a complete sentence and carries exactly one `verdict:` line (lanes
   1 and 2 measured; lane 3 read).
   - evidence: the last prose line of `docs/audits/dogfood-1.30.0-receipt.md:66`
     closes its paragraph, the single verdict line reads GO, and nothing but
     the marker and its blank line is added.

4. [completed] That PMAT-555, PMAT-547 and PMAT-562 shipped and their rows may
   say completed (lanes 1, 2 and 3 against git log).
   - evidence: #556, #548 and #563 are on main; the rows at
     `docs/roadmaps/roadmap.yaml:3982`, `docs/roadmaps/roadmap.yaml:4091` and
     `docs/roadmaps/roadmap.yaml:4116` change their `status:` line, and the
     cut and sync steps also bump `updated:` on PMAT-555 and PMAT-562 and add
     their release label; CB-2112's ISSUE-CLOSED count falls from 3 to 0.

5. [labels] That PMAT-526, PMAT-528 and PMAT-529 did not ship in v1.30.0, so
   moving their `release:` label to v1.31.0 is correct (all three lanes).
   - evidence: none of the three ids appears in a merge commit between the
     v1.29.0 and v1.30.0 tags; `release-goal.sh cut` moved them because the
     window it measured did not contain them.

## REFUTED

1. [commit-message] That commit `b3b04c1f`'s message "claims the cookbook field
   was replaced, but that commit retained 0be3e1ec" (lane 1, graded measured).
   - corrected: the message says the field "still names 0be3e1ec, which locks
     forjar 1.29.0; it is replaced by the paiml/forjar-cookbook#21 merge commit
     that locks 1.30.0 before this PR is opened" — a statement about that commit
     and a promise about the branch, both true: `d68fa31a` replaces it, and the
     row at `docs/roadmaps/releases.yaml:79` names 60acf9c9 before any PR exists.

2. [gate-t-measured] That the lanes measured gate T and the window (the brief
   asked them to run `tagged.sh` and `release-goal.sh window`).
   - corrected: `gh` returned 401 inside every sandbox, so both were UNMEASURED
     there; the lanes checked the row against `git log` instead and said so.
     The GitHub half — gate T PASS, the window — was run by the orchestrator on
     the branch, and the pmat digest records each intermediate FAIL it went
     through first.

3. [no-writes] That every lane honoured the no-writes brief.
   - corrected: lane 1 wrote `diff.txt` into its own sandbox clone;
     `agy-lane.sh` kept the clone and printed so; the shared worktree stayed at
     `2edf43c7`, and the clone was removed after the finding was recorded.

4. [status-only] That the three shipped rows "change only their `status:`
   line" (this author, in this digest as first written; found by the merge
   rail's lane 1, gemini-3.1-pro-high).
   - corrected: attributing every changed line of the roadmap diff to its row
     shows PMAT-555 and PMAT-562 also bump `updated:` and gain their release
     label — written by `release-goal.sh cut` and `sync`, not by hand — while
     PMAT-547 changes only `status:`. The sentence in the CONFIRMED item above
     now says exactly that; the change itself was right and the description of
     it was not.

5. [wrong-rows] That the diff marked PMAT-545 completed instead of PMAT-555 and
   moved PMAT-527's label instead of PMAT-526's (the merge rail's lane 3,
   gemini-3.1-pro-high, reading hunk context).
   - corrected: the YAML at `origin/main` and at HEAD was parsed and every row
     compared by its `id`, which no hunk layout can blur: 213 rows at base, 219
     at head, six added (PMAT-557/558/559/561/566/567), none removed; changed
     rows are exactly PMAT-526/528/529 (labels, updated), PMAT-547 (status),
     PMAT-555 and PMAT-562 (status, labels, updated); PMAT-545 and PMAT-527 are
     byte-identical as parsed. The same lane model repeated the claim in the
     merge rail's second round and was refuted by the same comparison; the two
     other lanes in that round passed. The lane read the line above each hunk
     as the row the hunk edits.

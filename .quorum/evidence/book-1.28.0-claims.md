# Quorum evidence — PMAT-227 — the claims as put to the lanes

Each lane was given the command that decides each claim, so a lane's verdict is a rerun and not a reading.

# PMAT-227 — claims for the quorum lanes (book v1.28.0 in the release ledger)

Branch PMAT-227-book-v1.28.0, two commits on main (cdcc0e80 = tag v1.28.0):
e82b6cd2 (the row, next v1.29.0, PMAT-227 minted with release:v1.29.0) and
a47ca249 (PMAT-228 row). Judge the diff `main...a47ca249`. This is a
kind: triage branch: classify + link, no code.

1. The v1.28.0 row in docs/roadmaps/releases.yaml is byte-identical to
   `scripts/release-goal.sh window v1.28.0` with the note/comment lines
   dropped: cut 2026-09-10T16:07:14Z; prs [493, 494, 496, 498, 500, 502,
   504, 505, 507, 508]; tickets [PMAT-215, PMAT-217, PMAT-219, PMAT-220,
   PMAT-221, PMAT-222, PMAT-223, PMAT-224, PMAT-225, PMAT-226]; dogfood
   docs/audits/dogfood-1.28.0-receipt.md and crux docs/audits/crux-1.28.0.md,
   both existing at HEAD.
2. The row's cut is the tag's creatordate: `git for-each-ref
   --format='%(creatordate:iso-strict)' refs/tags/v1.28.0` is
   2026-09-10T18:07:14+02:00, i.e. 16:07:14Z; the tag is annotated and points
   at cdcc0e80, the squash commit of PR #508, and origin carries it.
3. next.tag is v1.29.0 and next.due 2026-09-12T16:07:14Z is the cut plus
   exactly 172800 seconds.
4. Labels: PMAT-227 carries release:v1.29.0 (the open window, applied at
   mint time); every one of the ten window tickets carries release:v1.28.0
   and no other roadmap row does; `cut` moved no label.
5. Gate T with this ledger over main's HEAD passes (a worktree at cdcc0e80
   with the branch's two YAML files copied in and
   DOGFOOD_RELEASES_REF=worktree DOGFOOD_ROADMAP_REF=worktree:
   `GATE T PASS 6 tagged release(s) … 0 of 0 PR(s) merged since v1.28.0 …
   due 2026-09-12T16:07:14Z`); on the branch itself the gate is red on ONE
   arm only — its own unmerged commits since the tag with no merged PR
   containing them — which is the bypassed-review arm gates A and E share
   and which turns green at the merge.
6. The diff touches only docs/roadmaps/releases.yaml,
   docs/roadmaps/roadmap.yaml and (at the receipt commit) docs/audits/** and
   .quorum/**: the kind: triage rail PMAT-226 widened. No script, no test,
   no source.
7. `release-goal.sh cut` wrote one stray line into the ledger — the window
   library's `#490 … is inside v1.27.0 (the previous release) — not counted`
   note, printed on stdout and captured — which was removed by hand; no line
   matching "not counted" remains in docs/roadmaps/releases.yaml, and
   PMAT-228 (kind:code, status planned) records the defect with acceptance
   criteria naming the fix (stderr or filter) and its fixture test.
8. The ledger parses and the tool reads it: `scripts/release-goal.sh show`
   prints the v1.29.0 goal with `basis=docs/roadmaps/releases.yaml:L62`;
   `scripts/release-goal.sh sync --check` finds nothing to label.
9. forjar 1.28.0 is on crates.io (`cargo info forjar` or `cargo search
   forjar` shows 1.28.0), published from a detached worktree of the tag by
   scripts/publish-from-tag.sh; the GitHub release workflow run for the tag
   is in flight and is NOT claimed here.

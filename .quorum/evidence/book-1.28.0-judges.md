# Quorum evidence — PMAT-227 — adjudicated claims

One round of three sandboxed lanes, base pinned at cdcc0e80 (the commit `v1.28.0` names), the diff at a47ca249: 0/3 PASS, 3/3 FAIL. Seven claims were confirmed by all three lanes. C8 is a real error in the claim text and is corrected below. C4 was refuted by one lane and confirmed by the other two; the orchestrator re-ran it, the refutation does not reproduce, and it is recorded as a lane error rather than quietly dropped. Neither finding changes a byte of the diff, so the diff was not re-judged; what C8 exposed was filed as its own ticket instead.

The citations below resolve at the merge base: `docs/roadmaps/releases.yaml:55`, `:56` and `:57` are, at that commit, the three lines `next:` / `tag: v1.28.0` / `due: 2026-09-10T22:10:01Z` — the open goal this branch closes — and `docs/roadmaps/roadmap.yaml:3292` is `- id: PMAT-226`, the ticket whose rail widening lets this branch be judged as triage at all.

## CONFIRMED

1. [row-equals-window] THE ROW IS THE MEASURED WINDOW, NOT A TRANSCRIPTION — the v1.28.0 row `scripts/release-goal.sh cut` wrote is byte-identical to `scripts/release-goal.sh window v1.28.0` once comment lines are dropped: cut 2026-09-10T16:07:14Z, ten PRs, ten tickets, and the two receipt paths, both of which exist at HEAD.
- evidence: all three lanes ran `bash scripts/release-goal.sh window v1.28.0` in their own clone and diffed it against the row; the orchestrator ran the same diff before committing. The goal it closes is `docs/roadmaps/releases.yaml:56` at base (`tag: v1.28.0`), and the row replaces `docs/roadmaps/releases.yaml:55` at base (`next:`) in the file's line order.

2. [cut-is-the-tagger-date] THE CUT INSTANT IS THE TAG'S OWN CLOCK — `git for-each-ref refs/tags/v1.28.0` reports an annotated tag created 2026-09-10T18:07:14+02:00 (16:07:14Z) pointing at cdcc0e80, the squash commit of PR #508, and `git ls-remote --tags origin` shows origin carries it.
- evidence: lanes 1, 2 and 3 each ran both commands in their clones after `git fetch --tags`; the value matches `docs/roadmaps/releases.yaml:56`'s successor row byte for byte. The tagger date, not the commit date, is what `dogfood_tag_date` reads, which is why a re-tag would move the cadence clock (named as a gap in PMAT-225's receipt).

3. [cadence-arithmetic] THE OPEN GOAL IS THE CUT PLUS EXACTLY TWO DAYS — next.tag is v1.29.0 and next.due 2026-09-12T16:07:14Z is 172800 seconds after the cut, the arithmetic gate T's T6 re-derives and refuses when it disagrees.
- evidence: lanes 1, 2 and 3 computed the difference with `date -u -d <instant> +%s` and got exactly 172800; at base the same two fields are `docs/roadmaps/releases.yaml:56` and `:57` (`tag: v1.28.0`, `due: 2026-09-10T22:10:01Z`), the goal that fell due at the cut.

4. [labels] EVERY TICKET NAMES ITS TAG AND THE OPEN WINDOW IS ALREADY LABELLED — the ten tickets the row names each carry `release:v1.28.0`, exactly ten rows carry that label, and PMAT-227 carries `release:v1.29.0`, applied when the row was minted rather than at the next cut.
- evidence: lanes 2 and 3 confirmed by awk over `docs/roadmaps/roadmap.yaml`; the orchestrator re-ran the census at HEAD (`release:v1.28.0` on 10 rows, `release:v1.29.0` on PMAT-227 and, once filed, PMAT-228 and PMAT-229). The rail this branch rides is `docs/roadmaps/roadmap.yaml:3292`'s ticket, PMAT-226.
- lane error: lane 1 refuted this claim, reporting that PMAT-227's labels block held only `kind:triage`. It does not reproduce: at a47ca249 the block is `kind:triage`, `orch:fable`, `release:v1.29.0`, and two lanes read it correctly. Recorded as a lane misread, not as a finding, and not as a reason to change the tree.

5. [gate-t-both-ways] GATE T PASSES OVER MAIN AND IS RED ON THE BRANCH FOR ONE NAMED REASON — with this ledger in a worktree at cdcc0e80 the gate prints `GATE T PASS 6 tagged release(s) since v1.25.0 … 0 of 0 PR(s) merged since v1.28.0 carry release:v1.29.0; due 2026-09-12T16:07:14Z`; on the branch it exits 1 solely because its own two commits reach HEAD and no merged PR contains them.
- evidence: all three lanes ran both, and the orchestrator recorded them as `docs/audits/logs/PMAT-227-gate-T-main.log` and `docs/audits/logs/PMAT-227-gate-T-branch.log`. That red arm is the bypassed-review arm gates A and E share; it turns green when this PR merges, which is the same shape every ticket in the v1.28.0 window had.

6. [rail] THE DIFF STAYS ON THE TRIAGE RAIL — `git diff --name-only cdcc0e80 a47ca249` names `docs/roadmaps/releases.yaml` and `docs/roadmaps/roadmap.yaml` and nothing else; with the receipts it adds `docs/audits/**` and `.quorum/**`, the whole rail and no more.
- evidence: lanes 1, 2 and 3 each ran the diff. The rail admits `docs/roadmaps/releases.yaml` only because of the ticket at `docs/roadmaps/roadmap.yaml:3292` (PMAT-226), which is what makes this branch judgeable as triage instead of pushed with a waiver.

7. [stray-line-filed] THE TOOL'S ONE BAD LINE IS GONE AND TICKETED — `cut` pasted the window library's `#490 … is inside v1.27.0 (the previous release) — not counted` note into the ledger, where the leading `#` made it a YAML comment by accident; it was removed by hand and PMAT-228 records the defect with the fix and its fixture test.
- evidence: `grep -n 'not counted' docs/roadmaps/releases.yaml` exits 1 in all three lanes' clones; the PMAT-228 row sits after `docs/roadmaps/roadmap.yaml:3292`'s ticket in the same file and carries `kind: code`, `status: planned` and two acceptance criteria naming stderr-or-filter and the red-without-the-fix case.

8. [crates-io] THE ARTIFACT THIS ROW BOOKS IS PUBLISHED — crates.io serves forjar 1.28.0, published from a detached worktree of the tag by `scripts/publish-from-tag.sh`, and docs.rs reports `doc_status: true` for it.
- evidence: all three lanes checked from outside the checkout (`cargo search forjar --limit 1`), because `cargo info forjar` inside a forjar tree reports the local manifest and proves nothing; the orchestrator read `https://crates.io/api/v1/crates/forjar/1.28.0` (created_at 2026-09-10T16:12:45Z) and the docs.rs status endpoint. The GitHub release was in flight and is deliberately not claimed here.

## REFUTED

9. [status-line] AS WORDED: "`release-goal.sh show` prints the v1.29.0 goal with `basis=docs/roadmaps/releases.yaml:L62` and `sync --check` finds nothing to label" — refuted by all three lanes on both halves.
- evidence: on the branch, `bash scripts/release-goal.sh show` exits 2 printing `2 commit(s) reached HEAD since v1.28.0 and GitHub reports no merged PR containing any of them` and NO goal line at all; `sync --check` exits 2 the same way. In a worktree at cdcc0e80 with this ledger, `show` exits 0 and prints `basis=docs/roadmaps/releases.yaml:L61`, not L62 — the claim quoted a number measured before the stray line at `docs/roadmaps/releases.yaml:55`'s successor position was removed, which shifted every later line up by one.
- corrected: with this ledger at main's HEAD, `show` prints `v1.29.0 … due 2026-09-12T16:07:14Z basis=docs/roadmaps/releases.yaml:L61` and `sync --check` finds nothing to label, both exit 0. On a branch carrying unmerged commits, `show` and `sync` refuse with exit 2 on the bypassed-review arm. That refusal is right for gate T and wrong for a status line an operator runs from a feature branch — the due instant and the elapsed bar do not depend on the unmerged commits — and it is filed as PMAT-229 with the measurement, the acceptance criteria and the explicit requirement that `scripts/dogfood/tagged.sh` stay exactly as strict.

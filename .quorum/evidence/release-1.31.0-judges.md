# PMAT-574 — adjudicated claims

The rounds, their lanes and their verdicts are the table in
`release-1.31.0-lanes.md`, which is the only place they are counted. What
matters here is the adjudication, and it is stated once: the items below are
every claim this branch put at risk, six CONFIRMED and twelve REFUTED.
 Three of the refutations
came from instruments that ruled on this branch BEFORE any lane saw it — gate H,
the commit-msg hook, and `README.md` itself — and they are named below as the
refuting authority. A reader who sees `judges: 3` should read it as the three
lanes that were ASKED, one of which returned a verdict, not as three independent
tiers.

## CONFIRMED

1. [version-everywhere] That the version is bumped everywhere it is declared and
   nowhere is left behind.
   - evidence: `Cargo.toml:3` reads version 1.31.0, `Cargo.lock:1171` is the
     `forjar` package entry carrying the same string, and `README.md:96` and line 98
     read `1.31`. Gate D re-measures the join and reports that the version
     claims reconcile with Cargo.toml.

2. [window-and-no-other] That the CHANGELOG's section describes this window and
   no other.
   - evidence: `CHANGELOG.md:10` opens `## [1.31.0] - 2026-09-16` ABOVE the
     three behaviour paragraphs already on main and below an empty
     `## [Unreleased]`, which is the same edit the 1.30.0 cut made. The
     paragraphs name PMAT-564/#564, PMAT-560/#560 and PMAT-562/#562; gate T
     measured the window independently at 5 PRs and 5 tickets.

3. [crux-three-systems] That every behaviour bullet has a crux row naming at
   least three surveyed systems.
   - evidence: the three bullets are `CHANGELOG.md:12`, `CHANGELOG.md:32` and
     `CHANGELOG.md:56`, and `docs/audits/crux-1.31.0.md:31`, `:32` and `:33` are
     the rows that reconcile them; they survey Bazel/Ansible/Terraform/Kubernetes, systemd/Ansible/
     Puppet/Chef/Podman and Terraform/Puppet/Kubernetes/Ansible. Gate H agrees
     by its own count, reconciling 3 of 3 behaviour bullets, each naming at
     least 3 of the 28 surveyed systems.

4. [no-ceiling-raised] That the roadmap changes are bookkeeping the gates
   require and no ceiling is raised.
   - evidence: `scripts/ratchets/cb21xx-baseline.json:14` still declares
     `"CB-2115": 43` and the file is untouched in this diff; gate B measured
     CB-2112 34/35, CB-2114 34/34, CB-2115 42/43 on the committed tree. Two
     could be lowered from that measurement and neither is, because the tree
     carrying it is the one the next commit makes.

5. [shipped-vs-deferred] That the tickets marked completed shipped in this
   window, and the ones that did not are marked deferred rather than quietly
   carried.
   - evidence: the three shipped behaviours name their tickets in the section
     itself — `CHANGELOG.md:12` (PMAT-564), `CHANGELOG.md:32` (PMAT-560),
     `CHANGELOG.md:56` (PMAT-562) — and `docs/roadmaps/roadmap.yaml:4277` is
     PMAT-560's completed status, matching #568 merged at 22:59Z on 2026-09-15 and issue #560
     closed by it; PMAT-557 and PMAT-564 the same for #571 and #569. PMAT-565,
     PMAT-572 and PMAT-573 carry `release: 1.32.0`, and PMAT-565's exclusion is
     stated in the crux document's closing section rather than left implicit.

6. [receipt-reports-the-run] That the dogfood receipt reports the gate lines the
   run actually printed.
   - evidence: `docs/audits/dogfood-1.31.0-receipt.md:3` records the verdict for
     a run whose log tail is `DOGFOOD_RC=0` on the committed tree at 9e3a4744,
     and each of the nine table rows is the line that run emitted.

## REFUTED

1. [readme-already-right] That the README's version lines were already correct,
   as the first draft of the impl receipt asserted with `README.md — untouched`.
   - evidence: the file refutes it. `README.md:96` said `forjar = "1.30"`, and so
     did the `default-features = false` line below it. Both now read 1.31 and
     the receipt says what the cut DID instead of what the author assumed. Gate
     D passed either way, because Cargo reads `1.30` as `>=1.30.0, <2.0.0` —
     which is exactly why no gate would have caught it.

2. [completed-in-its-own-pr] That the cut's own ticket can be marked completed
   in the cut's own PR.
   - evidence: the commit-msg hook refuted it, at
     `docs/audits/logs/PMAT-574-cut.log:50`, which records the refusal
     verbatim: the ticket is completed, work belongs to an open item, and
     CB-2113 refuses this commit in CI.
     PMAT-574 stays `inprogress`; the booking PR closes it, as PMAT-557's
     booking closed PMAT-555.

3. [rows-written-is-satisfied] That writing the crux rows satisfies gate H.
   - evidence: gate H refuted it, at `docs/audits/logs/PMAT-574-cut.log:9`,
     which records GATE H FAIL over the three bullets at `CHANGELOG.md:12`,
     `CHANGELOG.md:32` and `CHANGELOG.md:56`, none of which had a row it could
     find. The rows
     existed; they carried backticks inside the span the gate greps as a FIXED
     string, so the key "forjar drift declines — exit 2, the" did not occur in
     a row that spelled it with backticks. The ratchet refuted its own author in the
     same sequence at `docs/audits/logs/PMAT-574-cut.log:23`.

4. [diff-list-complete] That the receipt's diff list is complete.
   - evidence: lane 1 refuted it (`gemini-3.1-pro-high`), cited at
     `docs/audits/impl-PMAT-574-receipt.md:42`, where the correction now lives:
     the list named the dogfood receipt and the cut log but not the receipt
     doing the listing, nor the quorum artifacts. The lane's accompanying claim
     — that the receipt is an unrequested file — is refuted in turn by
     `scripts/dogfood/harness.sh:23`, which states the rule gate A enforces:
     every merged PR's ticket needs its `impl-<ticket>-receipt.md` at HEAD.

5. [gap-explained-by-receipts] That the 42-vs-43 difference in CB-2115 was
   caused by "the receipts this commit had not yet added", as the first drafts
   of both the cut log and the dogfood receipt asserted.
   - evidence: lane 1 of round 3 refuted it — a markdown file cannot move
     ORPHAN-ROADMAP, ORPHAN-GITHUB or DRIFT, none of which reads
     `docs/audits/**`. Re-measured at 06:40Z: CB-2115 42, composition
     ORPHAN-ROADMAP 24 + ORPHAN-GITHUB 8 + DRIFT 10, which sums. `pmat comply`
     reads GitHub live and stamps each run with its own snapshot timestamp, so
     the 43 at 05:00Z and the 42 since are two measurements of a moving source.
     Both documents now say that instead.

6. [after-line-sums] That the cut log's after-composition was right as written.
   - evidence: lane 1 of round 3, arithmetic: the line gave CB-2115 as 43 over
     a breakdown of ORPHAN-ROADMAP 24, ORPHAN-GITHUB 8, DRIFT 10, and
     24 + 8 + 10 = 42. A
     total that contradicts its own breakdown is exactly the black box this
     receipt format exists to refuse. Corrected to the measured 42.

7. [gate-r-counts-six] That gate R's "6 PR(s) since v1.30.0" contradicts the 5
   the other gates report, as round 4's lane 1 claimed.
   - evidence: both numbers are right about different windows, and the receipt
     quotes each tool verbatim. `gh pr list` merged after the tag's timestamp
     returns exactly five: #571, #569, #568, #563, #548. Gate R's six are those
     five plus #556 — the release PR whose squash commit IS the v1.30.0 tag
     (`git rev-list -n1 v1.30.0` and #556's merge commit are both ddd0c441), so
     a window anchored on the tag COMMIT includes the PR that produced it while
     a window anchored after it does not. The receipt now says so where it
     quotes gate R.

8. [changelog-should-cite-the-pr] That the CHANGELOG entry for PMAT-564 should
   cite #569 (the PR) rather than #564, as round 4's lane 1 claimed.
   - evidence: the file's own convention is the ISSUE. Every 1.30.0 paragraph
     reads the same way — `(PMAT-549, #549)`, `(PMAT-534, #534)`,
     `(PMAT-540, #540)`, `(PMAT-542, #542)`, `(PMAT-535, #535)` — ticket then
     issue, never the PR. `CHANGELOG.md:12` follows it with
     `(PMAT-564, #564; paiml/infra#605 first signature)`. The paragraph also
     merged with #569 and rewriting it here would edit shipped text to match a
     convention the repository does not use.

9. [evidence-prose-is-current] That the evidence files described the rounds
   that had actually been run.
   - evidence: round 5's lane 1 refuted it three times over —
     `release-1.31.0-lanes.md` still opened with "Three rounds were run" and
     named 203d8a65 as the final head, `release-1.31.0-agy.md` still described
     "One round", and `release-1.31.0-claims.md` still said "The round reviewed
     head 203d8a65" — while the table below the first and this receipt both said
     four. A receipt whose prose contradicts its own table is the black box this
     format refuses. All three now describe every round, and say why the count
     moves: each real finding produces a fix, the fix moves the head, and the
     next round reviews a head no earlier round saw.

10. [receipt-counts-are-current] That this receipt's own counts matched its
    digest.
    - evidence: round 8's lane 1, four citations: `recorded_at` said three
      author claims where four had been refuted by lanes, `judges_note` said
      three rounds and four refutations where the digest said five and nine, and
      two "three rounds" phrases survived in the evidence prose. Corrected here,
      and the round count now tracks the table in `release-1.31.0-lanes.md`.

11. [counts-repeated-in-four-files] That restating the round count and the
    refutation count in each evidence file was workable.
    - evidence: rounds 8 and 9 each refuted it, and round 9 did so from two
      lanes at once — `release-1.31.0-agy.md` still said five rounds,
      `release-1.31.0-claims.md` still listed heads up to b0ccb46a, and this
      file's opening still said nine refutations while its items numbered ten.
      A count repeated in four files goes stale in three. The structural answer
      is in the tree: the per-round table in `release-1.31.0-lanes.md` is now
      the ONLY place rounds are counted and heads are named, the adjudication
      count is stated once here, and the other two files refer rather than
      repeat.

12. [structural-fix-landed] That the structural fix — one place for the round
    count — had landed everywhere it was needed.
    - evidence: round 11 refuted it from all three lanes at once. The
      lanes-table file's opening paragraph had never been replaced (the edit
      matched nothing and reported success), `agy_teamwork.mode` still read "one
      round of three sandboxed agy quorum lanes" beside `"rounds": 9`, and
      `docs/audits/impl-PMAT-574-receipt.md` still said "the round that produced
      it". All three are fixed, and this time the tree was SWEPT for every count
      beside the word "round" rather than assumed — which is how the surviving
      quotations were separated from the live claims.

## Lane findings that did NOT survive

Round 3's lane 1 raised six findings. Two are the refutations above. The other
four are refuted, each by re-reading the file the lane cited:

- "the first bullet is at CHANGELOG.md:13, not :12" — `CHANGELOG.md:10` is the
  `## [1.31.0] - 2026-09-16` heading, `:11` is blank and `:12` is the bullet.
- "crux-1.31.0.md:31 is the table separator; the rows are 32-34" — `:29` is the
  header row, `:30` the separator, `:31`, `:32` and `:33` the three rows.
- "README.md:96 is a comment; the version lines are 97 and 99" — `:95` is the
  comment `# the binary, everything on`, `:96` is `forjar = "1.31"`, `:97` the
  second comment and `:98` the library-only line. The diff's own hunk headers
  say `@@ -96 +96 @@` and `@@ -98 +98 @@`.
- "CB-2114 should be 32, not 34, because four rows were repaired" — three of
  those rows were MINTED with `release: 1.32.0` and so never contributed a
  NO-RELEASE finding to remove. Two rows were repaired (PMAT-560 and PMAT-574),
  and 36 - 2 = 34, which is what the gate measured.

All four off-by-one citations point the same direction, which is worth naming:
a lane reading a unified diff counts added lines, and a file counts its own. The
lane was right twice about arithmetic it could do from the text, and wrong four
times about line numbers it inferred rather than read.

Round 4 repeated the same mistake from two lanes at once — `gemini-3.1-pro-high`
and `gemini-3.6-flash-medium` both insisted the first bullet is `CHANGELOG.md:13`
and the version line `README.md:97`. Re-measured at HEAD by reading each blob
out of git and numbering it with awk, so the number printed is the file's own: `CHANGELOG.md:10` is the heading, `:11` is blank,
`:12` is the bullet; `README.md:95` is a comment, `:96` is the version line. Two
lanes agreeing does not move a measurement, and this is the second round in
which the agreement was wrong in the same direction.

## The kill rule

No lane finding was dismissed. Lane 1's was split: the true half is fixed in
105cf896, the false half is refuted by naming the gate that requires the file.
Two NO-VERDICT lanes are counted as no review — never as a PASS — and the merge
rail runs its own round on the final head before anything merges. A red gate
stops the cut, and no verdict overrides it.

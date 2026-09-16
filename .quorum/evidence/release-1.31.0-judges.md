# PMAT-574 — adjudicated claims

One round of three sandboxed agy quorum lanes: 1 FAIL, 2 NO-VERDICT, not
agreed. Six confirmations and four refutations. Three of the four refutations
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

## The kill rule

No lane finding was dismissed. Lane 1's was split: the true half is fixed in
105cf896, the false half is refuted by naming the gate that requires the file.
Two NO-VERDICT lanes are counted as no review — never as a PASS — and the merge
rail runs its own round on the final head before anything merges. A red gate
stops the cut, and no verdict overrides it.

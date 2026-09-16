# PMAT-574 — adjudicated claims

One round of three sandboxed `agy` lanes returned one FAIL with a cited finding
and two NO-VERDICTs (one lane wrote no verdict object, one hit a 503 with no
capacity for its model). This file does not pretend that round was a panel: on a
release cut the binding adjudication came from the GATES and the HOOKS, which
ruled on this branch before any lane saw it, and they are named below as the
refuting authority.

A reader who sees `claims_refuted: 4` and one FAIL lane should read it exactly
that way.

## REFUTED

**R1 — "the README's version lines are already correct."** Written into the
first draft of `docs/audits/impl-PMAT-574-receipt.md` as `README.md — untouched`.
The file said `forjar = "1.30"` on line 96 and `forjar = { version = "1.30",
default-features = false }` on line 98. Refuted by reading the file; both lines
now say 1.31 and the receipt says what the cut did instead of what the author
assumed. Gate D had passed either way, because Cargo reads `1.30` as
`>=1.30.0, <2.0.0`, which is precisely why nothing else would have caught it.

**R2 — "the cut's own ticket can be marked completed in the cut's own PR."**
Refuted by the commit-msg hook, which refused the first attempt at 9e3a4744:
`Pmat-Ticket PMAT-574 is completed — work belongs to an open item, and CB-2113
refuses this commit in CI. leave a ticket open in its own PR; the next ticket's
PR marks it completed`. PMAT-574 is `inprogress` in this branch and the booking
PR closes it, exactly as PMAT-557's booking closed PMAT-555.

**R3 — "the crux rows are written, so gate H is satisfied."** Refuted by gate H
on its first run: `3 behaviour bullet(s) under [1.31.0] have no row in
docs/audits/crux-1.31.0.md`. The rows were there; they read
`` | (1) `forjar drift` declines … `` and the gate greps the bullet's first six
words as a FIXED string, which backticks inside the key span defeat. The rows
carry no backticks in the key now.

**R4 — "the receipt's diff list is complete."** Refuted by lane 1, cited at
`docs/audits/impl-PMAT-574-receipt.md:1`: the list named the dogfood receipt and
the cut log but not the receipt doing the listing, nor the quorum artifacts.
Corrected. The lane's accompanying claim — that the receipt is an unrequested
file — is itself refuted by gate A, which requires it; both halves are recorded
in `release-1.31.0-lanes.md`.

## CONFIRMED

**C1 — the version is bumped everywhere it is declared.** `Cargo.toml` 1.31.0,
`Cargo.lock`'s `forjar` entry 1.31.0, README lines 96 and 98 at 1.31. Gate D:
`version claims reconcile with Cargo.toml`.

**C2 — the CHANGELOG section describes this window and no other.** Three
behaviour paragraphs under `[1.31.0]`, one per behaviour PR (#563 PMAT-562,
#569 PMAT-564, #568 PMAT-560). Gate T measured the window at 5 PRs / 5 tickets
and agreed with the ledger.

**C3 — every bullet has a crux row naming >= 3 surveyed systems.** `GATE H PASS
3 of 3 behaviour bullet(s) under [1.31.0] reconciled in
docs/audits/crux-1.31.0.md, each naming >= 3 of the 28 surveyed systems`.

**C4 — the roadmap changes are bookkeeping the gates require, and no ceiling is
raised.** `scripts/ratchets/cb21xx-baseline.json` is untouched in this diff;
CB-2112 34/35, CB-2114 34/34, CB-2115 42/43 measured by gate B on the committed
tree. Two ceilings could now be lowered and neither is: a ceiling may only be
lowered from a measurement of the tree that carries it.

**C5 — the completed tickets shipped, and the deferred ones are marked
deferred.** PMAT-557, PMAT-560 and PMAT-564 merged in this window (#571, #568,
#569) and their issues are closed; PMAT-565, PMAT-572 and PMAT-573 carry
`release: 1.32.0` and the 1.32.0 milestone. PMAT-565's exclusion is stated in
the crux document and in the impl receipt rather than left to be noticed.

**C6 — the dogfood receipt reports the lines the run printed.** Nine gate lines
in `docs/audits/dogfood-1.31.0-receipt.md`, each copied from the run whose tail
is `DOGFOOD_RC=0`, on the committed tree at 9e3a4744.

## The kill rule

No lane finding was dismissed. Lane 1's was split: the half that is true was
fixed, the half that is false was refuted by naming the gate that requires the
file. Two NO-VERDICT lanes are counted as what they are — no review — and the
merge rail runs its own round on the final head before anything merges.

# Claims: forjar#607, phase 2. The booked row is amended, never edited

The T3 rule (claims in PMAT-607-p1-claims.md) makes gate T read #643 and
#646 as PMAT-642 and PMAT-624. v1.33.0's row, booked by this PR's cut, does
not declare them. This is the second of the two pieces.

**Phase 2 (421e1082): the booked row.** The fixed rule measures v1.33.0's
window as including PMAT-624 and PMAT-642. The booked row does not declare
them. Gate T would stay red, and the cut moved PMAT-642's goal label forward
to v1.34.0. A booked row is the record of what was measured at the cut, so it
is never edited, re-cut or rewritten. The tool gains a forward path that
appends instead.

- A1: `release-goal.sh amend TAG` appends one record (tag, at, tickets, prs) to an `amendments:` block above `releases:`. The bytes from `releases:` down are unchanged; the test asserts this.
- A2: amend only adds. It refuses:
  - an unbooked tag;
  - a row whose prs differ from its measured window;
  - a declared ticket the window does not name;
  - an empty amendment.
- A3: lib/releases.sh refuses, by name, a hand-written record that does any of these:
  - names an unbooked tag;
  - re-declares a ticket, or names one twice;
  - cites a PR the tag does not declare;
  - is dated before the record above it.
- A4: gate T's T2 compares the row's tickets plus its amendments' with the measured set. No other check changes. prs, labels (both directions), receipts, cookbook and T7 still read the measured window. Nothing is narrowed or waived.
- A5: amend labels each added ticket `release:TAG`. It removes the `release:<following>` label the cut moved it to, unless the following window names it too.
- A6: a later `cut` books its row under `releases:`, never into `amendments:`. The resulting ledger loads.
- A7: the falsifier discriminates:
  - all 6 tests are red with the three scripts at phase 1;
  - one or more is red with gate T reading the row alone;
  - one or more is red with the fault check off;
  - one or more is red with amend keeping the forward label.

Running `amend v1.33.0` is the step after this PR, not part of it. It runs on
main once this merges, so the amendment record is written by the tool as
merged and reviewed, not by an unmerged branch. Then gate T, then the next cut.

# Claims: forjar#607, the T3 rule and phase 2. Gate T's two remaining reds on v1.33.0, fixed in the tool

Phase 1 (the parent PR) booked v1.33.0 with `release-goal.sh cut`, and its
claim C6 says gate T stays red. T3 is red on #643 and #646, which name their
tickets only as a bare `#642` and `#624`. That is the work this ticket still
owes: "book v1.33.0 with the tool gate T names". This branch carries the two
remaining pieces as two commits, each with its own round.

**The T3 rule (be506c19).** A PR that names no `PMAT-<n>` resolves
the first bare `#N` in its title, then its body. It resolves only through the
roadmap row whose `github_issue:` is N.

- With no such row, it stays red.
- The next `#N` is never tried.
- Two rows filed from one issue resolve to neither.

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

Out of scope here: running `amend` on v1.33.0. It runs from main after this
merges, then gate T, then the next cut.

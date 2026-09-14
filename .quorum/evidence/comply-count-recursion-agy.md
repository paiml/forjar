# PMAT-522 — agy delegation record

One `paiml-agy-delegate` dispatch, `lane=quorum`, `width=3`, `writes=false`,
opus, foreground, `out_dir` keyed by ticket AND session id, `--not-before`
pinned to the dispatch instant.

All three lanes returned structured output against the quorum schema, exits
0/0/0, durations 494s, 339s and 317s. Verdicts 3/3 FAIL.

**The dispatch happened only because the quorum gate refused a one-lane
receipt.** See `comply-count-recursion-lanes.md`: the round was nearly skipped
on the argument that the defect was already measured and the machine had just
been recovered, and the round then found three defects that argument would have
shipped.

The delegate stopped at its 30-turn limit while summarising, AFTER all three
lanes had completed and `reduce.json` was written. Fourth occurrence in this
session at exactly that point.

Slot accounting: at most one Claude subagent live at any instant, against a
floor of three. No hook denial.

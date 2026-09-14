# PMAT-531 — agy delegation record

One `paiml-agy-delegate` dispatch, `lane=quorum`, `width=3`, `writes=false`,
opus, foreground, `out_dir` keyed by ticket AND session id, `--not-before`
pinned to the dispatch instant.

All three lanes returned structured output against the quorum schema, exits
0/0/0, durations 730s, 430s and 611s. Verdicts 3/3 FAIL.

The delegate stopped at its 30-turn limit while reducing, after all three lanes
had completed. Sixth occurrence in this session at that same point; the lane
JSON was read directly each time. The pattern is consistent enough to be worth
a ticket in the paiml-implement repository rather than a seventh workaround.

Slot accounting: at most one Claude subagent live at any instant, against a
floor of three. No hook denial.

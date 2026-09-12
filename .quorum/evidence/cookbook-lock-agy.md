# PMAT-537 — agy delegation record

One `paiml-agy-delegate` dispatch, `lane=quorum`, `width=3`, `writes=false`,
opus, foreground, `out_dir` keyed by ticket AND session id, `--not-before`
pinned to the dispatch instant.

All three lanes returned structured output against the quorum schema, exits
0/0/0, durations 234s, 357s and 389s. Verdicts 3/3 FAIL, with all six numbered
claims confirmed.

The delegate stopped at its 30-turn limit while reducing, after all three lanes
had completed. **Eighth occurrence in this session**, always at the same point.
The lane JSON is read directly each time, which works; a pattern that
reproduces eight times in one session is a ticket for the paiml-implement
repository, not a ninth workaround.

Slot accounting: at most one Claude subagent live at any instant, against a
floor of three. No hook denial.

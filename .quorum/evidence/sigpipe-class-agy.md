# PMAT-240 — agy delegation record

One `paiml-agy-delegate` dispatch, `lane=quorum`, `width=3`, `writes=false`,
opus, foreground, `out_dir` keyed by ticket AND session id, `--not-before`
pinned to the dispatch instant.

All three lanes returned structured output against the quorum schema, exits
0/0/0, durations 311s, 433s and 441s. Verdicts 3/3 FAIL.

The delegate stopped at its 30-turn limit while reducing, after all three lanes
had completed. **Ninth occurrence in this session**, always at the same point.

Slot accounting: at most one Claude subagent live at any instant, against a
floor of three. No hook denial.

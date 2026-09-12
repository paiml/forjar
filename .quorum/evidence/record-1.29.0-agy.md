# PMAT-533 — agy delegation record

One `paiml-agy-delegate` dispatch, `lane=quorum`, `width=3`, `writes=false`,
opus, foreground, `out_dir` keyed by ticket AND session id, `--not-before`
pinned to the dispatch instant.

All three lanes returned structured output against the quorum schema, exits
0/0/0, durations 378s, 337s and 334s. Verdicts 3/3 FAIL.

The delegate stopped at its 30-turn limit while reducing, after all three lanes
had completed. **Seventh occurrence in this session at that same point.** The
lane JSON was read directly every time, which works, and a pattern that
reproduces seven times in one session belongs in the paiml-implement repository
as a ticket rather than an eighth workaround.

Slot accounting across the whole 1.29.0 run: at most one Claude subagent live
at any instant, against a floor of three. No hook denial in any ticket.

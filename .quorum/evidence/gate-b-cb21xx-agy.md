# PMAT-521 — agy delegation record

One `paiml-agy-delegate` dispatch, `lane=quorum`, `width=3`, `writes=false`,
opus, foreground, `out_dir` keyed by ticket AND session id, `--not-before`
pinned to the dispatch instant.

All three lanes returned structured output against the quorum schema, exits
0/0/0, durations 509s, 687s and 443s. Verdicts 3/3 FAIL.

The delegate itself stopped at its 30-turn limit while summarising, AFTER all
three lanes had completed and `reduce.json` had been written. The lane JSON was
read directly rather than resuming for a summary of output already on disk.
This is the THIRD ticket in this session where the delegate ran out of turns at
exactly that point — a pattern that deserves a ticket of its own rather than a
third workaround.

Slot accounting: at most one Claude subagent live at any instant, against a
floor of three. No hook denial in this ticket.

# PMAT-234 — agy delegation record

One `paiml-agy-delegate` dispatch, `lane=quorum`, `width=3`, `writes=false`,
opus, foreground, `out_dir` keyed by ticket AND session id, `--not-before`
pinned to the dispatch instant.

All three lanes returned structured output against the quorum schema, exits
0/0/0, durations 371s, 332s and 333s. Verdicts 1 PASS / 2 FAIL.

The delegate itself stopped at its 30-turn limit while summarising, AFTER all
three lanes had completed and `reduce.json` had been written. The lane JSON was
read directly rather than resuming the delegate for a summary of output already
on disk: a resume takes a slot and would have added nothing the lanes had not
already said. This is the second ticket in this session where the delegate ran
out of turns at exactly that point, which is a pattern worth a ticket rather
than a workaround.

Slot accounting: at most one Claude subagent live at any instant, against a
floor of three. No hook denial in this ticket.

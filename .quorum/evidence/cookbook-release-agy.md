# PMAT-241 — agy delegation record

Round two: one `paiml-agy-delegate` dispatch, `lane=quorum`, `width=3`,
`writes=false`, opus, foreground, out_dir keyed by ticket AND session id.

| lane | conversation | exit | duration | verdict |
|---|---|---|---|---|
| 1 | conv-756b15d5 | 0 | 559s | FAIL |
| 2 | conv-ecf29dee | 0 | 489s | FAIL |
| 3 | conv-c3a9cae0 | 0 | 495s | FAIL |

All three lanes returned structured output against the quorum schema, and all
three summaries read `C1 REFUTED, C2 REFUTED, C3 REFUTED, C4 CONFIRMED(N=10),
C5 CONFIRMED` — unanimous on every claim, which is unusual and is the reason
each refutation was re-run rather than taken on the count.

The delegate itself stopped at its 30-turn limit while summarising, AFTER all
three lanes had completed and `reduce.json` had been written. The lane JSON was
read directly rather than resuming the delegate for a summary of output already
on disk: a resume takes a slot and would have added nothing the lanes had not
already said.

Round one: one delegate dispatch, same shape, three lanes, 3/3 FAIL.

Slot accounting: at most one Claude subagent live at any instant in either
round (the delegate), against a floor of three. No hook denial in this ticket.

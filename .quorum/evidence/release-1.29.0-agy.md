# PMAT-520 — agy delegation record

Two dispatches for this cut.

**The crux survey.** One `paiml-agy-delegate`, `lane=quorum`, `width=1`,
`writes=false`, over the ten behaviour bullets that existed when it ran. It
returned ten rows, 28 surveyed systems and 30 `[X]` claims, plus a Method and a
limit paragraph. Rows 11 and 12 are the orchestrator's, written afterwards
because neither behaviour existed yet — pmat 3.40 turned gate B red the same
day, and the fork storm happened while fixing it. The audit's own limit section
says which rows are whose.

**The review.** One `paiml-agy-delegate`, `lane=quorum`, `width=3`,
`writes=false`, opus, foreground, `out_dir` keyed by ticket AND session id,
`--not-before` pinned to the dispatch instant. All three lanes returned
structured output against the quorum schema, exits 0/0/0, durations 367s, 478s
and 389s. Verdicts 3/3 FAIL.

The delegate stopped at its 30-turn limit while summarising, AFTER all three
lanes completed and `reduce.json` was written. Fifth occurrence in this session
at exactly that point; the lane JSON was read directly each time.

Slot accounting: at most one Claude subagent live at any instant, against a
floor of three. No hook denial in this ticket.

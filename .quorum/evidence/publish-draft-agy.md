# Quorum evidence — PMAT-232 — agy

- Version: agy 1.2.0; three sandboxed lanes, writes=false; conversations conv-179049d3,conv-803e03ae conv-02916c3e.
- One round on efa49822, base pinned at a138c809: 0/3 PASS with all eleven claims confirmed. The verdicts are FAIL because of the hostile-reader half of the brief, which is where both real defects were: a comment and a ticket note that a reader could check and find false.
- The brief asked for three things a text rule cannot give: re-run every mutation yourself rather than trust the log; work out by hand which of the three possible `state` strings takes which branch of the shell `case`; and say what the job does when `gh` fails or returns nothing. All three lanes did all three, and their agreement on the shell is the only evidence this change has that it behaves correctly at runtime — rule 5 reads the job's text and cannot execute it.
- One lane refuted the actionlint count (11 rather than 22). Re-run by the orchestrator on both trees: 22 lines carry a file:line:col: prefix on each side. Recorded as a lane error.
- The delegate hit its 30-turn cap after the lanes had written their files; the orchestrator read them directly.
- Residue: no lane wrote to the repository.

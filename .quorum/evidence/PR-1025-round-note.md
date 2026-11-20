# ROUND-NOTE: paiml/forjar PR #1025 (merge 02df42622c2f), redo of refuter 2 + judges
- Refuter 2 was re-run because its first two outputs (AGY-refuter-2.try1.out and AGY-refuter-2.out) were invalid JSON. The re-run (PROMPT-refuter-2-redo.md, AGY-refuter-2-redo.out) used the same prompt content with a one-line-per-claim tab format. It answered 65 of 65 claims; none missing. It was not shown refuter 1's or 3's output.
- The 3 judges were re-run (PROMPT-judge-<k>-redo.md, AGY-judge-<k>-redo.out) because the first judges voted without refuter 2. The redo judges saw CLAIMS.md and REFUTERS-redo.json (refuter 1, refuter 2 redo, refuter 3); the earlier judge outputs were not shown to them. Each judge answered 65 of 65 claims.
- Counts come from the redo files only: claims 65 (lane 1: 20, lane 2: 23, lane 3: 22), claims_refuted 1 (1.5, judges REFUTED/REFUTED/REFUTED = 3 of 3), claims_confirmed 64.
- Refuter 2 redo itself marked only 1.5 REFUTED.
- Old vs new per-claim outcome (old = first judges, no refuter 2): no claim differs. 1.5 refuted 3/3 in both; the other 64 survived in both. Individual judge votes are also identical in every claim.
- Original files kept unchanged: AGY-refuter-2.out, AGY-refuter-2.try1.out, AGY-judge-{1,2,3}.out, REFUTERS.json.

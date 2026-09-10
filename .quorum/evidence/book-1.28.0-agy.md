# Quorum evidence — PMAT-227 — agy

- Version: agy 1.2.0; lane guard installed; three sandboxed lanes through the delegate's fixed calling form (`--sandbox --dangerously-skip-permissions --print-timeout 25m --output-format json --json-schema <quorum schema>`), writes=false.
- Phase 2 (review): three lanes on the diff at a47ca249 with the base pinned at cdcc0e80, conversations conv-4372a570, conv-86493b36, conv-71645ff3; 0/3 PASS, 3/3 FAIL, lane-reduce dissent 3. Every lane refuted C8 (the status-line claim) and one lane also refuted C4 (the label claim).
- The brief named the exact command that decides each claim, so a lane verdict here is a rerun rather than a reading; every lane reported `grounding: measured` on eight of nine claims.
- The delegate hit its 30-turn cap after all three lanes had written their files, the same recovery as PMAT-222..226: the orchestrator read `<out_dir>/lane-*.json` and ran `lane-reduce.sh` itself.
- Both refutations were re-run by the orchestrator before they were recorded. C8 is a real error in the claim text, corrected in book-1.28.0-judges.md and filed as PMAT-229. C4's refutation does not reproduce — the label is in the row at a47ca249 and two lanes read it correctly — and is recorded as a lane misread. A lane verdict is a claim; this is the round where that mattered in both directions.
- Residue: `git status --porcelain` of the repository carried only the untracked files that predate this session before and after the dispatch; no lane wrote to the repository and no lane copied a `target/`.

# Quorum evidence — PMAT-235 — agy

- Version: agy 1.2.0; three sandboxed lanes, writes=false, conversations conv-e97e57d6,conv-e68657a3 conv-157bed15.
- One round on 691c1faa, base pinned at d7028f5b: 0/3 PASS. Nine claims confirmed by all three, one refuted by all three.
- The brief forbade judging a data change from a summary of it: every lane parsed BOTH versions of docs/roadmaps/roadmap.yaml into structured rows and diffed them field by field, ran the refused `pmat work edit` transition itself, and finished by reading the receipt as a hostile reader.
- That last instruction is what found the one defect. All three lanes quoted the same sentence — a row count taken before two rows were minted and reported as a description of the commit — and nothing else in the receipt.
- The delegate hit its 30-turn cap after the lanes had written their files; the orchestrator read them directly.
- Residue: no lane wrote to the repository; `git status --porcelain` unchanged around the round.

# Quorum evidence — PMAT-221 — agy

- Version: agy 1.1.28, lane guard installed; every lane through the delegate's fixed calling form (`--dangerously-skip-permissions --print-timeout 25m --output-format json --json-schema <quorum schema> -p=…`), `--sandbox` on every lane (writes=false throughout; no writing lane was dispatched on this ticket).
- Phase 1: one `/teamwork-preview` lane on the plan, conversation conv-c0b0dbb6; fan-out unmeasurable (no project directory, one turn) — recorded as single-lane, not consensus.
- Phase 3: three `--mode plan` lanes on the diff, conversations conv-3f70e5ed, conv-6193ebcb, conv-aced4c1e; an interrupted first run of lane 3 (conv-44063326) was excluded from the reduction.
- Reduction: `lane-reduce.sh` width 3, `--not-before` the run's own start; artifact carries dedup, dissent, partial_reasons and coverage_source=lanes verbatim.
- Every lane verdict was treated as a claim: the orchestrator re-ran acceptance, the touched suites, clippy, fmt, the workspace suite and gate G itself. No lane ran cargo.

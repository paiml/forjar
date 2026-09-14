# Quorum evidence — PMAT-228 + PMAT-229 — agy

- Version: agy 1.2.0; three sandboxed lanes, writes=false, one round at cdd03571 with the base pinned at 0c1277c6. Two returned (conv-3a9ae6fd, conv-14c10dab); one returned no structured output and is recorded as such rather than dropped.
- **One round for two tickets, deliberately.** Both are `release-goal.sh` telling an operator the wrong thing, both are a few lines, both are exercised by the same three test binaries. Three lanes each would have cost six lanes to judge nine changed lines; the repository's own cadence brief calls that waste, and batching is the cheapest place to answer it without lowering any floor.
- The brief did the work the lanes then confirmed: it asked, by name, whether the soft switch could leak from a caller's environment into a gate run. Both returning lanes answered yes. The hole was closed one commit later — the switch is a positional argument now — and a case pins it shut. Writing the attack into the brief is cheaper than hoping a lane finds it, and the lanes are what proved the attack was worth writing.
- The lanes judged cdd03571; HEAD is 50073b89 plus receipts. Their verdict is about the tree they saw and this evidence says so.
- The delegate hit its 30-turn cap after the lanes had written their files; the orchestrator read them directly.

# Quorum evidence — PMAT-770 r2 — lane summaries

Authors: Claude Sonnet 5.5 and Opus 5.5, so no lane uses either model id. Lanes were handed an inline evidence bundle (git facts, gh PR bases, workflow diffs, the epic's guard diff, the gate rule) and no tools: agy ran sandboxed in plan mode without --dangerously-skip-permissions, and the first agy attempts that tried to run git were auto-denied by headless mode, so the bundle was inlined.

- lane 1, agy gemini-4-argon-high: C1-C4 CONFIRMED, C5-C7 REFUTED, VERDICT a.
- lane 2, agy gemini-3.8-flash-high: C1-C4 CONFIRMED, C5-C7 REFUTED, VERDICT a (second attempt; the first was denied a command permission and produced no output, not a 429).
- lane 3, claude-haiku-4-5 via Claude Code in plan mode: C1-C4 CONFIRMED, C5-C7 REFUTED, VERDICT a.

No agy bucket returned 429. Refuters and judges: the same three lanes, unanimous on all seven claims; both planted controls (C6, C7) killed 3 of 3.

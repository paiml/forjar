# Earlier agy reports (`agy`) — not counted as evidence (PMAT-673 / forjar#914)

- Reports `review-forjar-914-7c60917-lane{1,2,3}.md` (Gemini 4 Carbon) exist for head `7c609178d784bb214105e1d8a4eaca081e900494`.
- Their content describes figures and test code (a 4-minute-per-pass bound, a 20/30-minute budget, `serde_yaml_ng` parsing) that are not in the tree at that commit. They are excluded: no verdict here rests on them.
- The verifiable facts are in the current diff: `seq 1 3`, `timeout-minutes: 45`, `fail-fast: false`, test asserts `passes * 9 + 10 <= timeout`.

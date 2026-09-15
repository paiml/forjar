# PMAT-564 — the lanes, and what each returned

One round of three sandboxed agy quorum lanes (`agy-lane.sh --mode plan`,
schema-enforced verdicts), review-only (`writes=false`), each in a
self-contained clone asserted byte-identical afterwards — except lane 1's,
which was KEPT because the lane wrote a three-line hello-world
`test_image_skip.rs` into its own clone (isolation held: exit 0, the shared
checkout clean). Author model: Claude Opus 5; every lane Gemini, measured
from its own log.

| lane | model (measured) | verdict | findings | duration |
|---|---|---|---|---|
| 1 (lane-1.json) | gemini-3.1-pro-high | FAIL | 6 | 348 s |
| 2 (lane-2.json) | gemini-3.7-flash-high | PASS | 6 | 171 s |
| 3 (lane-3.json) | gemini-3.1-pro-high | PASS | 6 | 319 s |

`lane-reduce.sh --width 3 --lane-models gemini-3.1-pro-high,gemini-3.7-flash-high,gemini-3.1-pro-high
--author-model opus` → `agreed=false` (2 PASS / 1 FAIL); `partial_reasons`
records that only two model ids were distinct across three lanes.
Conversation ids shortened: conv-55a57952, conv-b5db623c, conv-abd28ef5.

## Lane 1 — FAIL, and it was right

The only lane whose transcript touched `src/tripwire/drift/image.rs` (five
files mention it; the other two lanes' transcripts mention it zero times).
It found that the image detector walked the lock alone — no
`resources.contains_key(id)` — and that `census.inspected` uses `insert`
while `skipped` uses `or_insert_with`, so a later detector's inspection
OVERWRITES an earlier one's not-in-config skip. Therefore the CHANGELOG's
"by every detector" sentence and the contract's "never graded" invariant
were false as written. Graded `measured`. It agreed with the others on
everything else: N from the config never the lock, unmeasured outranks the
decline (exit 4), `--dry-run` alone exits 0, the mixed-reason message.

## Lanes 2 and 3 — PASS, over the same detector they did not read

Both ran the acceptance suites (lane 2: 39 tests, 0 failed; lane 3: 17
`test result:` lines) and both judged the contract and CHANGELOG "accurate" /
"completely true". Neither read `image.rs`. Lane 3 self-labelled every
finding `asserted` despite having run the tests. Lanes 1 and 3 said N is
counted pre-expansion; lane 2 said post-expansion. Re-read here:
`load_drift_config` calls `parse_and_validate`, which expands recipes,
`count:` and `for_each:` before the config reaches `cmd_drift`, so N counts
expanded resources — lane 2 was right and the two resamples of the same
model were wrong together.

## What the orchestrator re-ran

`image.rs` was read and the guard was absent (lane 1 confirmed); the guard
was added with a unit test that fails without it (mutation M5). The five
mutations below were run against the committed tree. The lane-1 clone was
removed after the finding was recorded.

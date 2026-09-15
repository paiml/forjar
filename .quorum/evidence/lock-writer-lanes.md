# PMAT-565 — the lanes, and what each returned

One round of three sandboxed agy quorum lanes (`agy-lane.sh --mode plan`,
schema-enforced verdicts), review-only, each clone asserted byte-identical
afterwards and removed. Author model: Claude Opus 5; every lane Gemini,
measured from its own log.

| lane | model (measured) | verdict | findings | duration |
|---|---|---|---|---|
| 1 (lane-1.json) | gemini-3.1-pro-high | FAIL | 6 | 235 s |
| 2 (lane-2.json) | gemini-3.7-flash-high | PASS | 6 | 340 s |
| 3 (lane-3.json) | gemini-3.1-pro-high | FAIL | 1 | 251 s |

`lane-reduce.sh --width 3 --lane-models gemini-3.1-pro-high,gemini-3.7-flash-high,gemini-3.1-pro-high
--author-model opus` → `agreed=false` (2 FAIL / 1 PASS); `partial_reasons`
records the duplicate model id. Conversation ids shortened: conv-f90c4be0,
conv-b2b2e525, conv-9c9d7a67.

## Lanes 1 and 3 — FAIL, on the same line, and they were right

Both named `src/cli/lock_repair.rs` writing a `StateLock` with a bare
`fs::write` — the minimal repaired lock and the normalised form — and lane 1
added `src/cli/lock_audit.rs` (`lock-migrate` at :334; `lock-restore` at
:234 and `lock-tag` at :300, which copy bytes). Lane 1 graded it `measured`,
lane 3 `asserted`. The contract sentence "Every path writes through
save_lock — apply's finalize, --refresh, repair, restamp" and the CHANGELOG's
"the one writer every path goes through" were therefore false as written.
Lane 1 also called restamp's one-level walk a gap against a CHANGELOG
sentence ("rewrites every lock under a state dir") that did say that.

Lane 1 answered the rest: an empty generator leaves `created_by` None and the
stamped generator satisfies `lock-audit`; no `deny_unknown_fields` on
`StateLock`, so an older forjar parses the file; the sidecar is consistent
because restamp writes through `save_lock`; no misplaced literal — a
`created_by` inside a GlobalLock or StackStamp literal would not compile.

## Lane 2 — PASS, over a claim the other two refuted

The one distinct model. It ran the acceptance suites and stated "no direct
serde_yaml_ng + fs::write exists in src/ for StateLock" — the opposite of
what lanes 1 and 3 found, and wrong: the sites are at `lock_repair.rs:41`
and `:99` (as reviewed) and `lock_audit.rs:334`.

## What the orchestrator re-ran

The five sites were read here. `lock-repair` (both writes) and
`lock-migrate` now go through `save_lock`, with a falsifier through the
binary that also checks the `.b3` sidecar those paths used to leave stale.
`lock-restore` and `lock-tag` copy bytes and are named as the exception. The
CHANGELOG, book and contract state restamp's scope instead of implying it.

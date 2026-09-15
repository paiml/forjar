# PMAT-562 — the lanes, and what each returned

One round of three sandboxed agy quorum lanes (`agy-lane.sh --mode plan`,
`--sandbox --dangerously-skip-permissions`, schema-enforced verdicts),
review-only (`writes=false`), each in a self-contained clone that the wrapper
asserted byte-identical afterwards. Author model: Claude Opus 5; every lane a
Gemini model, measured from its own log, none in the author's family.

| lane | model (measured) | envelope | verdict | findings | duration |
|---|---|---|---|---|---|
| 1 (lane-1.json) | gemini-3.1-pro-high | SUCCESS | FAIL | 10 | 285 s |
| 2 (lane-2.json) | gemini-3.8-flash-high | ERROR (API 503, no capacity) | NO-VERDICT — carried PASS, not counted | 9 | 1177 s |
| 3 (lane-3.json) | gemini-3.7-flash-high | SUCCESS | PASS | 8 | 304 s |

`lane-reduce.sh --width 3 --not-before 1789479913 --lane-models
gemini-3.1-pro-high,gemini-3.8-flash-high,gemini-3.7-flash-high --author-model
opus` → `agreed=false, partial=true` (one voided lane; the two counted split
FAIL/PASS). Conversation ids shortened: conv-0d819949, conv-3bfbedf5,
conv-35640912.

## Lane 1 — FAIL, and it earned it

Answered all six questions. On 1, 2, 4, 5 and 6 it agrees with lane 3: no
Ok-on-drift path; apply unchanged; contract and CHANGELOG true; the kept flag
is the right shape because removing it makes clap reject every fleet cron line
with "unexpected argument". Its FAIL is question 3: ten sentences the tree
still carried that sold `--tripwire` as the exit-code switch or said drift
exits 10 — `docs/book/src/01-getting-started.md:173` and `:701`,
`docs/book/src/06-cli.md:184` and `:1157`, `docs/specifications/forjar-spec.md:1293`,
`examples/image_drift.rs:41`, `src/core/error.rs:66` and `:166`, plus two
dated audit records (`docs/cli-defects.json:129`,
`docs/dogfood-1.12.3-cli-defects.json:128`) that quote the 1.12.3 help text
verbatim. All graded `measured` — the lane grepped.

It also reported the acceptance command FAILED in its clone: two cases in
`cli::tests_check_2` (`test_fj017_check_machine_filter`,
`test_fj017_check_resource_filter`) died with `NotFound`. Neither is in the
diff; both pass on this host and passed in lane 3's clone. Recorded as a
sandbox artefact, not a defect, and not counted as a refutation.

## Lane 3 — PASS

Ran the acceptance command (129 tests, 0 failed) and `cargo check`; read
`cmd_drift` end to end and named the return at `src/cli/drift.rs:381`; read
`dispatch_apply_b.rs:203`; read the contract, CHANGELOG and 06-cli.md. It did
NOT grep the tree for remaining `--tripwire` sentences, which is why it passed
question 3 while lane 1 failed it.

## Lane 2 — voided

Envelope `status=ERROR` on a 503 after 1177 s (nine turns waiting on `cargo
build --all-targets`). Its carried structured output — PASS, with nine of the
same doc sites lane 1 found — is UNREVIEWED CLAIM MATERIAL under
`partial_reasons` and was not counted. Where it overlaps lane 1 it corroborates;
nothing in it stands alone.

## What the orchestrator re-ran

Every claim above that mattered was re-executed on this host, not accepted:
the acceptance command (green), the ten doc sites (all real, all fixed in
`22a53170`), the two `tests_check_2` cases (green here), and three mutations
over the committed tree (see the judges digest).

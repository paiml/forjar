# PMAT-534 — the lanes, and what each returned

One round of three sandboxed agy quorum lanes, review-only (`writes=false`),
against a FULL standalone clone — a `--shared` clone's `objects/info/alternates`
points outside the sandbox and every git command in the lane fails. Dispatched
in a single message, `out_dir` keyed by ticket AND session id, `--not-before`
pinned to the dispatch instant.

| lane | subject | verdict | findings |
|---|---|---|---|
| 1 (conv, lane-1.json) | the arm | PASS | 7 |
| 2 (conv, lane-2.json) | the cases and the stub | FAIL | 5 |
| 3 (conv, lane-3.json) | the documents and the numbers | PASS | 7 |

Lane 2 is the one that mattered. Its finding: `if [ -z "$latest_rel" ]` is
uncovered by all four cases, because the broken-api stub exercises `rc != 0` and
nothing produces exit 0 with empty stdout. Its proposed fix named the case that
was missing and why the hole is real — without the guard the empty answer falls
to the comparison and the gate prints `still resolves to ` with a blank where
the tag goes.

Lanes 1 and 3 returned PASS. Lane 1's grounding was `asserted` on five of its
seven findings, and its closing claim — that no sentence in the documentation
is false — did not survive re-measurement (see the judges digest, REFUTED 1).

The delegate hit its 30-turn cap before writing a receipt, for the twelfth time
this session (filed as paiml-implement#141). All three lane JSONs were on disk
with `status: SUCCESS` and were read directly, together with the `reduce.json`
the delegate had already written.

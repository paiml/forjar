# PMAT-547 — the lanes, and what each returned

One round of three sandboxed agy quorum lanes, review-only (`writes=false`),
against a FULL standalone clone — a `--shared` clone's `objects/info/alternates`
points outside the sandbox and every git command in the lane fails. Dispatched
in a single message, `out_dir` keyed by ticket AND session id, `--not-before`
pinned to the dispatch instant.

| lane | subject | verdict | findings |
|---|---|---|---|
| 1 (lane-1.json) | will these jobs run on the fleet | FAIL | 7 |
| 2 (lane-2.json) | the test, and what it lets through | FAIL | 9 |
| 3 (lane-3.json) | the documents and the numbers | FAIL | 6 |

3/3 FAIL, and every lane earned it.

Lane 2 found the finding of the round: this branch BROKE
`tests/falsification_hosted_jobs_do_not_cache_target`, a suite the orchestrator
had never run, by moving `coverage.yml` to the fleet and emptying that suite's
denominator. Two cases were red on the branch as pushed to the lanes.

Lane 1 found that the new glibc step measures the host's glibc on a leg whose
binary was built inside a cross container — the builder line was reporting the
wrong machine.

Lanes 1 and 3 independently found the "six legs" error, which the orchestrator
had already caught and corrected by counting from the parsed YAML before either
lane reported. Lane 3 additionally found the fifth gap the receipt did not name:
three jobs delegate their runner to a reusable workflow in another repository and
every case in the new test passes over them in silence.

Five of lane 2's nine findings were `asserted` rather than measured — a lane
briefed NO WRITES cannot run the parser it is reasoning about. Four of those five
were true and are now fixtures; the fifth (`the_parser_finds_the_runners_that_
are_there` is too loose) was true as a matter of degree and the threshold moved.

The delegate hit its 30-turn cap before writing a receipt, for the thirteenth
time this session (paiml-implement#141). All three lane JSONs were on disk with
`status: SUCCESS` and were read directly.

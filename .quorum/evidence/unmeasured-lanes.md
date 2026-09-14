# PMAT-549 — the lanes, and what each returned

Two rounds, each of three sandboxed agy review lanes (`writes=false`) against a
standalone clone of the head under review, dispatched together with `out_dir`
keyed by ticket AND session id. Three judges read each round's lanes under a
binding kill rule. Each round also ran one lane beside the review: a prior-art
survey in round one, a `/teamwork-preview` review in round two.

ROUND ONE — head 830ff969, every lane gemini-3.1-pro-high.

| lane | claims | verdict | findings |
|---|---|---|---|
| 1 | 1-3: the reader, the first queries, the census | FAIL | 9 |
| 2 | 4-6: CLI, MCP, apply | FAIL | 20 |
| 3 | 7-8: the tests, the infra consumer | do-not-implement-as-written | 2 |
| survey | CRUX: prior art | FAIL (DOES NOT MATCH on two of three points) | 8 |

Lane 1 found two things:
- The lockless path built its report as a struct literal, so its census never
  learned what went unmeasured.
- `remote_path_digest` and `ensure_container` are transport calls outside the
  reader.

Lane 2 found two things:
- `apply` prints each finding's detail, so its output did change.
- A caller reading only `drifted`, or the exit status of a run without
  `--tripwire`, sees "clean".

Lane 3 found two things:
- No test asserted the image or task detectors' unmeasured arm, or the
  lockless census.
- paiml/infra's drift-tripwire.sh reads only `findings`, so it would pass an
  unreachable machine: 0 drift, 0 findings, exit 0.

ROUND TWO — head 37096656. Lanes and judges on gemini-3.1-pro-high,
gemini-3.8-flash-high and gemini-3.7-flash-high; teamwork on gemini-3.1-pro-high.

| lane | focus | verdict | findings |
|---|---|---|---|
| 1 | C1-C3: the reader, every transport call, the listing, the census | FAIL | 8 |
| 2 | C4-C6: CLI, MCP, exit codes, apply, alert, notify, remediate | PASS | 10 |
| 3 | C7-C8: the tests, the CHANGELOG, FALSIFY-VE-025 | PASS | 8 |
| teamwork | the whole change | PASS | 5 |

Lane 1's FAIL rested on two findings.

- **The CHANGELOG was wrong to say `apply` prints `not measured`**, because
  apply reads `remote_path_digest`, which swallows the error. All three judges
  rejected this finding. Apply's pre-apply drift check prints each finding's
  detail, and that detail comes from the drift detectors, not from
  `remote_path_digest`.
- **No test pins a remote script exiting 255 over SSH** without leaning on
  ssh's own "connect" wording. This was a real gap in the tests rather than a
  false sentence. f5bb1c01 adds the test.

Lane 2 passed and named three more properties that no test pinned. f5bb1c01
adds a test for each:
- the state-query detector's unmeasured arm;
- the tripwire exit when drift and unmeasured coexist;
- the alert staying silent when only unmeasured resources remain.

The teamwork lane passed. It cited the exit-4 split, `drift_count` preserved,
MCP's `unchecked`, and apply's unchanged consumption of every finding.

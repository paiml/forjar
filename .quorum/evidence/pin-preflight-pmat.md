# PMAT-579 / PMAT-581 / PMAT-582 — the instruments this registration was measured with

| tool | what it said |
|---|---|
| `git diff --name-only 583a58ea...HEAD` | four paths: `docs/roadmaps/roadmap.yaml` and the three receipts under `docs/audits/`; 137 insertions, 0 deletions before the evidence commit |
| `forjar history --json` on paiml/infra | `r-c61e1b0dc7cd` lambda-labs 10:24:19Z `forjar_version` 1.30.0; `r-c67eb3d5c454` intel 10:31:14Z 1.31.0 — the PMAT-579 measurement |
| `forjar drift` on paiml/infra, one host, one commit, three state dirs | 9 DRIFTED exit 1; `inspected 40 of 153`, `Drift detected: 2`, exit 1; 3 DRIFTED exit 1 — the PMAT-582 measurement |
| `grep -c` over `docs/roadmaps/roadmap.yaml` | `kind:code` 115 at the merge base and 118 on this branch; `kind:triage` 7 on both; `PMAT-580` rows 0 |
| `pmat --version` | 3.40.2 |
| `pmat analyze vacuous-tests --path docs` | refused: no tracked `.rs` under `docs`, so no test was examined — reported here as what it is, not as a clean zero |
| `pmat analyze vacuous-tests` (repository) | 432 of 19790 `#[test]` fns cannot fail (2.2%) across 2192 parsed files, plus 3 silent skips — the standing count, which this branch cannot have moved |
| `analyze_vacuous_tests` in touched paths | 0, because the touched paths contain no test; see the two rows above |
| commit-msg trailers | five commits, each `Pmat-Ticket: PMAT-579`, the branch's lead ticket; PMAT-581 and PMAT-582 are named in the receipts and the quorum artifact |
| `bashrs lint` | not applicable: no `.sh` file changed |
| `cargo test`, clippy, rustfmt | not run for this branch: no `.rs` changed, and the code gates have nothing to measure here; CI's `gate` context runs them regardless |
| `quorum-review.sh` round 3 | AGREED 3/3 on 5a6d607a; artifact posted to paiml/forjar#583 |
| `PRINT_HASH=1 scripts/quorum-gate.sh` | the `diff_sha256` in the receipt, computed after the evidence commit |

## The quality gate

This branch touches the roadmap and four documentation files under
`docs/audits/` (three receipts and, in the evidence commit, six evidence
files under `.quorum/evidence/`). It adds no code, so the code-shaped gates
have nothing to measure and are not claimed. The gate this branch exists to
satisfy is the triage rail plus a citable, refutable record of three
measurements, and the local run of `scripts/quorum-gate.sh` is quoted in the
receipt's `recorded_at`.

## Tool defect found while making this record

`pmat work add --github-issue N` refuses without a positional title even
though the issue already carries one; the rows were hand-authored in the
roadmap's own shape as a result. Reported only; not this branch's scope.

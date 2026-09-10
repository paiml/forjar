# Quorum evidence — PMAT-224 — the lanes

## Plan grill (phase 1) — one teamwork lane, sandboxed, read-only

- Mode: `/teamwork-preview` through the delegate; agy 1.2.0; 280 s; exit 0; conversation conv-048ea2e6; fan-out measured as UNKNOWN (no agents directory, no new teamwork project, one turn), so this is one model answering once, not a panel. The lane emitted its verdict twice with the same five findings — `implement-with-changes`, then `do-not-implement-as-written`; its own words ("the overall approach is sound, the plan must be amended") read as the milder one, and both are in the lane file.
- Findings, all adopted: (P1, refuted) widening the citation shape for every receipt would let a code branch anchor on the receipt it wrote itself — documentation anchors only for `kind: triage`; (P2, refuted) the falsification block's required-field loop (`test`, `reverted`, `observed_failure`) must be bypassed for triage too, not only `test_file`/`cargo_test_target`; (P3, confirmed) `not_applicable` is honest because the gate's mechanical falsification is `cargo test --test <target>` and a triage branch writes none; the receipt stays under `.quorum/**` because release-check reads it by slug; no reader outside the gate reads `falsification.*`.
- The lane also noted `kind` and `triage` appeared nowhere in the gate at 3cc65103 — the rail refusal is a check the fix adds, which is why P1's closure holds only together with it.
- Host tree byte-identical before and after.

## Diff review (phase 3) — three quorum lanes, --mode plan --sandbox, read-only, base pinned

- Diff: main (be863153) ... 15418d23; each lane read its own `--shared` clone; no lane ran cargo. Lanes may run `bashrs` and did.
- Lane 1, conv-e96ec28b: PASS, C1–C10 confirmed, C11 labelled refuted with the text "there are no hunks outside the ticket" (a label artifact), 858 s.
- Lane 2, conv-0bf1d1c3: PASS, C1–C4 and C6–C11 confirmed, C5 refuted: `receipt_identity` carries a `die` for an unknown `kind` that be863153 did not have, so the extraction is not strictly behaviour-preserving; demanded the note say so, no code change, 645 s.
- Lane 3, conv-fe39eeb3: PASS, all eleven confirmed after named refutation attempts (the `docs/audits.txt` edge of the rail, `sys.exit(0)` inside the python block, the required-field loop's position, `.quorum/evidence/` against the triage shape, the order of operations in the extracted helpers), 634 s.
- lane-reduce: agreed, exit 0. The delegate's own cross-checks: `bashrs lint` 0 errors / 15 warnings on both commits; the RED log names three FAILED and three ok cases; each mutation log names exactly one FAILED case; `receipt_kind` appears 0 times at be863153.

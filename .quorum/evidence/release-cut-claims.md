# Quorum evidence — PMAT-165 — the claims as put to the lanes

The 1.26.0 release cut: the version bump, the CHANGELOG's `[Unreleased]` becoming `## [1.26.0] - 2026-09-08`, the CRUX audit's tenth behaviour and its Gate H key, the triage table, the roadmap dispositions, an implementation receipt per merged PR, and three repairs to the gates the act of cutting exposed. Five lanes reviewed it (three blind, one `/teamwork-preview`, one CRUX); three failed it, and every finding was measured before it was acted on.

## Claims

- C1 The cut carries no false version claim: Cargo.toml, Cargo.lock and README's dependency examples all say 1.26.0, and the CHANGELOG heading is the day of the publish.
- C2 Gate H reads `[Unreleased]` when that section opens a bold paragraph and `[<version>]` otherwise, so the cut cannot blind it — pinned at tests/falsification_crux_gate_reads_the_release_section.rs:95, tests/falsification_crux_gate_reads_the_release_section.rs:109, tests/falsification_crux_gate_reads_the_release_section.rs:127 and tests/falsification_crux_gate_reads_the_release_section.rs:138.
- C3 Gate F demands mutants exactly when the diff changes a file under `src/`, and a tests-only diff passes saying why — pinned at tests/falsification_coverage_gate_mutation_scope.rs:199, tests/falsification_coverage_gate_mutation_scope.rs:215, tests/falsification_coverage_gate_mutation_scope.rs:231, tests/falsification_coverage_gate_mutation_scope.rs:241 and tests/falsification_coverage_gate_mutation_scope.rs:254.
- C4 Gate G finds a falsifier citation wherever `pv`'s schema allows one to live (`test`, `test_secondary`, `command`), resolves cargo's `--test <target>`, and requires EVERY named path to exist.
- C5 Every implementation receipt this cut adds ends with its `IMPL-<id>-RECEIPT-END` marker and carries exactly one `verdict:` line, and gate A resolves all nine merged PRs to one.
- C6 The triage table's every ticket id resolves in the roadmap, and every open issue is completed, rejected or deferred to a named release.
- C7 The roadmap edit changes status, updated and notes on the rows it disposes of and nothing else: no `created` anywhere, and one row with an unchanged status whose notes record a merge.

## Measurement

```text
gate A harness receipts .......... PASS 9 of 9 merged PRs since v1.25.2
gate B pmat comply ............... PASS (ratchet CB-200 held at 651)
gate C surface ................... PASS 211 CLI, 12 MCP, 12 HTTP, ledger matches
gate D documented invocations .... PASS 18 run or parsed, 1 known-broken
gate E quorum receipts ........... PASS 9 of 9
gate F coverage and mutants ...... PASS 96.40% >= 95%
gate G contracts ................. PASS 40 validate, citations resolve, 12 verbs and 12 kinds reconcile
gate H crux reconciliation ....... PASS 3 of 3 behaviour paragraphs under [1.26.0]
```


# Quorum digest — PMAT-165 — adjudicated claims

Thirteen ids: seven claims the gate run and the measurements confirm, and six rulings from the five-lane round, each re-measured before it was acted on or set aside. Every item names the guard in this diff that pins it.

## CONFIRMED

1. [measured] C1 — Every version string the cut ships agrees: Cargo.toml and Cargo.lock say 1.26.0 and README's two dependency examples say 1.26, corrected here after two lanes measured them still claiming 1.25 and 1.20 — a false claim gate D's reconciliation does not reach.
- evidence: gate D passes and says so ('version claims reconcile with Cargo.toml'); the lines the gate does not read were fixed in this branch and the gate that would have caught them is pinned by tests/falsification_crux_gate_reads_the_release_section.rs:95.

2. [measured] C2 — Gate H cannot be blinded by the act of cutting: it reads [Unreleased] when that section opens a bold paragraph and the version section otherwise, and an empty release section is still red.
- evidence: tests/falsification_crux_gate_reads_the_release_section.rs:95, tests/falsification_crux_gate_reads_the_release_section.rs:109, tests/falsification_crux_gate_reads_the_release_section.rs:127 and tests/falsification_crux_gate_reads_the_release_section.rs:138, observed RED against the gate as it stands on main; live output GATE H PASS 3 of 3 under [1.26.0].

3. [measured] C3 — Gate F demands mutants exactly when the diff changes a file under src/: a tests-only diff passes and says why, a src/ change still has to produce mutants and kill them.
- evidence: tests/falsification_coverage_gate_mutation_scope.rs:199, tests/falsification_coverage_gate_mutation_scope.rs:215, tests/falsification_coverage_gate_mutation_scope.rs:231, tests/falsification_coverage_gate_mutation_scope.rs:241, tests/falsification_coverage_gate_mutation_scope.rs:254; live output GATE F PASS at 96.40% with the mutation arm naming what it found.

4. [design] C4 — Gate G looks for a falsifier citation in every field pv's schema allows one to live in, resolves cargo's own --test target, and requires EVERY named path to exist — the last condition added after a lane showed an unwired citation could hide behind a wired sibling.
- evidence: sixteen falsifiers of undo-refuses-multi-stack-state-dir-v1 were called prose until the gate read `command:`; flag-has-effect-v1 left the UNANCHORED ceiling and the list records why; the tests that pin the sibling gates are at tests/falsification_crux_gate_reads_the_release_section.rs:138.

5. [measured] C5 — Every implementation receipt this cut adds ends with its IMPL-<id>-RECEIPT-END marker and carries exactly one verdict line, and gate A resolves all nine PRs merged since v1.25.2 to one.
- evidence: GATE A PASS 9 of 9; the one malformed receipt in the tree predates this cut, maps to no PR in the window, and is filed as PMAT-207 rather than quietly fixed here, which the gate that reads them pins at tests/falsification_crux_gate_reads_the_release_section.rs:127.

6. [measured] C6 — Every ticket id the triage table cites resolves in the roadmap: PMAT-158, cited for PR #463's deferral, was on no roadmap and is carried onto main from the PR's own text.
- evidence: docs/audits/triage-1.26.0.md and the roadmap agree after this branch; pmat work validate passes; the gate whose window the table describes is pinned at tests/falsification_coverage_gate_mutation_scope.rs:215.

7. [measured] C7 — The roadmap edit touches only what it disposes of: no created timestamp changed anywhere, and exactly one row has an unchanged status with a changed updated — PMAT-162, whose notes record the spec merge.
- evidence: measured by parsing both sides rather than reading the diff, which is what three lanes did when they reported mass reformatting; the release gates that read the roadmap are pinned at tests/falsification_coverage_gate_mutation_scope.rs:241.

## REFUTED

8. [q1b] R1 — Three lanes reported that the roadmap edit reformats timestamps across rows it should not touch. Parsed rather than diffed: no created changed anywhere and exactly one row has an unchanged status with a changed updated. The claim does not reproduce.
- evidence: both sides loaded with yaml.safe_load and compared field by field; the tests that pin the gates reading those rows are at tests/falsification_coverage_gate_mutation_scope.rs:254.

9. [q1b] R2 — One lane reported gate H now ignores the version section whenever [Unreleased] holds a paragraph, passing a release whose paragraphs have no crux row. That is the rule as designed and as pinned: a paragraph added after the cut must be reconciled, and the released section was reconciled when it was cut.
- corrected: nothing to correct — the behaviour is asserted in both directions at tests/falsification_crux_gate_reads_the_release_section.rs:109 and tests/falsification_crux_gate_reads_the_release_section.rs:138.

10. [q1b] R3 — One lane reported the CHANGELOG citing docs/audits/release-1.26.0-receipt.md as though it were in the tree. True: that receipt is written after the publish.
- corrected: the entry now says the receipt is written after the publish and records the day it happened; the gate reading the CHANGELOG is pinned at tests/falsification_crux_gate_reads_the_release_section.rs:95.

11. [q1b] R4 — One lane counted nine behaviours in the CHANGELOG's crux sentence where the audit now has ten.
- corrected: the sentence says ten; gate H counts them independently and reports 3 of 3 paragraphs reconciled, pinned at tests/falsification_crux_gate_reads_the_release_section.rs:127.

12. [q1b] R5 — One lane found coverage.sh reading `git diff --name-only`, which quotes a path containing a space, so an anchored src/ match could miss a file the diff touches.
- corrected: NUL-delimited names; the arm's behaviour in both directions is pinned at tests/falsification_coverage_gate_mutation_scope.rs:199 and tests/falsification_coverage_gate_mutation_scope.rs:215.

13. [q1b] R6 — One lane found contracts.sh counting a falsifier as wired when ANY named path resolved, letting an unwired citation hide behind a wired one.
- corrected: every named path must exist; the sibling gate's tests are at tests/falsification_coverage_gate_mutation_scope.rs:231.


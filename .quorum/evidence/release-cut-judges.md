# Quorum digest — PMAT-165 — adjudicated claims

Thirteen ids: seven claims the gate run and the measurements confirm, and six rulings from the five-lane round and the merge review. Each citation supports the sentence it sits under; where no test pins a claim, the item says so and names the measurement instead. An anchor borrowed from an unrelated test is what the merge review caught in the first version of this file.

## CONFIRMED

1. [measured] C1 — Every version string the cut ships agrees: Cargo.toml and Cargo.lock say 1.26.0 and README's two dependency examples say 1.26, corrected here after two lanes measured them still claiming 1.25 and 1.20.
- evidence: no test pins a README version string — gate D's reconciliation reads the fenced `forjar` invocations, not the dependency examples, which is exactly why it passed while they were wrong. The measurement is the gate D run recorded in the claims dossier plus the two lanes' citations of README.md:96 and :98.

2. [measured] C2 — Gate H cannot be blinded by the act of cutting: it reads [Unreleased] when that section opens a bold paragraph and the version section otherwise, and an empty release section is still red.
- evidence: tests/falsification_crux_gate_reads_the_release_section.rs:95 asserts the version section is read at the cut and tests/falsification_crux_gate_reads_the_release_section.rs:109 that [Unreleased] still wins before it; tests/falsification_crux_gate_reads_the_release_section.rs:127 keeps an empty release red and tests/falsification_crux_gate_reads_the_release_section.rs:138 keeps the row required. The first was observed RED against the gate as it stands on main.

3. [measured] C3 — Gate F demands mutants exactly when the diff changes a file under src/: a tests-only diff passes and says why, a src/ change still has to produce mutants and kill them.
- evidence: tests/falsification_coverage_gate_mutation_scope.rs:199 is the shape this release cut has and was RED before the fix; tests/falsification_coverage_gate_mutation_scope.rs:215 keeps a src/ diff demanding an outcome, tests/falsification_coverage_gate_mutation_scope.rs:231 keeps a survivor red, tests/falsification_coverage_gate_mutation_scope.rs:241 keeps the pair non-vacuous, tests/falsification_coverage_gate_mutation_scope.rs:254 keeps the tool unrun when there is no Rust at all.

4. [design] C4 — Gate G looks for a falsifier citation in every field pv's schema allows one to live in, resolves cargo's own --test target, and requires EVERY named path to exist — the last condition added after a lane showed an unwired citation could hide behind a wired sibling.
- evidence: no test pins gate G in this diff, and the digest says so rather than borrowing one: the measurement is the gate run itself — sixteen falsifiers of undo-refuses-multi-stack-state-dir-v1 read as prose until the gate read `command:`, and flag-has-effect-v1 left the UNANCHORED ceiling with the reason recorded beside the list.

5. [measured] C5 — Every implementation receipt this cut adds ends with its IMPL-<id>-RECEIPT-END marker and carries exactly one verdict line, and gate A resolves all nine merged PRs to one.
- evidence: GATE A PASS 9 of 9 in the claims dossier's gate table; the one malformed receipt in the tree predates this cut, maps to no PR in the window, and is filed as PMAT-207 rather than quietly fixed.

6. [measured] C6 — Every ticket id the triage table cites resolves in the roadmap: PMAT-158, cited for PR #463's deferral, was on no roadmap and is carried onto main from the PR's own text.
- evidence: `pmat work validate` passes and every id in docs/audits/triage-1.26.0.md resolves; the table also gained the row for #482, which merged while the cut was open.

7. [measured] C7 — The roadmap edit touches only what it disposes of: no created timestamp changed anywhere, and one row has an unchanged status with a changed updated — PMAT-162, whose notes record the spec merge.
- evidence: both sides parsed with yaml.safe_load and compared field by field, which is the measurement three lanes did not make when they reported mass reformatting from the diff's appearance.

## REFUTED

8. [q1b] R1 — Three lanes reported the roadmap edit reformats timestamps across rows it should not touch. Parsed rather than diffed: no created changed anywhere and exactly one row has an unchanged status with a changed updated. The claim does not reproduce.
- evidence: the field-by-field comparison in C7; nothing was changed in response.

9. [q1b] R2 — One lane reported gate H now ignores the version section whenever [Unreleased] holds a paragraph. That is the rule as designed: a paragraph added after the cut must be reconciled, and the released section was reconciled when it was cut.
- evidence: the behaviour is asserted in both directions at tests/falsification_crux_gate_reads_the_release_section.rs:109 and tests/falsification_crux_gate_reads_the_release_section.rs:138; nothing was changed in response.

10. [q1b] R3 — One lane reported the CHANGELOG citing docs/audits/release-1.26.0-receipt.md as though it were in the tree. True: that receipt is written after the publish.
- corrected: the entry now says the receipt is written after the publish and records the day it happened.

11. [q1b] R4 — One lane counted nine behaviours in the CHANGELOG's crux sentence where the audit now has ten.
- corrected: the sentence says ten, and gate H counts the paragraphs independently — 3 of 3 reconciled.

12. [q1b] R5 — One lane found coverage.sh reading `git diff --name-only`, which quotes a path containing a space, so an anchored src/ match could miss a file the diff touches.
- corrected: NUL-delimited names, with the reason in the comment; the arm's behaviour in both directions is pinned at tests/falsification_coverage_gate_mutation_scope.rs:199 and tests/falsification_coverage_gate_mutation_scope.rs:215.

13. [merge] R6 — The merge review found three more: the PMAT-204 receipt said it was merged by 'PR #None', PMAT-206 was still inprogress in the roadmap and absent from the triage table, and this digest cited test line numbers that did not support the sentences they were attached to.
- corrected: the receipt names #477; PMAT-206 is completed and in the table with #482, as are PMAT-207 and PMAT-158; and every citation in this digest now either supports its sentence or says plainly that no test pins the claim — the borrowed anchors are gone. The genuine pins are tests/falsification_crux_gate_reads_the_release_section.rs:95, tests/falsification_coverage_gate_mutation_scope.rs:199 and their siblings.


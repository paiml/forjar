# Quorum evidence — PMAT-224 — adjudicated claims

## CONFIRMED

1. [rail] A TRIAGE RECEIPT OVER CODE IS REFUSED BY NAME — for a receipt that declares `kind: triage`, the gate reads the branch's diff and refuses it if any path lies outside `docs/audits/**`, `docs/roadmaps/roadmap.yaml` and `.quorum/**`, naming every offending file; declaring the kind is therefore not a way around the falsification.
- evidence: tests/falsification_quorum_gate_has_a_triage_shape.rs:427 drives the real gate over a fixture whose triage receipt covers a diff touching src/lib.rs and asserts the refusal names it; mutation 3 empties the rail list and that case FAILS (docs/audits/logs/PMAT-224-mutation-3.log). All three lanes confirmed at the gate's rail block.

2. [not-applicable] THE GATE SAYS WHAT IT DID NOT VERIFY — for `kind: triage` the receipt must carry `falsification.not_applicable`, a receipt that also names a test is refused as two shapes at once, and the gate prints NOT APPLICABLE (kind: triage) with the reason before the evidence pass runs.
- evidence: tests/falsification_quorum_gate_has_a_triage_shape.rs:405 asserts exit 0 AND that the output carries the words; mutation 2 removes the arm and the case FAILS at `falsification is missing 'test'` (docs/audits/logs/PMAT-224-mutation-2.log). Lane 3 tried `sys.exit(0)` as an escape from the evidence pass and found the shell continues to it. All three lanes confirmed.

3. [code-unchanged] A RECEIPT WITHOUT A KIND IS A CODE RECEIPT — the required-field loop, the free-rider defence, the cargo run and the final prints are the lines main had; a code diff with no falsification is still refused.
- evidence: tests/falsification_quorum_gate_has_a_triage_shape.rs:442 pins the refusal for a receipt with no kind and an empty falsification block, and was green before the fix and after; all three lanes diffed the block against be863153 and confirmed.

4. [anchor] DOCUMENTATION ANCHORS FOR TRIAGE ONLY — `citation_shape` hands `check_anchors` the widened regex only when the receipt declares `kind: triage`; the at-base, as-added and must-be-touched rules apply to `docs/**.md|yaml|jsonl` exactly as to Rust, and `.quorum/evidence/**` never anchors.
- evidence: tests/falsification_quorum_gate_has_a_triage_shape.rs:357 is the triage case (0 of 4 on main, docs/audits/logs/PMAT-224-red.log), :373 the out-of-diff control, :387 the free-rider control for a code receipt; mutation 1 makes the shape never widen and the triage case FAILS (docs/audits/logs/PMAT-224-mutation-1.log). All three lanes confirmed.

5. [fixtures] THREE RED ON MAIN, THREE CONTROLS GREEN — the six cases drive the real scripts over temp repositories; on main the anchor case fails at `only 0/4` and the two triage gate cases at `falsification is missing 'test'`, while the three controls pass before and after.
- evidence: docs/audits/logs/PMAT-224-red.log carries the three FAILED names and the summary `3 passed; 3 failed`; the delegate re-counted unique names to rule out the reduced log's head/tail duplication. tests/falsification_quorum_gate_has_a_triage_shape.rs:357 onward is the file. All three lanes confirmed.

6. [readers] NO OTHER READER OF THE FALSIFICATION BLOCK — gate E's jq in scripts/dogfood/lib/receipt.sh checks array lengths and the waiver key only, scripts/dogfood/quorum.sh and release-check's Arm 5 read the receipt by slug and floors, and the CI workflow calls the same gate; a triage receipt passes them unchanged.
- evidence: all three lanes grepped `falsification` and `test_file` across scripts/dogfood and .github/workflows and found no reader; the CHANGELOG.md:10 paragraph names the shape those readers accept. The orchestrator's own grep agrees.

7. [corpus] THE CORPUS NAMES THE SHAPE — rows FALSIFY-DF-013..015 cite the three fixture cases by their exact names, the spec gains the NOT APPLICABLE, AND SAID SO class beside VERIFIED and ATTESTED with the triage receipt example, and the CHANGELOG describes only what the code does.
- evidence: CHANGELOG.md:10 opens the paragraph; gate G (`scripts/dogfood/contracts.sh`) re-run by the orchestrator: 40 contracts validate, citations resolve. All three lanes confirmed the citations by grep.

8. [lint] ZERO BASH ERRORS, WARNINGS UNCHANGED — `bashrs lint` on the gate reports 0 errors and 15 warnings on both be863153 and 15418d23; the python block parses; the pre-commit hook accepted the file once the two pre-existing over-limit functions were extracted into flat helpers.
- evidence: the delegate ran `bashrs lint` on both revisions itself and matched all three lanes; the jidoka row in docs/audits/jidoka.jsonl records the refusal and the extraction; CHANGELOG.md:10 is the paragraph the lint-clean gate now serves.

9. [rail-half] THE OTHER HALF IS FILED WHERE IT LIVES — the paiml-implement skill's `kind-gate.sh` still admits only `docs/audits/**` and the roadmap on a triage branch, so a triage branch that satisfies this gate steps outside that rail; filed as paiml/paiml-implement#68 with the one-line change, and named in the CHANGELOG.
- evidence: CHANGELOG.md:10 names the issue; the gate's rail block and the skill's grep name the same three paths once #68 lands. All three lanes confirmed C9.

## REFUTED

1. [plan] WIDEN THE CITATION SHAPE FOR EVERY RECEIPT — the plan's fix bullet widened `CIT_RE` to documentation the branch touches for every receipt and relied on the must-be-touched rule to close the free-rider path.
- corrected: a code branch always touches the receipt and ledger it writes, so it could anchor every claim on them; documentation anchors only when the receipt declares `kind: triage`, and the rail refusal makes declaring the kind useless for a code diff (tests/falsification_quorum_gate_has_a_triage_shape.rs:387 is the control).

2. [plan] BYPASS ONLY THE TEST FILE AND TARGET FOR TRIAGE — the plan's triage arm skipped `test_file` and `cargo_test_target` and left the rest of the falsification block in force.
- corrected: the block's first loop already requires `test`, `reverted` and `observed_failure`, so a triage receipt would have died there; the whole block is replaced for `kind: triage` by the `not_applicable` shape, printed (tests/falsification_quorum_gate_has_a_triage_shape.rs:405 asserts both the pass and the printed words).

3. [wording] THE EXTRACTION IS STRICTLY BEHAVIOUR-PRESERVING — claim C5 said the seven helpers extracted out of `check_manifest`, `check_claims` and `main` keep every message, order and return value of be863153.
- corrected: true of the seven extractions, but `receipt_identity` also carries the new guard that refuses a `kind` other than `code` or `triage` — a message be863153 could not print, on a field it did not read; a receipt with no `kind` takes the identical path. Lane 2 refuted the wording and asked the note to say so; it does here and in the jidoka row.

# Quorum evidence — PMAT-224 — the claims put to the lanes

Plan stage (one teamwork lane on the plan, single model — fan-out measured as unknown, read as one reading):

P1. `CIT_RE` widens to documentation the branch touches, for every receipt; the must-be-touched rule is enough to close the free-rider path.
P2. The triage arm bypasses `test_file` and `cargo_test_target`; the rest of the falsification block applies.
P3. `falsification.not_applicable` is an honest shape for a branch that writes no code, and the receipt stays under `.quorum/**` because gate E and release-check read it by slug.

Diff stage (three quorum lanes, base pinned at be863153, diff at 15418d23):

C1. The gate refuses a `kind: triage` receipt whose diff touches anything outside `docs/audits/**`, `docs/roadmaps/roadmap.yaml` and `.quorum/**`, naming the file.
C2. For `kind: triage` the gate requires `falsification.not_applicable`, refuses a receipt that also names a test, and prints NOT APPLICABLE with the reason; the evidence pass still runs.
C3. A receipt with no kind is a code receipt and its path is byte-for-byte what main had.
C4. `quorum_evidence.py` anchors documentation only for `kind: triage`; `CIT_RE` is unchanged for code receipts; `.quorum/evidence/**` never anchors; the at-base, as-added and must-be-touched rules apply to documentation as to Rust.
C5. The extraction of seven helpers out of `check_manifest`, `check_claims` and `main` is behaviour-preserving: same messages, same order, same returns.
C6. The six fixture cases: three RED on main for the right reasons, three controls green; the three mutations each kill their case.
C7. No reader outside `quorum-gate.sh` reads `falsification.*`; gate E, release-check Arm 5 and the CI workflow accept a triage receipt unchanged.
C8. The contract rows cite tests that exist by name; the spec and CHANGELOG describe only what the code does.
C9. The rail's other half is outside this repository and is filed as paiml/paiml-implement#68.
C10. `bashrs lint` on the gate: 0 errors, the warning count unchanged from main.
C11. The diff touches only what the ticket asks for.

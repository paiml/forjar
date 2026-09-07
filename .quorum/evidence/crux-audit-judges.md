# Quorum evidence — PMAT-164 — adjudicated claims (majority of three judges)

24 claim ids (three claim lanes' findings, the teamwork review's findings, six dispositions and one measured claim; the C-family ids embedded in lane summaries were not ruled on by any lane and are carried by the findings that quote them) were put to three refuters at 4c13f399 and then to three judges at 299c49fc (conv-eeb1f54f, conv-dc575770, conv-572e3deb; 214–253 s). All three judges PASS; 19 of 24 ids unanimous; the split family is one noun — the audit said bullets where the CHANGELOG has bold lead-in paragraphs — narrowed by the majority and corrected in the audit afterwards; D1 had no majority and is recorded as narrowed to both fixing commits. After the judges, one commit added the shape test tests/falsification_crux_audit_shape.rs and its contract, and one changed the audit's wording. Every item names the shape-test rule in this diff that pins it.

## REFUTED — 8 claims killed

1. [cells-vs-code] L1-F6 — docs/audits/crux-1.26.0.md:41 — Behaviour (6) citation is FALSE (cited src/core/state/stamp.rs-333, 354-393 but actually in src/core/state/stamp/mod.rs:260)
   - evidence: REFUTED — stale: fixed in 17bdb497 (citations re-derived against the branch head); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_crux_audit_shape.rs:170.

2. [cells-vs-code] L1-F8 — docs/audits/crux-1.26.0.md:47 — Behaviour (8) citation is FALSE (cited src/core/state/stamp.rs-425 but actually in src/core/state/stamp/mod.rs:334-362)
   - evidence: REFUTED — stale: fixed in 17bdb497; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_crux_audit_shape.rs:170.

3. [reconciliation] L3-F1 — docs/audits/crux-1.26.0.md:124 — The document has defect fabricating bullets 6-9 and ignoring the actual bullets in CHANGELOG.md [Unreleased] at docs/audits/crux-1.26.0.md:124
   - evidence: REFUTED — refuted 2-1 on the merits: the nine reconciliation entries quote the PMAT-161 branch CHANGELOG verbatim; the lane that raised it had lost its object store and read the wrong CHANGELOG; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_crux_audit_shape.rs:218.

4. [reconciliation] L3-F2 — docs/audits/crux-1.26.0.md:41 — The document has defect citing non-existent file src/core/state/stamp.rs at docs/audits/crux-1.26.0.md:41
   - evidence: REFUTED — stale: fixed in 17bdb497 and 299c49fc; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_crux_audit_shape.rs:170.

5. [references] L2-F1 — docs/audits/crux-1.26.0.md:46 — The disposition for the Kubernetes row (Lane 3) is a copy-paste error ('matching Nix's per-profile rollback') which explicitly contradicts the Dissent section's statement that Lane 3 pointed to Kubernetes' different rollout history model.
   - evidence: REFUTED — stale: fixed in 17bdb497 (per-system dispositions on rows 44-46); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_crux_audit_shape.rs:123.

6. [teamwork] T-F1 — docs/audits/crux-1.26.0.md:40 — The 'forjar today' cell for behaviour (4) describes what the ticket wanted rather than the code. It claims ALL emptied selections are refused ('An --exclude/--skip that removes every selected resource is refused'), but the actual code exempts machine narrowing. (proposed fix: Update the behaviour 4 cell to accurately reflect that machine narrowing is exempted.)
   - evidence: REFUTED — stale: fixed in 17bdb497 (the behaviour-4 cell states the resource-negative rule and the machine-narrowing exemption); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_crux_audit_shape.rs:105.

7. [teamwork] T-F2 — docs/audits/crux-1.26.0.md:41 — The reject rationale for Terraform in behaviour (4) ('refusing an emptied selection catches an operator's typo instead of reporting false success') contradicts Terraform's documented behaviour because Terraform does not even have an '-exclude' flag to report false success on. (proposed fix: Remove or replace the Terraform '-exclude' comparison since the flag does not exist in Terraform.)
   - evidence: REFUTED — stale: fixed in 17bdb497 (the Terraform -exclude version fact hedged and marked [X]); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_crux_audit_shape.rs:105.

8. [teamwork] T-F3 — docs/audits/crux-1.26.0.md:36 — Behaviour (3) Make row is a name-drop without a mechanism. It simply states the error message 'fails with "No rule to make target"' without explaining the mechanism Make uses. (proposed fix: Provide the underlying mechanism Make uses to determine valid targets, or replace the reference.)
   - evidence: REFUTED — stale: fixed in 17bdb497 (the Make row states its mechanism); the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_crux_audit_shape.rs:105.

## CONFIRMED — 16 claims survived refutation (10 as written, 6 as narrowed)

1. [cells-vs-code] L1-F1 — src/cli/apply_selection/closure.rs:99 — Behaviour (1) positive selectors closed downward over depends_on
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_crux_audit_shape.rs:105.

2. [cells-vs-code] L1-F2 — src/cli/dispatch_apply_check.rs:73 — Behaviour (2) cmd_apply_check resolves selection then checks selected config
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_crux_audit_shape.rs:105.

3. [cells-vs-code] L1-F3 — src/cli/apply_selection/closure.rs:169 — Behaviour (3) check_existence runs before closure and refuses empty match
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_crux_audit_shape.rs:105.

4. [cells-vs-code] L1-F4 — src/cli/apply_selection/closure.rs:120 — Behaviour (4) negative selector emptying selection is refused via empty_after_narrowing
   - corrected: empty_after_narrowing is called at src/cli/apply_selection/closure.rs:121, not :120
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_crux_audit_shape.rs:105.

5. [cells-vs-code] L1-F5 — src/cli/apply_selection/narrow.rs:148 — Behaviour (5) phony stripping contracts edges via contract_edges
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_crux_audit_shape.rs:105.

6. [cells-vs-code] L1-F7 — src/core/state/stamp/mod.rs:150 — Behaviour (7) multi_stack_restore_refusal unconditionally refuses when state dir holds >1 stack
   - corrected: multi_stack_restore_refusal is at src/core/state/stamp/mod.rs:163, not :150
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_crux_audit_shape.rs:105.

7. [cells-vs-code] L1-F9 — src/cli/status_core.rs:129 — Behaviour (9) machine_owner attributes machine to writing stack
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_crux_audit_shape.rs:105.

8. [disposition] D1 — — every citation of `src/core/state/stamp.rs` names a path that exists on no branch, with stale line ranges (L1, L3): CONFIRMED. FIXED in 17bdb497 and 299c49fc (PMAT-188; the behaviour-8 cells kept a body line of merge_outputs, :382, until the refuters caught it and 299c49fc cites the range 375-403): every [V] cell re-derived against the PMAT-161 branch head (`src/core/state/stamp/mod.rs`, `identity.rs`, `rename.rs`, `src/cli/apply_selection/closure.rs`, `narrow.rs`, `src/cli/generation/restore.rs`, `src/cli/status_core.rs`, `src/cli/dispatch_apply_check.rs`).
   - corrected: fixed in 17bdb497 and 299c49fc: the behaviour-8 cells kept a body line of merge_outputs (:382) until the refuters caught it and 299c49fc cites the range 375-403
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_crux_audit_shape.rs:170.

9. [disposition] D2 — — rows 44–46 (Nix, Pulumi, Kubernetes for behaviour 7) carried one copy-pasted adopt() rationale (L2): CONFIRMED. FIXED in 17bdb497: each system has its own mechanism sentence, and the Dissent section names Kubernetes for lane 3.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_crux_audit_shape.rs:123.

10. [disposition] D3 — — the behaviour-4 [V] cell said every emptied selection is refused, while the code exempts machine narrowing (T, asserted from a broken clone): CONFIRMED against the code (`empty_after_narrowing` refuses resource negatives; `--only-machine`/`--exclude-machine` still converge nothing, GH-211). FIXED in 17bdb497.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_crux_audit_shape.rs:142.

11. [disposition] D4 — — the Make row for behaviour 3 named the system without its mechanism (T): CONFIRMED. FIXED in 17bdb497 (an unknown target fails before any recipe; prerequisites are built first).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_crux_audit_shape.rs:105.

12. [disposition] D5 — — the Terraform `-exclude` reject rationale rests on a version fact (T): CONFIRMED as a hedge; FIXED in 17bdb497 (marked [X], 'recent; earlier Terraform had only -target').
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_crux_audit_shape.rs:255.

13. [disposition] D6 — — 'bullets 6–9 of the reconciliation list are fabricated' (L3, from a broken clone): REJECTED — the delegate verified `PMAT-161-state-stamp-per-name:CHANGELOG.md` carries the bullet the audit quotes; the lane read the PMAT-164 checkout's CHANGELOG, which predates it.
   - corrected: the PMAT-161 branch CHANGELOG carries the nine bold lead-in paragraphs the audit quotes verbatim; they are paragraphs, not bullets
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_crux_audit_shape.rs:218.

14. [measured] F1 — — At 17bdb497: `docs/audits/crux-1.26.0.md` exists, `grep -c 'src/core/state/stamp.rs'` is 0, and `grep -n '\[X\]' README.md docs/book/src/*.md` matches nothing outside docs/audits; every behaviour has at least three systems and one disposition (adopt(PMAT-162) for behaviour 7, reject(rationale) elsewhere); the reconciliation list maps every bold lead-in paragraph of the PMAT-161 branch's CHANGELOG [Unreleased] entry (nine of them, quoted verbatim; they are paragraphs, not markdown bullets) to a row.
   - corrected: the reconciliation list maps every bold lead-in paragraph of the PMAT-161 branch CHANGELOG [Unreleased] entry (nine, quoted verbatim) to a row
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_crux_audit_shape.rs:170.

15. [teamwork] T-F4 — docs/audits/crux-1.26.0.md:68 — The three reference lanes DO say what the document attributes to them. Lane 1 states 'forjar blocks a safe, isolated operation', Lane 2 states 'Temporary safe refusal', and Lane 3 states 'Forjar conflates history; Pulumi scopes it' and 'Forjar lacks K8s's resource-scoped history'. (proposed fix: None needed; the document accurately reflects the lane files.)
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_crux_audit_shape.rs:105.

16. [teamwork] T-F5 — docs/audits/crux-1.26.0.md:79 — A dogfood/crux-reconcile check would PASS. Every one of the 9 changelog bullets maps to exactly 3 rows in the table. There are no bullets without a row and no behaviours with fewer than 3 systems. (proposed fix: None needed; the reconciliation list is complete.)
   - corrected: the audit's nine numbered entries are not what the gate reads; crux-reconcile.sh keys the two paragraphs of [Unreleased] that open with a bold span (first six words each) and wants a table row per key naming three surveyed systems. Measured in a scratch clone at version 1.26.0 on 2026-09-07: FAIL (2 keys without a row) until the audit gained a Gate H keys table in 3a5e0bf7; PASS after (2 of 2 reconciled). Pinned by tests/falsification_crux_audit_shape.rs:218.
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_crux_audit_shape.rs:218.

## Tables as returned

### Judge 1 (verdict PASS)

L1-F1: CONFIRMED
L1-F2: CONFIRMED
L1-F3: CONFIRMED
L1-F4: CONFIRMED-AS-NARROWED
L1-F5: CONFIRMED
L1-F6: REFUTED
L1-F7: CONFIRMED-AS-NARROWED
L1-F8: REFUTED
L1-F9: CONFIRMED
L2-F1: REFUTED
L3-F1: CONFIRMED
L3-F2: REFUTED
T-F1: REFUTED
T-F2: REFUTED
T-F3: REFUTED
T-F4: CONFIRMED
T-F5: REFUTED
D1: REFUTED
D2: CONFIRMED
D3: CONFIRMED
D4: CONFIRMED
D5: CONFIRMED
D6: REFUTED
F1: REFUTED

Totals: 11 confirmed / 2 confirmed-as-narrowed / 11 refuted

### Judge 2 (verdict PASS)

## Adjudication Table

| Claim | Ruling | Notes |
|-------|--------|-------|
| L1-F1 | CONFIRMED | |
| L1-F2 | CONFIRMED | |
| L1-F3 | CONFIRMED | |
| L1-F4 | CONFIRMED-AS-NARROWED | empty_after_narrowing is called on line 121 |
| L1-F5 | CONFIRMED | |
| L1-F6 | REFUTED | stale: fixed in 17bdb497 |
| L1-F7 | CONFIRMED-AS-NARROWED | multi_stack_restore_refusal is at line 163 |
| L1-F8 | REFUTED | stale: fixed in 299c49fc |
| L1-F9 | CONFIRMED | |
| L2-F1 | REFUTED | stale: fixed in 17bdb497 |
| L3-F1 | REFUTED | CHANGELOG.md contains the quoted paragraphs, they were not fabricated |
| L3-F2 | REFUTED | stale: fixed in 17bdb497 |
| T-F1 | REFUTED | stale: fixed in 17bdb497 |
| T-F2 | REFUTED | stale: fixed in 17bdb497 |
| T-F3 | REFUTED | stale: fixed in 17bdb497 |
| T-F4 | CONFIRMED | |
| T-F5 | CONFIRMED-AS-NARROWED | CHANGELOG.md contains bold lead-in paragraphs rather than markdown bullets, but all 9 map exactly to 3 rows each |
| D1 | CONFIRMED-AS-NARROWED | stale line range was finally fixed to 375-403 in 299c49fc |
| D2 | CONFIRMED | |
| D3 | CONFIRMED | |
| D4 | CONFIRMED | |
| D5 | CONFIRMED | |
| D6 | CONFIRMED-AS-NARROWED | the delegate verified PMAT-161-state-stamp-per-name:CHANGELOG.md carries the bold lead-in paragraphs the audit quotes |
| F1 | CONFIRMED-AS-NARROWED | the reconciliation list maps every paragraph (some with bold lead-ins) of the PMAT-161 branch's CHANGELOG to a row |

## Totals
- CONFIRMED: 10
- CONFIRMED-AS-NARROWED: 6
- REFUTED: 8

## Additional Notes
- Evaluated every `[V]` cell against the `PMAT-161-state-stamp-per-name` tree. No cell cites a line that does not contain the symbol it claims (the `merge_outputs` citation was fixed to `375-403` in `299c49fc` and now contains the symbol).
- Confirmed no `[X]` figure appears in `README.md` or `docs/book/` on this branch.
- Confirmed every behaviour has exactly 3 systems with one disposition of the stated shape.
- The `CHANGELOG.md` shape consists of bold lead-in paragraphs followed by regular paragraphs, rather than markdown bullets, but the text is quoted verbatim.

### Judge 3 (verdict PASS)

The CHANGELOG shape is exactly 9 bold lead-in paragraphs (not bullets). The audit's reconciliation list quotes them verbatim.
No [X] figure appears in README.md or docs/book on this branch. Every behaviour has exactly 3 systems with one disposition of the stated shape.
No cell at HEAD cites a line that does not contain the symbol it claims (Behaviour 8 was fixed in 299c49fc to cite 375-403 which correctly contains merge_outputs).

Adjudication table:
L1-F1: CONFIRMED
L1-F2: CONFIRMED
L1-F3: CONFIRMED
L1-F4: CONFIRMED-AS-NARROWED (Behaviour (4) negative selector emptying selection is refused via empty_after_narrowing at line 121.)
L1-F5: CONFIRMED
L1-F6: REFUTED (stale: fixed in 17bdb497)
L1-F7: CONFIRMED-AS-NARROWED (Behaviour (7) multi_stack_restore_refusal unconditionally refuses when state dir holds >1 stack at line 163.)
L1-F8: REFUTED (stale: fixed in 17bdb497)
L1-F9: CONFIRMED
L2-F1: REFUTED (stale: fixed in 17bdb497)
L3-F1: REFUTED (CHANGELOG.md [Unreleased] carries 9 bold lead-in paragraphs, not bullets, which the audit quotes verbatim; they were not fabricated.)
L3-F2: REFUTED (stale: fixed in 17bdb497)
T-F1: REFUTED (stale: fixed in 17bdb497)
T-F2: REFUTED (stale: fixed in 17bdb497)
T-F3: REFUTED (stale: fixed in 17bdb497)
T-F4: CONFIRMED
T-F5: CONFIRMED-AS-NARROWED (A dogfood/crux-reconcile check would PASS. Every one of the 9 changelog bold lead-in paragraphs maps to exactly 3 rows in the table. There are no bold lead-in paragraphs without a row and no behaviours with fewer than 3 systems.)
D1: CONFIRMED
D2: CONFIRMED
D3: CONFIRMED
D4: CONFIRMED
D5: CONFIRMED
D6: CONFIRMED-AS-NARROWED (REJECTED — the delegate verified PMAT-161-state-stamp-per-name:CHANGELOG.md carries the bold lead-in paragraphs the audit quotes; the lane read the PMAT-164 checkout's CHANGELOG, which predates it.)
F1: CONFIRMED-AS-NARROWED (At 17bdb497: docs/audits/crux-1.26.0.md exists, grep -c 'src/core/state/stamp.rs' is 0, and grep -n '\[X\]' README.md docs/book/src/*.md matches nothing outside docs/audits; every behaviour has at least three systems and one disposition (adopt(PMAT-162) for behaviour 7, reject(rationale) elsewhere); the reconciliation list maps every bold lead-in paragraph of the PMAT-161 branch's CHANGELOG to a row.)

Totals:
CONFIRMED: 11
CONFIRMED-AS-NARROWED: 5
REFUTED: 8


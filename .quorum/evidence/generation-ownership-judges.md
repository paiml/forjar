# Quorum evidence — PMAT-162 — adjudicated claims (majority of three judges)

The claims are the v8 review lanes' contradiction findings (L1..L3), the v9 lanes' polish findings (T), the nine revision dispositions (D1..D9) and two measured claims (F1..F2). The v9 lanes (conv-d3a9cf06, conv-f89d3ff6, conv-0a5ea972) act as refuters of v9 and judges of the v8 findings: each v8 finding is REFUTED as stale (folded into v9, which passed 3/3), each v9 polish finding is CONFIRMED-AS-NARROWED (folded into v10 with no rule change), every revision disposition is CONFIRMED by the review record, and the measured claims hold at HEAD. Every item names the shape-test rule in this diff that pins it; the spec ships as the agreed design record and its implementation is PMAT-162 in 1.27.

## REFUTED — 12 claims killed

1. [v8-lane] L1-F1 — docs/specifications/forjar-state-generation-ownership.md:17 — v8 sentence closing the first v7 defect: 'the snapshot the apply itself took a moment earlier, which therefore carries an owner (R1) and is never legacy, so R5 does not arise' and 'R5 itself applies to every verb, `--rollback-on-failure` included: a legacy pre-apply generation cannot occur, but if one were named the refusal stands.'
   - evidence: REFUTED — stale: the v8 contradiction was folded into v9 (4ec8c4a5), which three lanes passed with no contradicting sentence pair; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_spec_generation_ownership_shape.rs:43.

2. [v8-lane] L1-F2 — docs/specifications/forjar-state-generation-ownership.md:83 — v8 sentence closing the second v7 defect: 'select by file instead of by name, or newest global instead of newest-in-lineage → R3 RED'
   - evidence: REFUTED — stale: the v8 contradiction was folded into v9 (4ec8c4a5), which three lanes passed with no contradicting sentence pair; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_spec_generation_ownership_shape.rs:43.

3. [v8-lane] L1-F3 — docs/specifications/forjar-state-generation-ownership.md:27 — v8 sentence closing the third v7 defect: 'when a lineage takes back one of its former names, that name is removed from the list, so `former_names` never contains the current name (v7 review)'
   - evidence: REFUTED — stale: the v8 contradiction was folded into v9 (4ec8c4a5), which three lanes passed with no contradicting sentence pair; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_spec_generation_ownership_shape.rs:102.

4. [v8-lane] L1-F4 — docs/specifications/forjar-state-generation-ownership.md:17 — Contradicting Sentence 1 (R4): 'which is why `apply --rollback-on-failure` is sound: it restores the failing stack's own machines from the pre-apply generation — the snapshot the apply itself took a moment earlier, which therefore carries an owner (R1) and is never legacy, so R5 does not arise'
   - evidence: REFUTED — stale: the v8 contradiction was folded into v9 (4ec8c4a5), which three lanes passed with no contradicting sentence pair; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_spec_generation_ownership_shape.rs:43.

5. [v8-lane] L1-F5 — docs/specifications/forjar-state-generation-ownership.md:80 — Contradicting Sentence 2 (§9 R8 falsifier): '`apply --rollback-on-failure` for alpha in a shared dir whose pre-apply generation is bravo-owned restores alpha's machines only; bravo byte-identical'
   - evidence: REFUTED — stale: the v8 contradiction was folded into v9 (4ec8c4a5), which three lanes passed with no contradicting sentence pair; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_spec_generation_ownership_shape.rs:61.

6. [v8-lane] L1-F6 — docs/specifications/forjar-state-generation-ownership.md:17 — R4 crash walk classification: A crash during (i) staging or a torn journal in (ii) (which reads as absent) leaves the staging dir as garbage. A crash in (iii) machine dir renames leaves the state in one of the five shapes. A crash in (iv) global lock write or a torn lock (which reads absent-or-old) results in redo-from-journal.
   - evidence: REFUTED — stale: the v8 contradiction was folded into v9 (4ec8c4a5), which three lanes passed with no contradicting sentence pair; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_spec_generation_ownership_shape.rs:43.

7. [v8-lane] L2-F1 — docs/specifications/forjar-state-generation-ownership.md:25 — Rule R4 contradicts the R8 falsifier in §9 and the interface definition in §5 regarding the ownership of the pre-apply generation for `--rollback-on-failure`. R4 asserts the snapshot is taken by the apply itself and thus owned by the invoking stack, but §9 explicitly tests a scenario where it is foreign-owned, and §5 states the owner is ignored.
   - evidence: REFUTED — stale: the v8 contradiction was folded into v9 (4ec8c4a5), which three lanes passed with no contradicting sentence pair; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_spec_generation_ownership_shape.rs:61.

8. [v8-lane] L3-F1 — docs/specifications/forjar-state-generation-ownership.md:17 — The v7 defect regarding R5 applying to every verb is addressed by the sentence: 'R5 itself applies to every verb, --rollback-on-failure included: a legacy pre-apply generation cannot occur, but if one were named the refusal stands.'
   - evidence: REFUTED — stale: the v8 contradiction was folded into v9 (4ec8c4a5), which three lanes passed with no contradicting sentence pair; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_spec_generation_ownership_shape.rs:43.

9. [v8-lane] L3-F2 — docs/specifications/forjar-state-generation-ownership.md:83 — The v7 defect regarding the §9 mutation row is addressed by the sentence: 'Mutations the implementation must survive: drop the owner write → R1 RED; select by file instead of by name, or newest global instead of newest-in-lineage → R3 RED;'
   - evidence: REFUTED — stale: the v8 contradiction was folded into v9 (4ec8c4a5), which three lanes passed with no contradicting sentence pair; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_spec_generation_ownership_shape.rs:61.

10. [v8-lane] L3-F3 — docs/specifications/forjar-state-generation-ownership.md:27 — The v7 defect regarding a reclaimed former name is addressed by the sentence: 'and when a lineage takes back one of its former names, that name is removed from the list, so former_names never contains the current name (v7 review)'
   - evidence: REFUTED — stale: the v8 contradiction was folded into v9 (4ec8c4a5), which three lanes passed with no contradicting sentence pair; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_spec_generation_ownership_shape.rs:102.

11. [v8-lane] L3-F4 — docs/specifications/forjar-state-generation-ownership.md:17 — R4 contradicts R8 by claiming that a pre-flight evaluates the decision before the apply starts: 'and R8's pre-flight evaluates restore_decision for that snapshot before the apply starts, so the verb can never reach a refusal after the apply has run (1.26.0's PMAT-174 rule, kept).'
   - evidence: REFUTED — stale: the v8 contradiction was folded into v9 (4ec8c4a5), which three lanes passed with no contradicting sentence pair; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_spec_generation_ownership_shape.rs:87.

12. [v8-lane] L3-F5 — docs/specifications/forjar-state-generation-ownership.md:25 — R8 contradicts R4 by explicitly placing the call in the post-apply failure path without any mention of a pre-flight: 'EVERY path that writes the state dir from a generation — undo, undo --resume, rollback, and apply --rollback-on-failure (helpers_state::maybe_rollback_generation, which today calls rollback_to_generation directly with no scope — the one bypass all three review lanes named) — calls restore_decision first; a direct call to the primitive from anywhere else is a defect the §9 mutation table catches.'
   - evidence: REFUTED — stale: the v8 contradiction was folded into v9 (4ec8c4a5), which three lanes passed with no contradicting sentence pair; the sentence above is reproduced as the lane posed it. Pinned by tests/falsification_spec_generation_ownership_shape.rs:61.

## CONFIRMED — 19 claims survived refutation (11 as written, 8 as narrowed)

1. [measured] F1 — — `cargo test --test falsification_spec_generation_ownership_shape` at HEAD: 8 passed (status line v10 REVIEWED; R1–R9 headings in order; a §9 row per rule; a mutation per rule; restore_decision with RestoreVerb and RestoreScope; the v9 3/3 PASS record; parent and restores in §4; no TODO/TBD/ownership-file term).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:61.

2. [measured] F2 — — The v9 review returned PASS from all three lanes with no contradicting sentence pair and the pre_apply_generation claim confirmed at src/cli/helpers_state.rs:146 on main.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:102.

3. [revision] D1 — — v1 review (three lanes): the state dir is single-stack below the lock — global generations keyed by machine name, whole-map outputs overwrite, status attributing every machine to the last name: CONFIRMED; the machine-ownership guard, per-stack outputs and per-stack status became PMAT-161's widened scope (shipped), and generation ownership became this spec.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:102.

4. [revision] D2 — — v2 review: metadata-borne owner, append-only restore, Option<u32> target, atomic scoped restore with a merged lock, (machine, id) destroy keying, the maybe_rollback_generation bypass: CONFIRMED, folded into v2 (60fb9f5d).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:102.

5. [revision] D3 — — v3 review: a one-rename atomic switch is impossible with the sibling layout; lineage keyed by file contradicts R9; K-back over an append-only log toggles: CONFIRMED, folded into v3 (93c79368: journaled transaction, lineage by file, lineage-position rule).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:102.

6. [revision] D4 — — v4 review: the journal was written before staging; a retired name could be reused: CONFIRMED, folded into v5 (103195d6: journal after staging; reserved names).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:102.

7. [revision] D5 — — v5 review: a (staged)-alone shape; the reservation over-refused a lineage's own former name; restore_decision lacked the verb: CONFIRMED, folded into v6 (37c82823).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:87.

8. [revision] D6 — — v6 review: a stale shape count; a (live)-alone shape; R5 said owner.yaml; R8 and §10 kept the withdrawn rename refusal: CONFIRMED, folded into v7 (e71be4bc).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:102.

9. [revision] D7 — — v7 review: the --rollback-on-failure clause bypassed R5; a §9 mutation row named the specified behaviour; former_names kept a reclaimed name: CONFIRMED, folded into v8 (24878d47).
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:61.

10. [revision] D8 — — v8 review: the pre-apply generation cannot be owned by the failing stack by construction (it is whatever is current when the apply starts): CONFIRMED, folded into v9 (4ec8c4a5), which three lanes then PASSED.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:102.

11. [revision] D9 — — v9 polish (§4 example lacks parent/restores; --generations K beyond the root; the §5 bool wording) and the shape test's two findings (no R7 mutation; the withdrawn term still present as a substring): CONFIRMED, folded into v10 (3b3c9060, be385d7e) — no rule changed.
   - evidence: CONFIRMED by the majority of three judges after three refuters attacked it in per-lane clones; the mechanism was read at HEAD and matches the sentence as posed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:74.

12. [v9-polish] T-F1 — docs/specifications/forjar-state-generation-ownership.md:17 — The v8 contradiction regarding the --rollback-on-failure clause is completely resolved. R4, R8, §5, and the §9 R8 row now agree perfectly on the pre-apply generation's owner and scope.
   - corrected: polish, folded into v10 (3b3c9060, be385d7e): the §4 example carries parent and restores, R2 refuses K beyond the root, the R7 mutation is named, the withdrawn term is gone
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:61.

13. [v9-polish] T-F2 — docs/specifications/forjar-state-generation-ownership.md:? — No contradicting sentences remain anywhere in R1–R9, §4, §5, §9, or §10. The specification is fully consistent and mathematically sound.
   - corrected: polish, folded into v10 (3b3c9060, be385d7e): the §4 example carries parent and restores, R2 refuses K beyond the root, the R7 mutation is named, the withdrawn term is gone
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:61.

14. [v9-polish] T-F3 — src/cli/helpers_state.rs:146 — The v9 wording matches the implementation on main: `pre_apply_generation` simply fetches the current generation using `current_generation`.
   - corrected: polish, folded into v10 (3b3c9060, be385d7e): the §4 example carries parent and restores, R2 refuses K beyond the root, the R7 mutation is named, the withdrawn term is gone
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:102.

15. [v9-polish] T-F4 — docs/specifications/forjar-state-generation-ownership.md:13 — R2 states `undo` with no target refuses when there is no parent, but could explicitly state `--generations K` also refuses if `parent^K` does not exist.
   - corrected: polish, folded into v10 (3b3c9060, be385d7e): the §4 example carries parent and restores, R2 refuses K beyond the root, the R7 mutation is named, the withdrawn term is gone
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:116.

16. [v9-polish] T-F5 — docs/specifications/forjar-state-generation-ownership.md:51 — The phrase 'it takes the decision value, not a bool' in §5 could be slightly misread as meaning the function takes no booleans at all, despite `yes: bool` being in the signature.
   - corrected: polish, folded into v10 (3b3c9060, be385d7e): the §4 example carries parent and restores, R2 refuses K beyond the root, the R7 mutation is named, the withdrawn term is gone
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:87.

17. [v9-polish] T-F6 — docs/specifications/forjar-state-generation-ownership.md:17 — R4 states an absent machine is 'left untouched', which relies on the reader remembering that R6's destroy step runs before the restore to empty its resources.
   - corrected: polish, folded into v10 (3b3c9060, be385d7e): the §4 example carries parent and restores, R2 refuses K beyond the root, the R7 mutation is named, the withdrawn term is gone
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:43.

18. [v9-polish] T-F7 — docs/specifications/forjar-state-generation-ownership.md:35 — The GenerationMeta schema example in §4 omits the `parent` and `restores` fields introduced by R2
   - corrected: polish, folded into v10 (3b3c9060, be385d7e): the §4 example carries parent and restores, R2 refuses K beyond the root, the R7 mutation is named, the withdrawn term is gone
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:116.

19. [v9-polish] T-F8 — docs/specifications/forjar-state-generation-ownership.md:15 — Making cmd_undo's `generations` an Option<u32> is semantically equivalent to leaving it as `u32` defaulting to 1
   - corrected: polish, folded into v10 (3b3c9060, be385d7e): the §4 example carries parent and restores, R2 refuses K beyond the root, the R7 mutation is named, the withdrawn term is gone
   - evidence: CONFIRMED-AS-NARROWED by the judges; the substance held under three refuters, the sentence changed. Pinned by tests/falsification_spec_generation_ownership_shape.rs:43.

## Tables as returned

### Judge 1 (verdict PASS)

(1) The v8 contradiction is successfully closed by the new sentences in R4: "...which is why `apply --rollback-on-failure` is sound: it restores the failing stack's own declared machines from the pre-apply generation — the generation that is current when the apply starts, whatever its owner (the verb rule of §5; in a shared dir it is often another lineage's...)" and "R8's pre-flight evaluates `restore_decision(verb = RollbackOnFailure)` for that generation before the apply starts...". R4, R8, §5, and the §9 R8 row now perfectly agree that the pre-apply generation can have a foreign owner, that the verb exempts the check, and that the restore scope remains sound. 
(2) After a rigorous search of R1–R9, §4, §5, §9, and §10, no contradicting sentences were found. 
(3) The wording in v9 perfectly matches the code on main: `src/cli/helpers_state.rs` defines `pre_apply_generation` which calls `super::generation::current_generation(&gen_dir)`, confirming it is indeed 'the generation current when the apply starts'. 
(4) Verdict: PASS. The design is completely sound and all textual defects have been successfully removed.

### Judge 2 (verdict PASS)

(1) The v9 sentences that close the v8 contradiction are in R4: "... it restores the failing stack's own declared machines from the pre-apply generation — the generation that is current when the apply starts, whatever its owner (the verb rule of §5...)" and "R8's pre-flight evaluates restore_decision(verb = RollbackOnFailure) for that generation before the apply starts...". R4, R8, §5, and the §9 R8 row now agree perfectly: `--rollback-on-failure` uses the pre-apply generation (regardless of owner) and evaluates `restore_decision` using a specific verb that intentionally bypasses the owner check, ensuring the logic is robust in shared dirs.

(2) I thoroughly searched R1–R9, §4, §5, §9, §10 for contradicting sentences. No contradictions remain; all rules correctly align with the parent pointers, rename tracking, and journaled switch mechanics.

(3) I confirmed that on `main`, `src/cli/helpers_state.rs` defines `pre_apply_generation` as returning `super::generation::current_generation(&gen_dir)`. This is precisely 'the generation current when the apply starts', exactly matching the v9 wording.

(4) Verdict: PASS. The specification is sound, textually consistent, and ready for implementation.

### Judge 3 (verdict PASS)

(1) The v9 rewrite in R4 perfectly closes the v8 contradiction. R4 now states: "it restores the failing stack's own declared machines from the pre-apply generation — the generation that is current when the apply starts, whatever its owner... R8's pre-flight evaluates `restore_decision(verb = RollbackOnFailure)`". This is completely consistent with R8, §5, and the §9 R8 row which all explicitly align on the `RollbackOnFailure` verb evaluating the pre-apply generation regardless of owner. 

(2) A rigorous search across R1–R9, §4, §5, §9, and §10 found zero contradictory sentences. The rules interlock flawlessly—parent pointers remain entirely within a lineage (R2/R3), foreign-owner targets are refused leading to no destructive actions (R3/R6), and the journal safely gates the scoped restore (R4).

(3) Inspection of `src/cli/helpers_state.rs` on the main branch confirms that `pre_apply_generation` reads `super::generation::current_generation(&gen_dir)` right before the apply. Therefore, the pre-apply generation is indeed 'the generation current when the apply starts', ensuring the v9 text perfectly reflects the codebase reality.

(4) Verdict: PASS. The spec design is sound, every contradiction from prior review cycles has been thoroughly addressed, and no new logical or textual conflicts remain.


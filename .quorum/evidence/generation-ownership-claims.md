# Quorum evidence — PMAT-162 — the claims as put to the refuters

The generation-ownership spec went through nine three-lane review rounds on 2026-09-06/07 (v1: spec, v2: spec2, … v9: spec9), each in per-lane clones, plus the plan-stage teamwork grill that first widened PMAT-161. The claims under test are the v8 lanes' findings (that v8 still carried contradictions — L1..L3, findings L<i>-F<j>) and the v9 lanes' findings (polish — T-F<j>); the orchestrator's dispositions D1..D9 record what each revision folded in; F1..F2 are measured.

## v8 review lane 1 (verdict FAIL)

1) The v8 sentences closing the v7 defects have been verified and cited.
2) Contradiction found between R4 and the §9 R8 falsifier. R4 claims the pre-apply generation is taken by the apply itself a moment earlier (so it would be alpha-owned, preventing R5 from triggering), but the R8 falsifier explicitly tests when the pre-apply generation is bravo-owned (from a previous apply).
3) R4 crash walk: (i) staging crash or (ii) torn journal leaves the staging dir as garbage; (iii) crash during machine renames leaves each machine in one of the five shapes; (iv) crash during lock write or torn lock results in redo-from-journal.
4) Verdict is FAIL because R4 explicitly contradicts the falsifier in §9 regarding the pre-apply generation's owner and the `apply` snapshot behavior.

Findings:
- L1-F1 [cited] docs/specifications/forjar-state-generation-ownership.md:17 — v8 sentence closing the first v7 defect: 'the snapshot the apply itself took a moment earlier, which therefore carries an owner (R1) and is never legacy, so R5 does not arise' and 'R5 itself applies to every verb, `--rollback-on-failure` included: a legacy pre-apply generation cannot occur, but if one were named the refusal stands.'
- L1-F2 [cited] docs/specifications/forjar-state-generation-ownership.md:83 — v8 sentence closing the second v7 defect: 'select by file instead of by name, or newest global instead of newest-in-lineage → R3 RED'
- L1-F3 [cited] docs/specifications/forjar-state-generation-ownership.md:27 — v8 sentence closing the third v7 defect: 'when a lineage takes back one of its former names, that name is removed from the list, so `former_names` never contains the current name (v7 review)'
- L1-F4 [cited] docs/specifications/forjar-state-generation-ownership.md:17 — Contradicting Sentence 1 (R4): 'which is why `apply --rollback-on-failure` is sound: it restores the failing stack's own machines from the pre-apply generation — the snapshot the apply itself took a moment earlier, which therefore carries an owner (R1) and is never legacy, so R5 does not arise'
- L1-F5 [cited] docs/specifications/forjar-state-generation-ownership.md:80 — Contradicting Sentence 2 (§9 R8 falsifier): '`apply --rollback-on-failure` for alpha in a shared dir whose pre-apply generation is bravo-owned restores alpha's machines only; bravo byte-identical'
- L1-F6 [asserted] docs/specifications/forjar-state-generation-ownership.md:17 — R4 crash walk classification: A crash during (i) staging or a torn journal in (ii) (which reads as absent) leaves the staging dir as garbage. A crash in (iii) machine dir renames leaves the state in one of the five shapes. A crash in (iv) global lock write or a torn lock (which reads absent-or-old) results in redo-from-journal.

## v8 review lane 2 (verdict FAIL)

(1) The v7 defects were closed by the following v8 sentences:
- Pre-apply snapshot owned by construction: Closed by line 25: "The owner tag selects (R3) and gates legacy (R5); it does not define the scope, which is why `apply --rollback-on-failure` is sound: it restores the failing stack's own machines from the pre-apply generation — the snapshot the apply itself took a moment earlier, which therefore carries an owner (R1) and is never legacy, so R5 does not arise;"
- Mutation row reading 'select by file instead of by name': Closed by line 83: "Mutations the implementation must survive: drop the `owner` write → R1 RED; select by file instead of by name, or newest global instead of newest-in-lineage → R3 RED;"
- Reclaimed former name removed: Closed by line 52: "...and when a lineage takes back one of its former names, that name is removed from the list, so `former_names` never contains the current name (v7 review)..."

(2) Contradiction found in Rule R4 vs §9 (and §5):
- Sentence 1 (R4, line 25): "The owner tag selects (R3) and gates legacy (R5); it does not define the scope, which is why `apply --rollback-on-failure` is sound: it restores the failing stack's own machines from the pre-apply generation — the snapshot the apply itself took a moment earlier, which therefore carries an owner (R1) and is never legacy, so R5 does not arise;"
- Sentence 2 (§9, line 67): "| R8 | `apply --rollback-on-failure` for alpha in a shared dir whose pre-apply generation is bravo-owned restores alpha's machines only; bravo byte-identical | single-stack `--rollback-on-failure` unchanged |"
These sentences contradict each other: R4 claims the pre-apply generation is a snapshot taken by the invoking stack's apply (alpha-owned), while §9 claims it can be foreign-owned (bravo-owned). §5 further contradicts R4 by stating the decision function must "ignore the owner for `--rollback-on-failure`".

(3) R4 Crash Walk:
- Torn journal (crash in step i or ii): Reads as absent. Leaves an orphan staging dir which the next verb removes as garbage.
- Torn lock (crash in step iv): Reads as absent-or-old. Step iv is redone from the journal's copy.
- Crash during switch (step iii): Journal is present. Every machine is in one of five shapes: (live, staged), (old, staged), (old, live), (staged), or (live). Replay performs only the renames that shape still needs.
- Crash in generation append (step v): Journal is present. Switch replay completes (no renames needed), lock is rewritten from journal (redo-from-journal), revert generation is appended unless already carrying the txid (redo-from-journal), and journal is removed.

(4) Verdict: FAIL. The design contains a textual contradiction regarding Rule R4 as shown above.

Findings:
- L2-F1 [cited] docs/specifications/forjar-state-generation-ownership.md:25 — Rule R4 contradicts the R8 falsifier in §9 and the interface definition in §5 regarding the ownership of the pre-apply generation for `--rollback-on-failure`. R4 asserts the snapshot is taken by the apply itself and thus owned by the invoking stack, but §9 explicitly tests a scenario where it is foreign-owned, and §5 states the owner is ignored.

## v8 review lane 3 (verdict FAIL)

(1) Quotes addressing v7 defects:
- Defect 1: 'R5 itself applies to every verb, --rollback-on-failure included: a legacy pre-apply generation cannot occur, but if one were named the refusal stands.' (R4).
- Defect 2: 'Mutations the implementation must survive: drop the owner write → R1 RED; select by file instead of by name, or newest global instead of newest-in-lineage → R3 RED;' (§9).
- Defect 3: 'and when a lineage takes back one of its former names, that name is removed from the list, so former_names never contains the current name (v7 review)' (R9).

(2) Contradiction found: R4 says 'and R8's pre-flight evaluates restore_decision for that snapshot before the apply starts, so the verb can never reach a refusal after the apply has run (1.26.0's PMAT-174 rule, kept).' However, R8 says 'EVERY path that writes the state dir from a generation — ... apply --rollback-on-failure (helpers_state::maybe_rollback_generation ...) — calls restore_decision first;'. R8 routes the call exclusively through the rollback error handler (which executes *after* the apply has failed), thereby contradicting R4's claim that a pre-flight evaluates the decision *before* the apply starts to prevent a post-apply refusal.

(3) Crash walk R4 (i)-(v): 
- Crash in (i) or torn journal leaves an absent journal -> staging dir is classified as garbage.
- Crash after (ii) leaves journal present -> machines are in shape `(live, staged)` not started, or `(staged)` alone. 
- Crash during (iii) leaves machines in shape `(old, staged)` or `(old, live)` / `(live)` alone.
- Crash before/during (iv) lock write (torn lock reads absent-or-old) -> machines are done, redo (iv) lock write from journal. 
- Crash after (iv) -> lock is new, rewrite lock (idempotent), append revert generation, remove journal.

(4) Verdict: FAIL due to the contradiction between R4 and R8.

Findings:
- L3-F1 [cited] docs/specifications/forjar-state-generation-ownership.md:17 — The v7 defect regarding R5 applying to every verb is addressed by the sentence: 'R5 itself applies to every verb, --rollback-on-failure included: a legacy pre-apply generation cannot occur, but if one were named the refusal stands.'
- L3-F2 [cited] docs/specifications/forjar-state-generation-ownership.md:83 — The v7 defect regarding the §9 mutation row is addressed by the sentence: 'Mutations the implementation must survive: drop the owner write → R1 RED; select by file instead of by name, or newest global instead of newest-in-lineage → R3 RED;'
- L3-F3 [cited] docs/specifications/forjar-state-generation-ownership.md:27 — The v7 defect regarding a reclaimed former name is addressed by the sentence: 'and when a lineage takes back one of its former names, that name is removed from the list, so former_names never contains the current name (v7 review)'
- L3-F4 [cited] docs/specifications/forjar-state-generation-ownership.md:17 — R4 contradicts R8 by claiming that a pre-flight evaluates the decision before the apply starts: 'and R8's pre-flight evaluates restore_decision for that snapshot before the apply starts, so the verb can never reach a refusal after the apply has run (1.26.0's PMAT-174 rule, kept).'
- L3-F5 [cited] docs/specifications/forjar-state-generation-ownership.md:25 — R8 contradicts R4 by explicitly placing the call in the post-apply failure path without any mention of a pre-flight: 'EVERY path that writes the state dir from a generation — undo, undo --resume, rollback, and apply --rollback-on-failure (helpers_state::maybe_rollback_generation, which today calls rollback_to_generation directly with no scope — the one bypass all three review lanes named) — calls restore_decision first; a direct call to the primitive from anywhere else is a defect the §9 mutation table catches.'

## v9 review lanes (verdicts PASS, PASS, PASS) — findings as T-F<j>

- T-F1 [cited] docs/specifications/forjar-state-generation-ownership.md:17 — The v8 contradiction regarding the --rollback-on-failure clause is completely resolved. R4, R8, §5, and the §9 R8 row now agree perfectly on the pre-apply generation's owner and scope.
- T-F2 [asserted] docs/specifications/forjar-state-generation-ownership.md:? — No contradicting sentences remain anywhere in R1–R9, §4, §5, §9, or §10. The specification is fully consistent and mathematically sound.
- T-F3 [cited] src/cli/helpers_state.rs:146 — The v9 wording matches the implementation on main: `pre_apply_generation` simply fetches the current generation using `current_generation`.
- T-F4 [asserted] docs/specifications/forjar-state-generation-ownership.md:13 — R2 states `undo` with no target refuses when there is no parent, but could explicitly state `--generations K` also refuses if `parent^K` does not exist.
- T-F5 [asserted] docs/specifications/forjar-state-generation-ownership.md:51 — The phrase 'it takes the decision value, not a bool' in §5 could be slightly misread as meaning the function takes no booleans at all, despite `yes: bool` being in the signature.
- T-F6 [asserted] docs/specifications/forjar-state-generation-ownership.md:17 — R4 states an absent machine is 'left untouched', which relies on the reader remembering that R6's destroy step runs before the restore to empty its resources.
- T-F7 [asserted] docs/specifications/forjar-state-generation-ownership.md:35 — The GenerationMeta schema example in §4 omits the `parent` and `restores` fields introduced by R2
- T-F8 [asserted] docs/specifications/forjar-state-generation-ownership.md:15 — Making cmd_undo's `generations` an Option<u32> is semantically equivalent to leaving it as `u32` defaulting to 1

## Orchestrator dispositions (each is itself a claim to refute)

- D1 — v1 review (three lanes): the state dir is single-stack below the lock — global generations keyed by machine name, whole-map outputs overwrite, status attributing every machine to the last name: CONFIRMED; the machine-ownership guard, per-stack outputs and per-stack status became PMAT-161's widened scope (shipped), and generation ownership became this spec.
- D2 — v2 review: metadata-borne owner, append-only restore, Option<u32> target, atomic scoped restore with a merged lock, (machine, id) destroy keying, the maybe_rollback_generation bypass: CONFIRMED, folded into v2 (60fb9f5d).
- D3 — v3 review: a one-rename atomic switch is impossible with the sibling layout; lineage keyed by file contradicts R9; K-back over an append-only log toggles: CONFIRMED, folded into v3 (93c79368: journaled transaction, lineage by file, lineage-position rule).
- D4 — v4 review: the journal was written before staging; a retired name could be reused: CONFIRMED, folded into v5 (103195d6: journal after staging; reserved names).
- D5 — v5 review: a (staged)-alone shape; the reservation over-refused a lineage's own former name; restore_decision lacked the verb: CONFIRMED, folded into v6 (37c82823).
- D6 — v6 review: a stale shape count; a (live)-alone shape; R5 said owner.yaml; R8 and §10 kept the withdrawn rename refusal: CONFIRMED, folded into v7 (e71be4bc).
- D7 — v7 review: the --rollback-on-failure clause bypassed R5; a §9 mutation row named the specified behaviour; former_names kept a reclaimed name: CONFIRMED, folded into v8 (24878d47).
- D8 — v8 review: the pre-apply generation cannot be owned by the failing stack by construction (it is whatever is current when the apply starts): CONFIRMED, folded into v9 (4ec8c4a5), which three lanes then PASSED.
- D9 — v9 polish (§4 example lacks parent/restores; --generations K beyond the root; the §5 bool wording) and the shape test's two findings (no R7 mutation; the withdrawn term still present as a substring): CONFIRMED, folded into v10 (3b3c9060, be385d7e) — no rule changed.

## Orchestrator's own measured claims

- F1 — `cargo test --test falsification_spec_generation_ownership_shape` at HEAD: 8 passed (status line v10 REVIEWED; R1–R9 headings in order; a §9 row per rule; a mutation per rule; restore_decision with RestoreVerb and RestoreScope; the v9 3/3 PASS record; parent and restores in §4; no TODO/TBD/ownership-file term).
- F2 — The v9 review returned PASS from all three lanes with no contradicting sentence pair and the pre_apply_generation claim confirmed at src/cli/helpers_state.rs:146 on main.


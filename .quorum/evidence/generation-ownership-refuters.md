# Quorum evidence — PMAT-162 — refuter rulings (the v9 review lanes attacking v9)

The three v9 lanes (conv-d3a9cf06, conv-f89d3ff6, conv-0a5ea972; 149–317 s; per-lane plain clones with the PMAT-161 branch fetched) were told to search R1–R9, §4, §5, §9 and §10 for ANY two contradicting sentences and to name any unsound rule. All three returned PASS: the v8 contradiction is closed by the rewritten R4 clause and the new R8 sentence (quoted by each), no contradicting pair remains, and the code claim holds. Their five polish findings became v10.

## v9 lane 1 as refuter (verdict PASS)

(1) The v8 contradiction is successfully closed by the new sentences in R4: "...which is why `apply --rollback-on-failure` is sound: it restores the failing stack's own declared machines from the pre-apply generation — the generation that is current when the apply starts, whatever its owner (the verb rule of §5; in a shared dir it is often another lineage's...)" and "R8's pre-flight evaluates `restore_decision(verb = RollbackOnFailure)` for that generation before the apply starts...". R4, R8, §5, and the §9 R8 row now perfectly agree that the pre-apply generation can have a foreign owner, that the verb exempts the check, and that the restore scope remains sound. 
(2) After a rigorous search of R1–R9, §4, §5, §9, and §10, no contradicting sentences were found. 
(3) The wording in v9 perfectly matches the code on main: `src/cli/helpers_state.rs` defines `pre_apply_generation` which calls `super::generation::current_generation(&gen_dir)`, confirming it is indeed 'the generation current when the apply starts'. 
(4) Verdict: PASS. The design is completely sound and all textual defects have been successfully removed.

Findings:
- R1-F1 [cited] docs/specifications/forjar-state-generation-ownership.md:17 — The v8 contradiction regarding the --rollback-on-failure clause is completely resolved. R4, R8, §5, and the §9 R8 row now agree perfectly on the pre-apply generation's owner and scope.
- R1-F2 [asserted] docs/specifications/forjar-state-generation-ownership.md:? — No contradicting sentences remain anywhere in R1–R9, §4, §5, §9, or §10. The specification is fully consistent and mathematically sound.
- R1-F3 [cited] src/cli/helpers_state.rs:146 — The v9 wording matches the implementation on main: `pre_apply_generation` simply fetches the current generation using `current_generation`.

## v9 lane 2 as refuter (verdict PASS)

(1) The v9 sentences that close the v8 contradiction are in R4: "... it restores the failing stack's own declared machines from the pre-apply generation — the generation that is current when the apply starts, whatever its owner (the verb rule of §5...)" and "R8's pre-flight evaluates restore_decision(verb = RollbackOnFailure) for that generation before the apply starts...". R4, R8, §5, and the §9 R8 row now agree perfectly: `--rollback-on-failure` uses the pre-apply generation (regardless of owner) and evaluates `restore_decision` using a specific verb that intentionally bypasses the owner check, ensuring the logic is robust in shared dirs.

(2) I thoroughly searched R1–R9, §4, §5, §9, §10 for contradicting sentences. No contradictions remain; all rules correctly align with the parent pointers, rename tracking, and journaled switch mechanics.

(3) I confirmed that on `main`, `src/cli/helpers_state.rs` defines `pre_apply_generation` as returning `super::generation::current_generation(&gen_dir)`. This is precisely 'the generation current when the apply starts', exactly matching the v9 wording.

(4) Verdict: PASS. The specification is sound, textually consistent, and ready for implementation.

Findings:
- R2-F1 [asserted] docs/specifications/forjar-state-generation-ownership.md:13 — R2 states `undo` with no target refuses when there is no parent, but could explicitly state `--generations K` also refuses if `parent^K` does not exist. (proposed fix: Change to: `--generations K` restores `parent^K(position)` and refuses if there are fewer than K parents.)
- R2-F2 [asserted] docs/specifications/forjar-state-generation-ownership.md:51 — The phrase 'it takes the decision value, not a bool' in §5 could be slightly misread as meaning the function takes no booleans at all, despite `yes: bool` being in the signature. (proposed fix: Change to: '(the scope parameter takes the decision value, rather than a stack-scoped boolean flag)')
- R2-F3 [asserted] docs/specifications/forjar-state-generation-ownership.md:17 — R4 states an absent machine is 'left untouched', which relies on the reader remembering that R6's destroy step runs before the restore to empty its resources. (proposed fix: Could optionally add a brief cross-reference: 'and left untouched (its resources having been destroyed by R6) when it does not'.)

## v9 lane 3 as refuter (verdict PASS)

(1) The v9 rewrite in R4 perfectly closes the v8 contradiction. R4 now states: "it restores the failing stack's own declared machines from the pre-apply generation — the generation that is current when the apply starts, whatever its owner... R8's pre-flight evaluates `restore_decision(verb = RollbackOnFailure)`". This is completely consistent with R8, §5, and the §9 R8 row which all explicitly align on the `RollbackOnFailure` verb evaluating the pre-apply generation regardless of owner. 

(2) A rigorous search across R1–R9, §4, §5, §9, and §10 found zero contradictory sentences. The rules interlock flawlessly—parent pointers remain entirely within a lineage (R2/R3), foreign-owner targets are refused leading to no destructive actions (R3/R6), and the journal safely gates the scoped restore (R4).

(3) Inspection of `src/cli/helpers_state.rs` on the main branch confirms that `pre_apply_generation` reads `super::generation::current_generation(&gen_dir)` right before the apply. Therefore, the pre-apply generation is indeed 'the generation current when the apply starts', ensuring the v9 text perfectly reflects the codebase reality.

(4) Verdict: PASS. The spec design is sound, every contradiction from prior review cycles has been thoroughly addressed, and no new logical or textual conflicts remain.

Findings:
- R3-F1 [asserted] docs/specifications/forjar-state-generation-ownership.md:35 — The GenerationMeta schema example in §4 omits the `parent` and `restores` fields introduced by R2 (proposed fix: Add `parent: <number|null>` to the YAML snippet in §4 to fully illustrate all schema 1.2 additions.)
- R3-F2 [asserted] docs/specifications/forjar-state-generation-ownership.md:15 — Making cmd_undo's `generations` an Option<u32> is semantically equivalent to leaving it as `u32` defaulting to 1 (proposed fix: None required; it's structurally cleaner to use Option<u32> for "no target", even if `parent^1(position)` yields the same result.)


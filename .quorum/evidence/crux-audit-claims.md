# Quorum evidence — PMAT-164 — the claims as put to the refuters

Three claim lanes (lenses: the [V] cells against the code; the reference cells and dispositions; the reconciliation and GO-spec sections) and one agy /teamwork-preview lane read docs/audits/crux-1.26.0.md at 240fb93f. Lanes 3 and 4 lost their clone's object store mid-run and answered code-facing questions from the document alone — those claims are unverifiable and marked so; lane 3's 'fabricated bullets' finding was an artifact of that. Their findings forced 17bdb497 (PMAT-188). Ids: L1..L3 (findings L<i>-F<j>), T, D1..D6, F1.

## Claim lane 1 (verdict FAIL)

C1: Behaviour (1)'s [V] cell claims `resolver::goal_closure` is used for downward closure, which is true at `src/cli/apply_selection/closure.rs:99`.
C2: Behaviour (7) evaluates against 3 reference systems (Nix, Pulumi, Kubernetes) and its disposition is `adopt(PMAT-162...)`.
C3: CHANGELOG bullet 4 maps exactly to the row for Behaviour (4) regarding negative selectors.
C4: No `[X]` figure leaked into README.md or docs/book (verified via `git grep -F "[X]"` which returned 1).
C5: The GO-spec finding adopts lane 2's formulation to "isolate generations by stack ID and replay only the target stack's resources", despite lane 3's dissent.
C6: The document has defect false citations at docs/audits/crux-1.26.0.md:41 and docs/audits/crux-1.26.0.md:47. It cites `src/core/state/stamp.rs` at lines 318-333, 354-393, and 395-425, but the actual file is `src/core/state/stamp/mod.rs` and the lines for `stack_conflict` and `merge_outputs` are 260-275 and 334-362 respectively. Therefore, the [V] cells for behaviours 6 and 8 are false.

Findings:
- L1-F1 [measured] src/cli/apply_selection/closure.rs:99 — Behaviour (1) positive selectors closed downward over depends_on
- L1-F2 [measured] src/cli/dispatch_apply_check.rs:73 — Behaviour (2) cmd_apply_check resolves selection then checks selected config
- L1-F3 [measured] src/cli/apply_selection/closure.rs:169 — Behaviour (3) check_existence runs before closure and refuses empty match
- L1-F4 [measured] src/cli/apply_selection/closure.rs:120 — Behaviour (4) negative selector emptying selection is refused via empty_after_narrowing
- L1-F5 [measured] src/cli/apply_selection/narrow.rs:148 — Behaviour (5) phony stripping contracts edges via contract_edges
- L1-F6 [measured] docs/audits/crux-1.26.0.md:41 — Behaviour (6) citation is FALSE (cited src/core/state/stamp.rs:318-333, 354-393 but actually in src/core/state/stamp/mod.rs:260)
- L1-F7 [measured] src/core/state/stamp/mod.rs:150 — Behaviour (7) multi_stack_restore_refusal unconditionally refuses when state dir holds >1 stack
- L1-F8 [measured] docs/audits/crux-1.26.0.md:47 — Behaviour (8) citation is FALSE (cited src/core/state/stamp.rs:395-425 but actually in src/core/state/stamp/mod.rs:334-362)
- L1-F9 [measured] src/cli/status_core.rs:129 — Behaviour (9) machine_owner attributes machine to writing stack

## Claim lane 2 (verdict FAIL)

C1: `docs/audits/crux-1.26.0.md:35` claims empty selections are refused, which is proven true by `src/cli/apply_selection/closure.rs:120-122` returning `Err(empty_after_narrowing(...))`.
C2: Behaviour 1 has 3 reference systems (Terraform, Ansible, Make) and uses the disposition `reject` for all three at `docs/audits/crux-1.26.0.md:26-28`.
C3: CHANGELOG bullet (1) "Every resource-set selector... closed downward over depends_on" maps to row `(1) selectors select the depends_on closure` at `CHANGELOG.md:24`.
C4: No `[X]` figure leaked into `README.md` or `docs/book/`; verified via the grep command `grep -r '\[X\]' README.md docs/book`.
C5: The GO-spec finding as stated upgrades the snapshot mechanism to a Nix-like stack-scoped restore, correctly identifying the future replacement for the unconditional refusal implemented in the cited lane files (`src/core/state/stamp.rs:140-167` and `src/cli/generation/restore.rs:30-44`).
C6: The document has a defect at `docs/audits/crux-1.26.0.md:46`: the disposition for the Kubernetes row erroneously copies the Nix row's disposition ("matching Nix's per-profile rollback"), which contradicts the dissent section (line 87) stating Lane 3 pointed to a different model entirely.

Findings:
- L2-F1 [cited] docs/audits/crux-1.26.0.md:46 — The disposition for the Kubernetes row (Lane 3) is a copy-paste error ('matching Nix's per-profile rollback') which explicitly contradicts the Dissent section's statement that Lane 3 pointed to Kubernetes' different rollout history model.

## Claim lane 3 (verdict FAIL)

C1: The `[V]` cell for behaviour (1) Terraform claims it closes downward over `depends_on`, which is true at `src/cli/apply_selection/closure.rs:99`.
C2: Behaviour (1) "selectors select the depends_on closure" evaluates 3 reference systems (Terraform, Ansible, Make) and has the disposition `reject` for all three.
C3: The document has defect mapping CHANGELOG bullets at docs/audits/crux-1.26.0.md:124; the actual CHANGELOG bullet `**apply's resource-set selectors resolved independently, one bug per selector (#466, #467, #468).**` does not map to any row, and bullets 6-9 in the reconciliation list are fabricated.
C4: No `[X]` figures leaked into the documentation, verified by running `grep -rn "\[X\]" README.md docs/book`.
C5: The GO-spec section faithfully states the majority finding (lanes 1 and 2 recommending Nix profile generations) and the dissent (lane 3 recommending Kubernetes rollout history) exactly as they appear in the cited lane JSON files.
C6: The document has defect citing non-existent file `src/core/state/stamp.rs` at docs/audits/crux-1.26.0.md:41, as the file does not exist in the codebase.

Findings:
- L3-F1 [measured] docs/audits/crux-1.26.0.md:124 — The document has defect fabricating bullets 6-9 and ignoring the actual bullets in CHANGELOG.md [Unreleased] at docs/audits/crux-1.26.0.md:124
- L3-F2 [measured] docs/audits/crux-1.26.0.md:41 — The document has defect citing non-existent file src/core/state/stamp.rs at docs/audits/crux-1.26.0.md:41

## Independent lane — agy /teamwork-preview (object store lost mid-run) (verdict FAIL)

I have completed the independent review of the CRUX audit document `docs/audits/crux-1.26.0.md` as requested. 

1. **Behaviour row mismatch**: Behaviour (4) claims that an emptied selection is refused entirely, which describes what the ticket wanted, rather than reflecting the code's exemption for machine narrowing.
2. **Contradictory rationale**: In behaviour (4), the document cites `Terraform -exclude` and rationalizes rejecting it because it can "report false success". This contradicts Terraform's documented behaviour because Terraform does not have an `-exclude` flag.
3. **Name-drop without mechanism**: Behaviour (3) cites Make and simply drops the error message (`fails with "No rule to make target"`) without explaining the resolution mechanism.
4. **Lane attributions**: All three lane citations are accurately attributed. Lane 1 calls it a blocked safe operation, Lane 2 calls it a "Temporary safe refusal", and Lane 3 uses the exact words "conflates history".
5. **Reconciliation check**: The document passes the structure check. There are exactly 9 behaviour bullets in the reconciliation list, and each behaviour has exactly 3 rows in the comparison table.

*(Note: Direct `git show` against `PMAT-161-state-stamp-per-name` failed due to missing Git objects in the isolated test environment, but the analysis was completed using the document and lane files.)*

Findings:
- T-F1 [asserted] docs/audits/crux-1.26.0.md:40 — The 'forjar today' cell for behaviour (4) describes what the ticket wanted rather than the code. It claims ALL emptied selections are refused ('An --exclude/--skip that removes every selected resource is refused'), but the actual code exempts machine narrowing. (proposed fix: Update the behaviour 4 cell to accurately reflect that machine narrowing is exempted.)
- T-F2 [asserted] docs/audits/crux-1.26.0.md:41 — The reject rationale for Terraform in behaviour (4) ('refusing an emptied selection catches an operator's typo instead of reporting false success') contradicts Terraform's documented behaviour because Terraform does not even have an '-exclude' flag to report false success on. (proposed fix: Remove or replace the Terraform '-exclude' comparison since the flag does not exist in Terraform.)
- T-F3 [asserted] docs/audits/crux-1.26.0.md:36 — Behaviour (3) Make row is a name-drop without a mechanism. It simply states the error message 'fails with "No rule to make target"' without explaining the mechanism Make uses. (proposed fix: Provide the underlying mechanism Make uses to determine valid targets, or replace the reference.)
- T-F4 [measured] docs/audits/crux-1.26.0.md:68 — The three reference lanes DO say what the document attributes to them. Lane 1 states 'forjar blocks a safe, isolated operation', Lane 2 states 'Temporary safe refusal', and Lane 3 states 'Forjar conflates history; Pulumi scopes it' and 'Forjar lacks K8s's resource-scoped history'. (proposed fix: None needed; the document accurately reflects the lane files.)
- T-F5 [measured] docs/audits/crux-1.26.0.md:79 — A dogfood/crux-reconcile check would PASS. Every one of the 9 changelog bullets maps to exactly 3 rows in the table. There are no bullets without a row and no behaviours with fewer than 3 systems. (proposed fix: None needed; the reconciliation list is complete.)

## Orchestrator dispositions (each is itself a claim to refute)

- D1 — every citation of `src/core/state/stamp.rs` names a path that exists on no branch, with stale line ranges (L1, L3): CONFIRMED. FIXED in 17bdb497 (PMAT-188): every [V] cell re-derived against the PMAT-161 branch head (`src/core/state/stamp/mod.rs`, `identity.rs`, `rename.rs`, `src/cli/apply_selection/closure.rs`, `narrow.rs`, `src/cli/generation/restore.rs`, `src/cli/status_core.rs`, `src/cli/dispatch_apply_check.rs`).
- D2 — rows 44–46 (Nix, Pulumi, Kubernetes for behaviour 7) carried one copy-pasted adopt() rationale (L2): CONFIRMED. FIXED in 17bdb497: each system has its own mechanism sentence, and the Dissent section names Kubernetes for lane 3.
- D3 — the behaviour-4 [V] cell said every emptied selection is refused, while the code exempts machine narrowing (T, asserted from a broken clone): CONFIRMED against the code (`empty_after_narrowing` refuses resource negatives; `--only-machine`/`--exclude-machine` still converge nothing, GH-211). FIXED in 17bdb497.
- D4 — the Make row for behaviour 3 named the system without its mechanism (T): CONFIRMED. FIXED in 17bdb497 (an unknown target fails before any recipe; prerequisites are built first).
- D5 — the Terraform `-exclude` reject rationale rests on a version fact (T): CONFIRMED as a hedge; FIXED in 17bdb497 (marked [X], 'recent; earlier Terraform had only -target').
- D6 — 'bullets 6–9 of the reconciliation list are fabricated' (L3, from a broken clone): REJECTED — the delegate verified `PMAT-161-state-stamp-per-name:CHANGELOG.md` carries the bullet the audit quotes; the lane read the PMAT-164 checkout's CHANGELOG, which predates it.

## Orchestrator's own measured claims

- F1 — At 17bdb497: `docs/audits/crux-1.26.0.md` exists, `grep -c 'src/core/state/stamp.rs'` is 0, and `grep -n '\[X\]' README.md docs/book/src/*.md` matches nothing outside docs/audits; every behaviour has at least three systems and one disposition (adopt(PMAT-162) for behaviour 7, reject(rationale) elsewhere); the reconciliation list maps every bold behaviour bullet of the PMAT-161 branch's CHANGELOG to a row.


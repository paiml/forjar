# Quorum evidence — PMAT-164 — refuter rulings

Three refuter lanes (conv-727b98cf, conv-69c74bde, conv-1a12bcc2; 224–351 s; standalone clones carrying the PMAT-161 branch) ruled on the same 24 ids at 4c13f399 (the C-family ids embedded in the claim lanes' summaries were not ruled on by any lane — a format artifact; they are carried by the L*-F* findings that quote them). Unanimous: the nine [V] cells' substance holds, dispositions D2–D5 hold, the seven round-1 complaints 17bdb497 answered are stale. Lane 3 alone found the behaviour-8 cells citing `stamp/mod.rs:382`, a body line of `merge_outputs` (which begins at 375) — corrected to `375-403` after this round — and objected that the CHANGELOG's bold lead-ins are paragraphs, not bullets (wording; the nine quoted fragments are verbatim). Two lanes relocated `multi_stack_restore_refusal` to `stamp/mod.rs:163`.

## Refuter 1 (verdict PASS)

L1-F1: SURVIVES
L1-F2: SURVIVES
L1-F3: SURVIVES
L1-F4: SURVIVES
L1-F5: SURVIVES
L1-F6: REFUTED
L1-F7: SURVIVES
L1-F8: REFUTED
L1-F9: SURVIVES
L2-F1: REFUTED
L3-F1: REFUTED
L3-F2: REFUTED
T-F1: REFUTED
T-F2: REFUTED
T-F3: REFUTED
T-F4: SURVIVES
T-F5: SURVIVES
D1: SURVIVES
D2: SURVIVES
D3: SURVIVES
D4: SURVIVES
D5: SURVIVES
D6: SURVIVES
F1: SURVIVES

Findings:
- R1-F1 [measured] docs/audits/crux-1.26.0.md:41 — L1-F6
- R1-F2 [measured] docs/audits/crux-1.26.0.md:47 — L1-F8
- R1-F3 [cited] docs/audits/crux-1.26.0.md:46 — L2-F1
- R1-F4 [measured] docs/audits/crux-1.26.0.md:124 — L3-F1
- R1-F5 [measured] docs/audits/crux-1.26.0.md:41 — L3-F2
- R1-F6 [asserted] docs/audits/crux-1.26.0.md:40 — T-F1
- R1-F7 [asserted] docs/audits/crux-1.26.0.md:41 — T-F2
- R1-F8 [asserted] docs/audits/crux-1.26.0.md:36 — T-F3

## Refuter 2 (verdict PASS)

L1-F1: SURVIVES
L1-F2: SURVIVES
L1-F3: SURVIVES
L1-F4: NARROWED (line 121)
L1-F5: SURVIVES
L1-F6: REFUTED (stale complaint, fixed at HEAD)
L1-F7: NARROWED (line 163)
L1-F8: REFUTED (stale complaint, fixed at HEAD)
L1-F9: SURVIVES
L2-F1: REFUTED (stale complaint, fixed at HEAD)
L3-F1: REFUTED (hallucinated complaint, bullets exist in CHANGELOG.md)
L3-F2: REFUTED (stale complaint, fixed at HEAD)
T-F1: REFUTED (stale complaint, fixed at HEAD)
T-F2: REFUTED (stale complaint, fixed at HEAD)
T-F3: REFUTED (stale complaint, fixed at HEAD)
T-F4: SURVIVES
T-F5: SURVIVES
D1: SURVIVES
D2: SURVIVES
D3: SURVIVES
D4: SURVIVES
D5: SURVIVES
D6: SURVIVES
F1: SURVIVES

Findings:
- R2-F1 [measured] src/cli/apply_selection/closure.rs:121 — L1-F4 (proposed fix: NARROWED: empty_after_narrowing is called on line 121, not 120.)
- R2-F2 [measured] docs/audits/crux-1.26.0.md:41 — L1-F6 (proposed fix: REFUTED: The document correctly cites src/core/state/stamp/mod.rs at HEAD, not stamp.rs.)
- R2-F3 [measured] src/core/state/stamp/mod.rs:163 — L1-F7 (proposed fix: NARROWED: The refusal logic is actually at line 163, not 150.)
- R2-F4 [measured] docs/audits/crux-1.26.0.md:47 — L1-F8 (proposed fix: REFUTED: The document correctly cites src/core/state/stamp/mod.rs at HEAD.)
- R2-F5 [cited] docs/audits/crux-1.26.0.md:46 — L2-F1 (proposed fix: REFUTED: The document no longer contains the copy-paste error; it explicitly names Kubernetes' rollout undo.)
- R2-F6 [measured] docs/audits/crux-1.26.0.md:124 — L3-F1 (proposed fix: REFUTED: CHANGELOG.md [Unreleased] contains exactly the 9 bullets quoted; they were not fabricated.)
- R2-F7 [measured] docs/audits/crux-1.26.0.md:41 — L3-F2 (proposed fix: REFUTED: The document correctly cites src/core/state/stamp/mod.rs at HEAD.)
- R2-F8 [asserted] docs/audits/crux-1.26.0.md:40 — T-F1 (proposed fix: REFUTED: The document at HEAD explicitly states that machine narrowing is exempt from the refusal.)
- R2-F9 [asserted] docs/audits/crux-1.26.0.md:41 — T-F2 (proposed fix: REFUTED: The document at HEAD explicitly hedges the -exclude flag as a comparatively recent addition.)
- R2-F10 [asserted] docs/audits/crux-1.26.0.md:36 — T-F3 (proposed fix: REFUTED: The document at HEAD explicitly describes Make's mechanism (prerequisites are built first, unknown target fails before any recipe).)

## Refuter 3 (verdict FAIL)

L1-F1: SURVIVES
L1-F2: SURVIVES
L1-F3: SURVIVES
L1-F4: SURVIVES
L1-F5: SURVIVES
L1-F6: REFUTED
L1-F7: NARROWED
L1-F8: REFUTED
L1-F9: SURVIVES
L2-F1: REFUTED
L3-F1: SURVIVES
L3-F2: REFUTED
T-F1: REFUTED
T-F2: REFUTED
T-F3: REFUTED
T-F4: SURVIVES
T-F5: REFUTED
D1: REFUTED
D2: SURVIVES
D3: SURVIVES
D4: SURVIVES
D5: SURVIVES
D6: REFUTED
F1: REFUTED

Findings:
- R3-F1 [measured] docs/audits/crux-1.26.0.md:41 — L1-F6 (proposed fix: REFUTED (evidence the sentence is false at HEAD — quote docs/audits/crux-1.26.0.md:41))
- R3-F2 [measured] src/core/state/stamp/mod.rs:163 — L1-F7 (proposed fix: L1-F7 [measured] src/core/state/stamp/mod.rs:163 — Behaviour (7) multi_stack_restore_refusal unconditionally refuses when state dir holds >1 stack)
- R3-F3 [measured] docs/audits/crux-1.26.0.md:47 — L1-F8 (proposed fix: REFUTED (evidence the sentence is false at HEAD — quote docs/audits/crux-1.26.0.md:47))
- R3-F4 [cited] docs/audits/crux-1.26.0.md:46 — L2-F1 (proposed fix: REFUTED (evidence the sentence is false at HEAD — quote docs/audits/crux-1.26.0.md:46))
- R3-F5 [measured] docs/audits/crux-1.26.0.md:41 — L3-F2 (proposed fix: REFUTED (evidence the sentence is false at HEAD — quote docs/audits/crux-1.26.0.md:41))
- R3-F6 [asserted] docs/audits/crux-1.26.0.md:35 — T-F1 (proposed fix: REFUTED (evidence the sentence is false at HEAD — quote docs/audits/crux-1.26.0.md:35))
- R3-F7 [asserted] docs/audits/crux-1.26.0.md:35 — T-F2 (proposed fix: REFUTED (evidence the sentence is false at HEAD — quote docs/audits/crux-1.26.0.md:35))
- R3-F8 [asserted] docs/audits/crux-1.26.0.md:33 — T-F3 (proposed fix: REFUTED (evidence the sentence is false at HEAD — quote docs/audits/crux-1.26.0.md:33))
- R3-F9 [measured] CHANGELOG.md:24 — T-F5 (proposed fix: REFUTED (evidence the sentence is false at HEAD — quote CHANGELOG.md:24))
- R3-F10 [asserted] docs/audits/crux-1.26.0.md:48 — D1 (proposed fix: REFUTED (evidence the sentence is false at HEAD — quote docs/audits/crux-1.26.0.md:48) - The cited line 382 for Behaviour 8 Pulumi/Kubernetes does not contain the symbol 'merge_outputs'.)
- R3-F11 [asserted] CHANGELOG.md:24 — D6 (proposed fix: REFUTED (evidence the sentence is false at HEAD — quote CHANGELOG.md:24) - CHANGELOG carries a paragraph, not a bullet.)
- R3-F12 [measured] docs/audits/crux-1.26.0.md:107 — F1 (proposed fix: REFUTED (evidence the sentence is false at HEAD — quote docs/audits/crux-1.26.0.md:107) - The reconciliation list maps paragraphs, not the bold bullets from the CHANGELOG.)


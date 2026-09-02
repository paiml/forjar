# Quorum evidence — #366 / #369 (fix/policy-scope-and-rule-ids) — adjudicated claims

## CONFIRMED — 6 claims survived refutation (two narrowed by the agy lane)

1. [probe] (explains-symptom) A policy rule scoped to a multi-word resource type was silently inert: `matches_scope` compared the document's serde spelling against the Debug spelling.
   - evidence: `src/core/parser/policy.rs:155` at the merge-base built `actual` as `format!("{:?}", resource.resource_type).to_lowercase()` — `systemdunit` for a type the document spells `systemd_unit` — so a `resource_type:` scope written the only way the parser accepts never matched, and the rule applied to nothing without a diagnostic. Same spelling in `src/core/compliance_gate.rs:38`. Pinned by `a_rule_scoped_to_a_multi_word_type_is_enforced`, `the_debug_spelling_is_not_a_scope`, `display_is_the_serde_spelling_for_every_variant` and the binary case `forjar_policy_blocks_on_a_multi_word_scope` in `tests/falsification_policy_scope_spelling.rs`.

2. [probe] (explains-symptom) `forjar query` reported and filtered resource types in the Debug spelling too, so the type it printed was not one it accepted back. SCOPE, narrowed by the agy lane: this branch fixes the surfaces #366 names — policy scope, the compliance gate and `query`; roughly fifty other CLI display sites still print the Debug spelling (`src/cli/fleet_reporting.rs:261`, `check_test.rs:333`, `privilege_analysis.rs:94`, …) and are filed as #433.
   - evidence: `src/cli/infra_query.rs:75`, `:88` and `:95` at the merge-base. Pinned by `the_serde_spelling_of_a_multi_word_type_finds_the_resource`, `the_reported_type_is_the_one_the_document_declared`, `single_word_types_are_unaffected` in `tests/falsification_query_type_spelling.rs`.

3. [probe] (explains-symptom) Two rules with no `id:` sharing a `message:` were ONE rule to every consumer: `policy-coverage` could not add up and `remediate --policy-id` applied the sibling's edit.
   - evidence: `src/core/types/policy_rule_types.rs:141` (`display_id_of(None, message)` → `RULE-<slug of the message>`) was used as an identity; `src/core/policy_coverage/mod.rs:128` counted `total_rules` by index while `:174` (`trigger_split`) counted distinct id STRINGS, so `{"total_rules": 2, "rules_triggered": 1, "untriggered_rules": []}` — two is not one plus zero. On `remediate` (`Bucket::Unified`, shipped as `forjar_remediate`) `--policy-id RULE-baseline-hardening` selected BOTH rules, and the `(resource, id)` reason map reported an unfixable `assert` under the sibling `deny`'s reason. Pinned by `tests/falsification_policy_rule_identity.rs` (7 cases, 5 RED on unfixed code, library and shipped binary).

4. [design] `display_id_at(index)` — the explicit `id:` when there is one, else `RULE-<index>-<slug>` — is the identity on BOTH surfaces at once, and `trigger_split` splits by rule INDEX so `total_rules == rules_triggered + untriggered_rules.len()` is structural.
   - evidence: the ids the report PRINTS are `display_id_at`, so a name in `untriggered_rules` is one `remediate --policy-id` accepts; splitting these two changes would have manufactured the cross-surface disagreement #356 existed to delete. An explicit `id:` is verbatim; only the generated spelling moves (`RULE-<slug>` → `RULE-0-<slug>`), user-visible on a shipped MCP tool and done while the old spelling is provably ambiguous. Pinned by `an_explicit_id_is_still_the_identity`, `a_reported_policy_id_selects_exactly_the_rule_that_reported_it`, `an_unknown_policy_id_selects_nothing`, `each_unfixable_rule_reports_its_own_reason`, `the_printed_report_accounts_for_every_rule`.

5. [design] Two pins that asserted the WRONG answer on purpose are discharged by inversion, not deletion; `policy-coverage` stays withdrawn from the verb surface deliberately.
   - evidence: `two_unnamed_rules_sharing_a_message_*` in `src/core/policy_coverage/tests.rs` and `tests/falsification_policy_coverage_withdrawn.rs` now measure the right answer (the unit fixture also never had a satisfied rule — it required `resource_type`, not in `FIELD_PRESENCE` — and now declares a provider the second rule requires). Re-publishing a tool on every transport answers to the verb-surface suites and is its own decision; the ledger row, registry comment, withdrawal test and book say so instead of describing a repaired answer as still wrong.

6. [design] The comments say only what the code does — three that claimed more were corrected on the branch's own adversarial review.
   - evidence: `display_id_at` is injective over rules that declare NO `id:`; two rules both declaring `id: SEC-1` still collide on `remediate` (measured, both applied) — the doc says which half is closed and what closing the rest would take. The `FIELD_VALUES` claim about `remediate` refusing a `type`-keyed candidate was unreachable (`type` is not in `SETTABLE`); the `deny`/`assert` half is real and pinned by `the_type_field_reads_the_serde_spelling`.

## REFUTED — 4 claims killed

1. [design] refuted 1/1 — "Keep `RULE-<slug>` and make the slug unique by appending a counter only on collision."
   - corrected: a collision-only suffix makes an id depend on which OTHER rules exist, so adding a rule renames its sibling; index-based ids are stable under everything but reordering, which already changes what the config means.

2. [design] refuted 1/1 — "Fixing `display_id_at` closes #369 entirely."
   - corrected: only the un-id'd half; duplicate EXPLICIT ids are not diagnosed by `validate` or `lint` and still collide on `remediate`. Recorded in the receipt's known limits, not claimed.

3. [design] refuted 1/1 (agy lane) — "The Debug-spelling defect is resolved."
   - corrected: resolved where it was a silent comparison (#366's two surfaces) and where `query` printed a type it would not accept back; ~50 display-only sites remain and are #433. The lanes/design text above was reworded from "everywhere" to the surfaces named.

4. [design] refuted 1/1 (agy lane) — "`RULE-<index>-<slug>` is a stable cross-run identity."
   - corrected: it is unique within a config and stable under everything but reordering — which is exactly what an agent does between reading a coverage report and calling `forjar_remediate` (insert a rule above, the index shifts, the wrong rule is edited). The industry answer (Kyverno, Sentinel, OPA/Gatekeeper, ansible-lint) is a mandatory explicit unique id; that, plus rejecting duplicate and `RULE-`-prefixed explicit ids, is #434. This branch keeps the index form because it repairs the collapse #369 reported without changing the schema.

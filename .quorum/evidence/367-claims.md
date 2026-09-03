# Quorum evidence — #367 / #371 / #375 (fix/mcp-workspace-and-annotations) — adjudicated claims

## CONFIRMED — 7 claims survived refutation

1. [probe] (explains-symptom) The verb/MCP surface read the wrong directory once a workspace was selected: `forjar workspace select prod` moved the CLI's state to `state/prod/`, and every verb kept reading `state/`.
   - evidence: at the merge-base `src/cli/workspace.rs:235` (`resolve_state_dir`) joined the active workspace and `src/mcp/paths.rs:48` (`resolve_state_dir`) did not. Measured on 1.24.0 with `.forjar/workspace = prod`: `forjar plan` → 1 unchanged; `verb call plan` → `to_create: 1`; `verb call audit` → `event_count: 0` against a 4-line events.jsonl; `verb call status` → `machines: []`. None of the three answers carries a tell — GH-208's "no state" rendered as "you have no state", through a second door. Pinned by `plan_reads_the_lock_the_selection_points_at`, `audit_reads_the_trail_the_selection_points_at`, `status_sees_the_machines_under_the_selected_workspace` in `tests/falsification_verb_honours_the_workspace.rs`.

2. [design] The join happens only on the default branch; an explicit `state_dir` stays verbatim, and the workspace verb enumerates the UNJOINED base.
   - evidence: the documented workaround was to hand `workspace_state_dir` back as the next verb's `state_dir`; joining onto that resolves `state/prod/prod`. `resolve_state_base` is the new unjoined half and `WorkspaceHandler` (`src/mcp/handlers_ops.rs:25` at the merge-base, the seventh caller, the one that does its own `join(active)`) takes it. Pinned by `an_explicit_state_dir_is_not_joined_a_second_time`, `a_project_with_no_selection_still_resolves_the_bare_state_dir`, `the_workspace_verb_still_enumerates_the_state_base_not_the_joined_dir`, and the seven cases of `tests/falsification_verb_workspace_report.rs` (sorted listing, default as null, follows the config not the cwd, an empty listing distinguishable from a missing base, and `the_cli_and_the_verb_surface_honour_the_same_selection`).

3. [probe] (explains-symptom) `readOnlyHint` never reached a client: `forjar mcp --schema` published it for all twelve tools, and over real stdio every tool object was exactly `['description', 'inputSchema', 'name']`.
   - evidence: `src/mcp/registry.rs:28` at the merge-base emitted `annotations.readOnlyHint` into the `--schema` document only; the wire went through pforge's adapter, whose `metadata()` answers a bare `ToolInfo::new(..)` that hard-sets `annotations: None` (byte-identical in pforge 0.2.1, so a bump fixes nothing). forjar now builds the pmcp server itself (`src/mcp/adapter.rs`) and fills the field from `v.effects.read_only()`, never a literal. Pinned by `every_tool_sends_a_read_only_hint_over_the_wire` and `the_wire_and_the_schema_agree_per_tool` in `tests/falsification_mcp_publishes_readonly_hint_over_stdio.rs`.

4. [design] Dispatch stays on pforge's `HandlerRegistry`, and every advertised tool is dispatched by the suite, not only listed.
   - evidence: the verb table's own `invoke` builds a `tokio::runtime::Runtime` internally and would panic inside the async adapter; `every_advertised_tool_still_dispatches` calls each of the twelve tools over stdio and requires a non-error result. `build_forge_config` (`src/mcp/registry.rs:84` at the merge-base) described a value no user reached and is gone with `pforge-config`; `tests/falsification_default_features_trim.rs` pins that the default feature set still enables everything the binary needs.

5. [probe] (explains-symptom) `docs/mcp-schema.json` was a checked-in copy of the verb surface that nothing generated and nothing checked.
   - evidence: present at the merge-base (`docs/mcp-schema.json`), absent on the branch; `tests/falsification_mcp_surface_has_no_checked_in_copy.rs` pins that no JSON under `docs/` restates the tool surface, that the book does not tell anyone to write the snapshot back, and that no book page states a tool count the surface does not have.

6. [design] An `outputSchema` on the wire is a promise pmcp cannot keep, so it is NOT published — the first cut's free extra was a regression and was removed on adversarial review.
   - evidence: MCP 2025-06-18 (the revision this server negotiates, measured) makes an output schema an obligation to return conforming `structuredContent`; pmcp 1.20's `handle_call_tool` attaches `structuredContent` only through widget enrichment, so no `ToolHandler` value can reach it. Measured end to end with `@modelcontextprotocol/sdk` 1.30.0: with `output_schema` set, every `tools/call` failed with `-32600 … has an output schema but did not return structured content`; without it, `readOnlyHint` still true. `no_tool_promises_a_structured_result_the_server_does_not_send` asserts the PAIRING against a real succeeding call; re-adding the schema turns it red. `--schema` still documents `output_schema`, where it documents rather than promises.

7. [design] The falsifiers cannot pass vacuously.
   - evidence: the workspace cases apply real state under one workspace and assert the verb's numbers against the files on disk (`events_on_disk`, `applied_under`); the stdio cases drive the built binary through a real MCP session and assert per-tool fields; the docs case walks `docs/` for JSON files and would fail on the merge-base's checked-in copy.

## REFUTED — 3 claims killed

1. [design] refuted 1/1 — "Route `WorkspaceHandler` through the joined resolver too; one resolver is simpler."
   - corrected: that handler ENUMERATES `state/` and does its own `join(active)`; through the joined resolver its listing becomes the MACHINE directories under the active workspace — the same double-join, re-entered from inside the fix. Two halves, one unjoined, is the honest shape.

2. [design] refuted 1/1 — "Bump pforge to get annotations on the wire."
   - corrected: pforge 0.2.1's adapter is byte-identical on this point; the discard is in `metadata()` answering a bare `ToolInfo::new`. Building the pmcp server in-tree is the only path that fills the field.

3. [probe] refuted 1/1 (first cut, by the branch's own adversarial review) — "Publishing `outputSchema` on the wire is a free extra."
   - corrected: it broke every `tools/call` for the most widely deployed client stack; the presence-asserting test was green over the break and was replaced by the pairing test. Recorded above as claim 6.

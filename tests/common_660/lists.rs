//! The scan tests' hand-kept lists, each row with its reason.

pub const BASE_TOKENS: &[&str] = &[
    "parse_and_validate",
    "parse_and_validate_for_render",
    "parse_and_validate_unresolved",
    "parse_and_validate_opts",
    "load_resolved_value",
    "resolve_resource_templates",
    "resolve_resource_templates_with_secrets",
    "resolve_data_sources_with_base",
    "resolve_outputs",
    "require_data_sources",
    "finish_resolving",
];

/// Every `load_*` / `prepare_*` fn of `src/core/resolver` and `src/cli` that reads a
/// MANIFEST. The swallow scan watches each. A new one is found by `derive_loaders`
/// and is RED until it is listed here or in `NOT_A_MANIFEST_LOADER`.
pub const MANIFEST_LOADERS: &[&str] = &[
    "load_resolved_value",
    "load_resolved_config",
    "load_resolved_config_for_render",
    "load_resolved_config_with",
    "load_resolved_config_lenient",
    "load_apply_config",
    "prepare_config",
    "load_for_contracts",
    "load_drift_config",
    "load_config",
    "load_configs",
    "load_stacks",
];

/// `load_*` / `prepare_*` fns that read something other than a manifest.
pub const NOT_A_MANIFEST_LOADER: &[(&str, &str)] = &[
    ("load_rulebook_config", "reads a rulebook"),
    ("load_rulebook", "reads a rulebook"),
    ("load_snapshot", "reads a stored snapshot"),
    ("load_catalog", "reads the service catalog"),
    ("load_index", "reads the recipe index"),
    ("load_agent_registry", "reads the agent registry"),
    ("load_convergence_history", "reads state history"),
    ("load_plan_file", "reads a saved plan file"),
    ("load_check_locks", "reads lock files"),
    ("load_all_lock_resources", "reads lock files"),
    (
        "load_env_params",
        "reads the flat --env-file params mapping",
    ),
    ("load_lock_from_yaml", "reads a lock file"),
    ("load_env_resources", "reads lock files"),
    ("load_machine_events", "reads an event log"),
    ("load_destroy_entries", "reads a destroy log"),
    ("load_machine_locks", "reads lock files"),
    ("load_all_locks", "reads lock files"),
    ("load_generation_locks", "reads lock files"),
    ("load_baseline_from", "reads a benchmark baseline"),
    ("load_drift_trend", "reads drift history"),
    ("load_gen_locks", "reads lock files"),
];
pub const RAW_PARSE_ALLOWLIST: &[(&str, &str)] = &[
    ("src/cli/apply_helpers.rs:load_env_params", "parses the flat --env-file params mapping, not a manifest"),
    ("src/cli/destroy.rs:cmd_rollback", "parses a PREVIOUS git revision of the manifest to diff resource sets; data: sources must not run against history"),
    ("src/cli/lint_fix.rs:lint_auto_fix", "lint --fix rewrites the manifest text; it edits the raw document and must not run data: commands"),
    ("src/cli/lsp_validate.rs:validate_yaml", "checks YAML syntax of an editor buffer, the unsaved text of a file"),
    ("src/cli/reseal.rs:reseal_one", "reseal rewrites the manifest bytes in place; resolving would bake data: output into the file"),
    ("src/cli/state_identity.rs:applied_under_before", "reads only `name:` of a stored applied-config snapshot under the state dir"),
    ("src/cli/status_trends.rs:count_changes_from_logs", "parses one event-log line, not a manifest"),
    ("src/cli/template_cmd.rs:classify", "asks which top-level keys the file has (recipe vs config) before the loader runs; no value is read"),
    ("src/cli/template_cmd.rs:yaml_scalar", "parses one `-V KEY=VALUE` scalar, not a manifest"),
    ("src/cli/undo_helpers.rs:parse_replay_resource", "parses a resource fragment recorded in a destroy log"),
    ("src/cli/validate_policy.rs:cmd_validate_policy_file", "reads a separate POLICY file, not the manifest"),
    ("src/cli/verify.rs:recorded_output_hash", "reads a lock/attestation document under the state dir, not the manifest"),
    ("src/core/remediate/mod.rs:emit_in_style", "round-trips one candidate scalar to choose a quoting style"),
    ("src/core/state/integrity.rs:check_lock_file", "checks that a LOCK file is well-formed YAML, not a manifest"),
    ("src/core/store/convert_exec.rs:apply_conversion", "WRITES the converted manifest in place after a backup; resolving would bake data: output into the file"),
    ("src/core/store/derivation.rs:parse_derivation", "reads a derivation document of the store, not a manifest"),
    ("src/core/store/ingest.rs:ingest_generations", "reads a stored generation snapshot, not the live manifest"),
    ("src/core/store/ingest.rs:ingest_lock_file", "reads a state LOCK file into the store index"),
    ("src/core/store/ingest.rs:upsert_machine", "reads a state LOCK file into the store index"),
    ("src/core/store/meta.rs:read_meta", "reads store metadata, not a manifest"),
    ("src/core/store/provider.rs:parse_import_config", "reads an import-provider config, not a manifest"),
    ("src/core/store/secret_scan.rs:scan_yaml_str", "scans arbitrary YAML text for secrets; the text is the caller's, nothing resolves"),
    ("src/core/yaml_edit/verify.rs:changed_paths_of_text", "diffs two texts of an edit (before/after); the edit is made on the raw document"),
];

pub const UNRESOLVED_LOADER_ALLOWLIST: &[(&str, &str)] = &[
    ("src/cli/helpers.rs:parse_and_validate_unresolved", "defines the unresolved loader the rows below call"),
    ("src/mcp/handlers.rs:handle", "ValidateHandler parses unresolved FIRST only to tell an invalid manifest (a `valid: false` result) from a failing data: source (an error); it then calls the resolving loader"),
    ("src/cli/contracts.rs:load_for_contracts", "an UNPARSABLE manifest is graceful (codebase contracts still print); one that parses is then loaded through the resolving loader and its failure is returned"),
    ("src/cli/complexity_analysis.rs:cmd_complexity", "counts structure on the unresolved manifest; a failing data: source still fails it via helpers::require_data_sources"),
    ("src/cli/extract.rs:cmd_extract", "EMITS a sub-manifest: templates and `data:` sources stay in it; a failing source still fails it via helpers::require_data_sources"),
    ("src/cli/config_merge.rs:cmd_config_merge", "WRITES the merged manifest: resolving would bake data: output into the file; a failing source still fails it via helpers::require_data_sources"),
    ("src/cli/infra_bench.rs:run_benchmarks", "times parse+validate; the sources are checked once, outside the timed loops, via helpers::require_data_sources"),
    ("src/cli/init.rs:cmd_fmt", "fmt REWRITES the file, so it serializes the unresolved manifest; a failing source still fails it via helpers::require_data_sources"),
    ("src/cli/lsp_validate.rs:validate_yaml", "validates an editor buffer that is not on disk; no base dir, no data: run"),
    ("src/cli/undo_helpers.rs:build_replay_config", "rebuilds a config from a destroy-log record; the record holds already-resolved resources"),
];

pub const SWALLOW_ALLOWLIST: &[(&str, &str)] = &[
    ("src/cli/contracts.rs:load_for_contracts", "the Err(_) arm is the UNRESOLVED parse: a manifest that does not parse at all prints the codebase contracts; a manifest that parses but fails to resolve returns the resolve error"),
    ("src/cli/apply_variants.rs:refreshed_live_hash", "the config was loaded through the resolving loader, which already failed on an unknown reference; the fallback only covers a secrets-provider error, and the refresh query then runs the raw resource"),
    ("src/core/resolver/resource.rs:resolve_or_fallback", "documented fallback: warns on stderr and keeps the raw resource for destroy/refresh scripts"),
    ("src/cli/print_helpers.rs:print_plan_change", "display only: falls back to the raw text for a resource the planner already reported as unresolvable"),
    ("src/core/executor/refresh.rs:refresh_check_fails", "an unresolvable resource counts as a failing check, i.e. it is re-applied, which then reports the error"),
    ("src/core/executor/refresh_seed.rs:check_passes_on", "an unresolvable resource does not pass its check, which then reports the error on apply"),
];

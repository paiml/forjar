=== REFUTER 1 ===
[
  {"claim":"1.1","verdict":"SURVIVED","evidence":"src/core/parser/tests_known_fields_completeness.rs:69-74 `const ALIASES: &[&str] = &[ \"type\", \"moved_from\", \"max_age\", \"pattern\", \"entrypoint\",` confirms \"entrypoint\" was kept in ALIASES."},
  {"claim":"1.2","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:63,438-456 `if resource.provider.as_deref() != Some(\"aws\")`, `--count 1`, `MarketType=spot,SpotOptions={{MaxPrice={},SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}}`, `--metadata-options 'HttpTokens=required,HttpPutResponseHopLimit=1,HttpEndpoint=enabled'`; no Command outside `#[cfg(test)]`."},
  {"claim":"1.3","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:167-190,398-407,425-428 `if [ -e {kill} ] || [ -L {kill} ]`, `[ -f {gate} ] && grep -qx 'parity-gate: pass' {gate} && [ \"$(grep -c '^parity-gate:' {gate})\" = 1 ]`, `worst_case_usd > cap` all refuse before any aws call."},
  {"claim":"1.4","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:230-232,408-412 `\"#!/bin/sh\\nshutdown -h +{}\\nexport FORJAR_IMAGE={}\\n\"`, `squote_keep_newlines`, and src/resources/cloud_instance/tests_b1.rs:189 `log.contains(\"--user-data #!/bin/sh\\nshutdown -h +60\\nexport FORJAR_IMAGE='\")` confirm newline-preserving user-data and line-2 shutdown."},
  {"claim":"1.5","verdict":"REFUTED","evidence":"src/resources/cloud_instance/mod.rs:258-266,301,313,336 `for i in ${var}; do` without `set -f` and `if [ -n \"$ids\" ]; then echo \"present $ids\"` show PR_BODY.md:9 is FALSE, not TRUE: whitespace-only output (\" \" or \"\\t\") performs 0 loop iterations and passes lookup_ids (causing check_script present and state_query_script to exit 0 as present), and glob replies like `i-*` pass when matching a local file; `drift` is also not in `./tree`."},
  {"claim":"1.6","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:345-374,384-386 `if is_absent(resource) { return destroy_script(&s); }`, `if [ -z \"$ids\" ]; then echo 'absent: no owned instance'; exit 0; fi`, `aws ec2 terminate-instances`, `aws ec2 cancel-spot-instance-requests`, `if [ -n \"$left\" ]; then echo \"NOT destroyed: still pending or running: $left\" >&2; exit 1; fi` confirm PR_BODY.md:10."},
  {"claim":"1.7","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:168-171,425-426,445-446 `.filter(|p| p.starts_with('/') && !p.chars().any(char::is_control))`, `kill = sh_squote(&kill)`, `gate = sh_squote(&gate)` confirm kill_switch and parity_gate reach shell without a character allow-list."},
  {"claim":"1.8","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:70,110,143-164 `let name = field(&resource.name, \"name\")?;`, `!t.starts_with(\"forjar-key=\") && !t.starts_with(\"forjar-owner=\")`, `pub fn validate_unique_names`, and src/core/parser/mod.rs:200 confirm reserved-tag refusal and parse-time name/region uniqueness."},
  {"claim":"1.9","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:128-138,329-338,430-437 `format!(\"{}@{}@{}\", s.name, s.region, &digest[..12])`, `format!(\"{}@{}\", s.name, s.region)`, `REFUSED: image changed while instance$other is alive`, and src/resources/cloud_instance/validate.rs:19-70 (no allow-list permits `@`) confirm PR_BODY.md:12."},
  {"claim":"1.10","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests.rs:50-139,202-241 and src/resources/cloud_instance/tests_b1.rs:36-92,113-162,217-254,332-411 `const STUB`, `for evil in [` (7 hostile values x 7 fields), `readers(sb)` confirm PR_BODY.md:16."},
  {"claim":"1.11","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests.rs:141-311 (11 tests), tests_b1.rs:20-411 (20 tests), tests_b4.rs:20-207 (11 tests) total 42 `#[test]` functions across the 36-file `./tree` snapshot with no Cargo.toml or git history to verify PR_BODY.md:17."},
  {"claim":"1.12","verdict":"SURVIVED","evidence":".github/workflows/ci.yml:83,129-133,214-215,380,441 `heavy_on: train`, `if: needs.classify.outputs.code == 'true'`, `run: \"cargo test --locked --lib resources::cloud_instance::\"`, `ok \"examples-validate\"` confirm the new required CI step while external SHA/workflow behavior remains unverified."},
  {"claim":"1.13","verdict":"SURVIVED","evidence":"PR_BODY.md:21,25 `Claude Opus FAIL at 112ed7d2`, `1.95 run: 22 errors` cannot be verified from `./tree`, while the listed code fixes are present in src/resources/cloud_instance/{mod.rs,validate.rs,tests.rs,tests_b1.rs,tests_b4.rs}."},
  {"claim":"1.14","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:143-164,192-224,398-407,409,425,452, src/core/parser/resource_types.rs:48-52, and docs/roadmaps/roadmap.yaml:7506-7510 `status: planned` confirm PR_BODY.md:29-34,37."},
  {"claim":"1.15","verdict":"SURVIVED","evidence":"src/core/parser/resource_types.rs:51 `|| resource.tags.iter().any(|t| t.contains(\"{{\"));` skips all `spec_of` validation, while src/core/resolver/resource.rs:222-242 never resolves `r.tags` and src/resources/cloud_instance/validate.rs:62-70 `is_tag` rejects `{` and `}`."},
  {"claim":"1.16","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:258-266,301,313,336,364-367,432-436 `for i in ${var}; do` without `set -f` accepts whitespace-only output (leaving `[ -n \"$ids\" ]` true and passing zero IDs to terminate-instances) and glob-expands unquoted `${var}` against cwd."},
  {"claim":"1.17","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:430-444 `{lookup_owned}{lookup_keyed}if [ -z \"$owned\" ]; then :; else ... fi` unconditionally calls `lookup_keyed` and discards `$keyed` whenever `$owned` is empty before running `aws ec2 run-instances`."},
  {"claim":"1.18","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:142-143 `/// Values still holding a `{{` template are skipped (checked once resolved).` is contradicted by src/core/parser/mod.rs:200,367,388 (its only non-test call site, pre-template-resolution) and src/core/planner/mod.rs:62 (resolves templates without calling `validate_unique_names`)."},
  {"claim":"1.19","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:362-372 `set -eu`, `sirs=\"$({sirs})\"` runs before `aws ec2 terminate-instances` (aborting teardown on `sirs` error), and `left=\"$({ids})\"` bypasses `lookup_ids` without emitting `instance-lookup-failed`."},
  {"claim":"1.20","verdict":"SURVIVED","evidence":"src/core/observe/classify_e01.rs:78-89 `\"cloud_region\" | ... => Observability::Unmigrated(\"#861 — the provider query keys on name, region and image digest; this field is applied by the create call and the gates but never read back\")` is stale against src/resources/cloud_instance/mod.rs:329-338 (`forjar-owner`, `name@region`) and contradicts `--region`."},
  {"claim":"2.1","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:63-65,438-456 `if resource.provider.as_deref() != Some(\"aws\")`, `--count 1`, `MarketType=spot,SpotOptions={{MaxPrice={},SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}}`, `--metadata-options 'HttpTokens=required,HttpPutResponseHopLimit=1,HttpEndpoint=enabled'` confirm PR_BODY.md:6."},
  {"claim":"2.2","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:387-407,425-428 `gates_of(resource)`, `worst_case_usd > cap`, `if [ -e {kill} ] || [ -L {kill} ]`, `[ -f {gate} ] && grep -qx 'parity-gate: pass' {gate} && [ \"$(grep -c '^parity-gate:' {gate})\" = 1 ]` confirm the gate and budget claims in PR_BODY.md:8."},
  {"claim":"2.3","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:258-266,301,313,336,432-437 `for i in ${var}; do` without `set -f` lets whitespace-only replies (`\" \"` / `\"\\t\"`) and matching globs bypass `lookup_ids` without error, falsifying PR_BODY.md:9."},
  {"claim":"2.4","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:345-374,384-386 `if is_absent(resource) { return destroy_script(&s); }`, `\"forjar-owner\"`, `if [ -z \"$ids\" ]; then echo 'absent: no owned instance'; exit 0; fi`, `if [ -n \"$left\" ]; then echo \"NOT destroyed: still pending or running: $left\" >&2; exit 1; fi` confirm PR_BODY.md:10."},
  {"claim":"2.5","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:167-171,425-426 `v.clone().filter(|p| p.starts_with('/') && !p.chars().any(char::is_control))` confirms `cloud_kill_switch` and `cloud_parity_gate` are not allow-listed, falsifying PR_BODY.md:11."},
  {"claim":"2.6","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:128-138,329-334,430-437 and src/resources/cloud_instance/validate.rs:19-30 confirm `forjar-key`, `forjar-owner`, `@` exclusion, owner-first `apply_script`, and owner-based `state_query_script`."},
  {"claim":"2.7","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:230-232,408-412,457 `\"#!/bin/sh\\nshutdown -h +{}\\nexport FORJAR_IMAGE={}\\n\"`, `squote_keep_newlines(&user_data)`, and src/resources/cloud_instance/tests_b1.rs:189 confirm PR_BODY.md:13."},
  {"claim":"2.8","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests.rs:50-72,202-241 and src/resources/cloud_instance/tests_b1.rs:36-79,113-162,217-254,332-400 confirm the sandbox stub tests and call-log assertions in PR_BODY.md:16."},
  {"claim":"2.9","verdict":"SURVIVED","evidence":"PR_BODY.md:17,25 `357 passed, 0 failed` and `1.95 run: 22 errors` cannot be verified from the 36-file read-only `./tree` snapshot without Cargo.toml or git history."},
  {"claim":"2.10","verdict":"SURVIVED","evidence":".github/workflows/ci.yml:83,129-131,214-215,380,441 `heavy_on: train`, `run: \"cargo test --locked --lib resources::cloud_instance::\"`, `ok \"examples-validate\"` confirm PR_BODY.md:18."},
  {"claim":"2.11","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:143-164,220-222,398-407,425,438-444, src/core/parser/resource_types.rs:48-51, and docs/roadmaps/roadmap.yaml:7506-7510 `status: planned` confirm PR_BODY.md:29-34."},
  {"claim":"2.12","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:260-265,301,313,336,364-367,432-437 `for i in ${var}; do` executes 0 iterations on whitespace-only output while `[ -n \"$ids\" ]` is true and `set -- $ids` yields 0 args for `terminate-instances`."},
  {"claim":"2.13","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:261,366,368,433-434 `for i in ${var}; do`, `set -- $ids`, `for r in $sirs; do` run without `set -f`, allowing pathname globbing of malformed output like `i-*` against local files."},
  {"claim":"2.14","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests_b4.rs:65-68 `!out.contains(\"alive i-00a1b2c3\")` can never fail against src/resources/cloud_instance/mod.rs:435 `REFUSED: image changed while instance$other is alive` because `$other` always precedes `is alive`."},
  {"claim":"2.15","verdict":"SURVIVED","evidence":"src/core/parser/resource_types.rs:48-55 `|| resource.tags.iter().any(|t| t.contains(\"{{\"));` skips all `spec_of` checks while src/core/resolver/resource.rs:222-242 never resolves `r.tags`, causing guaranteed codegen failure in `is_tag`."},
  {"claim":"2.16","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:142-155 `/// Values still holding a `{{` template are skipped (checked once resolved).` has its only non-test caller in pre-resolution src/core/parser/mod.rs:200 `validate_unique_names(config)`."},
  {"claim":"2.17","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:67,197-224,384-386 `plan_diff` never checks `is_absent(resource)` and always emits `+ cloud_instance` and `gates (apply refuses unless all declared):`, and no test calls `plan_diff` with `state: absent`."},
  {"claim":"2.18","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:430-438 `{lookup_owned}{lookup_keyed}if [ -z \"$owned\" ]; then :; else ... fi` executes `lookup_keyed` unconditionally and ignores `$keyed` when `$owned` is empty before launching."},
  {"claim":"2.19","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:308-318 `lookup = lookup_ids(\"ids\", &describe(&s))` queries only `forjar-key` and exits 0 (`instance-present`) when both a matching-key and stale-key owned instance exist, whereas `apply_script` at mod.rs:430-437 refuses with exit 1."},
  {"claim":"2.20","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:365,368-371 `left=\"$({ids})\"` bypasses `lookup_ids`, `case \"$r\" in sir-[0-9a-z]*)` checks only one char after `sir-`, and src/resources/cloud_instance/tests*.rs never test `sirs`, `cancel-spot-instance-requests`, or `left` lookup failures."},
  {"claim":"2.21","verdict":"SURVIVED","evidence":"scripts/cb200-ratchet.sh:85-95 `msg=$(grep -m 1 'below minimum grade' <<<\"$msgs\" || true)` stops at the first matching message even without `N function`, diverging from the replaced Python walker in ANGLE.diff:77-81."},
  {"claim":"2.22","verdict":"SURVIVED","evidence":"src/core/observe/classify_e01.rs:78-89 claims `state_query` keys on `name, region and image digest` and classifies `cloud_region` as `never read back`, contradicting itself and src/resources/cloud_instance/mod.rs:329-334 (`forjar-owner`)."},
  {"claim":"2.23","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests.rs:52,202-225 and tests_b4.rs:102-123,154-168,191-207 leave relative gate paths, non-overflowing invalid `budget_cap_usd_day` in `apply_script`, `worst_case_usd == cap`, and `lookup_keyed`-only failure untested, and `echo \"$@\" >> \"$STUB_LOG\"` flattens args in the log."},
  {"claim":"3.1","verdict":"SURVIVED","evidence":"src/core/parser/tests_known_fields_completeness.rs:69-74 `\"entrypoint\",` confirms `PR_BODY.md:3` kept the `entrypoint` alias in `ALIASES`."},
  {"claim":"3.2","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:63-65,438-456 `--count 1`, `MarketType=spot,SpotOptions={{MaxPrice={},SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}}`, `--metadata-options 'HttpTokens=required,HttpPutResponseHopLimit=1,HttpEndpoint=enabled'` confirm `PR_BODY.md:6`."},
  {"claim":"3.3","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:387-407,425-428 `if [ -e {kill} ] || [ -L {kill} ]`, `grep -qx 'parity-gate: pass' {gate} && [ \"$(grep -c '^parity-gate:' {gate})\" = 1 ]`, `worst_case_usd > cap` confirm `PR_BODY.md:8`."},
  {"claim":"3.4","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:258-266,313,336 `for i in ${var}; do` without `set -f` lets whitespace-only (`\" \"`/`\"\\t\"`) and matching glob replies pass `lookup_ids` without error, falsifying `PR_BODY.md:9`."},
  {"claim":"3.5","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:345-374,384-386 `if is_absent(resource) { return destroy_script(&s); }`, `if [ -z \"$ids\" ]; then echo 'absent: no owned instance'; exit 0; fi`, `if [ -n \"$left\" ]; then echo \"NOT destroyed: still pending or running: $left\" >&2; exit 1; fi` confirm `PR_BODY.md:10`."},
  {"claim":"3.6","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:168-171,425-426,445-446 `.filter(|p| p.starts_with('/') && !p.chars().any(char::is_control))`, `kill = sh_squote(&kill)`, `gate = sh_squote(&gate)` confirm `cloud_kill_switch` and `cloud_parity_gate` are not allow-listed, falsifying `PR_BODY.md:11`."},
  {"claim":"3.7","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:128-138,329-338,430-437 and src/resources/cloud_instance/validate.rs:19-30 confirm `@`-joined ownership tags, `@` exclusion, owner-first apply refusal on image change, and owner-based `state_query`."},
  {"claim":"3.8","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:230-232,408-412,457 `\"#!/bin/sh\\nshutdown -h +{}\\nexport FORJAR_IMAGE={}\\n\"` and src/resources/cloud_instance/tests_b1.rs:188-192 confirm `PR_BODY.md:13`."},
  {"claim":"3.9","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/tests.rs:51-125,202-241 and src/resources/cloud_instance/tests_b1.rs:36-80,118-162,217-254,332-411 confirm the sandbox stub tests and 7x7 injection matrix claimed in `PR_BODY.md:16`."},
  {"claim":"3.10","verdict":"SURVIVED","evidence":"PR_BODY.md:17 `cargo fmt --all -- --check` and `357 passed, 0 failed` cannot be executed or verified from the 36-file `./tree` snapshot."},
  {"claim":"3.11","verdict":"SURVIVED","evidence":".github/workflows/ci.yml:83,129-133,214-215,441 `heavy_on: train`, `run: \"cargo test --locked --lib resources::cloud_instance::\"`, `ok \"examples-validate\"` confirm `PR_BODY.md:18`."},
  {"claim":"3.12","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:197-224,409,425, src/core/parser/resource_types.rs:48-51, and docs/roadmaps/roadmap.yaml:7506-7510 `status: planned` confirm the known limits in `PR_BODY.md:29-34`."},
  {"claim":"3.13","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:258-266,366,433 `for i in ${var}; do`, `set -- $ids`, `set -- $keyed` omit `set -f`, allowing glob metacharacters like `i-*` to expand against cwd during validation and again downstream."},
  {"claim":"3.14","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:260-265,313,336,364-367,432-436 `for i in ${var}; do` skips validation on whitespace-only output (`\" \"`/`\"\\t\"`), leaving `[ -n \"$ids\" ]` true and invoking `terminate-instances --instance-ids` with zero IDs."},
  {"claim":"3.15","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:365-371 `for r in $sirs` runs without `set -f`, `case \"$r\" in sir-[0-9a-z]*) ;; *) continue;; esac;` checks only one char after `sir-` and ignores garbage, and `left=\"$({ids})\"` bypasses `lookup_ids`."},
  {"claim":"3.16","verdict":"SURVIVED","evidence":"src/core/parser/resource_types.rs:47-60 skips all `spec_of` checks when `name`, `image`, `cloud_region`, or `tags` contains `\"{{\"` while src/core/resolver/resource.rs:34-242 resolves `provider` and `state` (lines 123-124) but never resolves `r.tags`."},
  {"claim":"3.17","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:140-143 `/// Values still holding a `{{` template are skipped (checked once resolved).` is false because src/core/parser/mod.rs:198-203 is the only non-test caller of `validate_unique_names`."},
  {"claim":"3.18","verdict":"SURVIVED","evidence":"src/core/observe/classify_e01.rs:14-17,78-89,165-168 misstates the `state_query` key (`forjar-owner` in mod.rs:329-334, not image digest), contradicts `cloud_region` usage in `describe_by` (mod.rs:242-246), and marks controller-side gates `Unmigrated` instead of `Unobservable`."},
  {"claim":"3.19","verdict":"SURVIVED","evidence":"src/resources/cloud_instance/mod.rs:196-224 `- destroy: after {life} min (self-terminate) or when the kill switch exists` falsely claims the kill switch destroys running instances (mod.rs:425 only blocks apply), and `plan_diff` ignores `resource.state`."},
  {"claim":"3.20","verdict":"SURVIVED","evidence":"scripts/cb200-ratchet.sh:21-30,85-95 (43 diff lines in ANGLE.diff:46-106 replacing `python3` JSON parsing with bash/grep) is omitted from PR_BODY.md:1-40."},
  {"claim":"3.21","verdict":"SURVIVED","evidence":"scripts/cb200-ratchet.sh:26-29,85-95 `[[ $(<\"$BASE\") =~ \\\"ceiling\\\"[[:space:]]*:[[:space:]]*([0-9]+) ]]` matches `12` from `12.5`/`12abc`, `grep -oE` accepts truncated JSON, and `grep -m 1 'below minimum grade'` diverges from ANGLE.diff:77-81 when an earlier message lacks `N function`."},
  {"claim":"3.22","verdict":"SURVIVED","evidence":"src/core/types/resource.rs:470-473 `pub budget: DiskBudgetSpec,` and src/core/types/disk_budget_types.rs:75-97 remove five `pub` fields from `pub struct Resource` across 9 files in ANGLE.diff with zero mention in PR_BODY.md:1-40."}
]
VERDICT: FAIL
text_only: yes
refuter: 1
reviewer_model: gemini-4-neon-high

=== REFUTER 2 (redo, tab format) ===
1.1	SURVIVED	src/core/parser/tests_known_fields_completeness.rs:69-74 "const ALIASES: &[&str] = &[ ... \"entrypoint\"," confirms entrypoint remains in ALIASES.
1.2	SURVIVED	src/resources/cloud_instance/mod.rs:63,438-456 "if resource.provider.as_deref() != Some(\"aws\")", "--count 1", "MarketType=spot,SpotOptions={{MaxPrice={},SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}}", "--metadata-options 'HttpTokens=required,HttpPutResponseHopLimit=1,HttpEndpoint=enabled'" confirms one AWS one-time spot instance with IMDSv2 hop limit 1 and shell-only generation.
1.3	SURVIVED	src/resources/cloud_instance/mod.rs:387-407,425-428 "if [ -e {kill} ] || [ -L {kill} ]", "[ -f {gate} ] && grep -qx 'parity-gate: pass' {gate} && [ \"$(grep -c '^parity-gate:' {gate})\" = 1 ]", "worst_case_usd > cap" confirms apply refuses before any provider call on gate, kill-switch, or budget failure.
1.4	SURVIVED	src/resources/cloud_instance/mod.rs:230-232,409 and src/resources/cloud_instance/tests_b1.rs:189 "\"#!/bin/sh\\nshutdown -h +{}\\nexport FORJAR_IMAGE={}\\n\"" and "log.contains(\"--user-data #!/bin/sh\\nshutdown -h +60\\nexport FORJAR_IMAGE='\")" confirm user-data order and newline preservation.
1.5	REFUTED	src/resources/cloud_instance/mod.rs:260-265,313,336 "for i in ${var}; do ... done" without "set -f" executes 0 iterations on whitespace-only replies (" " or "\\t") and glob-expands replies like "i-*" against cwd, so non-instance-id replies pass lookup_ids without exiting 1 and evaluate "[ -n \"$ids\" ]" as true; labeling PR_BODY line 9 TRUE is false.
1.6	SURVIVED	src/resources/cloud_instance/mod.rs:345-374,384-386 "if is_absent(resource) { return destroy_script(&s); }", "if [ -z \"$ids\" ]; then echo 'absent: no owned instance'; exit 0; fi", "aws ec2 terminate-instances", "aws ec2 cancel-spot-instance-requests", "NOT destroyed: still pending or running: $left" confirms state: absent teardown behavior and omission of gates.
1.7	SURVIVED	src/resources/cloud_instance/mod.rs:168-171,425-426,445-446 ".filter(|p| p.starts_with('/') && !p.chars().any(char::is_control))" and "kill = sh_squote(&kill)" / "gate = sh_squote(&gate)" confirm cloud_kill_switch and cloud_parity_gate reach shell without a character allow-list.
1.8	SURVIVED	src/resources/cloud_instance/mod.rs:70,109-112,143-164 and src/core/parser/mod.rs:199-203 "let name = field(&resource.name, \"name\")?;", "!t.starts_with(\"forjar-key=\") && !t.starts_with(\"forjar-owner=\")", "validate_unique_names(config)" confirm reserved tags are refused and name is required and uniqueness-checked per config and region.
1.9	SURVIVED	src/resources/cloud_instance/mod.rs:128-138,329-338,430-437 and src/resources/cloud_instance/validate.rs:19-70 "format!(\"{}@{}@{}\", s.name, s.region, &digest[..12])", "format!(\"{}@{}\", s.name, s.region)", "REFUSED: image changed while instance$other is alive" confirm ownership tags, @ exclusion, apply owner-first check, and owner-based state_query.
1.10	SURVIVED	src/resources/cloud_instance/tests.rs:50-72,202-241 and src/resources/cloud_instance/tests_b1.rs:36-92,118-161,217-254,332-411 "const STUB", "for evil in [...]" across 7 fields, "two_declared_boxes_get_two_keys_two_instances_and_independent_teardown", and "readers" confirm the described sandbox stub test matrix.
1.11	SURVIVED	src/resources/cloud_instance/tests.rs:141-311, tests_b1.rs:20-411, tests_b4.rs:20-207 contain 42 #[test] functions and ./tree has only 36 changed files with no Cargo.toml or git history, so PR_BODY line 17 test/fmt run counts and commit 781a21b1 are unverified.
1.12	SURVIVED	.github/workflows/ci.yml:83,129-133,214-215,380,441 "heavy_on: train", "if: needs.classify.outputs.code == 'true'", "run: \"cargo test --locked --lib resources::cloud_instance::\"", "ok \"examples-validate\"" confirm the new required CI step on code PRs.
1.13	SURVIVED	PR_BODY.md:21,25 prior review SHAs and rustc 1.95/1.93 clippy outputs cannot be verified from ./tree, while the listed round-4 fixes are present in src/resources/cloud_instance/mod.rs:137,170,261-264,430-436 and validate.rs:43-52.
1.14	SURVIVED	src/resources/cloud_instance/mod.rs:143-164,197-224,398-407,409,425,438-452, src/core/parser/resource_types.rs:48-51, and docs/roadmaps/roadmap.yaml:7506-7510 ("status: planned", "resolve:ssm:/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64") confirm the disclosed known limits and gaps.
1.15	SURVIVED	src/core/parser/resource_types.rs:48-55 "|| resource.tags.iter().any(|t| t.contains(\"{{\"))" skips all spec_of checks while src/core/resolver/resource.rs:222-242 never resolves r.tags and src/resources/cloud_instance/validate.rs:62-70 is_tag rejects { and }.
1.16	SURVIVED	src/resources/cloud_instance/mod.rs:260-265,301,313,336,364-367,432-436 "for i in ${var}; do" without "set -f" passes whitespace-only output (" "/"\\t") with "[ -n \"$ids\" ]" true and glob-expands patterns like "i-*" against cwd files.
1.17	SURVIVED	src/resources/cloud_instance/mod.rs:430-438 "{lookup_owned}{lookup_keyed}if [ -z \"$owned\" ]; then :; else ... fi" runs lookup_keyed unconditionally before checking $owned and ignores $keyed whenever $owned is empty, proceeding to "aws ec2 run-instances".
1.18	SURVIVED	src/resources/cloud_instance/mod.rs:142 "/// Values still holding a `{{` template are skipped (checked once resolved)." is contradicted by ANGLE.diff and src/core/parser/mod.rs:199-203 showing validate_unique_names is only called inside pre-resolution validate_config.
1.19	SURVIVED	src/resources/cloud_instance/mod.rs:362-371 "set -eu", "sirs=\"$({sirs})\"", "aws ec2 terminate-instances", and "left=\"$({ids})\"" confirm sirs failure aborts before terminate-instances and post-terminate re-read bypasses lookup_ids.
1.20	SURVIVED	src/core/observe/classify_e01.rs:78-89 "\"#861 — the provider query keys on name, region and image digest; this field is applied by the create call and the gates but never read back\"" contradicts src/resources/cloud_instance/mod.rs:242-246,329-334 which query by forjar-owner (name@region) and pass --region.
2.1	SURVIVED	src/resources/cloud_instance/mod.rs:63-65,438-456 "if resource.provider.as_deref() != Some(\"aws\")", "--count 1", "MarketType=spot,SpotOptions={{MaxPrice={},SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}}", "--metadata-options 'HttpTokens=required,HttpPutResponseHopLimit=1,HttpEndpoint=enabled'" confirms PR_BODY.md:6.
2.2	SURVIVED	src/resources/cloud_instance/mod.rs:387-407,425-428 "gates_of(resource)", "worst_case_usd > cap", "if [ -e {kill} ] || [ -L {kill} ]", "grep -qx 'parity-gate: pass' {gate} && [ \"$(grep -c '^parity-gate:' {gate})\" = 1 ]" confirms PR_BODY.md:8.
2.3	SURVIVED	src/resources/cloud_instance/mod.rs:258-266,301,313,336,432-437 "for i in ${var}; do" without "set -f" passes whitespace-only (" "/"\\t") and matching glob replies without exiting 1, falsifying PR_BODY.md:9.
2.4	SURVIVED	src/resources/cloud_instance/mod.rs:345-374,384-386 "if is_absent(resource) { return destroy_script(&s); }", "\"forjar-owner\"", "aws ec2 terminate-instances", "aws ec2 cancel-spot-instance-requests", "NOT destroyed: still pending or running: $left" confirms PR_BODY.md:10.
2.5	SURVIVED	src/resources/cloud_instance/mod.rs:167-171,425-426,445-446 ".filter(|p| p.starts_with('/') && !p.chars().any(char::is_control))" and "kill = sh_squote(&kill)" / "gate = sh_squote(&gate)" falsify PR_BODY.md:11's claim that every declared value reaching shell is allow-listed.
2.6	SURVIVED	src/resources/cloud_instance/mod.rs:128-138,329-334,430-437 and src/resources/cloud_instance/validate.rs:19-30 "format!(\"{}@{}@{}\", s.name, s.region, &digest[..12])", "format!(\"{}@{}\", s.name, s.region)", "REFUSED: image changed while instance$other is alive" confirms PR_BODY.md:12.
2.7	SURVIVED	src/resources/cloud_instance/mod.rs:230-232,408-412,457 and src/resources/cloud_instance/tests_b1.rs:188-191 "\"#!/bin/sh\\nshutdown -h +{}\\nexport FORJAR_IMAGE={}\\n\"" and "log.contains(\"--user-data #!/bin/sh\\nshutdown -h +60\\nexport FORJAR_IMAGE='\")" confirms PR_BODY.md:13.
2.8	SURVIVED	src/resources/cloud_instance/tests.rs:50-72,202-241 and src/resources/cloud_instance/tests_b1.rs:36-79,113-162,217-254,332-400 confirm the sandbox stub tests and call-log assertions summarized in PR_BODY.md:16.
2.9	SURVIVED	PR_BODY.md:17,25 fmt/test/clippy runs and commit 781a21b1 cannot be executed or verified from the 36-file partial ./tree snapshot.
2.10	SURVIVED	.github/workflows/ci.yml:83,129-131,214-215,380,441 "heavy_on: train", "run: \"cargo test --locked --lib resources::cloud_instance::\"", "ok \"examples-validate\"" confirms PR_BODY.md:18.
2.11	SURVIVED	src/resources/cloud_instance/mod.rs:143-164,220-222,398-407,425,438-444, src/core/parser/resource_types.rs:48-51, and docs/roadmaps/roadmap.yaml:7506-7510 ("status: planned") confirm PR_BODY.md:29-34.
2.12	SURVIVED	src/resources/cloud_instance/mod.rs:260-265,301,313,336,364-367,432-437 "for i in ${var}; do" executes 0 iterations on whitespace-only output (" "/"\\t") while "[ -n \"$ids\" ]" is true, misreporting present and invoking terminate-instances with zero IDs.
2.13	SURVIVED	src/resources/cloud_instance/mod.rs:261,366,368,433-434 "for i in ${var}; do", "set -- $ids", "for r in $sirs; do", "set -- $keyed", "for i in $owned; do" run without "set -f", enabling pathname globbing against cwd.
2.14	SURVIVED	src/resources/cloud_instance/tests_b4.rs:66 "!out.contains(\"alive i-00a1b2c3\")" vs src/resources/cloud_instance/mod.rs:435 "REFUSED: image changed while instance$other is alive;" confirms "alive" always follows $other so the negative assertion is vacuous.
2.15	SURVIVED	src/core/parser/resource_types.rs:48-55 "|| resource.tags.iter().any(|t| t.contains(\"{{\"))" vs src/core/resolver/resource.rs:222-242 (which never resolves r.tags) and src/resources/cloud_instance/validate.rs:62-70 (is_tag rejects { and }) confirms templated tags pass validate and fail at codegen.
2.16	SURVIVED	src/resources/cloud_instance/mod.rs:142-155 and src/core/parser/mod.rs:198-203,367-388 show validate_unique_names is only invoked in validate_config on unresolved templates, never post-resolution, contradicting "(checked once resolved)".
2.17	SURVIVED	src/resources/cloud_instance/mod.rs:67,197-224,384-386 show plan_diff never calls is_absent(resource) and always renders "+ cloud_instance" and "gates (apply refuses unless all declared):", contradicting apply_script's ungated destroy_script for state: absent, and no test calls plan_diff on absent.
2.18	SURVIVED	src/resources/cloud_instance/mod.rs:430-438 "{lookup_owned}{lookup_keyed}if [ -z \"$owned\" ]; then :; else ... fi" runs lookup_keyed unconditionally and discards $keyed when $owned is empty before calling aws ec2 run-instances.
2.19	SURVIVED	src/resources/cloud_instance/mod.rs:308-318,430-437 "lookup = lookup_ids(\"ids\", &describe(&s))" in check_script queries only forjar-key and exits 0 when a matching-key instance exists alongside a stale-key owned instance, whereas apply_script refuses with exit 1.
2.20	SURVIVED	src/resources/cloud_instance/mod.rs:365,368-371 "sirs=\"$({sirs})\"", "case \"$r\" in sir-[0-9a-z]*) ;; *) continue;; esac;", "left=\"$({ids})\"" and tests.rs:50-72 confirm left bypasses lookup_ids, sir pattern checks only one char after sir-, and those failure paths are untested.
2.21	SURVIVED	scripts/cb200-ratchet.sh:85-95 "msg=$(grep -m 1 'below minimum grade' <<<\"$msgs\" || true)" vs ANGLE.diff:77-81 ("if \"below minimum grade\" in m: n = re.search(r\"(\\d+) function\", m)") confirms bash stops at the first matching message even without an N function count, yielding now=-1.
2.22	SURVIVED	src/core/observe/classify_e01.rs:78-89 "\"cloud_region\" | ... => Observability::Unmigrated(\"#861 — the provider query keys on name, region and image digest; this field is applied by the create call and the gates but never read back\")" is stale and self-contradictory against mod.rs:329-334.
2.23	SURVIVED	src/resources/cloud_instance/tests.rs:52,202-225 and tests_b4.rs:102-123,154-168,191-207 confirm relative gate/kill paths, non-overflowing invalid budget_cap_usd_day, exact spend equality, and second-describe failure are untested, and tests.rs:52 "echo \"$@\" >> \"$STUB_LOG\"" flattens arguments so tests_b4.rs:165 cannot alone distinguish argument splitting.
3.1	SURVIVED	src/core/parser/tests_known_fields_completeness.rs:69-74 "\"entrypoint\"," confirms ALIASES retains entrypoint.
3.2	SURVIVED	src/resources/cloud_instance/mod.rs:63-65,438-456 "--count 1", "MarketType=spot,SpotOptions={{MaxPrice={},SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}}", "--metadata-options 'HttpTokens=required,HttpPutResponseHopLimit=1,HttpEndpoint=enabled'" confirms PR_BODY.md:6.
3.3	SURVIVED	src/resources/cloud_instance/mod.rs:387-407,425-428 "if [ -e {kill} ] || [ -L {kill} ]", "grep -qx 'parity-gate: pass' {gate} && [ \"$(grep -c '^parity-gate:' {gate})\" = 1 ]", "worst_case_usd > cap" confirms PR_BODY.md:8.
3.4	SURVIVED	src/resources/cloud_instance/mod.rs:258-266,313,336 "for i in ${var}; do", "if [ -n \"$ids\" ]; then echo \"present $ids\"" confirms whitespace-only and matching glob replies bypass lookup_ids without exiting 1, falsifying PR_BODY.md:9.
3.5	SURVIVED	src/resources/cloud_instance/mod.rs:345-374,384-386 "if is_absent(resource) { return destroy_script(&s); }", "if [ -z \"$ids\" ]; then echo 'absent: no owned instance'; exit 0; fi", "NOT destroyed: still pending or running: $left" confirms PR_BODY.md:10.
3.6	SURVIVED	src/resources/cloud_instance/mod.rs:168-171,425-426,445-446 ".filter(|p| p.starts_with('/') && !p.chars().any(char::is_control))" and "kill = sh_squote(&kill)" / "gate = sh_squote(&gate)" falsify PR_BODY.md:11's claim that every declared value reaching shell is allow-listed.
3.7	SURVIVED	src/resources/cloud_instance/mod.rs:128-138,329-338,430-437 and src/resources/cloud_instance/validate.rs:19-30 confirm @-joined ownership tags, @ exclusion from allow-lists, owner-first apply refusal on image change, and owner-based state_query.
3.8	SURVIVED	src/resources/cloud_instance/mod.rs:230-232,408-412,457 and src/resources/cloud_instance/tests_b1.rs:188-192 "\"#!/bin/sh\\nshutdown -h +{}\\nexport FORJAR_IMAGE={}\\n\"" and "log.contains(\"--user-data #!/bin/sh\\nshutdown -h +60\\nexport FORJAR_IMAGE='\")" confirms PR_BODY.md:13.
3.9	SURVIVED	src/resources/cloud_instance/tests.rs:51-125,202-241 and src/resources/cloud_instance/tests_b1.rs:36-80,118-162,217-254,332-411 confirm the sandbox stub tests and call-log assertions described in PR_BODY.md:16.
3.10	SURVIVED	PR_BODY.md:17 container test/fmt run and pass count cannot be verified from the 36 changed files in ./tree without Cargo.toml or commit 781a21b1.
3.11	SURVIVED	.github/workflows/ci.yml:83,129-133,214-215,441 "heavy_on: train", "run: \"cargo test --locked --lib resources::cloud_instance::\"", "ok \"examples-validate\"" confirms PR_BODY.md:18.
3.12	SURVIVED	src/resources/cloud_instance/mod.rs:197-224,409,425, src/core/parser/resource_types.rs:48-51, and docs/roadmaps/roadmap.yaml:7506-7510 ("status: planned") confirm PR_BODY.md:29-34.
3.13	SURVIVED	src/resources/cloud_instance/mod.rs:258-266,366,433 "for i in ${var}; do h=\"${{i#i-}}\";", "set -- $ids", "set -- $keyed" run without "set -f", allowing glob expansion of "i-*" against cwd files while leaving raw "i-*" in ${var}.
3.14	SURVIVED	src/resources/cloud_instance/mod.rs:260-265,313,336,364-367,432-436 "for i in ${var}; do" executes 0 iterations on whitespace-only (" "/"\\t") while "[ -n \"$ids\" ]" is true, reporting present for zero instances and calling terminate-instances with zero IDs.
3.15	SURVIVED	src/resources/cloud_instance/mod.rs:365-371 "sirs=\"$({sirs})\"", "for r in $sirs; do case \"$r\" in sir-[0-9a-z]*) ;; *) continue;; esac;", "left=\"$({ids})\"" confirm unquoted $sirs without set -f, weak sir- check silently skipping garbage, and left bypassing lookup_ids.
3.16	SURVIVED	src/core/parser/resource_types.rs:47-60 and src/core/resolver/resource.rs:92,123-124,222-242 confirm templated tags skip all spec_of validation yet r.tags is never resolved, any {{ in those fields masks unrelated broken fields, and provider/state are resolved but missing from the skip list.
3.17	SURVIVED	src/resources/cloud_instance/mod.rs:140-143 "/// Values still holding a `{{` template are skipped (checked once resolved)." is contradicted by src/core/parser/mod.rs:198-203 and ANGLE.diff showing no post-resolution call to validate_unique_names.
3.18	SURVIVED	src/core/observe/classify_e01.rs:14-17,78-89,165-168 vs src/resources/cloud_instance/mod.rs:242-246,290-295,329-334,425-429 confirm stale digest comment, self-contradictory cloud_region classification, and controller-side apply gates marked Unmigrated instead of Unobservable.
3.19	SURVIVED	src/resources/cloud_instance/mod.rs:196-224,425 "- destroy: after {life} min (self-terminate) or when the kill switch exists" falsely claims kill switch destroys running instances (line 425 only refuses launch) and plan_diff ignores state: absent.
3.20	SURVIVED	scripts/cb200-ratchet.sh:21-30,85-95, ANGLE.diff:46-107 (43 diff lines), and PR_BODY.md:1-40 confirm the python3-to-bash rewrite of scripts/cb200-ratchet.sh is omitted from PR_BODY.md.
3.21	SURVIVED	scripts/cb200-ratchet.sh:26-29,85-95 and ANGLE.diff:77-81 confirm the ceiling regex matches 12 in "{\"ceiling\": 12.5}" or "12abc", grep -oE accepts truncated JSON, and "grep -m 1 'below minimum grade'" diverges from the replaced Python walker when an earlier matching message lacks an N function count.
3.22	SURVIVED	src/core/types/resource.rs:470-473 "pub budget: DiskBudgetSpec,", src/core/types/disk_budget_types.rs:75-97, ANGLE.diff (9 files modifying budget fields), and PR_BODY.md:1-40 confirm five pub fields were moved out of Resource into DiskBudgetSpec with zero mention in PR_BODY.md.
VERDICT: FAIL
text_only: yes
refuter: 2
reviewer_model: gemini-4-neon-high

=== REFUTER 3 ===
[
  {
    "claim": "1.1",
    "verdict": "SURVIVED",
    "evidence": "src/core/parser/tests_known_fields_completeness.rs:69-74 `const ALIASES: &[&str] = &[ ... \"entrypoint\",`"
  },
  {
    "claim": "1.2",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:63,438-456 `resource.provider.as_deref() != Some(\"aws\")`, `--count 1`, `MarketType=spot,SpotOptions={{MaxPrice={},SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}}`, `--metadata-options 'HttpTokens=required,HttpPutResponseHopLimit=1,HttpEndpoint=enabled'`"
  },
  {
    "claim": "1.3",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:167-190,398-407,425-428 `worst_case_usd > cap`, `if [ -e {kill} ] || [ -L {kill} ]`, `[ -f {gate} ] && grep -qx 'parity-gate: pass' {gate} && [ \"$(grep -c '^parity-gate:' {gate})\" = 1 ]`"
  },
  {
    "claim": "1.4",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:230-232,408-412 `\"#!/bin/sh\\nshutdown -h +{}\\nexport FORJAR_IMAGE={}\\n\"` and src/resources/cloud_instance/tests_b1.rs:189 `log.contains(\"--user-data #!/bin/sh\\nshutdown -h +60\\nexport FORJAR_IMAGE='\")`"
  },
  {
    "claim": "1.5",
    "verdict": "REFUTED",
    "evidence": "src/resources/cloud_instance/mod.rs:258-266,301,313,336 `for i in ${var}; do` and `if [ -n \"$ids\" ]; then echo \"present $ids\"` contradict the TRUE verdict on PR_BODY line 9: whitespace-only output (`\" \"` or `\"\\t\"`) executes 0 loop iterations and exits 0 (reporting `present` / `instance-present` instead of exiting 1), and a glob reply matching local `i-<hex>` files passes without error"
  },
  {
    "claim": "1.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:345-374,384-386 `if is_absent(resource) { return destroy_script(&s); }`, `if [ -z \"$ids\" ]; then echo 'absent: no owned instance'; exit 0; fi`, `aws ec2 terminate-instances`, `aws ec2 cancel-spot-instance-requests`, `NOT destroyed: still pending or running: $left`"
  },
  {
    "claim": "1.7",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:168-171,425-426,445-446 `.filter(|p| p.starts_with('/') && !p.chars().any(char::is_control))`, `kill = sh_squote(&kill)`, `gate = sh_squote(&gate)` confirm `cloud_kill_switch` and `cloud_parity_gate` reach shell without a character allow-list"
  },
  {
    "claim": "1.8",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:70,109-112,143-164 `let name = field(&resource.name, \"name\")?;`, `!t.starts_with(\"forjar-key=\") && !t.starts_with(\"forjar-owner=\")`, `pub fn validate_unique_names` and src/core/parser/mod.rs:199-203"
  },
  {
    "claim": "1.9",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:128-138,329-338,430-444 `format!(\"{}@{}@{}\", s.name, s.region, &digest[..12])`, `format!(\"{}@{}\", s.name, s.region)`, `REFUSED: image changed while instance$other is alive` and src/resources/cloud_instance/validate.rs:19-70 (no allow-list permits `@`)"
  },
  {
    "claim": "1.10",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests.rs:50-72,202-241 and src/resources/cloud_instance/tests_b1.rs:36-93,118-162,217-254,332-411 (`const STUB`, 7 hostile values across 7 fields, independent two-box teardown, `readers()`)"
  },
  {
    "claim": "1.11",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests.rs:141-311, tests_b1.rs:20-411, tests_b4.rs:20-207 (42 `#[test]` functions across 36 changed files in `./tree`; no Cargo.toml, git history, or rest of workspace to verify the 357-test run or commit `781a21b1`)"
  },
  {
    "claim": "1.12",
    "verdict": "SURVIVED",
    "evidence": ".github/workflows/ci.yml:83,129-133,214-215,380,441 `heavy_on: train`, `if: needs.classify.outputs.code == 'true'`, `- name: cloud_instance unit falsifiers (FJ-861)`, `run: \"cargo test --locked --lib resources::cloud_instance::\"`, `ok \"examples-validate\"`"
  },
  {
    "claim": "1.13",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:21,25 prior commit SHAs and clippy counts cannot be verified from `./tree`, while each listed code fix is present in src/resources/cloud_instance/mod.rs:130,137,170,258-266,430-436 and validate.rs:43-52"
  },
  {
    "claim": "1.14",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:143-164,220-222,398-407,409,425,452 (`status(&c.parity_gate)`, `resolve:ssm:/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64`), src/core/parser/resource_types.rs:48-51, and docs/roadmaps/roadmap.yaml:7506-7510 `status: planned`"
  },
  {
    "claim": "1.15",
    "verdict": "SURVIVED",
    "evidence": "src/core/parser/resource_types.rs:48-55 `|| resource.tags.iter().any(|t| t.contains(\"{{\"))`, src/core/resolver/resource.rs:222-242 (never resolves `r.tags`, only `r.tag` at line 92), and src/resources/cloud_instance/validate.rs:62-70 (`is_tag` rejects `{` and `}`)"
  },
  {
    "claim": "1.16",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:258-266,301,313,336,364-367,432-436 `for i in ${var}; do` without `set -f`: whitespace-only output runs 0 iterations yet passes `[ -n \"$ids\" ]`, and unquoted globs like `i-*` expand against cwd filenames"
  },
  {
    "claim": "1.17",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:430-438 `{lookup_owned}{lookup_keyed}if [ -z \"$owned\" ]; then :; else ... fi\\n aws ec2 run-instances` unconditionally runs `lookup_keyed` and discards `$keyed` whenever `$owned` is empty"
  },
  {
    "claim": "1.18",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:142-143 `/// Values still holding a '{{' template are skipped (checked once resolved).` vs src/core/parser/mod.rs:199-203 (the sole non-test call site of new fn `validate_unique_names`, invoked only inside pre-resolution `validate_config`)"
  },
  {
    "claim": "1.19",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:362-372 `set -eu`, `sirs=\"$({sirs})\"` before `aws ec2 terminate-instances`, and `left=\"$({ids})\"` bypassing `lookup_ids`"
  },
  {
    "claim": "1.20",
    "verdict": "SURVIVED",
    "evidence": "src/core/observe/classify_e01.rs:78-89 `check and state query ask the provider for instances carrying a key derived from name + region + image digest only` vs src/resources/cloud_instance/mod.rs:242-246,329-338 (`state_query_script` queries `forjar-owner` = `name@region` with `--region`)"
  },
  {
    "claim": "2.1",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:63-65,438-456 `resource.provider.as_deref() != Some(\"aws\")`, `--count 1`, `MarketType=spot,SpotOptions={{MaxPrice={},SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}}`, `HttpTokens=required,HttpPutResponseHopLimit=1,HttpEndpoint=enabled`"
  },
  {
    "claim": "2.2",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:387-407,425-428 `gates_of(resource)`, `worst_case_usd > cap`, `if [ -e {kill} ] || [ -L {kill} ]`, `grep -qx 'parity-gate: pass' {gate} && [ \"$(grep -c '^parity-gate:' {gate})\" = 1 ]`"
  },
  {
    "claim": "2.3",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:258-266,301,313,336,432-437 `for i in ${var}; do` runs 0 iterations on `\" \"`/`\"\\t\"` and expands globs without `set -f`, falsifying PR_BODY.md:9"
  },
  {
    "claim": "2.4",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:345-374,384-386 `if is_absent(resource) { return destroy_script(&s); }`, `\"forjar-owner\"`, `absent: no owned instance`, `terminate-instances`, `cancel-spot-instance-requests`, `NOT destroyed: still pending or running: $left`"
  },
  {
    "claim": "2.5",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:168-171,425-426,445-446 `.filter(|p| p.starts_with('/') && !p.chars().any(char::is_control))` shows `cloud_kill_switch` and `cloud_parity_gate` are not allow-listed despite reaching shell"
  },
  {
    "claim": "2.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:128-138,329-334,430-437 and src/resources/cloud_instance/validate.rs:19-30 (`name@region@digest12`, `name@region`, `@` excluded from allow-lists, owner-first lookup in apply and state_query)"
  },
  {
    "claim": "2.7",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:230-232,408-412,457 `squote_keep_newlines(&user_data)` and src/resources/cloud_instance/tests_b1.rs:189 `log.contains(\"--user-data #!/bin/sh\\nshutdown -h +60\\nexport FORJAR_IMAGE='\")`"
  },
  {
    "claim": "2.8",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests.rs:50-72,202-241 and src/resources/cloud_instance/tests_b1.rs:36-79,113-162,217-254,332-400"
  },
  {
    "claim": "2.9",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:17,25 container test/fmt/clippy runs and commit `781a21b1` cannot be verified from the 36 changed files in `./tree`"
  },
  {
    "claim": "2.10",
    "verdict": "SURVIVED",
    "evidence": ".github/workflows/ci.yml:83,129-131,214-215,380,441 `heavy_on: train`, `run: \"cargo test --locked --lib resources::cloud_instance::\"`, `ok \"examples-validate\"`"
  },
  {
    "claim": "2.11",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:143-164,220-222,398-407,425,438-444, src/core/parser/resource_types.rs:48-51, docs/roadmaps/roadmap.yaml:7506-7510 `status: planned`"
  },
  {
    "claim": "2.12",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:260-265,301,313,336,364-367,432-437 `for i in ${var}; do` skips empty word-split of `\" \"`/`\"\\t\"` while `[ -n \"$ids\" ]` is true and `set -- $ids` yields 0 args"
  },
  {
    "claim": "2.13",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:261,362,366,368,424,433-434 uses `set -eu` without `set -f`, allowing pathname expansion on unquoted `${var}`, `$ids`, `$sirs`, `$keyed`, `$owned`"
  },
  {
    "claim": "2.14",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests_b4.rs:65-68 `!out.contains(\"alive i-00a1b2c3\")` vs src/resources/cloud_instance/mod.rs:435 `REFUSED: image changed while instance$other is alive;` (`is alive` always follows `$other`, so `alive <id>` never occurs)"
  },
  {
    "claim": "2.15",
    "verdict": "SURVIVED",
    "evidence": "src/core/parser/resource_types.rs:48-55 `|| resource.tags.iter().any(|t| t.contains(\"{{\"))` vs src/core/resolver/resource.rs:222-242 (`r.tags` is never resolved) and src/resources/cloud_instance/validate.rs:62-70"
  },
  {
    "claim": "2.16",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:142-155 `/// Values still holding a '{{' template are skipped (checked once resolved).` vs src/core/parser/mod.rs:199-203 (only non-test caller is `validate_config`, never called after template resolution)"
  },
  {
    "claim": "2.17",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:67,197-224,384-386 `plan_diff` never checks `is_absent(resource)` and unconditionally prints `+ cloud_instance` and `gates (apply refuses unless all declared):`; no test calls `plan_diff` with `state: absent`"
  },
  {
    "claim": "2.18",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:430-438 `{lookup_owned}{lookup_keyed}if [ -z \"$owned\" ]; then :; else ... fi\\n aws ec2 run-instances`"
  },
  {
    "claim": "2.19",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:308-318 `lookup = lookup_ids(\"ids\", &describe(&s))` (queries only `forjar-key`) vs mod.rs:430-437 (`apply_script` checks `$owned` against `$keyed` and refuses on mixed-key owned boxes as in tests_b4.rs:56-70)"
  },
  {
    "claim": "2.20",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:365,368-371 `sirs=\"$({sirs})\"`, `case \"$r\" in sir-[0-9a-z]*)`, `left=\"$({ids})\"`, `NOT destroyed: still pending or running: $left`; tests.rs:50-72 has no failure mode for `sirs`, `cancel-spot-instance-requests`, or `left` re-read"
  },
  {
    "claim": "2.21",
    "verdict": "SURVIVED",
    "evidence": "scripts/cb200-ratchet.sh:85-95 `msg=$(grep -m 1 'below minimum grade' <<<\"$msgs\" || true)` vs ANGLE.diff:77-81 (`if \"below minimum grade\" in m: n = re.search(r\"(\\d+) function\", m); if n: out.append(...)`)"
  },
  {
    "claim": "2.22",
    "verdict": "SURVIVED",
    "evidence": "src/core/observe/classify_e01.rs:78-89 `\"cloud_region\" | ... => Observability::Unmigrated(\"#861 — the provider query keys on name, region and image digest; this field is applied by the create call and the gates but never read back\")` vs src/resources/cloud_instance/mod.rs:329-334 (`forjar-owner`)"
  },
  {
    "claim": "2.23",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests.rs:52 `echo \"$@\" >> \"$STUB_LOG\"` flattens args, tests_b4.rs:165 `term.ends_with(\"--instance-ids i-00a1b2c3 i-00b2c3d4\")`, and tests*.rs never test relative gate/kill paths, non-overflowing invalid budget strings in `apply_script`, `worst_case_usd == cap`, or second-call `lookup_keyed` failure"
  },
  {
    "claim": "3.1",
    "verdict": "SURVIVED",
    "evidence": "src/core/parser/tests_known_fields_completeness.rs:69-74 `\"entrypoint\",` retained in `ALIASES`"
  },
  {
    "claim": "3.2",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:63-65,438-456 `--count 1`, `MarketType=spot,SpotOptions={{MaxPrice={},SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}}`, `--metadata-options 'HttpTokens=required,HttpPutResponseHopLimit=1,HttpEndpoint=enabled'`"
  },
  {
    "claim": "3.3",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:387-407,425-428 `if [ -e {kill} ] || [ -L {kill} ]`, `grep -qx 'parity-gate: pass' {gate} && [ \"$(grep -c '^parity-gate:' {gate})\" = 1 ]`, `worst_case_usd > cap`"
  },
  {
    "claim": "3.4",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:258-266,313,336 `for i in ${var}; do` passes whitespace-only and matching-glob replies without error, falsifying PR_BODY.md:9"
  },
  {
    "claim": "3.5",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:345-374,384-386 `if is_absent(resource) { return destroy_script(&s); }`, `absent: no owned instance`, `NOT destroyed: still pending or running: $left`"
  },
  {
    "claim": "3.6",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:168-171,425-426,445-446 `.filter(|p| p.starts_with('/') && !p.chars().any(char::is_control))` confirms `cloud_kill_switch` and `cloud_parity_gate` are not allow-listed"
  },
  {
    "claim": "3.7",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:128-138,329-338,430-437 and src/resources/cloud_instance/validate.rs:19-30"
  },
  {
    "claim": "3.8",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:230-232,408-412,457 and src/resources/cloud_instance/tests_b1.rs:188-192"
  },
  {
    "claim": "3.9",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/tests.rs:51-125,202-241 and src/resources/cloud_instance/tests_b1.rs:36-80,118-162,217-254,332-411"
  },
  {
    "claim": "3.10",
    "verdict": "SURVIVED",
    "evidence": "PR_BODY.md:17 test/fmt run on `781a21b1` cannot be verified from the 36 changed files in `./tree`"
  },
  {
    "claim": "3.11",
    "verdict": "SURVIVED",
    "evidence": ".github/workflows/ci.yml:83,129-133,214-215,441 `heavy_on: train`, `run: \"cargo test --locked --lib resources::cloud_instance::\"`, `ok \"examples-validate\"`"
  },
  {
    "claim": "3.12",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:197-224,409,425, src/core/parser/resource_types.rs:48-51, docs/roadmaps/roadmap.yaml:7506-7510 `status: planned`"
  },
  {
    "claim": "3.13",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:258-266,366,433 `for i in ${var}; do`, `set -- $ids`, `set -- $keyed` run without `set -f`, allowing cwd glob expansion"
  },
  {
    "claim": "3.14",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:260-265,313,336,364-367,432-436 whitespace-only `$()` output skips `for i in ${var}` yet passes `[ -n \"$ids\" ]` and fails `[ -z \"$ids\" ]`"
  },
  {
    "claim": "3.15",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:365-371 `sirs=\"$({sirs})\"`, `for r in $sirs; do case \"$r\" in sir-[0-9a-z]*) ;; *) continue;; esac;`, `left=\"$({ids})\"`"
  },
  {
    "claim": "3.16",
    "verdict": "SURVIVED",
    "evidence": "src/core/parser/resource_types.rs:47-60 (`resource.tags.iter().any(|t| t.contains(\"{{\"))`, skips all `spec_of` errors, omits `provider`/`state`) and src/core/resolver/resource.rs:92,123-124,222-242 (resolves `r.state`/`r.provider`/`r.tag` but never `r.tags`)"
  },
  {
    "claim": "3.17",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:140-143 `/// Values still holding a '{{' template are skipped (checked once resolved).` vs src/core/parser/mod.rs:198-203 (only wired into pre-resolution `validate_config`)"
  },
  {
    "claim": "3.18",
    "verdict": "SURVIVED",
    "evidence": "src/core/observe/classify_e01.rs:14-17,78-89,165-167 vs src/resources/cloud_instance/mod.rs:242-246,290-295,329-334 (`state_query_script` and absent `check_script` use `forjar-owner`, `cloud_region` is used in queries, and local apply gates are marked `Unmigrated` unlike `quality_gate` `Unobservable`)"
  },
  {
    "claim": "3.19",
    "verdict": "SURVIVED",
    "evidence": "src/resources/cloud_instance/mod.rs:197-224,425 `- destroy: after {life} min (self-terminate) or when the kill switch exists` (kill switch only refuses `apply_script`, never destroys; `plan_diff` ignores `resource.state`)"
  },
  {
    "claim": "3.20",
    "verdict": "SURVIVED",
    "evidence": "ANGLE.diff:46-106 (43 changed lines in `scripts/cb200-ratchet.sh` removing `python3`) and PR_BODY.md:1-40 (zero mention of `cb200-ratchet.sh` or `python3`)"
  },
  {
    "claim": "3.21",
    "verdict": "SURVIVED",
    "evidence": "scripts/cb200-ratchet.sh:26-29,85-95 `if [[ $(<\"$BASE\") =~ \\\"ceiling\\\"[[:space:]]*:[[:space:]]*([0-9]+) ]]` matches `12.5`/`12abc` as `12`, `grep -oE` accepts truncated JSON, and `grep -m 1 'below minimum grade'` fails to match Python walker rule in ANGLE.diff:77-81"
  },
  {
    "claim": "3.22",
    "verdict": "SURVIVED",
    "evidence": "ANGLE.diff:223-244,405-421,497-532,557-600,2151-2408 removes 5 `pub` `budget_*` fields from `pub struct Resource` into `pub budget: DiskBudgetSpec` across 9 files with zero mention in PR_BODY.md:1-40"
  }
]
VERDICT: FAIL
text_only: yes
refuter: 3
reviewer_model: gemini-4-neon-high

=== refuter 2 first attempts (invalid JSON, not used) ===
see AGY-refuter-2.out and AGY-refuter-2.try1.out in round dir

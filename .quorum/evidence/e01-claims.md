# Quorum evidence — #403 (CRUX audit E01) — adjudicated claims

## CONFIRMED — 8 claims survived refutation

1. [probe] (explains-symptom) The old hasher was an allowlist of 35 fields, so a changed `uid`, `tag`, `checksum`, `driver_version`, `timeout`, `working_dir`, `sudo` or `ssh_authorized_keys` produced an IDENTICAL desired-state hash.
   - evidence: `collect_core_fields` at src/core/planner/hashing.rs:26 and `collect_phase2_fields` at src/core/planner/hashing.rs:76 pushed 34 named fields and nothing else; `hash_desired_state` at src/core/planner/hashing.rs:206 joined only those. Reproduced through the real YAML parser in tests/falsification_e01_hash_the_whole_resource.rs — with main's `hashing.rs` restored, 8 of 10 cases went red, one per field.

2. [probe] (explains-symptom) A hash collision is reported to the operator as `unchanged`, not as an error, so the defect was invisible in green.
   - evidence: `determine_absent_action` and `determine_present_action` at src/core/planner/mod.rs:301 return `NoOp` iff `rl.hash == hash_desired_state(resource)`; the executor then prints `unchanged`. The ticket's own measurement — two six-resource configs differing in eleven fields, byte-identical `state.lock.yaml` — is exactly this path. No test on main could see it because no test on main hashed a field outside the allowlist.

3. [design] The denylist polarity is the right one: it fails loudly, and the reflection guard makes the loudness mechanical.
   - evidence: `NON_IDENTITY_FIELDS` in the new src/core/planner/hashing.rs names eleven fields and nothing else is excluded; `tests_hash_completeness::every_identity_field_moves_the_desired_state_hash` walks `serde_yaml_ng::to_value(Resource)` and asserts every key not on the denylist moves the hash. A field added to `Resource` later is hashed until someone argues it onto the list — the opposite failure mode from the allowlist, which was patched piecemeal five times (FJ-127, FJ-035, GH-206, #390, FJ-036) at src/core/planner/hashing.rs:76 without ever becoming general.

4. [design] The generation bump making every `state: absent` resource replan as `Destroy` once is safe, because every destroy script this repo generates is guarded on existence.
   - evidence: Refuted twice and survived both. The panel's concern was `userdel` on an already-absent user failing the apply: the user destroy script at src/resources/user.rs:30 is wrapped in `if id '{username}' >/dev/null 2>&1; then`, mount is `if mountpoint -q`, docker is `|| true`, file is `rm -rf`. The agy lane's concern was non-idempotent user-defined cleanup on `task`/`recipe`: `codegen::dispatch` short-circuits `state: absent` for Task, Build and WasmBundle to an echo ("no absent form"), and recipes are expanded before planning. src/core/planner/mod.rs:288 already documents the fixed-point argument GH-339 relied on.

5. [design] `machine`, `tags`, `arch`, `resource_group`, `when`, `count`, `for_each`, `depends_on`, `lifecycle`, `triggers` and `phony` are all correctly excluded — each decides whether or where a run touches a resource, never what it converges to.
   - evidence: Each entry is argued individually in the `NON_IDENTITY_FIELDS` block of src/core/planner/hashing.rs. The two the agy lane contested both read from CONFIG at run time, never from the lock: `triggers` is consulted by `classify_resource` in src/core/executor/machine_b.rs:286, which asks whether any named resource converged THIS run, so a changed list takes effect on the next apply with no re-hash; `lifecycle.ignore_drift` is read by src/core/parser/validation.rs:145 and the drift census, and removing the config removes the resource. `selection_filters_do_not_move_the_hash` pins the polarity and is green on both trees by design.

6. [probe] Folding in the missing fields exposed 45 fields the observability registry had never classified, and each now carries a decision rather than a shrug.
   - evidence: `classify` at src/core/observe/mod.rs:145 returned `None` for every field outside its match; the existing `every_hashed_field_is_classified` gate went red with 45 names the moment the hash grew. `classify_e01.rs` gives each an `Observed`, `Unobservable` or `Unmigrated` arm with the state query that would close it — `rocm_version` and `cuda_version` are recorded as near-misses (the query reads the kernel-module and driver versions, not the stack versions).

7. [agy] (partially-explains) The `*_does_not_move_the_hash` guards could have passed vacuously, because an unknown YAML key is a warning in this parser rather than an error.
   - evidence: `validate_unknown_fields` at src/core/parser/mod.rs:277 returns warnings, so a field renamed away from `Resource` would have made both declarations parse to identical defaults and the equality guard would hold having checked nothing. Taken: each guard in tests/falsification_e01_hash_the_whole_resource.rs now asserts the two parsed forms DIFFER before asserting the hashes match, and the insertion-order guard proves its map parsed with two entries.

8. [agy] (partially-explains) The flattened `BackupSpec` and `ArchiveSpec` keys needed their own pin, even though the hasher already saw them.
   - evidence: `backup` and `archive` are `#[serde(flatten)]` on `Resource` (src/core/types/resource.rs:471 and :475), so `serde_yaml_ng::to_value` puts `backup_schedule` and `archive_destination` at top level and the canonical form hashes them — but nothing asserted that through the flatten. `flattened_backup_schedule_moves_the_hash` and `flattened_archive_destination_moves_the_hash` now do, through the YAML parser.

## REFUTED — 5 claims killed

1. [design] refuted 1/1 — The canonical encoding is injective as written.
   - corrected: It was not. Strings and numbers were length-prefixed but tags were rendered bare, so tag `a` over tagged(`b`, `~`) and tag `a!!b` over `~` both produced `!!a!!b~`. A tag only reaches the hasher through `inputs:`, so no user has hit it, but the one function whose contract is "different declaration, different bytes" cannot carry a collision. Fixed by length-prefixing the tag in src/core/planner/hashing.rs and pinned by `tagged_input_values_do_not_collide`, which goes red against the unprefixed form.

2. [probe] refuted 1/1 — The first injectivity counterexample (`a` / `a!b`) collides.
   - corrected: It does not — the first version of the collision test stayed GREEN against the reverted encoding. `Tag`'s Display carries its own leading `!`, so the nested form rendered `!!a!!b~` and the flat `a!b` rendered `!!a!b~`, one byte apart. The agy lane offered the identical wrong pair independently. The counterexample was corrected to `a!!b` and the test re-run both ways before the fix commit was kept; the wrong first attempt is recorded here rather than erased.

3. [design] refuted 1/1 — `forjar reseal` is the migration path after the hash-identity bump.
   - corrected: `reseal` recomputes the `.b3` sidecar over the lock file's BYTES; it never touches the per-resource `hash` field inside it, so running it after the bump changes nothing about what `plan` sees. The ticket's "gate behind a schema bump and `forjar reseal`" was half right. The migration is `forjar plan` then one `forjar apply` — every resource replans as `Update` and re-records its own hash — and src/cli/reseal.rs now says so at the top so nobody reaches for it.

4. [agy] refuted 1/1 — `backup_sync` and `nas_archive` generate shell scripts that `canonical_generated_script` does not hash, so their machines will run stale scripts forever.
   - corrected: Every declared `backup_*` and `archive_*` key is hashed directly through the flatten (pinned by the two `flattened_*` cases), so a changed declaration re-converges. `canonical_generated_script` at src/core/planner/hashing.rs:231 exists for a different reason — a forjar UPGRADE changing codegen for an unchanged `disk_budget` declaration — and that concern applies to every resource type, predates this branch, and is recorded under known_limits rather than folded into a hash-identity fix.

5. [agy] refuted 1/1 — `source_unreadable:{src}:{e}` collides whenever `src` contains a colon.
   - corrected: `source` is itself part of the canonical declaration (component 2 of `hash_desired_state`, src/core/planner/hashing.rs:206 on main and the same shape now), so two resources with different `source` values already differ before the unreadable-marker component is appended. The marker cannot produce a whole-hash collision on its own.

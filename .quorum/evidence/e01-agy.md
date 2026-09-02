# Independent review — agy /teamwork (plan mode) — #403

Ran against the branch after its first two quorum commits. Five refutations
and two unique findings came back; each was checked against the code rather
than accepted or dismissed on its wording.

TAKEN (changed the branch):

> "The `assert_holds` tests use `assert_eq!`, meaning if `tags` or
> `overlay_hosts` were accidentally deleted from `Resource`, the YAML parser
> would silently drop them, yielding identical default structs that pass the
> equality check vacuously."

Correct — unknown keys are a WARNING in `parse_config`, not an error. Each
guard now asserts the two parsed forms differ before asserting the hashes
match; the insertion-order guard proves its map parsed with two entries.

> "`BackupSpec` and `ArchiveSpec` use `#[serde(flatten)]` … reflection-based
> field probing might skip testing their inner keys entirely."

The hasher was already right (flatten puts the keys at top level of the
serialised form), but nothing PINNED it. Two cases now do.

> "`Value::Tagged` … not delimited, causing `Tagged("A", Tagged("B", Null))`
> and `Tagged("A!B", Null)` to collide."

Independently found the same class of collision the panel had; the tag is now
length-prefixed. Note the pair as written does not collide — `Tag`'s Display
carries its own `!`, so the colliding partner is `A!!B` — the same slip the
panel's first counterexample made.

REFUTED (did not survive the code):

> "`triggers` is an identity field … modifying a resource's trigger list yields
> `unchanged` … and silently breaks the trigger feature."

The trigger check reads `resource.triggers` from the CONFIG at apply time
(`classify_resource`, executor/machine_b.rs) and asks whether any named
resource converged THIS run. The lock never stores triggers, so a changed
list takes effect on the very next apply with no re-hash needed. Excluding it
is correct: it decides WHEN a resource re-runs, not what it converges to.

> "Re-running `Destroy` … for `Task` or `Recipe` resources … will blindly
> trigger potentially non-idempotent user-defined cleanup scripts again."

`codegen::dispatch` short-circuits `state: absent` for Task, Build and
WasmBundle to an `echo` — "resources have no absent form — nothing to remove".
Recipes are expanded before planning and never reach the hasher. Every other
destroy script this repo generates is guarded on existence (`if id`,
`if mountpoint -q`, `rm -f`, `|| true`).

> "`BackupSync` and `NasArchive` also generate shell scripts, but
> `canonical_generated_script` restricts script hashing to `DiskBudget`."

Every declared `backup_*`/`archive_*` key is now hashed directly, so a changed
declaration re-converges. What this reviewer is reaching for is a different
limit — a forjar UPGRADE that changes codegen for an unchanged declaration —
which applies to every type, predates this branch, and is recorded as a known
limit rather than folded into a hash-identity fix.

> "`source_unreadable:{src}:{e}` collides if `src` contains colons."

`source` is itself part of the canonical declaration (component 2), so two
resources with different `source` values already differ before component 3 is
appended. No whole-hash collision exists.

> "changes to `lifecycle.ignore_drift` do not move the hash … fail to suppress
> drift if the config is later removed."

Drift reads `lifecycle` from the config (`parser/validation.rs`,
`tripwire/drift/census.rs`), never from the lock. Removing the config removes
the resource. Nothing to suppress.

Its CRUX verdict — that a serialise-and-denylist hash is "well below the
industry default" next to Terraform's schema-aware state — is noted and
disagreed with on the evidence in the lanes file: Terraform's per-attribute
diff IS a denylist (`Computed`/`ignore_changes`) over the whole schema; the
allowlist was the thing below the default.

# CRUX Audit — forjar 1.26.0

## Method

Three independent [agy](https://github.com/paiml) panel lanes (`lane-1`, `lane-2`,
`lane-3`) were run against release 1.26.0's nine behaviour changes, each from
documentation memory only — no network access, no live invocation of any
reference system. Every claim about a third-party system is therefore marked
`[X]` (asserted from training/documentation memory, not measured against a
running instance). Every claim about forjar is marked `[V]` and carries a
`file:line` citation resolved against the head of branch
`PMAT-161-state-stamp-per-name` (the branch that introduced behaviours 6-9;
behaviours 1-5 landed earlier on the same lineage under PMAT-160). A citation
that could not be pinned to a specific line cites the nearest named function
instead of a bare filename.

The nine behaviours audited are the ones `CHANGELOG.md`'s `[Unreleased]`
section documents as changes in observable behaviour (see "Reconciliation"
below) — not every change in the release, only the ones a caller of `apply`,
`check`, `undo` or `status` can see from the outside.

## Findings

| Behaviour | System | How they do it [X] | forjar today [V] | Delta | Disposition |
|-----------|--------|---------------------|-------------------|-------|--------------|
| (1) selectors select the `depends_on` closure | Terraform `-target` | Selects the named resource and its dependencies — a downward closure [X] | Resolves the positive selectors, then closes them downward over `depends_on` (`resolver::goal_closure`) before anything is pruned [V `src/cli/apply_selection/closure.rs:99`] | Matches Terraform's targeting semantics | reject(forjar's downward closure over `depends_on` guarantees a targeted apply never runs against an unconverged prerequisite, matching Terraform `-target`) |
| (1) selectors select the `depends_on` closure | Ansible `--tags` | Filters tasks by tag alone; no automatic inclusion of a tagged task's dependencies [X] | Same closure as above — every positive selector, not only `make` goals, closes downward [V `src/cli/apply_selection/closure.rs:87-99`] | forjar is stricter/safer than Ansible's linear tag filter | reject(closure over dependencies is safer than Ansible's tag-only filtering, which can select a task whose dependency never ran) |
| (1) selectors select the `depends_on` closure | Make | A goal builds the named target and every prerequisite, recursively [X] | Same closure [V `src/cli/apply_selection/closure.rs:99`] | Matches Make | reject(matches Make's own closure rule, which forjar's docstring explicitly cites as precedent) |
| (2) `apply --check` honours every selector, exit 0 when only unselected resources are red | Terraform `plan -target` | Restricts the plan to the target; exits clean even if drift exists elsewhere in state [X] | `cmd_apply_check` resolves the selection first, then checks only the selected config; exits 0 when only resources outside the selection are drifted [V `src/cli/dispatch_apply_check.rs:64-73`] | Exact alignment with Terraform | reject(scoping the check to the resolved selection before evaluating it is the same contract `plan -target` gives operators) |
| (2) `apply --check` honours every selector, exit 0 when only unselected resources are red | Ansible `--check --tags`/`--limit` | Only evaluates the tagged/limited scope in check mode [X] | Same resolve-then-check ordering [V `src/cli/dispatch_apply_check.rs:73`] | Exact alignment with Ansible | reject(matches Ansible's check-mode scoping) |
| (2) `apply --check` honours every selector, exit 0 when only unselected resources are red | Kubernetes `diff`/`apply --dry-run` | Diffs only the applied manifest; drift elsewhere in the cluster is invisible to that run [X] | Same [V `src/cli/dispatch_apply_check.rs:73`] | Exact alignment with Kubernetes | reject(matches the industry-standard shape of a scoped dry-run) |
| (3) standalone `check` follows the closure and refuses an unknown selector | Terraform `plan -target=typo` | Errors immediately: "Resource target not found" [X] | `check_existence` runs before the closure and refuses any selector matching nothing, with the list of what IS known [V `src/cli/apply_selection/closure.rs:169-178`] | Matches Terraform's up-front refusal | reject(refusing an unknown selector before doing any work prevents a typo from silently reporting "0 pass, 0 fail" at exit 0) |
| (3) standalone `check` follows the closure and refuses an unknown selector | Make | `make unknown-target` fails with "No rule to make target" [X] | Same up-front check [V `src/cli/apply_selection/closure.rs:169`] | Matches Make | reject(same fail-fast contract as Make) |
| (3) standalone `check` follows the closure and refuses an unknown selector | Ansible `--check --tags unknown` | Frequently exits 0, having matched and therefore run nothing [X] | Refuses instead of silently converging nothing [V `src/cli/apply_selection/closure.rs:169`] | forjar is stricter than Ansible here | reject(forjar's refusal is strictly safer than Ansible's silent no-op) |
| (4) a resource negative that empties the selection is refused | Terraform `-exclude` | Excluding everything selected can report "No changes" at exit 0 [X] | An `--exclude`/`--skip` that removes every selected resource is refused with `no resources remain: ...` rather than converging nothing [V `src/cli/apply_selection/closure.rs:120-122,144-158`] | forjar is stricter than Terraform | reject(refusing an emptied selection catches an operator's typo instead of reporting false success) |
| (4) a resource negative that empties the selection is refused | Ansible `--skip-tags`/`--limit` | Matching nothing typically exits 0 having done nothing, sometimes with a warning [X] | Same refusal [V `src/cli/apply_selection/closure.rs:147`] | forjar is stricter than Ansible | reject(same rationale — false success is worse than a loud refusal) |
| (4) a resource negative that empties the selection is refused | SaltStack targeting | An empty target match returns an error [X] | Same refusal [V `src/cli/apply_selection/closure.rs:120`] | Matches SaltStack | reject(matches SaltStack's own fail-closed targeting) |
| (5) make's phony stripping contracts edges | Make | Executes phony targets as ordinary goals; has no notion of "stripping" one and reattaching its neighbours [X] | `contract_edges` rewrites `a -> phony -> c` to `a -> c` when `phony` is dropped from the selection, instead of deleting the edge or leaving it dangling [V `src/cli/apply_selection/narrow.rs:148-176`] | forjar improves on the naive "just drop the node" behaviour no listed system offers | reject(contracting the edge is the only option that both honours the operator's exclusion and keeps the graph the rest of the apply relies on) |
| (5) make's phony stripping contracts edges | Terraform | Removing a resource from scope has no edge-contraction step; a dependent's reference to it is a state-level concern, not a graph rewrite [X] | Same contraction [V `src/cli/apply_selection/narrow.rs:148`] | forjar's model is closer to a build system's than Terraform's | reject(no direct Terraform equivalent; the contraction fixes a class of bug -target-only tools don't have) |
| (5) make's phony stripping contracts edges | Ansible | `block`/`include` grouping lets task ordering "pass through" a skipped group, which is analogous but not identical [X] | Same contraction [V `src/cli/apply_selection/narrow.rs:148`] | Loosely analogous to Ansible's grouping semantics | reject(closest analogue is grouping, not a first-class dependency graph) |
| (6) per-name state stamp, warning only on same-name-other-file or a machine owned by another stack | Pulumi | Each stack owns its own isolated state file; no shared-file collision is possible by construction [X] | `stacks: {name -> StackStamp}` keyed by config name in one shared lock; `stack_conflict` warns only on a same-name-different-file re-apply or a machine another stack's stamp claims [V `src/core/state/stamp.rs:318-333`, `src/core/state/stamp.rs:354-393`] | forjar achieves similar isolation inside one shared file rather than by separate files | reject(keying by name inside a shared lock gives Pulumi-equivalent isolation without requiring one state dir per stack, which paiml/infra's six-manifest layout needs) |
| (6) per-name state stamp, warning only on same-name-other-file or a machine owned by another stack | Terraform workspaces | Each workspace gets its own state file inside the backend [X] | Same per-name stamp inside one shared lock [V `src/core/state/stamp.rs:354`] | forjar shares the file but isolates the section | reject(a shared file with per-name sections is a deliberate design choice for a shared `state/` directory, not a regression from workspace isolation) |
| (6) per-name state stamp, warning only on same-name-other-file or a machine owned by another stack | Nix profile generations | Distinct named profiles live side by side in one profile directory, isolated by name [X] | Same [V `src/core/state/stamp.rs:34-65`] | Matches Nix's per-name isolation inside a shared directory | reject(this is the behaviour forjar's fix was modelled on — closest match of the three) |
| (7) undo/rollback refused while a state dir holds >1 stack (stack-scoped restore = PMAT-162) | Nix profile generations | `nix-env --rollback` is scoped to one profile; other profiles in the same store are untouched [X] | `multi_stack_restore_refusal` refuses ANY restore outright once the lock's `stacks:` map has more than one entry, regardless of which name's generation was asked for [V `src/core/state/stamp.rs:140-167`, called from `src/cli/generation/restore.rs:30-44`] | forjar has no scoped restore yet; it refuses instead of reverting every stack | adopt(PMAT-162: give each generation an owning stack name so `undo`/`rollback` can replay only the invoking stack's subset of a shared snapshot, matching Nix's per-profile rollback) |
| (7) undo/rollback refused while a state dir holds >1 stack (stack-scoped restore = PMAT-162) | Pulumi per-stack state | A stack's update/rollback operations are scoped to that stack's own state; other stacks are never touched [X] | Same outright refusal [V `src/core/state/stamp.rs:150`] | forjar blocks a safe, isolated operation Pulumi permits by construction | adopt(PMAT-162: give each generation an owning stack name so `undo`/`rollback` can replay only the invoking stack's subset of a shared snapshot, matching Nix's per-profile rollback) |
| (7) undo/rollback refused while a state dir holds >1 stack (stack-scoped restore = PMAT-162) | Kubernetes rollout history | `kubectl rollout undo deployment/x` is scoped to that Deployment's own revision history; sibling Deployments in the namespace are untouched [X] | Same outright refusal [V `src/core/state/stamp.rs:150`] | forjar blocks a safe, isolated operation Kubernetes permits by construction | adopt(PMAT-162: give each generation an owning stack name so `undo`/`rollback` can replay only the invoking stack's subset of a shared snapshot, matching Nix's per-profile rollback) |
| (8) outputs merge per stack | Terraform workspaces | Outputs live in the workspace's own state file; cross-workspace reads go through `terraform_remote_state` [X] | `merge_outputs` withdraws only the keys the applying stack previously owned, then inserts its new keys, leaving every other stack's keys untouched in the shared `outputs` map [V `src/core/state/stamp.rs:395-425`] | forjar merges within one shared map instead of requiring remote-state plumbing | reject(merging per stack inside a shared map gives cross-stack `{{stack.*}}` references without the ceremony of `terraform_remote_state`) |
| (8) outputs merge per stack | Pulumi | Cross-stack references require an explicit `StackReference` to another stack's isolated outputs [X] | Same merge [V `src/core/state/stamp.rs:402`] | forjar's shared map is simpler but less isolated than Pulumi's explicit reference | reject(the simplification is intentional for a shared `state/` directory with implicit cross-stack references via templating) |
| (8) outputs merge per stack | Kubernetes | No global outputs map exists; Server-Side Apply merges FIELDS by owning manager, not a discrete outputs map [X] | Same merge [V `src/core/state/stamp.rs:402`] | Loosely analogous to SSA's per-manager field ownership | reject(closest analogue is field-manager merging, not a literal outputs map) |
| (9) status attributes machines per stack | Kubernetes | `ownerReferences`/managed fields attribute an object to the controller or field manager that wrote it [X] | `machine_owner` in `forjar status` reads which stack's stamp claims a machine and attributes it to that stack rather than to whichever config applied last [V `src/cli/status_core.rs:121-138`] | Matches Kubernetes' explicit-ownership model | reject(explicit per-machine attribution prevents a shared dir's status output from crediting the wrong stack, matching Kubernetes ownership) |
| (9) status attributes machines per stack | Pulumi | Stack resources are tracked per stack via URNs; ownership is unambiguous by construction [X] | Same attribution [V `src/cli/status_core.rs:129`] | forjar achieves the same clarity inside a shared lock instead of separate stack state | reject(same rationale — explicit attribution, no separate storage required) |
| (9) status attributes machines per stack | Terraform | Ownership of a resource is implicit in which state file it lives in, not an explicit field [X] | Same attribution, but explicit [V `src/cli/status_core.rs:129`] | forjar is more explicit than Terraform's implicit-by-file-location model | reject(explicit attribution is strictly more informative than Terraform's implicit-by-state-file model for a directory shared by six manifests) |

### Dissent

All three lanes agree behaviour (7) is the one `adopt` row — the field
disagrees only in how it *frames* the current refusal:

- **Lanes 1 and 2** frame it as a deliberate, temporary safety choice.
  Lane 1: "forjar blocks a safe, isolated operation" (repeated as the Delta
  for all three of its behaviour-7 rows, disposition `adopt(Scope
  undo/rollback to the specific stack's generation history rather than
  locking the entire shared state directory)`). Lane 2 states it most
  explicitly in its Pulumi row's Delta: "Temporary safe refusal."
- **Lane 3** frames the same code as a design limitation rather than a
  deliberate stance, using the word "conflating": "Forjar conflates history;
  Pulumi scopes it" (Delta, behaviour-7 Pulumi row), and again for the
  Kubernetes row: "Forjar lacks K8s's resource-scoped history."

Both framings describe the same fact — `multi_stack_restore_refusal`
(`src/core/state/stamp.rs:150`) refuses unconditionally once a state dir
holds more than one stack, because generations are not yet split per stack —
and both conclude `adopt`. The disagreement is rhetorical (a safety net vs. a
gap), not about what forjar does today or what PMAT-162 should build.

## What the GO spec (PMAT-162) should copy

**Majority finding (lanes 1 and 2): Nix profile generations.** Nix's
`nix-env --rollback` (and named-generation switches) work because each
profile's generation history is independent of every other profile sharing
the same Nix store — rolling back one profile never touches another's
current generation. Applied to forjar, this means: generations must record
their owning stack, and `undo`/`rollback` must replay only the invoking
stack's subset of a shared snapshot, leaving every other stack's resources
exactly as they were.

**Dissent (lane 3): Kubernetes rollout history.** Lane 3 instead points to
Kubernetes, where a Deployment's revision history is attached to that
Deployment's own identity (via its ReplicaSets and their `ownerReferences`),
so `kubectl rollout undo` is inherently scoped without needing a separate
per-resource generation concept — the lineage IS the object's identity, not
a parallel structure keyed by owner.

**The one sentence that most changes the spec** (lane 2's formulation, and
the one this audit adopts): *the GO spec must isolate generations by stack
ID and replay only the target stack's resources, upgrading forjar's
whole-dir snapshot mechanism to a Nix-like stack-scoped restore.*

## Reconciliation

Every behaviour-change bullet in `CHANGELOG.md`'s `[Unreleased]` section
(read from `PMAT-161-state-stamp-per-name`), mapped to its behaviour number,
so `scripts/dogfood/crux-reconcile.sh` can check each bullet has a row above:

1. **(1)** "Every resource-set selector `-r`, `-g`, `--subset`,
   `--resource-filter` and `make` goals now resolves exactly once ... the
   positive selection is then closed downward over `depends_on` ... so a
   targeted apply cannot run against an unconverged prerequisite it never
   selected."
2. **(2)** "`--check` now honours `--subset`/`--exclude`/`-g`/`--skip`/`-m`
   exactly as `apply` does, prints the same `Subset filter '...': N selected`
   line, and exits 0 when only resources outside the selection are drifted."
3. **(3)** "The standalone `forjar check` command ... resolves its own
   `-r`/`-t` through the same resolver ... a typo in `check -r` or `check -t`
   is refused up front ... rather than silently reported as `0 pass, 0 fail`
   at exit 0."
4. **(4)** "a negative selector that removes every selected resource
   (`--exclude '*'` ...) is now refused with `no resources remain: ...
   removed every selected resource` instead of converging nothing at exit 0."
5. **(5)** "`make`'s stripping of an unrequested phony target now contracts
   the `depends_on` edges through it (`a -> phony -> c` keeps `a` after `c`)
   instead of scrubbing them, which could reorder a goal ahead of its
   transitive prerequisite."
6. **(6)** "The stamp (`forjar.lock.yaml`'s `stacks:` map, schema `1.1`) is
   now keyed by name exactly as the rest of the lock is; `apply` writes only
   its own name's entry and warns on exactly two conditions: the SAME name
   last applied from a DIFFERENT `-f` ... or a machine this apply would write
   recorded under ANOTHER stack's entry."
7. **(7)** "`apply`, `undo`, `undo --resume`, and `rollback` ... all three
   refuse outright whenever the state dir holds more than one stack,
   regardless of which name's generation was asked for ... Stack-scoped
   restore (schema `1.2`, owner and machines recorded per generation) is
   tracked as PMAT-162."
8. **(8)** "Outputs merge per stack instead of being replaced wholesale, so a
   second stack's apply no longer wipes the first's `{{stack.*}}` values."
9. **(9)** "`forjar status` now attributes each machine to the stack that
   actually wrote it rather than to whichever config applied last."

## Provenance

- Lane conversations: `conv-e1814b17` (108 s), `conv-67822dca` (113 s),
  `conv-5ace27f8` (83 s).
- All three lanes ran from documentation memory only, with no network access
  and no live invocation of any reference system named above.
- Caveat: lane 3 grounded 7 of its 9 findings in `CHANGELOG.md` rather than
  in forjar source, citing source directly for only behaviours (4) and (6);
  its `[V]` claims for the other seven behaviours are corroborated in this
  audit's Findings table against the actual `file:line` locations on
  `PMAT-161-state-stamp-per-name`, not taken on lane 3's word alone.

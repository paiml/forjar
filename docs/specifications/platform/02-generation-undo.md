# 02: Generation Model and Undo

> Extended Nix-style generations with config tracking, active undo, and multi-machine coordination.

**Spec IDs**: FJ-2002 (generations), FJ-2003 (undo), FJ-2005 (undo-destroy) | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## Current State

Forjar already has Nix-style generations (`src/cli/generation.rs`):
- `create_generation()` copies state/ into `state/generations/<N>/` with `.generation.yaml`
- `rollback_to_generation()` restores lock files from a numbered generation
- `atomic_symlink_switch()` updates `current` link via temp-symlink + rename(2)
- `gc_generations()` prunes old generations (keep N newest)
- Named snapshots (`src/cli/snapshot.rs`) provide save/restore/delete

**What's missing**: Generations store lock files only. They don't store the config that produced them, don't know which resources changed, and rollback doesn't re-execute anything.

---

## Extended Generation Model

Generations are **global** (one number sequence per `state_dir`), matching existing `create_generation()`. Each generation contains all machines' state files.

### State Directory Locking

All operations that modify state acquire an exclusive file lock before proceeding:

```
fn acquire_state_lock(state_dir) -> FileLock:
    let lock_path = state_dir / ".forjar.lock"
    let lock = flock(lock_path, LOCK_EX | LOCK_NB)
    if lock.failed():
        // Another forjar instance is running
        error("state directory is locked by another forjar process (PID in .forjar.lock)")
    write(lock_path, format!("pid={}\nstarted={}", getpid(), now()))
    lock  // dropped on scope exit → auto-unlock
```

This prevents:
- Two `forjar apply` runs from creating the same generation number
- `forjar undo` interleaving with `forjar apply` on the same state directory
- Ingest reading half-written state files (see [01-sqlite-query-engine.md](01-sqlite-query-engine.md))

The lock is per-state-directory, not per-machine. Cross-machine operations hold the lock for the entire operation. The lock is **advisory** (flock), not mandatory — external tools can still modify state files, but two forjar processes cannot.

Config recovery uses **git-first, snapshot-fallback**: store `git_ref` (commit SHA) at apply time, retrieve config via `git show {ref}:forjar.yaml`. Only store full YAML when the working tree is dirty.

```
Generation 0: Initial apply
  config_hash: blake3:abc123...
  git_ref: a1b2c3d
  intel:  [bash-aliases(CREATE), gitconfig(CREATE), cargo-tools(CREATE), ...]
  jetson: [cuda-toolkit(CREATE), bashrc(CREATE), ...]
  lambda: [training-output(CREATE), build-apr(CREATE), ...]

Generation 1: Update stack-tools, add zshrc
  config_hash: blake3:def456...
  git_ref: e4f5g6h
  intel:  [stack-tools(UPDATE), zshrc(CREATE)]
  jetson: []  (unchanged)
  lambda: []  (unchanged)
  delta:  +zshrc, ~stack-tools

Generation 2: forjar undo (active undo — re-applies gen 0 config)
  config_hash: blake3:abc123...  (same as gen 0)
  git_ref: NULL (dirty working tree — undo modifies config in-flight)
  config_snapshot: <full YAML stored as fallback>
  action: undo
  intel:  [zshrc(DESTROY), stack-tools(UPDATE)]  (reverted)
```

---

## Commands

```bash
# --- EXISTING (keep as-is) ---
forjar generations                       # list generations
forjar rollback --generation 5 --yes     # restore lock files from gen 5
forjar snapshot save pre-upgrade         # named checkpoint
forjar snapshot restore pre-upgrade --yes
forjar destroy --yes                     # reverse-DAG teardown

# --- NEW: Active Undo ---
forjar undo                              # undo last apply across all machines
forjar undo --machine intel              # single machine
forjar undo --generations 3              # go back 3 generations
forjar undo --dry-run                    # show what would change

# --- NEW: Undo Destroy ---
forjar undo-destroy                      # re-apply from destroy-log.jsonl
forjar undo-destroy --machine intel
# NOTE: reliable for files with inline content. Best-effort for packages
# (version float), services (runtime state), source: files (external path).
# Tasks/Users/Networks/Models/Recipes skipped unless --force.

# --- NEW: Generation Diff ---
forjar diff --generation 3 7
forjar diff --generation 3 7 --machine intel
```

---

## Undo Algorithm

Key distinction: existing `rollback_to_generation()` restores lock files only. **Active undo** re-applies the previous config to actually converge machines.

```
fn undo(steps, machine_filter):
    # Phase 0: Load generation state
    let current_gen = current_generation(gen_dir)
    let target_gen = current_gen - steps

    # Phase 1: Load config from git or snapshot
    let current_config = load_config(current_gen)   // git show or config_snapshot
    let target_config = load_config(target_gen)

    # Phase 2: Compute resource diff across machines
    for machine in affected_machines(machine_filter):
        let current_resources = resources_at_generation(machine, current_gen)
        let target_resources = resources_at_generation(machine, target_gen)
        added = current_resources - target_resources       # need DESTROY
        removed = target_resources - current_resources     # need CREATE
        modified = resources where hash differs             # need UPDATE

    # Phase 3: Reversibility check (existing classify() from reversibility.rs)
    for resource in added:
        if classify(&resource, &Destroy) == Irreversible:
            warn and require --force

    # Phase 4: Build combined execution plan
    plan = Plan {
        destroys: reverse_dag_order(added),
        creates: dag_order(removed),
        updates: dag_order(modified),
    }

    # Phase 5: Execute (reuse cmd_apply infrastructure)
    create_generation(state_dir)
    for machine in dependency_order(affected_machines):
        record_pre_undo_state(machine)       // append to destroy-log.jsonl
        write_progress(machine, "in_progress")
        execute_plan(machine, plan)
        write_progress(machine, "completed")
        if failed:
            write_progress(machine, "failed", failed_resources)
            continue  // DO NOT abort other machines

    # Phase 6: Record undo generation
    create_generation(state_dir)
    write_generation_meta(action="undo", parent=current_gen, git_ref=get_git_ref())
```

---

## Multi-Machine Atomicity

True distributed atomicity is impossible without 2PC. We explicitly **do not attempt it**. Instead: **best-effort ordered execution with resume-on-failure**.

```
Phase 1 — Pre-flight (all machines, parallel)
  - Verify SSH connectivity
  - Verify current state matches expected generation
  - Detect any drift that would conflict
  - FAIL FAST: if any machine unreachable, abort before changes

Phase 2 — Snapshot (all machines, parallel)
  - Record pre-undo state in destroy-log.jsonl
  - Create generation entry with action="undo"

Phase 3 — Execute (machines in dependency order)
  - Independent machines: parallel
  - Dependent machines: respect cross-machine deps
  - Per-resource success/failure recorded in progress file

Phase 4 — Verify (all machines, parallel)
  - Drift detection against target generation
  - Report convergence status

Phase 5 — On failure: resume (NOT abort)
  - Mark generation as "partial"
  - Log succeeded/failed to state/<machine>/undo-progress.yaml
  - `forjar undo --resume` picks up where it left off
  - NO `--abort`: rolling back completed machines while the failing machine
    is broken creates WORSE inconsistency. Fix the machine, then --resume.
```

### Undo-Resume State Machine

The progress file records exactly where the undo stopped:

```yaml
# state/<machine>/undo-progress.yaml
generation_from: 12
generation_to: 10
started_at: "2026-03-05T14:30:00Z"
status: partial  # pending | in_progress | partial | completed
resources:
  bash-aliases:   { status: completed, at: "2026-03-05T14:30:01Z" }
  gitconfig:      { status: completed, at: "2026-03-05T14:30:02Z" }
  cargo-tools:    { status: failed, error: "SSH timeout", at: "2026-03-05T14:30:05Z" }
  stack-tools:    { status: pending }
  zshrc:          { status: pending }
```

Resume behavior:
1. **Re-check completed resources for drift** before proceeding. If resource `bash-aliases` was manually modified since the partial undo, re-apply it (it's now stale).
2. **Retry the failed resource** (`cargo-tools`) first.
3. **Continue with pending resources** in DAG order.
4. Resume is **idempotent**: running `--resume` on a completed undo is a no-op.
5. If the progress file is missing or corrupted, refuse to resume — require `forjar undo` (fresh start) or `--force-resume` which treats all resources as pending.

**TOCTOU mitigation**: Resume re-runs pre-flight (Phase 1) before executing. If a machine is unreachable or has unexpected drift, the user is warned before changes begin. This doesn't eliminate the race (the machine could change between pre-flight and execution), but it catches the common case of stale state.

**Why no `--abort`**: Aborting a distributed operation requires undoing completed work on machines that succeeded while the trigger machine is broken. This is the exact 2PC problem we don't solve. The safe path is always forward: fix and resume.

---

## Stack Destroy

```bash
forjar destroy --yes

# Algorithm:
# 1. Build combined reverse-DAG across all machines
# 2. Respect cross-machine dependencies
# 3. Parallel destroy within independent groups
# 4. Record every destruction in destroy-log.jsonl with pre-state
# 5. Update state.db generation with action="destroy"
# 6. Write state.lock.yaml tombstone (status: destroyed)
#
# KNOWN LIMITATION (destroy.rs:cleanup_state_files): current code removes
# state.lock.yaml even if some resources failed. This spec requires fixing:
# only remove lock entries for resources that succeeded.
```

## Stack Undo

```bash
forjar undo --yes

# 1. Read latest generation per machine
# 2. Read parent generation (N-1) per machine
# 3. Compute diff per machine
# 4. Build combined execution plan
# 5. Execute in dependency order
# 6. Create new generation with action="undo"
```

## Cross-Machine Dependency Resolution

```yaml
resources:
  nfs-server:
    type: service
    machine: intel
    service: nfs-kernel-server
  nfs-mount:
    type: mount
    machine: jetson
    depends_on: [nfs-server]    # cross-machine dep
    source: intel:/export/data
    target: /mnt/data
```

Destroy order: `nfs-mount` (jetson) → `nfs-server` (intel)
Undo-destroy order: `nfs-server` (intel) → `nfs-mount` (jetson)

---

## Implementation

### Phase 2: Extended Generations (FJ-2002) -- PARTIAL
- [x] `GenerationMeta` type with config_hash, git_ref, action, parent_generation, operator, forjar_version, bashrs_version
- [x] `MachineDelta` type for per-machine resource create/update/destroy deltas
- [x] `get_git_ref()` and `git_is_dirty()` helpers for config recovery
- [x] Backward-compatible YAML parsing (old format still works)
- [x] YAML roundtrip with skip_serializing_if for clean output
- [x] Wire `GenerationMeta` into `create_generation()` (replaces manual YAML) — enriches with git_ref, forjar_version via builder pattern
- [ ] Populate SQLite `generations` table from `state/generations/` on ingest
- [x] Enrich `forjar generations` with resource count, delta, action type
- [x] `forjar diff --generation 3 7`: `GenerationDiff`, `ResourceDiff`, `DiffAction`, `diff_resource_sets()`
- **Extends**: `src/cli/generation.rs`

### Phase 3: Stack Undo (FJ-2003)
- [x] Undo plan types: `UndoPlan`, `UndoResourceAction`, `UndoAction` (Destroy/Create/Update)
- [x] Undo progress types: `UndoProgress`, `ResourceProgress`, `ResourceProgressStatus`, `UndoStatus`
- [x] `UndoPlan::format_summary()` with irreversibility warnings
- [x] `UndoProgress` counts: `completed_count()`, `failed_count()`, `pending_count()`
- [x] YAML serialization for `undo-progress.yaml` resume file
- [x] Generation diff: `diff_resource_sets()` compares resource sets between gen N and gen N-K
- [x] `forjar undo --dry-run`
- [x] Active undo: config snapshot from target gen, re-run `cmd_apply` with force
- [ ] Multi-machine coordination: phased execution with pre-flight SSH check
- [x] Undo-resume: record progress, `--resume` picks up
- **Extends**: `src/cli/destroy.rs:cmd_rollback`

### Phase 5: Undo-Destroy (FJ-2005) -- PARTIAL
- [x] `DestroyLogEntry` type with JSONL serialization for undo-destroy recovery
- [x] Pre-destroy state recording: machine, resource_id, pre_hash, config_fragment, reliable_recreate
- [x] Extend `destroy_single_resource()` to write pre-state to `destroy-log.jsonl` — `write_destroy_log_entry()` captures pre_hash, config_fragment, reliable_recreate
- [x] Fix `cleanup_state_files()` — only remove lock entries for succeeded resources — `cleanup_succeeded_entries()` reads lock YAML, removes succeeded entries, preserves failed
- [x] `forjar undo-destroy` — replay from destroy-log.jsonl
- [x] Reversibility gate: skip irreversible, warn with `--force`
- [x] Round-trip test: `destroy_log_roundtrip` + `cleanup_succeeded_entries_partial` in tests_destroy.rs — verify destroy-log.jsonl writing/parsing and partial lock cleanup
- **Extends**: `src/cli/destroy.rs:cmd_destroy`

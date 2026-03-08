# 19: Defect Analysis — Five Whys, Chain of Thought, Toyota Way

> Root cause analysis of 5 production defects discovered during qwen-coder-deploy.

**Spec IDs**: FJ-3000 (exit code), FJ-3010 (force rebuild), FJ-3020 (runtime score), FJ-3030 (LD_LIBRARY_PATH lint), FJ-3040 (health check lint) | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

**Issues**: [#64](https://github.com/paiml/forjar/issues/64), [#65](https://github.com/paiml/forjar/issues/65), [#66](https://github.com/paiml/forjar/issues/66), [#67](https://github.com/paiml/forjar/issues/67), [#68](https://github.com/paiml/forjar/issues/68)

---

## Defect Priority Order (Toyota Way: defects first)

| Priority | Issue | Category | Impact | Toyota Principle |
|----------|-------|----------|--------|-----------------|
| P0 | #68 | Silent false-success | Data corruption equivalent — user trusts a lie | **Jidoka** (stop the line) |
| P1 | #65 | Force kills running services | 30–60s downtime on every `--force` | **Heijunka** (level the load) |
| P1 | #66 | Score always 0 | Users can't improve — no actionable feedback | **Mieruka** (make problems visible) |
| P2 | #64 | LD_LIBRARY_PATH hazard | 3h debugging "works manually, fails via forjar" | **Poka-yoke** (mistake-proof) |
| P2 | #67 | Health check race | Fragile sleep values, no retry pattern | **Poka-yoke** (mistake-proof) |

---

## #68: Exit Code Swallowed — Silent False-Success (P0)

**Symptom**: `forjar apply` reports 9/9 converged, but the running server is the wrong build (old binary, 0.3 tok/s instead of 20 tok/s). Discovered only during benchmarking.

### Five Whys

1. **Why did forjar report success?** The health check `curl -sf http://127.0.0.1:8083/health` returned 200.
2. **Why did curl succeed when the new server failed to start?** A manually-started old instance was still running on port 8083 from a debugging session.
3. **Why didn't forjar detect this was the wrong process?** `completion_check` validates "is something responding" — not "is it the process I launched."
4. **Why doesn't forjar track process identity?** Task resources are fire-and-forget: `nohup` detaches from forjar's control. No PID tracking, no version check.
5. **Why are tasks fire-and-forget?** Forjar's convergence model is declarative (check → apply → verify), but `nohup` creates a stateful daemon that outlives the apply. **Fundamental impedance mismatch** between declarative IaC and stateful process management.

### Root Cause

Task resources conflate two operational modes:
- **Idempotent state mutation** (install package, write file) — fits the convergence model
- **Daemon lifecycle management** (start/stop/health-check a long-running process) — requires process identity, not just endpoint health

### Chain of Thought

The codegen already injects `set -euo pipefail` (`task.rs` lines 49, 104, 138, 187), so individual command failures DO propagate. The problem is structural: when the user writes `;`-chained commands in a single `command:` field, the generated script wraps them but the final `curl` health check's success masks earlier failures. The `set -e` helps for `&&` chains and multiline `|` blocks, but NOT for user-written `;` chains passed as a single string.

### Toyota Way Analysis

- **Jidoka** (自働化 — stop the line): Forjar MUST NOT report success when it cannot verify process identity. A health check that hits port 8083 without knowing WHO is listening is a **quality gate that always passes** — the worst kind.
- **Genchi Genbutsu** (現地現物 — go and see): The failure was invisible until manual benchmarking. The feedback loop between apply and verification is broken — forjar should detect "stale process" at apply time.
- **Andon cord**: If a task's `nohup` starts a process, forjar should record the PID and verify that PID is alive during `completion_check`. If the PID is dead but the port is open, that's an Andon signal.

### Fix Specification (FJ-3000)

**Phase 1 (lint)**: bashrs warn on `;`-chained commands in task `command:` fields. Suggest `&&` or multiline with `set -e`.

```
# bashrs lint pattern
FJ-3000: "semicolon chain in task command — earlier failures masked by later success. Use '&&' or multiline '|' block"
```

**Phase 2 (codegen)**: For task resources with `nohup`, inject PID capture and process identity verification:

```bash
set -euo pipefail
# Launch
nohup server --port 8083 > /tmp/log 2>&1 &
FORJAR_PID=$!
echo "$FORJAR_PID" > /tmp/forjar-llamacpp-serve.pid
# Health check: verify PID is alive AND endpoint responds
for i in $(seq 1 60); do
  if kill -0 "$FORJAR_PID" 2>/dev/null && curl -sf http://127.0.0.1:8083/health; then
    exit 0
  fi
  sleep 1
done
echo "FAIL: process $FORJAR_PID died or health check timed out" >&2
exit 1
```

**Phase 3 (first-class)**: Add `process_identity` field to task resources:

```yaml
llamacpp-serve:
  type: task
  task_mode: service
  command: "nohup llama-server --port 8083 ..."
  health_check:
    url: "http://127.0.0.1:8083/health"
    interval: 1s
    timeout: 60s
  process_identity:
    pid_file: /tmp/forjar-llamacpp-serve.pid
    verify_pid: true  # kill -0 PID before trusting health check
```

---

## #65: `--force` Rebuilds Kill Running Services (P1)

**Symptom**: `forjar apply --force` on qwen-coder-deploy kills all 3 inference servers during cmake rebuild. 30–60s downtime on every `--force`.

### Five Whys

1. **Why did the running servers crash?** `--force` re-ran `cmake --build` which replaced `.so` files on disk while `llama-server` had them mmap'd.
2. **Why did `--force` re-run build resources?** `--force` passes empty locks to the planner (`executor/mod.rs` line 199: `HashMap::new()`), so ALL resources are treated as new.
3. **Why isn't `--force` selective?** It was designed for secret rotation (FJ-2300) — the use case is "templates changed but hashes didn't, re-resolve everything."
4. **Why can't users target only specific resources?** `--resource` flag exists, but `--force --resource serve` is an undiscoverable workaround. Users expect `--force` to mean "re-converge smartly."
5. **Why does rebuilding a dep kill its dependent?** The DAG captures `build → serve`, but `--force` treats the DAG as "rebuild from scratch" instead of "re-evaluate convergence from leaves."

### Root Cause

`--force` is a blunt instrument: all-or-nothing bypass of the hash comparison planner. It conflates three distinct operations:
- **Re-evaluate** convergence (re-run check scripts, compare live state)
- **Re-apply** resources whose check fails (actual reconvergence)
- **Re-execute** everything regardless of state (nuclear option)

Users want (a), forjar does (c).

### Chain of Thought

The planner (`planner/mod.rs`) compares `hash_desired_state(resource)` against `lock.hash`. With `--force`, empty locks mean every resource gets `Action::Create` or `Action::Update`. The planner doesn't distinguish "force re-evaluate" from "force re-apply." The completion_check exists but isn't consulted during force mode.

### Toyota Way Analysis

- **Heijunka** (平準化 — level the load): `--force` creates a spike of unnecessary work. A leveled approach checks completion first, only re-applies what actually diverged.
- **Muda** (無駄 — waste): Rebuilding already-correct binaries is waste. Killing running services to rebuild unchanged code is **double waste**.
- **Muri** (無理 — overburden): The user is overburdened with choosing between "no force" (miss secret rotation) and "force" (kill everything).

### Fix Specification (FJ-3010)

**Phase 1**: `--force` should consult `completion_check` / `check_script()` before re-applying:

```
fn plan_with_force(config, order):
    for resource in order:
        check_result = run_check_script(resource)
        if check_result.status == "converged" && !resource.has_secret_refs():
            skip  // completion_check passes, no secrets to re-resolve
        else:
            plan_apply(resource)
```

**Phase 2**: Add `--force-tag <TAG>` for selective forcing:

```bash
forjar apply --force-tag service   # Only re-apply resources tagged "service"
forjar apply --force-tag serve     # Only re-apply serve resources
```

**Phase 3**: Add `--refresh` flag (softer than `--force`):

```bash
forjar apply --refresh  # Re-run check scripts, only re-apply what fails
forjar apply --force    # Nuclear: re-apply everything (current behavior, renamed)
```

---

## #66: Score Runtime Dimensions Always 0 (P1)

**Symptom**: After 9/9 successful apply, `forjar score` shows COR=0, IDM=0, PRF=0, Grade D/pending.

### Five Whys

1. **Why are COR/IDM/PRF always 0?** `input.runtime == None` — `cmd_score()` passes `runtime: None` (`score.rs` line 18).
2. **Why is runtime None?** No code reads apply results into `RuntimeData`.
3. **Why doesn't score read apply results?** Scoring was designed as static analysis first. Runtime was planned but never bridged.
4. **Why doesn't apply record scorer-relevant metrics?** Apply DOES record metrics in `events.jsonl` and `state.lock.yaml`. The data exists. Score doesn't read it.
5. **Why does the UI show 0/100?** The scorer always computes runtime dimensions (returning 0 when runtime is None) instead of hiding them or explaining how to earn them.

### Root Cause

Pipeline gap: `apply → events.jsonl` ✓ → `events.jsonl → RuntimeData` ✗ → `RuntimeData → score` ✗. The apply command writes the data; score doesn't read it.

### Chain of Thought

The scorer (`scoring.rs`) has fully-implemented COR/IDM/PRF functions (lines 260–373). `score_correctness()` awards +40 for `first_apply_pass`, +15 for `all_resources_converged`. `score_idempotency()` awards +25 for `second_apply_pass` and +25 for `zero_changes_on_reapply`. All the logic exists — the bridge is missing.

The data needed:
- **COR**: Did apply succeed? (from `events.jsonl`: `apply_completed` with `resources_failed == 0`)
- **IDM**: Did re-apply change nothing? (from second `apply_completed` with `resources_converged == 0`)
- **PRF**: How long did apply take? (from `duration_secs` in events)

All of this is already in `events.jsonl`. The fix is a reader function.

### Toyota Way Analysis

- **Mieruka** (見える化 — make problems visible): Showing 0/100 with no actionable path violates mieruka. The user sees "D grade" but can't see what to do.
- **Hansei** (反省 — reflection): The design assumed users would run `forjar score --status qualified` manually. This was a design mistake — qualification should be earned automatically.

### Fix Specification (FJ-3020)

**Phase 1**: `score` reads last 2 apply events from `events.jsonl`:

```rust
fn build_runtime_data(state_dir: &Path, machine: &str) -> Option<RuntimeData> {
    let events = read_events(state_dir, machine)?;
    let applies: Vec<_> = events.iter()
        .filter(|e| e.event_type == "apply_completed")
        .collect();
    let first = applies.last()?;  // most recent
    let second = applies.get(applies.len().checked_sub(2)?);  // second most recent

    Some(RuntimeData {
        first_apply_pass: first.resources_failed == 0,
        all_resources_converged: first.resources_converged > 0,
        first_apply_ms: (first.total_seconds * 1000.0) as u64,
        second_apply_pass: second.map(|s| s.resources_failed == 0).unwrap_or(false),
        zero_changes_on_reapply: second.map(|s| s.resources_converged == 0).unwrap_or(false),
        second_apply_ms: second.map(|s| (s.total_seconds * 1000.0) as u64),
        ..Default::default()
    })
}
```

**Phase 2**: Show qualification guidance when runtime is None:

```
Runtime Grade: pending
  Tip: run `forjar apply` twice to earn runtime grade
       (first apply = COR, second apply = IDM/PRF)
```

---

## #64: bashrs Should Flag LD_LIBRARY_PATH Hazards (P2)

**Symptom**: `nohup llama-server ...` silently fails at runtime because `libggml-blas.so` isn't in the linker search path. 3 hours debugging.

### Five Whys

1. **Why did llama-server crash at runtime?** `libggml-blas.so` not found by dynamic linker.
2. **Why wasn't the shared lib found?** `LD_LIBRARY_PATH` not set, and the .so is in the build directory (not a standard lib path).
3. **Why didn't forjar warn?** bashrs only validates shell syntax (SC-patterns), not binary runtime dependencies.
4. **Why doesn't bashrs check runtime deps?** bashrs is a static linter — it doesn't have access to `ldd` output or the binary's RPATH.
5. **Why is this a forjar concern?** Forjar generates the execution context. It runs on the target machine and CAN check `ldd` output before launching.

### Root Cause

bashrs lint scope is syntax-only. It doesn't validate that binaries referenced in commands have their runtime dependencies satisfied. For `nohup` commands specifically, the child process inherits the current environment but may lack paths set during interactive sessions.

### Chain of Thought

The purifier (`purifier.rs`) runs bashrs's `lint_shell()` which checks SC2xxx patterns. Adding a new lint rule for `nohup <path>` where `<path>` is an absolute binary path would require:
1. Detecting the pattern: `nohup /absolute/path/binary`
2. Checking if `dirname(binary)` contains `.so` files
3. Warning if `LD_LIBRARY_PATH` is not set or doesn't include that directory

This can be done as a forjar-specific lint rule (not upstream bashrs), similar to how `lint.rs` already runs custom checks beyond bashrs.

### Toyota Way Analysis

- **Poka-yoke** (ポカヨケ — mistake-proof): The error is entirely preventable with a lint check. The fix doesn't require changing behavior, just adding a warning.
- **Genchi Genbutsu**: The root cause was only visible on the target machine. Forjar should "go and see" the binary's deps before launching.

### Fix Specification (FJ-3030)

**Lint rule** (in `lint.rs`, not bashrs upstream):

```
FJ-3030: nohup launches binary with shared libs in non-standard path

Pattern: nohup <absolute-path> (where dirname contains .so files)
Fix: Add LD_LIBRARY_PATH=<dirname> before nohup
Severity: warn
```

**Check script enhancement** for task resources:

```bash
# Injected before nohup in check_script()
if command -v ldd >/dev/null 2>&1; then
  MISSING=$(ldd /path/to/binary 2>&1 | grep "not found" || true)
  if [ -n "$MISSING" ]; then
    echo "warn: missing shared libraries: $MISSING" >&2
  fi
fi
```

---

## #67: Lint Should Catch nohup Health-Check Race (P2)

**Symptom**: `nohup server & sleep 15; curl health` fails under CI load because model loading takes 25–40s. Fixed sleep is fragile.

### Five Whys

1. **Why did the health check fail?** `sleep 15` was too short for model loading under CI load.
2. **Why was the sleep hardcoded?** No first-class health check mechanism — users improvise with shell.
3. **Why is the sleep fragile?** It's a fixed-duration wait with no feedback. Works on fast hardware, fails under load.
4. **Why doesn't forjar have proper health checks?** Task resources use `completion_check` — a one-shot test, not a polling loop with retries.
5. **Why doesn't lint warn about this anti-pattern?** bashrs's pattern set doesn't include `nohup...sleep...curl` as a recognized anti-pattern.

### Root Cause

Forjar lacks a first-class `health_check` field for daemon-mode task resources. Users are forced into fragile shell workarounds that lint doesn't recognize as problematic.

### Chain of Thought

The `health_check` field already exists on `Resource` struct (`resource.rs` line 394: `pub health_check: Option<String>`) but it's only used for service resources (service status checking). Task resources with `task_mode: service` should be able to use it too.

The fix has two parts:
1. **Lint rule**: Detect `nohup ... & sleep N; curl` and suggest polling or `health_check` field
2. **Codegen improvement**: When `health_check` is set on a task resource, generate a polling loop instead of requiring the user to write one

### Toyota Way Analysis

- **Poka-yoke**: The anti-pattern is so common (#67 notes "appears in nearly every service-type task") that it should be detected and guided toward the correct pattern.
- **Standardization**: A first-class `health_check` field replaces N different hand-rolled polling loops with one tested codegen template.

### Fix Specification (FJ-3040)

**Lint rule**:

```
FJ-3040: Fixed sleep before health check in nohup command

Pattern: nohup ... & sleep <N>; curl
Fix: Use health_check field or polling pattern: for i in $(seq 1 60); do curl -sf URL && exit 0; sleep 1; done; exit 1
Severity: warn (--strict only)
```

**Codegen enhancement**: When task has `health_check` and `task_mode: service`:

```yaml
llamacpp-serve:
  type: task
  task_mode: service
  command: "nohup llama-server --model ... &"
  health_check:
    url: "http://127.0.0.1:8083/health"
    interval: 1s
    timeout: 60s
```

Generated script:

```bash
set -euo pipefail
nohup llama-server --model ... > /tmp/forjar-llamacpp-serve.log 2>&1 &
FORJAR_PID=$!
# FJ-3040: Polling health check (replaces fragile sleep+curl)
for i in $(seq 1 60); do
  if ! kill -0 "$FORJAR_PID" 2>/dev/null; then
    echo "FAIL: process $FORJAR_PID died during startup" >&2
    tail -20 /tmp/forjar-llamacpp-serve.log >&2
    exit 1
  fi
  if curl -sf "http://127.0.0.1:8083/health" >/dev/null 2>&1; then
    echo "OK: health check passed after ${i}s"
    exit 0
  fi
  sleep 1
done
echo "FAIL: health check timed out after 60s" >&2
exit 1
```

---

## Cross-Cutting Analysis

### Systemic Theme: Task Resources Are Underspecified

All 5 issues stem from the same architectural gap: **task resources are treated as "run a command" when they should be "manage a lifecycle."** The task framework (spec 15, FJ-2700) defined batch/pipeline/service/dispatch modes but stopped at codegen. The runtime semantics — process identity, health monitoring, graceful shutdown, artifact verification — were left to user-written shell.

### Toyota Way Summary

| Principle | Japanese | Issue | Application |
|-----------|---------|-------|-------------|
| Jidoka | 自働化 | #68 | Stop reporting success when process identity is unverified |
| Heijunka | 平準化 | #65 | Level `--force` into graduated modes (refresh/force-tag/force) |
| Mieruka | 見える化 | #66 | Make runtime qualification visible and actionable |
| Poka-yoke | ポカヨケ | #64, #67 | Mistake-proof nohup commands with lint rules |
| Genchi Genbutsu | 現地現物 | #64 | Go and see: check `ldd` on target before launch |
| Muda | 無駄 | #65 | Eliminate waste: don't rebuild what's already correct |
| Hansei | 反省 | #66 | Reflect: "pending" with no path forward was a design mistake |

### Dependency Graph

```
#68 (P0: false-success) ─── blocks ──→ #66 (score correctness)
        │                                      │
        └──── enables ──→ #67 (health lint)    │
                                               │
#65 (P1: force rebuild) ── independent ──→ #64 (LD_LIBRARY lint)
```

Fix #68 first — it's a correctness bug. #67's health check polling becomes the implementation mechanism for #68's process identity verification. #66 depends on #68 because runtime scoring requires trustworthy apply results.

---

## Implementation Phases

### Phase 19a: P0 — Exit Code Safety (FJ-3000)
- [ ] bashrs lint: warn on `;`-chained commands in task `command:` fields
- [ ] Codegen: inject PID capture for `nohup` task commands
- [ ] Codegen: generate polling health check with PID liveness verification
- [ ] Tests: false-success scenario (stale process on port, new process fails to start)
- **Deliverable**: `forjar apply` never reports success when PID died

### Phase 19b: P1 — Graduated Force (FJ-3010)
- [ ] `--refresh` flag: re-run check scripts, only re-apply failures
- [ ] `--force-tag <TAG>`: selective force by tag
- [ ] `--force` remains as nuclear option (current behavior)
- [ ] Document graduated force in `forjar apply --help`
- **Deliverable**: `forjar apply --refresh` doesn't kill running services

### Phase 19c: P1 — Runtime Score Bridge (FJ-3020)
- [ ] `build_runtime_data()` reads last 2 apply events from `events.jsonl`
- [ ] `cmd_score()` passes runtime data when available
- [ ] Show qualification guidance when runtime is None
- [ ] Tests: score after 1 apply (COR only), after 2 applies (COR+IDM+PRF)
- **Deliverable**: `forjar score` shows earned runtime grade after apply

### Phase 19d: P2 — Lint Rules (FJ-3030, FJ-3040)
- [ ] FJ-3030: lint rule for `nohup <absolute-path>` with `.so` in dirname
- [ ] FJ-3040: lint rule for `nohup ... & sleep N; curl` anti-pattern
- [ ] `ldd` check injection in task check scripts
- [ ] Health check field support for task resources (`task_mode: service`)
- **Deliverable**: `forjar lint --strict` catches both patterns

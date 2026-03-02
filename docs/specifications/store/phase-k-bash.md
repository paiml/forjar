# Phase K: Bash Provability (FJ-1357)

**Status**: ✅ Complete
**Implementation**: `src/transport/mod.rs`, `src/core/executor/machine_wave.rs`, `src/core/purifier.rs`

---

## 1. Invariant I8

**No raw shell execution — all shell is bashrs-purified.**

Every shell script must pass bashrs validation before reaching any transport layer (local, SSH, container, pepita). This is the I8 invariant.

## 2. Enforcement Points

### 2.1 Transport Layer (`src/transport/mod.rs`)

`validate_before_exec()` is called at the entry of:
- `exec_script()` — primary execution path
- `exec_script_timeout()` — timeout-wrapped execution
- `query()` — read-only queries

`exec_script_retry()` delegates to `exec_script_timeout()` and is covered transitively.

### 2.2 Hook Execution (`src/core/executor/machine_wave.rs`)

Pre-apply and post-apply hooks are validated before execution:
- `run_pre_hook()` validates the hook script before calling transport
- `run_post_hook_if_success()` validates the hook script before calling transport

### 2.3 Purifier Enhancement (`src/core/purifier.rs`)

`validate_or_purify()` tries validation first, falls back to full purification:
- If `validate_script()` passes, return the script as-is
- If validation fails, attempt `purify_script()` to fix the script
- If purification also fails, return the error

## 3. Previously Violated

Before FJ-1357, the following functions executed shell without bashrs validation:
- `exec_script()` — line 42
- `exec_script_timeout()` — line 65
- `exec_script_retry()` — line 96
- `query()` — line 154
- `run_pre_hook()` — line 47
- `run_post_hook_if_success()` — line 60

All are now gated.

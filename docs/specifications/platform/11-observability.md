# 11: Observability and Diagnostics

> Transport output capture, structured logging, progress reporting, and failure debugging.

**Spec ID**: FJ-2301 | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## The Problem

The transport layer captures full stdout/stderr in `ExecOutput { exit_code, stdout, stderr }`. But after the executor extracts a hash or error message, the raw output is discarded:

| Stage | stdout | stderr |
|-------|--------|--------|
| Transport (`exec_script`) | Captured in `ExecOutput.stdout` | Captured in `ExecOutput.stderr` |
| Executor (success) | Hashed for drift detection, then **discarded** | **Discarded** |
| Executor (failure) | **Discarded** | `.trim()` stored in `ResourceFailed` event |
| Event log | Not stored | Only `error` field (trimmed stderr) |
| User terminal | Never shown | Only on failure, trimmed |
| Lock file | Not stored | Not stored |

A `pip install` that prints 500 lines before failing on line 480? You get `"error": "Could not find a version that satisfies..."` — one line. The 479 lines of context (what DID install, what was attempted, version conflicts) are gone.

---

## Run Log: Persistent Transport Output

Every `forjar apply`, `forjar destroy`, or `forjar undo` invocation creates a **run log** that captures the full output of every script executed on every machine.

### Storage Layout

```
state/<machine>/runs/
  <run_id>/
    meta.yaml                    # run metadata
    <resource_id>.check.log      # check script output (state query)
    <resource_id>.apply.log      # apply script output
    <resource_id>.destroy.log    # destroy script output
    <resource_id>.script         # the actual script that was executed
```

### Run Metadata

```yaml
# state/intel/runs/r-c7d16accaf62/meta.yaml
run_id: r-c7d16accaf62
machine: intel
command: apply
generation: 13
operator: noah
started_at: "2026-03-05T14:30:00.000Z"
finished_at: "2026-03-05T14:30:04.200Z"
duration_secs: 4.2
resources:
  bash-aliases:   { action: noop }
  gitconfig:      { action: update, exit_code: 0, duration_secs: 0.54 }
  cargo-tools:    { action: update, exit_code: 100, duration_secs: 1.8, failed: true }
  stack-tools:    { action: skipped, reason: "dependency cargo-tools failed" }
summary:
  total: 17
  converged: 15
  noop: 14
  updated: 1
  failed: 1
  skipped: 1
```

### Log File Format

Each `.log` file is a structured log with delimiters, not raw output. This allows tooling to parse sections while remaining human-readable:

```
# state/intel/runs/r-c7d16accaf62/cargo-tools.apply.log

=== FORJAR TRANSPORT LOG ===
resource: cargo-tools
type: package
action: apply
machine: intel
transport: ssh
started: 2026-03-05T14:30:01.400Z
script_hash: blake3:f8a9b2...

=== SCRIPT ===
#!/bin/bash
set -euo pipefail
apt-get update -qq
apt-get install -y cargo-watch cargo-edit

=== STDOUT ===
Hit:1 http://archive.ubuntu.com/ubuntu jammy InRelease
Get:2 http://archive.ubuntu.com/ubuntu jammy-updates InRelease [128 kB]
Reading package lists...
Building dependency tree...
Reading state information...
cargo-edit is already the newest version (0.12.2-1).
The following packages will be installed:
  cargo-watch
E: Unable to locate package cargo-watch

=== STDERR ===
E: Unable to locate package cargo-watch

=== RESULT ===
exit_code: 100
duration_secs: 1.8
finished: 2026-03-05T14:30:03.200Z
```

For successful resources, the log has the same structure but with `exit_code: 0` and typically shorter output.

### Capture Pipeline

```
fn execute_and_capture(machine, resource_id, script, run_dir) -> ExecOutput:
    // 1. Save the script itself (for reproducibility)
    write(run_dir / "{resource_id}.script", script)

    // 2. Execute via transport (existing code path)
    let output = transport::exec_script(machine, script)

    // 3. Persist full output
    let log = format_transport_log(resource_id, machine, script, output)
    write(run_dir / "{resource_id}.apply.log", log)

    // 4. Return ExecOutput for existing executor logic (unchanged)
    output
```

The capture wraps the existing `exec_script` call. No changes to the transport layer itself. The executor continues to work with `ExecOutput` exactly as before — the logs are a side effect.

### What Gets Captured

| Script Phase | When | Captured |
|-------------|------|----------|
| Check script | Every apply (for hash comparison) | `<resource_id>.check.log` |
| Apply script | On Create or Update | `<resource_id>.apply.log` |
| Destroy script | On Destroy | `<resource_id>.destroy.log` |
| State query script | On drift detection | `<resource_id>.check.log` |

NoOp resources have no log files (no script was executed). Skipped resources get an entry in `meta.yaml` but no log file.

---

## Viewing Logs

### `forjar logs` Command

```bash
# Show last run for a machine
forjar logs --machine intel

# Show a specific run
forjar logs --run r-c7d16accaf62

# Show output for a specific resource (most recent failure)
forjar logs --machine intel --resource cargo-tools

# Show only failures
forjar logs --machine intel --failed

# Show the script that was executed (for reproduction)
forjar logs --machine intel --resource cargo-tools --script

# Follow live output during apply (tail -f equivalent)
forjar apply -v --capture  # default: capture is always on
forjar logs --machine intel --follow  # in another terminal

# Cross-machine failure report
forjar logs --failed --all-machines

# Output as JSON (for CI log aggregation)
forjar logs --machine intel --failed --json
```

### Default Output

```
$ forjar logs --machine intel --resource cargo-tools

Run r-c7d16accaf62 (2026-03-05T14:30:00Z, gen 13)
Resource: cargo-tools (package, apply)
Duration: 1.8s | Exit code: 100 | FAILED

--- stdout (12 lines) ---
Hit:1 http://archive.ubuntu.com/ubuntu jammy InRelease
Get:2 http://archive.ubuntu.com/ubuntu jammy-updates InRelease [128 kB]
Reading package lists...
Building dependency tree...
Reading state information...
cargo-edit is already the newest version (0.12.2-1).
The following packages will be installed:
  cargo-watch
E: Unable to locate package cargo-watch

--- stderr (1 line) ---
E: Unable to locate package cargo-watch

--- script ---
#!/bin/bash
set -euo pipefail
apt-get update -qq
apt-get install -y cargo-watch cargo-edit
```

### Failure Summary

```
$ forjar logs --failed --all-machines

FAILURES (last 7 days)
======================

intel/cargo-tools (package) — r-c7d16accaf62, 2026-03-05T14:30:00Z
  Exit 100: E: Unable to locate package cargo-watch
  Tip: forjar logs --run r-c7d16accaf62 --resource cargo-tools

jetson/cuda-toolkit (package) — r-a1b2c3d4e5f6, 2026-03-03T09:12:00Z
  Exit 1: dpkg: error processing cuda-toolkit-12-4 (--configure)
  Tip: forjar logs --run r-a1b2c3d4e5f6 --resource cuda-toolkit

2 failures across 2 machines (last 7 days)
```

---

## Retention Policy

Run logs can grow large, especially for image builds with verbose compiler output.

### Default Retention

```yaml
# In forjar config (optional)
policy:
  logs:
    keep_runs: 10           # keep last N runs per machine (default: 10)
    keep_failed: 50         # keep last N failed runs regardless (default: 50)
    max_log_size: 10MB      # truncate individual log files at this size (default: 10MB)
    max_total_size: 500MB   # total log storage budget per machine (default: 500MB)
```

### Truncation

When a single script produces output exceeding `max_log_size`:

```
=== STDOUT (truncated: 10.0MB of 47.2MB, first 5MB + last 5MB) ===
<first 5MB of output>

... [37.2MB truncated — full output at state/intel/runs/r-abc123/cargo-tools.apply.log.full] ...

<last 5MB of output>
```

The full output is saved with `.full` suffix but excluded from the retention budget. A separate `forjar logs --gc` command cleans `.full` files.

### Garbage Collection

```bash
forjar logs --gc                    # apply retention policy
forjar logs --gc --dry-run          # show what would be deleted
forjar logs --gc --keep-failed      # only delete successful run logs
```

GC runs automatically before each apply if total log size exceeds `max_total_size`. It deletes the oldest successful runs first, keeping all failed runs up to `keep_failed`.

---

## Query Integration

Run logs feed into `state.db` for searchable failure history:

```sql
CREATE TABLE run_logs (
    id          INTEGER PRIMARY KEY,
    machine_id  INTEGER NOT NULL REFERENCES machines(id),
    run_id      TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    action      TEXT NOT NULL,    -- check, apply, destroy
    exit_code   INTEGER NOT NULL,
    duration_secs REAL NOT NULL,
    stdout_lines INTEGER NOT NULL,
    stderr_lines INTEGER NOT NULL,
    stdout_preview TEXT,           -- first 500 chars
    stderr_preview TEXT,           -- first 500 chars (always stored for failures)
    log_path    TEXT NOT NULL,     -- relative path to .log file
    created_at  TEXT NOT NULL,
    UNIQUE(run_id, resource_id, action)
);

CREATE INDEX idx_run_logs_machine ON run_logs(machine_id);
CREATE INDEX idx_run_logs_exit ON run_logs(exit_code);
CREATE INDEX idx_run_logs_resource ON run_logs(resource_id);
CREATE INDEX idx_run_logs_created ON run_logs(created_at DESC);

-- FTS5 for searching error messages
CREATE VIRTUAL TABLE run_logs_fts USING fts5(
    resource_id, stderr_preview,
    tokenize='porter unicode61 remove_diacritics 2'
);
```

This enables:

```bash
forjar query "Unable to locate package" --type error   # FTS5 search across all stderr
forjar query --resource cargo-tools --failures         # failure history for one resource
forjar query --failed --since 7d --timing              # failure rate + duration over time
```

---

## Log Levels

```bash
forjar apply                    # default: summary output (logs still captured to disk)
forjar apply -v                 # verbose: per-resource status lines
forjar apply -vv                # debug: transport commands, hash comparisons, script names
forjar apply -vvv               # trace: stream raw script output to terminal AS IT RUNS
forjar apply --quiet            # errors only (logs still captured to disk)
forjar apply --json             # structured JSON to stdout (for CI)
```

| Level | Flag | Terminal Output | Disk Capture |
|-------|------|----------------|-------------|
| Error | (default on failure) | Failed resources, error context, suggestion | Always |
| Summary | (default) | Resource counts, duration, generation | Always |
| Verbose | `-v` | Per-resource action, duration, hash | Always |
| Debug | `-vv` | Transport commands, hash comparisons | Always |
| Trace | `-vvv` | **Raw script stdout/stderr streamed live** | Always |

**Key**: Disk capture is always on regardless of verbosity. The log level only controls what appears on the terminal. You can run `forjar apply --quiet` and still get full logs on disk for later inspection.

---

## Structured Output

### JSON (`--json`)

```json
{
  "version": "1.0",
  "command": "apply",
  "run_id": "r-c7d16accaf62",
  "generation": 13,
  "duration_secs": 4.2,
  "machines": [
    {
      "name": "intel",
      "resources": [
        {
          "id": "bash-aliases",
          "type": "file",
          "action": "noop",
          "duration_secs": 0.0,
          "hash": "blake3:a1b2c3..."
        },
        {
          "id": "cargo-tools",
          "type": "package",
          "action": "update",
          "exit_code": 100,
          "duration_secs": 1.8,
          "failed": true,
          "error": "E: Unable to locate package cargo-watch",
          "log_path": "state/intel/runs/r-c7d16accaf62/cargo-tools.apply.log"
        }
      ],
      "summary": { "total": 17, "converged": 15, "noop": 14, "updated": 1, "failed": 1, "skipped": 1 }
    }
  ],
  "exit_code": 1
}
```

The `log_path` field points to the full log file for any resource, allowing CI pipelines to upload artifacts on failure.

---

## Exit Codes

| Code | Meaning | When |
|------|---------|------|
| 0 | Success | All resources converged |
| 1 | Partial failure | Some resources failed, others converged |
| 2 | Total failure | No resources converged (all machines unreachable) |
| 3 | Pre-flight failure | SSH check failed, state directory locked |
| 4 | Config error | Invalid YAML, missing machine, cycle in DAG |
| 10 | Drift detected | `forjar drift` found differences (informational) |

---

## Progress Reporting

Long-running operations show progress on stderr:

```
$ forjar apply

[1/3] intel (17 resources)
  [============================] 17/17 converged (4.2s)
[2/3] jetson (8 resources)
  [==================          ] 6/8 converging... (cuda-toolkit)
[3/3] lambda (waiting for jetson)
```

At `-vvv`, progress bars are replaced by streaming output:

```
$ forjar apply -vvv

[intel] cargo-tools (package/apply):
  + apt-get update -qq
  Hit:1 http://archive.ubuntu.com/ubuntu jammy InRelease
  Get:2 http://archive.ubuntu.com/ubuntu jammy-updates InRelease [128 kB]
  + apt-get install -y cargo-watch cargo-edit
  cargo-edit is already the newest version (0.12.2-1).
  E: Unable to locate package cargo-watch
  [FAILED] exit 100 (1.8s) — full log: state/intel/runs/r-c7d16acc/cargo-tools.apply.log
```

---

## Diagnostic Commands

```bash
forjar doctor                    # check system prerequisites
forjar doctor --machine intel    # check SSH, transport readiness
forjar plan                      # show what apply would do (no changes)
forjar plan --json               # machine-readable plan
forjar state                     # current state summary
forjar state --machine intel     # per-machine detail
forjar state --raw               # dump raw lock file
forjar logs --machine intel      # last run output
forjar logs --failed             # all recent failures with context
forjar logs --gc --dry-run       # show what log cleanup would delete
```

### `forjar doctor` Output

```
$ forjar doctor

System:
  forjar version: 0.9.0
  state directory: ./state/ (exists, writable)
  state.db: 2.3MB, schema v3, last ingest 5m ago
  run logs: 47MB across 3 machines (budget: 500MB)

Machines:
  intel:    SSH OK (0.12s), 17 resources, gen 13, 10 runs stored
  jetson:   SSH OK (0.34s), 8 resources, gen 5, 4 runs stored
  lambda:   SSH FAILED — Connection refused (10.0.2.50:22)

Tools:
  bashrs:   v0.4.0 (OK)
  blake3:   (builtin)
  docker:   24.0.7 (OK)
  pepita:   kernel 6.8.0, unshare OK, cgroups v2 OK

Issues:
  WARNING: lambda is unreachable — apply will skip lambda resources
  INFO: 2 failed runs in last 7 days (forjar logs --failed)
```

---

## Image Build Logs

Container image builds (`forjar build`) produce especially verbose output. The same run log system captures build output per layer:

```
state/images/training-image/runs/
  <run_id>/
    meta.yaml
    base-resolve.log             # base image pull output
    layer-0-python-runtime.log   # package layer build
    layer-1-ml-deps.log          # build layer (pip install — can be huge)
    layer-2-training-code.log    # file layer assembly
    manifest.log                 # OCI assembly output
    push.log                     # registry push output (if --push)
```

For `type: build` layers (pepita sandbox), the log captures the full sandbox output including compilation, downloads, and errors. This is the primary debugging surface for image build failures.

```bash
forjar logs --image training-image                          # last build
forjar logs --image training-image --layer ml-deps          # specific layer
forjar logs --image training-image --layer ml-deps --tail 50  # last 50 lines
```

---

## Implementation

### Phase 18: Observability (FJ-2301)
- [ ] Run log directory: `state/<machine>/runs/<run_id>/`
- [ ] Capture wrapper: persist `ExecOutput` to `.log` files with structured delimiters
- [ ] Script capture: save executed script as `.script` file for reproduction
- [ ] `meta.yaml` per run with resource-level exit codes and durations
- [ ] `forjar logs` command: view by machine, run, resource, or failure filter
- [ ] `forjar logs --follow` for live streaming during apply
- [ ] Retention policy: `keep_runs`, `keep_failed`, `max_log_size`, `max_total_size`
- [ ] Truncation: first N + last N bytes for oversized logs
- [ ] `forjar logs --gc` for manual cleanup
- [ ] `run_logs` table + FTS5 in state.db for searchable failure history
- [ ] Image build logs: per-layer output capture
- [ ] Log level flags: `-v`, `-vv`, `-vvv`, `--quiet`
- [ ] `-vvv` streams raw output to terminal in real-time
- [ ] `--json` structured output with `log_path` for CI artifact upload
- [ ] Exit codes: 0/1/2/3/4/10
- [ ] Progress bars to stderr
- [ ] `forjar doctor` — system + log health check
- **Deliverable**: Every script execution is captured to disk; `forjar logs --failed` shows full context for debugging; CI can upload `log_path` artifacts on failure

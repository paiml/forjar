# Query Engine Enrichments

Forjar's `state-query` command provides a SQLite FTS5-backed query engine over your infrastructure state. Beyond basic search, enrichment flags add operational intelligence without external tooling.

## Core Query

```bash
# Full-text search across resources
forjar state-query "nginx"

# Filter by resource type
forjar state-query "nginx" --resource-type package

# Output formats
forjar state-query "nginx" --json
forjar state-query "nginx" --csv
forjar state-query "nginx" --sql   # show generated SQL
```

## Enrichment Flags

### Event History (`--history`)

Shows the apply/converge/fail event timeline for a specific resource:

```bash
forjar state-query "nginx" --history
```

```
Resource  Run       Event              Timestamp             Duration
nginx-pkg run-007   resource_converged 2026-03-08T12:00:00   150ms
nginx-pkg run-006   resource_converged 2026-03-07T12:00:00   142ms
nginx-pkg run-005   resource_failed    2026-03-06T12:00:00   3200ms
```

### Timing (`--timing`)

Aggregates p50/p95 apply latency for matched resources:

```bash
forjar state-query "nginx" --timing
```

### Drift Detection (`--drift`)

Finds resources where the live hash differs from the content hash:

```bash
forjar state-query --drift
```

```
Resource    Machine  Type     Content Hash  Live Hash
nginx.conf  intel    file     abc123...     def456...
```

### Change Frequency (`--churn`)

Ranks resources by how often they change across runs:

```bash
forjar state-query --churn
```

```
Resource     Events  Distinct Runs
nginx.conf   47      12
app.service  23      8
```

### Reversibility (`--reversibility`)

Shows which resources can be safely rolled back:

```bash
forjar state-query "nginx" --reversibility
```

### Git History Fusion (`-G`)

Fuses git commit history via Reciprocal Rank Fusion (RRF) to find resources by intent — why they were created, not just what they're named:

```bash
forjar state-query "fix memory leak" -G
```

### Stack Health (`--health`)

Stack-wide health summary across all machines:

```bash
forjar state-query --health
forjar state-query --health --json
```

```
Machine      Resources  Converged  Drifted  Failed
intel        16         15         1        0
web-server   8          7          0        1
─────────────────────────────────────────────────
Total        24         22         1        1
Health: 91.7%
```

## New Enrichment Flags (FJ-2001)

### Recent Events (`--events`)

Shows recent events across all resources, not scoped to a single resource like `--history`:

```bash
# All recent events
forjar state-query --events

# Events in the last hour
forjar state-query --events --since 1h

# Events for a specific run
forjar state-query --events --run run-007
```

### Failure History (`--failures`)

Shows only failed events with exit codes and stderr tails for debugging:

```bash
# All failures
forjar state-query --failures

# Failures in the last 7 days
forjar state-query --failures --since 7d
```

```
Run       Resource   Machine  Event            Timestamp            Exit  Stderr
run-005   nginx-pkg  intel    resource_failed  2026-03-06T12:00:00  1     E: Package not found
```

### Time Filter (`--since`)

Accepts relative durations or ISO 8601 timestamps:

```bash
--since 1h              # last hour
--since 7d              # last 7 days
--since 30m             # last 30 minutes
--since 2026-03-01T00:00:00  # absolute timestamp
```

### Status Filter (`--status`)

Filter resources by convergence status:

```bash
forjar state-query "nginx" --status converged
forjar state-query --status failed
forjar state-query --status drifted --json
```

### Run Filter (`--run`)

Scope events to a specific apply run:

```bash
forjar state-query --events --run run-007
```

## Combining Flags

Flags compose naturally:

```bash
# Failed events in the last day, as JSON
forjar state-query --failures --since 1d --json

# Drifted resources with their event history
forjar state-query --drift --history

# Events for a specific run on a specific resource type
forjar state-query --events --run run-007 --resource-type package
```

## Example

```bash
cargo run --example query_enrichments
```

This demo shows relative time resolution, date calculation without chrono, and the SQL queries used internally.

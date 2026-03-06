# 01: SQLite Query Engine

> Sub-second inventory, health, drift, and history queries across the entire stack.

**Spec IDs**: FJ-2001 (foundation), FJ-2004 (enrichments) | **Parent**: [forjar-platform-spec.md](../forjar-platform-spec.md)

---

## Schema (state.db)

Modeled after pmat's `context.db` (5,584 functions, 8,015 call edges, 6MB, sub-second FTS5).

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE machines (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    hostname    TEXT,
    transport   TEXT NOT NULL DEFAULT 'local',  -- local, ssh, container, pepita
    ssh_host    TEXT,
    ssh_user    TEXT,
    ssh_port    INTEGER DEFAULT 22,
    first_seen  TEXT NOT NULL,
    last_seen   TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'active'  -- active, destroyed, unreachable
);

-- generation_num is global (per state_dir), not per-machine.
CREATE TABLE generations (
    id              INTEGER PRIMARY KEY,
    generation_num  INTEGER NOT NULL UNIQUE,
    run_id          TEXT NOT NULL,
    config_hash     TEXT NOT NULL,        -- BLAKE3 of config YAML
    git_ref         TEXT,                 -- commit SHA (NULL if dirty)
    config_snapshot TEXT,                 -- full YAML only when git_ref is NULL
    operator        TEXT,
    created_at      TEXT NOT NULL,
    parent_gen      INTEGER REFERENCES generations(id),
    action          TEXT NOT NULL DEFAULT 'apply'  -- apply, rollback, destroy, undo
);

CREATE TABLE resources (
    id              INTEGER PRIMARY KEY,
    resource_id     TEXT NOT NULL,
    machine_id      INTEGER NOT NULL REFERENCES machines(id),
    generation_id   INTEGER NOT NULL REFERENCES generations(id),
    resource_type   TEXT NOT NULL,
    status          TEXT NOT NULL,         -- converged, failed, drifted, destroyed
    state_hash      TEXT,
    content_hash    TEXT,
    live_hash       TEXT,
    applied_at      TEXT NOT NULL,
    duration_secs   REAL NOT NULL DEFAULT 0.0,
    details_json    TEXT NOT NULL DEFAULT '{}',
    path            TEXT,
    reversibility   TEXT NOT NULL DEFAULT 'reversible',
    UNIQUE(resource_id, machine_id, generation_id)
);

CREATE TABLE events (
    id          INTEGER PRIMARY KEY,
    machine_id  INTEGER NOT NULL REFERENCES machines(id),
    run_id      TEXT NOT NULL,
    event_type  TEXT NOT NULL,
    resource_id TEXT,
    ts          TEXT NOT NULL,
    action      TEXT,
    duration_secs REAL,
    hash        TEXT,
    error       TEXT,
    details_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE drift_findings (
    id              INTEGER PRIMARY KEY,
    machine_id      INTEGER NOT NULL REFERENCES machines(id),
    resource_id     TEXT NOT NULL,
    resource_type   TEXT NOT NULL,
    expected_hash   TEXT NOT NULL,
    actual_hash     TEXT NOT NULL,
    detail          TEXT NOT NULL,
    detected_at     TEXT NOT NULL,
    resolved_at     TEXT,
    resolved_by     TEXT
);

-- Derived from state/<machine>/destroy-log.jsonl (CQRS)
CREATE TABLE destroy_log (
    id              INTEGER PRIMARY KEY,
    machine_id      INTEGER NOT NULL REFERENCES machines(id),
    generation_id   INTEGER NOT NULL REFERENCES generations(id),
    resource_id     TEXT NOT NULL,
    resource_type   TEXT NOT NULL,
    pre_destroy_hash TEXT,
    pre_destroy_details TEXT,
    destroyed_at    TEXT NOT NULL,
    success         INTEGER NOT NULL DEFAULT 1,
    error           TEXT
);

-- Indexes (cover all query patterns)
CREATE INDEX idx_resources_machine ON resources(machine_id);
CREATE INDEX idx_resources_type ON resources(resource_type);
CREATE INDEX idx_resources_status ON resources(status);
CREATE INDEX idx_resources_gen ON resources(generation_id);
CREATE INDEX idx_resources_rid ON resources(resource_id);
CREATE INDEX idx_resources_path ON resources(path);
CREATE INDEX idx_events_machine ON events(machine_id);
CREATE INDEX idx_events_type ON events(event_type);
CREATE INDEX idx_events_ts ON events(ts);
CREATE INDEX idx_events_run ON events(run_id);
CREATE INDEX idx_events_resource ON events(resource_id);
CREATE INDEX idx_generations_num ON generations(generation_num DESC);
CREATE INDEX idx_generations_config ON generations(config_hash);
CREATE INDEX idx_drift_machine ON drift_findings(machine_id);
CREATE INDEX idx_drift_resolved ON drift_findings(resolved_at);
CREATE INDEX idx_destroy_machine ON destroy_log(machine_id);
CREATE INDEX idx_destroy_resource ON destroy_log(resource_id);

-- FTS5 (extract fields, never index raw JSON)
CREATE VIRTUAL TABLE resources_fts USING fts5(
    resource_id, resource_type, path, packages, content_preview,
    tokenize='porter unicode61 remove_diacritics 2'
);

CREATE VIRTUAL TABLE events_fts USING fts5(
    event_type, resource_id, error, action,
    tokenize='porter unicode61 remove_diacritics 2'
);
```

### FTS5 Field Extraction

Each resource type maps to specific FTS5 columns:

| Resource Type | `packages` | `content_preview` |
|--------------|-----------|-------------------|
| `package` | Joined package names (`curl jq tree`) | Provider name |
| `file` | — | First 200 chars of `content:` (skip leading comments), or `source:` path |
| `service` | — | Service name + `restart:` policy |
| `cron` | — | `schedule:` expression + `command:` first 100 chars |
| `mount` | — | `source:` + `target:` paths |
| `docker` | — | `image:` + `command:` |
| `gpu` | — | Backend (`nvidia`/`rocm`/`cpu`) + version |
| `image` | — | `name:tag` + layer count |
| `task` | — | `action:` + first 100 chars of `command:` |

**Rule**: Never index raw `details_json`. Extract semantic fields that users would search for. If a field is empty, the FTS5 row is still created (other columns are searchable).

### Size Estimates

| Stack | Machines | Resources | Generations (1yr) | Events (1yr) | Est. DB |
|-------|----------|-----------|-------------------|--------------|---------|
| Small (current) | 3 | ~50 | ~500 | ~10K | <5MB |
| Medium | 10 | ~200 | ~2K | ~100K | ~35MB |
| Large | 50 | ~1000 | ~10K | ~1M | ~250MB |

---

## Query CLI (forjar query)

Modeled after `pmat query` — same UX, sub-second, enrichment flags.

### Core Queries

```bash
forjar query "bash"                           # FTS5 search across all resources
forjar query "bash" --machine intel           # filter to machine
forjar query --type package                   # filter by resource type
forjar query --status drifted                 # find drifted resources
forjar query "cargo-tools" --history          # generation history for resource
forjar query --run r-c7d16accaf62             # events for a run
forjar query --since "7d" --machine intel     # last 7 days on intel
forjar query --health                         # stack-wide health summary
forjar query --drift                          # current drift findings
forjar query --drift --age ">1h"              # drift older than 1 hour
forjar query "gitconfig" --all-machines       # cross-machine search
forjar query --diff-machines intel jetson     # compare machine configs
```

### Enrichment Flags

| Flag | Description | Source |
|------|-------------|--------|
| `--history` | Generation history for matched resources | `generations` + `resources` JOIN |
| `--drift` | Drift findings | `drift_findings` table |
| `--events` | Recent events | `events` table |
| `--timing` | Duration stats (avg, p50, p95, p99) | `resources.duration_secs` |
| `--churn` | Change frequency across generations | `resources` GROUP BY |
| `--timing` sample rule | Min 5 data points for percentiles; fewer shows raw values | `resources.duration_secs` |
| `--failures` | Failure history and errors | `events WHERE type=resource_failed` |
| `-G` / `--git-history` | Fuse with git log (RRF ranking) | `git log` + RRF |
| `--destroy-log` | Destroy history | `destroy_log` table |
| `--reversibility` | Reversibility classification | `resources.reversibility` |
| `--json` / `--csv` / `--sql` | Output format | All queries |

### Output Examples

```
$ forjar query "bash" --history --timing

 RESOURCE        MACHINE  TYPE     STATUS     GEN  APPLIED              DURATION
 bash-aliases    intel    file     converged  12   2026-02-16T16:32:55  0.54s
 bashrc-hook     intel    file     converged  12   2026-02-16T16:40:20  0.27s
 bashrc-d-dir    intel    file     converged  12   2026-02-16T16:32:55  0.32s

 Timing: avg=0.37s p50=0.32s p95=0.54s  |  Churn: 2/12 gens (17%)

$ forjar query --health

 MACHINE   RESOURCES  CONVERGED  DRIFTED  FAILED  LAST APPLY            GEN
 intel     17         17         0        0       2026-02-16T16:44:39   12
 jetson    8          7          1        0       2026-03-01T09:12:00   5
 lambda    7          7          0        0       2026-03-03T13:44:15   3
 ─────────────────────────────────────────────────────────────────────────
 TOTAL     32         31         1        0       Stack health: 97%
```

### Query Pipeline

```
Input: "forjar query 'bash' --machine intel --history"
  │
  ▼ Parse
  keywords=["bash"], filters={machine:"intel"}, enrichments=[history]
  │
  ▼ FTS5 Search (<1ms)
  SELECT r.*, rank FROM resources r
  JOIN resources_fts ON r.rowid = resources_fts.rowid
  WHERE resources_fts MATCH 'bash' AND r.machine_id = ?
  ORDER BY rank LIMIT 50
  │
  ▼ Enrich (<10ms)
  JOIN generations (--history), JOIN drift_findings (--drift),
  Aggregate duration_secs (--timing), GROUP BY resource_id (--churn)
  │
  ▼ Rank & Format
  RRF fusion if multiple signals → table / JSON / CSV
```

---

## Ingest Pipeline

### Full Ingest

```
fn ingest_to_db(state_dir, db):
    for machine_dir in state_dir.read_dir():
        let machine_id = upsert_machine(machine_dir.name)
        // state.lock.yaml → resources
        let lock = parse(machine_dir / "state.lock.yaml")
        let gen_id = create_generation(machine_id, lock)
        for (rid, rl) in lock.resources:
            insert_resource(rid, rl, machine_id, gen_id)
            upsert_fts(rid, rl)  // extract packages, content_preview
        // events.jsonl → events
        for line in machine_dir / "events.jsonl":
            insert_event(parse(line), machine_id)
        // destroy-log.jsonl → destroy_log
        for line in machine_dir / "destroy-log.jsonl":
            insert_destroy_log(parse(line), machine_id)
    // generation metadata
    for gen_dir in state_dir / "generations":
        upsert_generation(parse(gen_dir / ".generation.yaml"))
    db.execute("INSERT INTO resources_fts(resources_fts) VALUES('optimize')")
```

### Incremental Ingest

```sql
CREATE TABLE ingest_cursor (
    machine_id  INTEGER PRIMARY KEY REFERENCES machines(id),
    last_event_offset INTEGER NOT NULL DEFAULT 0,
    last_lock_hash TEXT
);
```

Only re-parse if `state.lock.yaml` hash changed or `events.jsonl` grew.

### Ingest Consistency

Flat files are written in order: lock file first, then events.jsonl. A concurrent ingest may see a partial state (lock updated, events not yet appended). Mitigations:

1. **Ingest acquires `state/.ingest.lock`** (flock) before reading flat files
2. **Apply acquires the same lock** while writing state files
3. If lock is held, `forjar query` uses the existing `state.db` (stale but consistent) rather than blocking

This ensures ingest never reads a half-written state. The worst case is a slightly stale query result, not an inconsistent one.

---

## Implementation

### Phase 1: SQLite Foundation (FJ-2001)
- [x] Query result types: `QueryResult`, `QueryParams`, `QueryOutputFormat`
- [x] Health summary types: `HealthSummary`, `MachineHealthRow` with `stack_health_pct()`
- [x] `HealthSummary::format_table()` for human-readable output
- [x] `SqliteConfig` with `db_path()`, `pragma_statements()` (WAL, cache, mmap)
- [x] `SchemaV1` DDL constants: resources, generations, run_logs, FTS5, indexes
- [x] `IngestCursor` with `is_ingested()`, `mark_ingested()` for incremental ingest
- [x] `IngestResult` with Display for ingest summary
- [x] `QueryEnrichments` with 8 boolean flags and `any_enabled()`
- [x] Add `rusqlite` with `bundled-full` (includes FTS5)
- [x] `src/core/store/db.rs` — schema creation, WAL, pragma tuning
- [ ] Ingest pipeline from existing state files
- [x] Wire `forjar query` subcommand
- [ ] FTS5 search: `forjar query "bash"` → sub-100ms
- **Extends**: `src/core/store/`
- **Deliverable**: `forjar query "bash"` from real state data

### Phase 4: Query Enrichments (FJ-2004)
- [x] Timing stats types: `TimingStats::from_sorted()` with percentiles
- [x] Churn metric types: `ChurnMetric` with `churn_pct()`
- [ ] `--history`, `--drift`, `--timing`, `--churn`, `--health`
- [ ] `-G` git history fusion via RRF
- [ ] `--json`, `--csv`, `--sql` output modes
- [ ] `--destroy-log`, `--reversibility`
- **New module**: `src/cli/query.rs`
- **Deliverable**: Full pmat-style query UX

# CLI Reference

## Global Usage

```
forjar [OPTIONS] <COMMAND>
```

### Global Options

| Flag | Description |
|------|-------------|
| `-v, --verbose` | Enable verbose output (diagnostic info to stderr) |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

## Commands

### `forjar init`

Initialize a new forjar project.

```bash
forjar init [PATH]
```

| Argument | Default | Description |
|----------|---------|-------------|
| `PATH` | `.` | Directory to initialize |

Creates `forjar.yaml` and `state/` directory.

### `forjar validate`

Validate configuration without connecting to machines.

```bash
forjar validate -f <FILE>
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |

Checks:
- YAML parse validity
- Version is "1.0"
- Name is non-empty
- Resources reference valid machines
- Dependencies reference valid resources
- No circular dependencies

### `forjar plan`

Show execution plan (what would change).

```bash
forjar plan -f <FILE> [-m MACHINE] [-r RESOURCE] [--state-dir DIR] [--json]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `-m, --machine` | all | Filter to specific machine |
| `-r, --resource` | all | Filter to specific resource |
| `--state-dir` | `state` | Directory for lock files |
| `--json` | false | Output plan as JSON |

Output symbols (text mode):
- `+` Create (new resource)
- `~` Update (state changed)
- `-` Destroy (state=absent)
- ` ` No-op (unchanged)

JSON mode outputs the full `ExecutionPlan` with changes, actions, and summary counts.

### `forjar apply`

Converge infrastructure to desired state.

```bash
forjar apply -f <FILE> [-m MACHINE] [-r RESOURCE] [--force] [--dry-run] [--no-tripwire] [-p KEY=VALUE] [--auto-commit] [--state-dir DIR]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `-m, --machine` | all | Filter to specific machine |
| `-r, --resource` | all | Filter to specific resource |
| `--force` | false | Re-apply all resources (ignore cache) |
| `--dry-run` | false | Show plan without executing |
| `--no-tripwire` | false | Skip provenance event logging (faster) |
| `-p, --param` | — | Override parameter: `-p env=production` |
| `--auto-commit` | false | Git commit state after successful apply |
| `--state-dir` | `state` | Directory for lock files |

### `forjar drift`

Detect unauthorized changes (tripwire).

```bash
forjar drift -f <FILE> [-m MACHINE] [--state-dir DIR] [--tripwire] [--alert-cmd CMD] [--json]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `-m, --machine` | all | Filter to specific machine |
| `--state-dir` | `state` | Directory for lock files |
| `--tripwire` | false | Exit non-zero on any drift (for CI/cron) |
| `--alert-cmd` | — | Run command on drift detection (sets `$FORJAR_DRIFT_COUNT`) |
| `--json` | false | Output drift report as JSON |

JSON mode outputs `{ "drift_count": N, "findings": [...] }` with machine, resource, expected/actual hash for each finding.

### `forjar status`

Show current state from lock files.

```bash
forjar status [--state-dir DIR] [-m MACHINE]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--state-dir` | `state` | Directory for lock files |
| `-m, --machine` | all | Filter to specific machine |

### `forjar history`

Show apply history from event logs.

```bash
forjar history [--state-dir DIR] [-m MACHINE] [-n LIMIT] [--json]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--state-dir` | `state` | Directory for lock files |
| `-m, --machine` | all | Filter to specific machine |
| `-n, --limit` | `10` | Show last N apply events |
| `--json` | false | Output as JSON |

Reads `state/{machine}/events.jsonl` and displays apply start/complete events in reverse chronological order.

### `forjar graph`

Show resource dependency graph.

```bash
forjar graph -f <FILE> [--format mermaid|dot]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `--format` | `mermaid` | Output format: `mermaid` or `dot` |

Mermaid output can be pasted into GitHub markdown or rendered with mermaid-cli. DOT output is compatible with Graphviz.

### `forjar destroy`

Remove all managed resources (reverse teardown).

```bash
forjar destroy -f <FILE> [-m MACHINE] [--yes] [--state-dir DIR]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `-m, --machine` | all | Filter to specific machine |
| `--yes` | false | **Required** — confirm destructive operation |
| `--state-dir` | `state` | Directory for lock files |

Resources are removed in reverse topological order (dependents first). On success, state lock files are cleaned up. Requires `--yes` flag as a safety gate.

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (validation, apply failure, etc.) |
| 1 | Drift detected (with `--tripwire` flag) |

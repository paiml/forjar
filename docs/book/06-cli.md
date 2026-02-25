# CLI Reference

## Global Usage

```
forjar [OPTIONS] <COMMAND>
```

### Global Options

| Flag | Description |
|------|-------------|
| `-v, --verbose` | Enable verbose output (diagnostic info to stderr) |
| `--no-color` | Disable colored output (also honors `NO_COLOR` env) |
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
- File state is valid (file, directory, symlink, absent)
- Service state is valid (running, stopped, enabled, disabled)
- Mount state is valid (mounted, unmounted, absent)
- Docker state is valid (running, stopped, absent)
- Network protocol is valid (tcp, udp) and action is valid (allow, deny, reject)
- Cron schedule has exactly 5 fields (min hour dom mon dow)
- Symlink resources have a target field

### `forjar plan`

Show execution plan (what would change).

```bash
forjar plan -f <FILE> [-m MACHINE] [-r RESOURCE] [-t TAG] [--state-dir DIR] [--json] [--output-dir DIR]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `-m, --machine` | all | Filter to specific machine |
| `-r, --resource` | all | Filter to specific resource |
| `-t, --tag` | all | Filter to resources with this tag |
| `--state-dir` | `state` | Directory for lock files |
| `--json` | false | Output plan as JSON |
| `--output-dir` | — | Write generated scripts to directory for auditing |

Output symbols (text mode):
- `+` Create (new resource)
- `~` Update (state changed)
- `-` Destroy (state=absent)
- ` ` No-op (unchanged)

JSON mode outputs the full `ExecutionPlan` with changes, actions, and summary counts.

The `--output-dir` flag writes all generated scripts (check, apply, state_query) per resource to the specified directory. Useful for auditing, code review, and offline inspection of what forjar would execute.

### `forjar apply`

Converge infrastructure to desired state.

```bash
forjar apply -f <FILE> [-m MACHINE] [-r RESOURCE] [-t TAG] [--force] [--dry-run] [--no-tripwire] [-p KEY=VALUE] [--auto-commit] [--timeout SECS] [--state-dir DIR]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `-m, --machine` | all | Filter to specific machine |
| `-r, --resource` | all | Filter to specific resource |
| `-t, --tag` | all | Filter to resources with this tag |
| `--force` | false | Re-apply all resources (ignore cache) |
| `--dry-run` | false | Show plan without executing |
| `--no-tripwire` | false | Skip provenance event logging (faster) |
| `-p, --param` | — | Override parameter: `-p env=production` |
| `--auto-commit` | false | Git commit state after successful apply |
| `--timeout` | — | Timeout per transport operation (seconds) |
| `--state-dir` | `state` | Directory for lock files |

### `forjar drift`

Detect unauthorized changes (tripwire).

```bash
forjar drift -f <FILE> [-m MACHINE] [--state-dir DIR] [--tripwire] [--alert-cmd CMD] [--dry-run] [--json]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `-m, --machine` | all | Filter to specific machine |
| `--state-dir` | `state` | Directory for lock files |
| `--tripwire` | false | Exit non-zero on any drift (for CI/cron) |
| `--alert-cmd` | — | Run command on drift detection (sets `$FORJAR_DRIFT_COUNT`) |
| `--auto-remediate` | false | Auto-fix drift: force re-apply all resources to restore desired state |
| `--dry-run` | false | List resources that would be checked without connecting to machines |
| `--json` | false | Output drift report as JSON |

Drift detection covers **all resource types**:
- **File** resources: BLAKE3 hash of file content on disk vs lock
- **Non-file** resources (package, service, mount, user, cron, docker, network): re-runs the resource's `state_query_script` via transport and compares the BLAKE3 hash of the output against the `live_hash` stored at apply time

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

### `forjar show`

Show the fully resolved config (recipes expanded, templates resolved, secrets injected).

```bash
forjar show -f <FILE> [-r RESOURCE] [--json]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `-r, --resource` | all | Show specific resource only |
| `--json` | false | Output as JSON instead of YAML |

Useful for debugging template resolution, recipe expansion, and secrets injection without running apply.

```bash
# Show full resolved config
forjar show -f forjar.yaml

# Show single resource (e.g., from a recipe)
forjar show -f forjar.yaml -r web/site-config

# Pipe to jq for structured queries
forjar show -f forjar.yaml --json | jq '.resources | keys'
```

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

### `forjar import`

Scan a machine and generate a forjar.yaml from its current state.

```bash
forjar import --addr <HOST> [--user USER] [--name NAME] [--output FILE] [--scan types]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--addr` | — | Machine address (IP, hostname, or `localhost`) |
| `--user` | `root` | SSH user |
| `--name` | derived from addr | Machine name in config |
| `--output` | `forjar.yaml` | Output file path |
| `--scan` | `packages,files,services` | Comma-separated scan types |

Scan types: `packages` (dpkg), `services` (systemctl), `files` (/etc/*.conf), `users` (non-system users), `cron` (root crontab). The generated config should be reviewed and customized before applying.

```bash
# Import from a remote machine
forjar import --addr 10.0.0.1 --name prod-web --output prod-web.yaml

# Import just packages from localhost
forjar import --addr localhost --scan packages -v

# Import users and cron jobs
forjar import --addr localhost --scan users,cron -v
```

### `forjar diff`

Compare two state snapshots to see what changed between applies.

```bash
forjar diff <FROM> <TO> [-m MACHINE] [--json]
```

| Flag | Default | Description |
|------|---------|-------------|
| `FROM` | — | First state directory (older) |
| `TO` | — | Second state directory (newer) |
| `-m, --machine` | all | Filter to specific machine |
| `--json` | false | Output as JSON |

Output symbols (text mode):
- `+` Resource added
- `-` Resource removed
- `~` Resource changed (hash or status differs)

```bash
# Compare before/after state
forjar diff state-before/ state-after/

# JSON output for scripting
forjar diff state-v1/ state-v2/ --json

# Filter to one machine
forjar diff state-v1/ state-v2/ -m web-server
```

Useful for auditing what changed across apply runs, reviewing infrastructure changes, and debugging state drift.

### `forjar check`

Run check scripts against live machines to verify pre-conditions without applying.

```bash
forjar check -f <FILE> [-m MACHINE] [-r RESOURCE] [--tag TAG] [--json]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `-m, --machine` | all | Filter to specific machine |
| `-r, --resource` | all | Filter to specific resource |
| `--tag` | all | Filter to resources with this tag |
| `--json` | false | Output as JSON |

```bash
# Check all resources
forjar check -f forjar.yaml -v

# Check a specific resource
forjar check -f forjar.yaml -r nginx-config

# Check only web-tagged resources
forjar check -f forjar.yaml --tag web

# JSON output for CI pipelines
forjar check -f forjar.yaml --json
```

Exits non-zero if any check fails. Useful for pre-flight validation in CI/CD pipelines before running `apply`.

### `forjar fmt`

Format (normalize) a forjar.yaml config file.

```bash
forjar fmt -f <FILE> [--check]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `--check` | false | Check formatting without writing (exit non-zero if unformatted) |

```bash
# Format a config file in place
forjar fmt -f forjar.yaml

# Check formatting (useful in CI)
forjar fmt -f forjar.yaml --check
```

Parses the YAML, validates it, and re-serializes in canonical format. Idempotent — running twice produces the same output. Use `--check` in CI to enforce consistent formatting.

### `forjar lint`

Check config for best practice warnings beyond basic validation.

```bash
forjar lint -f <FILE> [--json]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `--json` | false | Output as JSON |

Detects:
- Unused machines (defined but not referenced by any resource)
- Resources without tags (when config has many resources)
- Duplicate content across file resources
- Dependencies on non-existent resources
- Package resources with empty package lists

```bash
# Lint a config file
forjar lint -f forjar.yaml

# JSON output for CI
forjar lint -f forjar.yaml --json
```

### `forjar rollback`

Rollback to a previous config revision from git history.

```bash
forjar rollback -f <FILE> [-n REVISION] [-m MACHINE] [--dry-run] [--state-dir DIR]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `-n, --revision` | `1` | How many git revisions back to rollback (HEAD~N) |
| `-m, --machine` | all | Filter to specific machine |
| `--dry-run` | false | Show what would change without applying |
| `--state-dir` | `state` | Directory for lock files |

Reads the previous `forjar.yaml` from `git show HEAD~N:<file>`, compares it to the current config, and re-applies the old config with `--force` to converge to the previous desired state.

```bash
# Preview what rollback would change
forjar rollback --dry-run

# Rollback to previous version
forjar rollback

# Rollback to 3 versions ago
forjar rollback -n 3

# Rollback specific machine only
forjar rollback -m web-server
```

### `forjar anomaly`

Detect anomalous resource behavior from event history.

```bash
forjar anomaly [--state-dir DIR] [-m MACHINE] [--min-events N] [--json]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--state-dir` | `state` | Directory for lock files |
| `-m, --machine` | all | Filter to specific machine |
| `--min-events` | `3` | Minimum events to consider (ignore resources with fewer) |
| `--json` | false | Output as JSON |

Analyzes event logs to find resources with:
- **High churn**: Abnormally high converge frequency (z-score > 1.5)
- **High failure rate**: More than 20% failures (with at least 2 failures)
- **Drift events**: Any drift detected in history

```bash
# Check all machines
forjar anomaly --state-dir state

# Lower threshold for small deployments
forjar anomaly --min-events 1

# JSON output for CI/monitoring
forjar anomaly --json

# Filter to specific machine
forjar anomaly -m web-server
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (no errors, no drift with `--tripwire`) |
| 1 | Error (validation failure, apply failure, drift detected with `--tripwire`, unformatted file with `fmt --check`) |

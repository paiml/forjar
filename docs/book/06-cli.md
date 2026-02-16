# CLI Reference

## Global Usage

```
forjar <COMMAND> [OPTIONS]
```

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
forjar plan -f <FILE> [-m MACHINE] [-r RESOURCE]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `-m, --machine` | all | Filter to specific machine |
| `-r, --resource` | all | Filter to specific resource |

Output symbols:
- `+` Create (new resource)
- `~` Update (state changed)
- `-` Destroy (state=absent)
- ` ` No-op (unchanged)

### `forjar apply`

Converge infrastructure to desired state.

```bash
forjar apply -f <FILE> [-m MACHINE] [-r RESOURCE] [--force] [--dry-run] [--state-dir DIR]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `-m, --machine` | all | Filter to specific machine |
| `-r, --resource` | all | Filter to specific resource |
| `--force` | false | Re-apply all resources (ignore cache) |
| `--dry-run` | false | Show plan without executing |
| `--state-dir` | `state` | Directory for lock files |

### `forjar drift`

Detect unauthorized changes (tripwire).

```bash
forjar drift -f <FILE> [-m MACHINE] [--state-dir DIR] [--tripwire]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `-m, --machine` | all | Filter to specific machine |
| `--state-dir` | `state` | Directory for lock files |
| `--tripwire` | false | Exit non-zero on any drift (for CI/cron) |

### `forjar status`

Show current state from lock files.

```bash
forjar status [--state-dir DIR] [-m MACHINE]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--state-dir` | `state` | Directory for lock files |
| `-m, --machine` | all | Filter to specific machine |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (validation, apply failure, etc.) |
| 1 | Drift detected (with `--tripwire` flag) |

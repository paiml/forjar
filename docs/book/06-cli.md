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

**Extended validation checks:**

```bash
# Check resource count per machine (warn if over threshold)
forjar validate -f forjar.yaml --check-resource-count 50

# Detect duplicate file paths across resources
forjar validate -f forjar.yaml --check-duplicate-paths

# JSON output for CI integration
forjar validate -f forjar.yaml --check-duplicate-paths --json

# Detect circular dependency chains
forjar validate -f forjar.yaml --check-circular-deps

# Verify all machine references in resources exist
forjar validate -f forjar.yaml --check-machine-refs

# Check provider consistency per machine
forjar validate -f forjar.yaml --check-provider-consistency

# Verify state field values for each resource type
forjar validate -f forjar.yaml --check-state-values

# Detect machines defined but not referenced by any resource
forjar validate -f forjar.yaml --check-unused-machines

# Verify resource tags follow kebab-case naming conventions
forjar validate -f forjar.yaml --check-tag-consistency

# Verify all depends_on targets reference existing resources
forjar validate -f forjar.yaml --check-dependency-exists

# Detect resources targeting the same file path on the same machine
forjar validate -f forjar.yaml --check-path-conflicts-strict

# Detect duplicate resource base names across groups
forjar validate -f forjar.yaml --check-duplicate-names

# Verify resource groups are non-empty
forjar validate -f forjar.yaml --check-resource-groups
```

### `forjar plan`

Show execution plan (what would change).

```bash
forjar plan -f <FILE> [-m MACHINE] [-r RESOURCE] [-t TAG] [--state-dir DIR] [--json] [--output-dir DIR] [--env-file PATH]
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
| `--env-file` | — | Load param overrides from external YAML file |

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
forjar apply -f <FILE> [-m MACHINE] [-r RESOURCE] [-t TAG] [--force] [--dry-run] [--check] [--no-tripwire] [-p KEY=VALUE] [--auto-commit] [--timeout SECS] [--state-dir DIR] [--json] [--report] [--env-file PATH]
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
| `--json` | false | Output apply results as JSON |
| `--env-file` | — | Load param overrides from external YAML file |
| `--report` | false | Print per-resource timing report after apply |
| `--check` | false | Run check scripts instead of apply (exit 0=converged, non-zero=needs changes) |
| `--force-unlock` | false | Remove a stale state lock and proceed (use when a previous apply was interrupted) |

State locking: When apply starts, forjar creates `state/.forjar.lock` containing the current PID. If another apply is already running against the same state directory, the command exits with an error suggesting `--force-unlock`. Stale locks (PID no longer running, detected via `/proc/<pid>`) are reported but not automatically removed. The lock file is removed on completion.

### `forjar drift`

Detect unauthorized changes (tripwire).

```bash
forjar drift -f <FILE> [-m MACHINE] [--state-dir DIR] [--tripwire] [--alert-cmd CMD] [--dry-run] [--json] [--env-file PATH]
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
| `--env-file` | — | Load param overrides from external YAML file |

Drift detection covers **all resource types**:
- **File** resources: BLAKE3 hash of file content on disk vs lock
- **Non-file** resources (package, service, mount, user, cron, docker, network): re-runs the resource's `state_query_script` via transport and compares the BLAKE3 hash of the output against the `live_hash` stored at apply time

JSON mode outputs `{ "drift_count": N, "findings": [...] }` with machine, resource, expected/actual hash for each finding.

### `forjar status`

Show current state from lock files.

```bash
forjar status [--state-dir DIR] [-m MACHINE] [--json]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--state-dir` | `state` | Directory for lock files |
| `-m, --machine` | all | Filter to specific machine |
| `--json` | false | Output status as JSON |

**Status dashboard flags:**

```bash
# Show convergence percentage per machine
forjar status --state-dir state --convergence-percentage

# Show failed resource count per machine
forjar status --state-dir state --failed-count

# Show drifted resource count per machine
forjar status --state-dir state --drift-count

# JSON output for all dashboard metrics
forjar status --state-dir state --convergence-percentage --json

# Show last apply duration per resource
forjar status --state-dir state --resource-duration

# Show which resources target each machine
forjar status -f forjar.yaml --machine-resource-map

# Fleet-wide convergence summary
forjar status --state-dir state --fleet-convergence

# Show BLAKE3 hashes per resource
forjar status --state-dir state --resource-hash

# Drift percentage per machine
forjar status --state-dir state --machine-drift-summary

# Show total apply count per machine from event log
forjar status --state-dir state --apply-history-count

# Show number of lock files across fleet
forjar status --state-dir state --lock-file-count

# Show resource type breakdown
forjar status -f forjar.yaml --resource-type-distribution

# Show time since last apply per resource
forjar status --state-dir state --resource-apply-age

# Show time since first apply per machine
forjar status --state-dir state --machine-uptime

# Show apply frequency per resource over time
forjar status --state-dir state --resource-churn

# Show timestamp of last drift detection per resource
forjar status --state-dir state --last-drift-time

# Show resource count per machine
forjar status -f forjar.yaml --machine-resource-count

# Weighted convergence score across fleet
forjar status --state-dir state --convergence-score
```

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

**Graph analysis flags:**

```bash
# Show root resources (no dependencies)
forjar graph -f forjar.yaml --root-resources

# Output as edge list (source → target pairs)
forjar graph -f forjar.yaml --edge-list

# JSON output for both
forjar graph -f forjar.yaml --root-resources --json
forjar graph -f forjar.yaml --edge-list --json

# Show disconnected subgraphs (connected components)
forjar graph -f forjar.yaml --connected-components

# Output graph as adjacency matrix
forjar graph -f forjar.yaml --adjacency-matrix

# Show longest dependency chain
forjar graph -f forjar.yaml --longest-path

# Show in-degree (dependents) per resource
forjar graph -f forjar.yaml --in-degree

# Show out-degree (dependencies) per resource
forjar graph -f forjar.yaml --out-degree

# Show graph density (edges / max-possible-edges)
forjar graph -f forjar.yaml --density

# Output resources in valid topological execution order
forjar graph -f forjar.yaml --topological-sort

# Show resources on the longest dependency chain
forjar graph -f forjar.yaml --critical-path-resources

# Show sink resources (nothing depends on them)
forjar graph -f forjar.yaml --sink-resources

# Check if dependency graph is bipartite
forjar graph -f forjar.yaml --bipartite-check
```

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
forjar lint -f <FILE> [--json] [--strict]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file path |
| `--json` | false | Output as JSON |
| `--strict` | false | Enable built-in policy rules (FJ-221) |

Detects:
- Unused machines (defined but not referenced by any resource)
- Resources without tags (when config has many resources)
- Duplicate content across file resources
- Dependencies on non-existent resources
- Package resources with empty package lists

With `--strict`, additionally enforces:
- **no_root_owner** — file resources owned by `root` must be tagged `system`
- **require_tags** — all resources must have at least one tag
- **no_privileged_containers** — container machines must not use `--privileged`
- **require_ssh_key** — non-local machines must have `ssh_key` configured

```bash
# Lint a config file
forjar lint -f forjar.yaml

# Strict mode with built-in policy rules
forjar lint -f forjar.yaml --strict

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

Analyzes event logs using ML-inspired anomaly detection (FJ-051):
- **High churn**: Abnormally high converge frequency via isolation scoring
- **High failure rate**: Disproportionate failures via isolation scoring
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

### `forjar trace`

View trace provenance data from apply runs (FJ-050).

```bash
forjar trace [--state-dir DIR] [-m MACHINE] [--json]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--state-dir` | `state` | Directory for state/lock files |
| `-m, --machine` | all | Filter to specific machine |
| `--json` | false | Output as JSON |

Shows W3C-compatible trace spans recorded during `forjar apply`, including causal ordering via Lamport logical clocks, duration, exit codes, and resource metadata.

```bash
# View all traces
forjar trace --state-dir state

# View traces for specific machine
forjar trace -m web-server

# JSON output for analysis tools
forjar trace --json | jq '.spans[] | select(.exit_code != 0)'

# Find slowest resources
forjar trace --json | jq '.spans | sort_by(-.duration_us) | .[0:5]'
```

Example output:

```
Trace: 00000000000000005ce9737d21745945  (3 spans)
  [  1] web apply:data-dir — create ok (122.1ms)
  [  2] web apply:dev-tools — create ok (19.0s)
  [  3] web apply:tool-config — create ok (165.7ms)
```

### `forjar migrate`

Migrate Docker resources to pepita kernel isolation.

```bash
forjar migrate -f <CONFIG> [-o <OUTPUT>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | required | Input config file with Docker resources |
| `-o, --output` | none | Output file for migrated config (prints to stdout if omitted) |

Converts Docker container resources to pepita kernel isolation resources:
- Docker `image` → overlay_lower hint (with warning)
- Docker `ports` → `netns: true` (network namespace isolation)
- Docker `volumes` → bind mount warnings
- Docker `state: running` → `state: present`
- Docker `state: stopped` → `state: absent`
- Docker restart/environment → warnings with migration guidance

Non-Docker resources are passed through unchanged.

```bash
# Preview migration (stdout)
forjar migrate -f docker-infra.yaml

# Write migrated config to file
forjar migrate -f docker-infra.yaml -o pepita-infra.yaml

# Review warnings, then apply
forjar migrate -f docker-infra.yaml -o new.yaml
forjar validate -f new.yaml
forjar plan -f new.yaml
```

## Workspaces

Workspaces provide isolated state directories for multi-environment workflows. Each workspace stores state in `state/<workspace>/<machine>/`.

```bash
# Create and select a workspace
forjar workspace new staging
forjar workspace new production

# List workspaces (* = active)
forjar workspace list

# Switch workspace
forjar workspace select production

# Show current workspace
forjar workspace current

# Delete a workspace
forjar workspace delete staging --yes
```

### Using workspaces with commands

Use `-w <name>` to override the active workspace:

```bash
# Plan against staging state
forjar plan -f forjar.yaml -w staging

# Apply to production
forjar apply -f forjar.yaml -w production

# Drift check for staging
forjar drift -f forjar.yaml -w staging
```

The `{{params.workspace}}` template variable is automatically injected, enabling workspace-aware configs:

```yaml
resources:
  config:
    type: file
    machine: web
    path: "/etc/app/{{params.workspace}}.conf"
    content: "env={{params.workspace}}"
```

**Workspace resolution** (first match wins):

1. `-w <name>` flag on command
2. `.forjar/workspace` file (set by `workspace select`)
3. `"default"` (no workspace isolation)

## Policy Enforcement

Define policy rules in `forjar.yaml` to enforce standards at plan time:

```yaml
policies:
  - type: require
    message: "file resources must specify owner"
    resource_type: file
    field: owner

  - type: deny
    message: "files must not be owned by root"
    resource_type: file
    condition_field: owner
    condition_value: root

  - type: warn
    message: "all resources should be tagged"
    field: tags
```

```bash
# Check policies without applying
forjar policy -f forjar.yaml

# JSON output for CI
forjar policy -f forjar.yaml --json
```

Rule types:

| Type | Behavior |
|------|----------|
| `require` | Resource must have the `field` set. Blocks apply. |
| `deny` | Blocks if `condition_field == condition_value`. |
| `warn` | Advisory only. Logged but does not block. |

Filters: `resource_type` limits to one resource type; `tag` limits to resources with a specific tag.

## Triggers

Define `triggers:` on any resource to force re-apply when a dependency changes:

```yaml
resources:
  nginx-config:
    type: file
    machine: web
    path: /etc/nginx/nginx.conf
    content: |
      server { listen 80; }

  nginx-service:
    type: service
    machine: web
    name: nginx
    depends_on: [nginx-config]
    triggers: [nginx-config]
```

When `nginx-config` converges (content changed), `nginx-service` is forced to re-apply even if its own state hasn't changed. This is the general-purpose version of the service-specific `restart_on` field — it works on **any** resource type.

Key differences:

| Feature | `depends_on` | `restart_on` | `triggers` |
|---------|-------------|-------------|------------|
| Purpose | Execution order | Service restart | Force re-apply |
| Scope | All types | Services only | All types |
| Effect | Runs first | Restarts service | Re-applies resource |

Multiple triggers are supported: `triggers: [config-a, config-b]` fires if **either** source converges.

## Notification Hooks

Configure `policy.notify` to run shell commands after apply or drift events:

```yaml
policy:
  notify:
    on_success: "curl -X POST https://hooks.slack.com/... -d '{\"text\": \"{{machine}}: {{converged}} converged\"}'"
    on_failure: "echo 'ALERT: {{machine}} failed {{failed}} resources' | mail -s 'forjar failure' ops@example.com"
    on_drift: "echo 'Drift detected on {{machine}}: {{drift_count}} resources' >> /var/log/drift.log"
```

Template variables:

| Hook | Variables |
|------|-----------|
| `on_success` | `{{machine}}`, `{{converged}}`, `{{unchanged}}`, `{{failed}}` |
| `on_failure` | `{{machine}}`, `{{converged}}`, `{{unchanged}}`, `{{failed}}` |
| `on_drift` | `{{machine}}`, `{{drift_count}}` |

Notification hooks are **advisory** — failures are logged as warnings but do not affect the exit code. Hooks run per machine after apply/drift completes.

## Data Sources

Define external data sources in the `data:` block. Values are resolved once at plan time and available as `{{data.key}}` templates:

```yaml
data:
  hostname:
    type: command
    value: "hostname -f"
  app_version:
    type: file
    value: "VERSION"
    default: "0.0.0"
  dns_addr:
    type: dns
    value: "api.example.com"
    default: "127.0.0.1"

resources:
  config:
    type: file
    machine: web
    path: /etc/app/config.yaml
    content: |
      hostname: {{data.hostname}}
      version: {{data.app_version}}
      api_addr: {{data.dns_addr}}
```

Data source types:

| Type | Behavior |
|------|----------|
| `file` | Read file contents (trimmed). Falls back to `default` if missing. |
| `command` | Run shell command, capture stdout (trimmed). Falls back to `default` on failure. |
| `dns` | Resolve hostname to IP address. Falls back to `default` on failure. |

Data sources are evaluated before template resolution, so `{{data.*}}` variables work anywhere `{{params.*}}` works.

## Environment Files

Use `--env-file` to load param overrides from an external YAML file. This enables
environment-specific configurations without modifying `forjar.yaml`:

```yaml
# envs/production.yaml
data_dir: /mnt/prod/data
log_level: warn
replicas: "3"

# envs/staging.yaml
data_dir: /tmp/staging
log_level: debug
replicas: "1"
```

```bash
# Plan with production params
forjar plan -f forjar.yaml --env-file envs/production.yaml

# Apply staging
forjar apply -f forjar.yaml --env-file envs/staging.yaml

# Drift check with production params
forjar drift -f forjar.yaml --env-file envs/production.yaml --state-dir state
```

**Param precedence** (last wins):

1. `params:` in `forjar.yaml` (base defaults)
2. `--env-file` values (environment overrides)
3. `--param KEY=VALUE` flags (CLI overrides)

## Command Cheat Sheet

Quick reference for the most common workflows:

```bash
# First time setup
forjar init my-project
cd my-project

# Edit → Validate → Plan → Apply cycle
$EDITOR forjar.yaml
forjar validate -f forjar.yaml
forjar plan -f forjar.yaml --state-dir state/
forjar apply -f forjar.yaml --state-dir state/

# Verify idempotency (should report 0 converged)
forjar apply -f forjar.yaml --state-dir state/

# Check for unauthorized changes
forjar drift -f forjar.yaml --state-dir state/

# View what happened
forjar history --state-dir state/ -n 10
forjar status --state-dir state/

# Debug a specific resource
forjar show -f forjar.yaml -r <resource-id> --json
forjar plan -f forjar.yaml --output-dir /tmp/scripts/

# Clean up / format
forjar fmt -f forjar.yaml
forjar lint -f forjar.yaml
```

## Pipeline Patterns

### CI Validation Gate

Run in CI before merging config changes:

```bash
#!/bin/bash
set -euo pipefail

# Validate config syntax and structure
forjar validate -f forjar.yaml

# Lint for style issues
forjar lint -f forjar.yaml

# Preview changes (informational)
forjar plan -f forjar.yaml --state-dir state/

# Check for formatting issues
forjar fmt -f forjar.yaml --check
```

### Production Deploy Pipeline

```bash
#!/bin/bash
set -euo pipefail

# 1. Check for drift before applying
forjar drift -f forjar.yaml --state-dir state/ --tripwire || {
    echo "Drift detected — review before deploying"
    exit 1
}

# 2. Apply changes
forjar apply -f forjar.yaml --state-dir state/

# 3. Verify state
forjar status --state-dir state/

# 4. Commit state changes
git add state/
git commit -m "forjar: deploy $(date -I)"
```

### Scheduled Drift Monitor

```bash
#!/bin/bash
# Run via cron or systemd timer

forjar drift -f forjar.yaml --state-dir state/ --tripwire \
  --alert-cmd "/opt/scripts/notify.sh" \
  --json > /var/log/forjar-drift.json 2>&1

# --tripwire exits non-zero on drift
# --alert-cmd runs notify script with $FORJAR_DRIFT_COUNT
```

## Global Flags

These flags work with all commands:

| Flag | Description |
|------|-------------|
| `-f, --file` | Path to forjar.yaml config file |
| `--state-dir` | Path to state directory (default: `state`) |
| `-m, --machine` | Filter to specific machine |
| `-r, --resource` | Filter to specific resource |
| `-t, --tag` | Filter to resources with specific tag |
| `--json` | Output in JSON format (for scripting) |
| `--verbose` | Increase output verbosity |
| `--quiet` | Suppress non-error output |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (no errors, no drift with `--tripwire`) |
| 1 | Error (validation failure, apply failure, drift detected with `--tripwire`, unformatted file with `fmt --check`) |

## Command Reference

### forjar validate

Validates configuration without touching any machine. Checks:
- YAML syntax
- Schema compliance (version, machines, resources)
- Resource type-specific field requirements
- Machine references
- Dependency graph (cycles, unknown refs, self-deps)
- Recipe expansion and input validation

```bash
# Validate a config file
forjar validate -f forjar.yaml

# Validate with verbose output (shows expanded resources)
forjar validate -f forjar.yaml --verbose

# Validate in CI (non-zero exit on failure)
forjar validate -f forjar.yaml || exit 1
```

### forjar plan

Shows what would change without applying. Compares desired state hashes against the lock file:

```bash
# Basic plan
forjar plan -f forjar.yaml --state-dir state/

# Plan specific machine
forjar plan -f forjar.yaml --state-dir state/ -m web-server

# Plan with JSON output
forjar plan -f forjar.yaml --state-dir state/ --json

# Plan only tagged resources
forjar plan -f forjar.yaml --state-dir state/ --tag critical
```

Plan output symbols:
- `+` — Resource will be created (no previous hash)
- `~` — Resource will be updated (hash changed)
- `-` — Resource will be destroyed (state: absent)
- ` ` — Resource is unchanged (hash matches)

### forjar apply

Executes the plan — converges infrastructure to desired state:

```bash
# Standard apply
forjar apply -f forjar.yaml --state-dir state/

# Force re-apply everything (ignore hash comparison)
forjar apply -f forjar.yaml --state-dir state/ --force

# Dry run — show plan only, don't execute
forjar apply -f forjar.yaml --state-dir state/ --dry-run

# Apply with timeout (per-resource)
forjar apply -f forjar.yaml --state-dir state/ --timeout 120

# Apply specific resource only
forjar apply -f forjar.yaml --state-dir state/ -r nginx-config
```

### forjar apply --report

Print per-resource timing report after apply:

```bash
forjar apply -f forjar.yaml --report
```

Shows resource ID, type, status, and duration for each resource. Combine with `--json` for machine-readable output that includes a `resource_reports` array. Apply reports are also persisted to `state/<machine>/last-apply.yaml`.

### forjar drift

Detects unauthorized changes by comparing live state to lock file:

```bash
# Basic drift check
forjar drift -f forjar.yaml --state-dir state/

# Tripwire mode (non-zero exit on drift)
forjar drift -f forjar.yaml --state-dir state/ --tripwire

# Full drift (re-query all resource types via transport)
forjar drift -f forjar.yaml --state-dir state/ --full

# Auto-remediate (re-apply drifted resources)
forjar drift -f forjar.yaml --state-dir state/ --auto-remediate

# With custom alert
forjar drift -f forjar.yaml --state-dir state/ --alert-cmd ./notify.sh
```

### forjar anomaly

Analyzes event log history for suspicious patterns:

```bash
# Run anomaly detection
forjar anomaly --state-dir state/

# JSON output for monitoring integration
forjar anomaly --state-dir state/ --json
```

### forjar history

Shows the event log for a machine:

```bash
# Last 10 events
forjar history --state-dir state/

# Last 50 events for specific machine
forjar history --state-dir state/ -m web-server -n 50

# JSON output
forjar history --state-dir state/ --json
```

### forjar graph

Generates a dependency graph in Mermaid format:

```bash
# Output Mermaid diagram
forjar graph -f forjar.yaml

# Render in terminal (requires mmdc/mermaid-cli)
forjar graph -f forjar.yaml | mmdc -o graph.png

# Breadth-first traversal order (topological BFS)
forjar graph -f forjar.yaml --breadth-first

# Depth-first traversal order
forjar graph -f forjar.yaml --depth-first

# JSON output for tooling
forjar graph -f forjar.yaml --breadth-first --json
```

### forjar validate --check-cron-syntax

Validates cron schedule expressions in all resources with a `schedule:` field:

```bash
forjar validate -f forjar.yaml --check-cron-syntax
```

Checks each cron field against its valid range (minute 0-59, hour 0-23, day 1-31, month 1-12, weekday 0-6). Reports invalid fields with the resource name.

### forjar status --resource-health / --machine-health-summary

Per-resource and per-machine health views:

```bash
# Per-resource health (converged/failed/drifted)
forjar status --state-dir state/ --resource-health

# Per-machine health summary with percentages
forjar status --state-dir state/ --machine-health-summary

# JSON output for monitoring
forjar status --state-dir state/ --machine-health-summary --json
```

### forjar apply --notify-ntfy / --only-machine

Push notifications and machine targeting:

```bash
# Send apply events to ntfy.sh topic
forjar apply -f forjar.yaml --notify-ntfy my-infra-alerts

# Apply only to a specific machine
forjar apply -f forjar.yaml --only-machine web-server
```

### forjar fmt

Formats forjar.yaml for consistent style:

```bash
# Format in-place
forjar fmt -f forjar.yaml

# Check formatting (CI mode — non-zero exit if unformatted)
forjar fmt -f forjar.yaml --check
```

## Shell Completion

### Bash

```bash
# Generate completion script
forjar completions bash > /etc/bash_completion.d/forjar

# Or in user directory
forjar completions bash > ~/.local/share/bash-completion/completions/forjar
```

### Zsh

```zsh
# Generate completion script
forjar completions zsh > ~/.zfunc/_forjar

# Add to .zshrc
fpath=(~/.zfunc $fpath)
autoload -Uz compinit && compinit
```

### Fish

```fish
forjar completions fish > ~/.config/fish/completions/forjar.fish
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `FORJAR_SECRET_*` | Secret values referenced by `{{secrets.X}}` templates |
| `FORJAR_CONFIG` | Default config file path (alternative to `-f`) |
| `FORJAR_STATE_DIR` | Default state directory (alternative to `--state-dir`) |
| `FORJAR_LOG_LEVEL` | Log verbosity: `error`, `warn`, `info`, `debug`, `trace` |
| `NO_COLOR` | Disable colored output (standard convention) |

## Command Patterns

### Pipeline Workflows

Chain commands for common workflows:

```bash
# Validate → Plan → Apply (full pipeline)
forjar validate -f forjar.yaml && \
forjar plan -f forjar.yaml --state-dir state/ && \
forjar apply -f forjar.yaml --state-dir state/

# Drift check → Auto-remediate
forjar drift -f forjar.yaml --state-dir state/ --tripwire || \
forjar apply -f forjar.yaml --state-dir state/ --force
```

### Multi-Environment Management

```bash
# Apply to staging first, then production
forjar apply -f forjar.yaml --state-dir state-staging/ -p env=staging
forjar drift -f forjar.yaml --state-dir state-staging/ --tripwire
# If staging looks good:
forjar apply -f forjar.yaml --state-dir state-production/ -p env=production
```

### Script Auditing

```bash
# Export all scripts for security review
forjar plan -f forjar.yaml --output-dir /tmp/audit/

# Review specific resource scripts
cat /tmp/audit/check_nginx-conf.sh
cat /tmp/audit/apply_nginx-conf.sh

# Run a check script manually
bash /tmp/audit/check_nginx-conf.sh && echo "Already converged" || echo "Needs apply"
```

### Selective Application

```bash
# Apply only to specific machine
forjar apply -f forjar.yaml --state-dir state/ -m web-server

# Apply only tagged resources
forjar apply -f forjar.yaml --state-dir state/ --tag critical

# Apply only specific resource
forjar apply -f forjar.yaml --state-dir state/ -r nginx-conf
```

### Import and Bootstrap

```bash
# Import existing machine state (no changes made)
forjar import -f forjar.yaml -m web-01 --state-dir state/

# Initialize a new project
forjar init ./my-infra
cd my-infra
# Edit forjar.yaml, then:
forjar validate -f forjar.yaml
forjar apply -f forjar.yaml --state-dir state/
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (validation, apply failure, etc.) |
| 2 | Drift detected (for `drift` command) |

Scripts can use exit codes for automation:

```bash
if forjar drift -f forjar.yaml --state-dir state/ 2>/dev/null; then
    echo "No drift detected"
else
    echo "Drift found — reconverging"
    forjar apply -f forjar.yaml --state-dir state/
fi
```

## bashrs Lint Integration

Starting with FJ-036, `forjar lint` includes **bashrs script diagnostics** alongside the config-level lint checks described above. When you run `forjar lint`, forjar generates all three scripts (check, apply, state_query) for every resource in the config and runs each through the bashrs linter. The results are merged into the lint report.

### What Gets Checked

For each resource in the config, forjar calls `codegen::check_script`, `codegen::apply_script`, and `codegen::state_query_script` to produce the shell scripts that would be executed during `forjar apply`. Each script is then passed to `purifier::lint_script()`, which runs the bashrs linter and returns diagnostics with severity levels and diagnostic codes.

Error-severity diagnostics are reported individually by resource and script kind. Warning-severity diagnostics are counted in aggregate. The summary line at the end shows total errors and warnings across all resources.

### Diagnostic Code Prefixes

bashrs diagnostics use prefixed codes that indicate the category of the finding:

| Prefix | Category | Meaning |
|--------|----------|---------|
| **SEC** | Security | Injection risk, unquoted variable expansion, unsafe eval patterns |
| **DET** | Determinism | Non-deterministic commands (date, random, pid) that break reproducibility |
| **IDEM** | Idempotency | Operations that create or modify without checking current state first |
| **SC** | ShellCheck | Standard ShellCheck-equivalent rules (SC2162, SC2086, etc.) |

SEC-prefixed findings identify shell injection vectors and unsafe variable handling. DET-prefixed findings flag commands whose output varies between runs, which matters for hash-based drift detection. IDEM-prefixed findings flag operations that are not idempotent, meaning running them twice may produce different results.

### Example Output

Running `forjar lint` on a config with package and file resources:

```
$ forjar lint -f forjar.yaml
  warn: bashrs: web-packages/apply [SEC002] unquoted variable: $SUDO
  warn: bashrs: deploy-user/apply [SEC002] unquoted variable: $SUDO
  warn: bashrs script lint: 0 error(s), 4 warning(s) across 6 resources

Lint: 3 warning(s)
```

In this example, the SEC002 warnings come from the `$SUDO` privilege escalation pattern used by package, user, cron, and network resource handlers. This pattern is intentional -- when `$SUDO` is empty (running as root), the empty expansion causes it to disappear cleanly. bashrs reports it as a warning rather than an error because the pattern is recognized as safe.

Handlers that produce zero-diagnostic scripts include file, directory, symlink, service, and mount. These handlers use only static, single-quoted arguments with no dynamic variable expansion.

### JSON Output

With `--json`, bashrs diagnostics appear in the `findings` array alongside config-level warnings:

```bash
forjar lint -f forjar.yaml --json
```

```json
{
  "warnings": 3,
  "findings": [
    "bashrs: web-packages/apply [SEC002] unquoted variable: $SUDO",
    "bashrs: deploy-user/apply [SEC002] unquoted variable: $SUDO",
    "bashrs script lint: 0 error(s), 4 warning(s) across 6 resources"
  ]
}
```

### CI Integration

In CI pipelines, `forjar lint` returns exit code 0 even when warnings are present (warnings are informational). To fail on any bashrs error-severity finding, combine lint with validation:

```bash
# Validate structure + lint for style and script safety
forjar validate -f forjar.yaml && forjar lint -f forjar.yaml
```

If bashrs reports Error-severity diagnostics (as opposed to warnings), those are included as individual lint findings. Since `forjar apply` also validates scripts before execution via `validate_script()`, Error-severity bashrs findings will block apply regardless, but catching them at lint time provides earlier feedback.

## Exit Code Reference

Forjar uses a minimal set of exit codes. The main binary (`src/main.rs`) dispatches to command handlers; if any handler returns `Err`, the process exits with code 1. Success always returns code 0. The `drift` command with `--tripwire` uses code 2 to signal drift detection without implying an error in the tool itself.

| Code | Meaning | Commands |
|------|---------|----------|
| 0 | Success: operation completed without errors or findings | All commands |
| 1 | Error: validation failure, apply failure, parse error, I/O error, or unformatted file (`fmt --check`) | All commands |
| 2 | Drift detected: live state does not match lock file (not an error in forjar itself) | `drift --tripwire` |

### Per-Command Details

| Command | Exit 0 | Exit 1 | Exit 2 |
|---------|--------|--------|--------|
| `init` | Project initialized | Directory creation failed | -- |
| `validate` | Config is valid | Parse error, schema violation, cycle detected | -- |
| `plan` | Plan generated | Config invalid, state directory unreadable | -- |
| `apply` | All resources converged or unchanged | Any resource failed, config invalid, transport error | -- |
| `drift` | No drift found | Config invalid, state unreadable, transport error | Drift detected (`--tripwire`) |
| `status` | Status displayed | State directory missing or unreadable | -- |
| `history` | Events displayed | Event log missing or corrupt | -- |
| `show` | Resolved config displayed | Config invalid, template resolution failure | -- |
| `graph` | Graph generated | Config invalid | -- |
| `destroy` | Resources removed | `--yes` not provided, transport error | -- |
| `import` | Config generated | Connection failed, scan error | -- |
| `diff` | Diff displayed | State directories missing or unreadable | -- |
| `check` | All checks passed | Any check failed, transport error | -- |
| `fmt` | File formatted (or already formatted) | Parse error; unformatted file with `--check` | -- |
| `lint` | Lint completed (warnings are informational) | Config invalid | -- |
| `rollback` | Rollback applied | Git history unavailable, apply failure | -- |
| `anomaly` | Analysis completed | State directory unreadable | -- |

### Scripting with Exit Codes

```bash
# Gate deployment on validation + drift check
forjar validate -f forjar.yaml || exit 1
forjar drift -f forjar.yaml --tripwire
case $? in
    0) echo "Clean — proceeding with deploy" ;;
    2) echo "Drift detected — investigate before deploying"; exit 1 ;;
    *) echo "Error running drift check"; exit 1 ;;
esac
forjar apply -f forjar.yaml
```

The distinction between exit code 1 (tool error) and exit code 2 (drift signal) allows scripts to differentiate between "forjar failed to run" and "forjar ran successfully and found drift."

### `forjar mcp`

Start the MCP (Model Context Protocol) server using pforge. This enables
AI agents to manage infrastructure through the same validated pipeline.

```bash
forjar mcp
```

The server runs on stdio transport and exposes 9 tools:
`forjar_validate`, `forjar_plan`, `forjar_drift`, `forjar_lint`,
`forjar_graph`, `forjar_show`, `forjar_status`, `forjar_trace`,
`forjar_anomaly`.

Configure in your MCP client:

```json
{
  "mcpServers": {
    "forjar": {
      "command": "forjar",
      "args": ["mcp"]
    }
  }
}
```

Export tool schemas as JSON (for external consumers, IDEs, documentation):

```bash
forjar mcp --schema > docs/mcp-schema.json
```

See Architecture chapter for full tool reference and handler details.

### `forjar bench`

Run inline performance benchmarks that validate spec §9 targets.

```bash
forjar bench
forjar bench --iterations 10000
forjar bench --json
```

| Flag | Description |
|------|-------------|
| `--iterations N` | Iterations per benchmark (default: 1000) |
| `--json` | Output results as JSON |

Benchmarks:
- **validate** — Parse + validate a 3-machine, 20-resource config
- **plan** — Full plan pipeline: parse → DAG → diff
- **drift** — Load lock + drift detection on 100 resources
- **blake3** — BLAKE3 hash of a 4KB string

Example output:

```
Forjar Performance Benchmarks (1000 iterations)

  Operation                         Average       Target
  --------------------------------------------------------
  validate (3m, 20r)                 62.0µs       < 10ms
  plan (3m, 20r)                     84.0µs         < 2s
  drift (100 resources)             356.0µs         < 1s
  blake3 hash (4KB)                   0.5µs        < 1µs
```

### `forjar state-list`

Tabular view of all resources in state with type, status, hash prefix, and timestamp.

```bash
forjar state-list
forjar state-list --machine web01
forjar state-list --json
```

| Flag | Description |
|------|-------------|
| `--state-dir PATH` | State directory (default: `state`) |
| `--machine NAME` | Filter to specific machine |
| `--json` | Output as JSON array |

### `forjar state-mv`

Rename a resource in state without re-applying. Preserves hash and metadata.

```bash
forjar state-mv old-resource-id new-resource-id
forjar state-mv old-name new-name --machine web01
```

| Flag | Description |
|------|-------------|
| `--state-dir PATH` | State directory (default: `state`) |
| `--machine NAME` | Target specific machine |

### `forjar state-rm`

Remove a resource from state without destroying it on the machine.

```bash
forjar state-rm deprecated-resource
forjar state-rm old-config --machine web01
forjar state-rm legacy --force
```

| Flag | Description |
|------|-------------|
| `--state-dir PATH` | State directory (default: `state`) |
| `--machine NAME` | Target specific machine |
| `--force` | Skip dependency check |

### `forjar output`

Show resolved output values from the `outputs:` block in forjar.yaml.

```bash
forjar output                    # all outputs
forjar output app_url            # single key
forjar output --json             # JSON format
forjar output -f other.yaml      # different config
```

| Flag | Description |
|------|-------------|
| `-f PATH` | Path to forjar.yaml (default: `forjar.yaml`) |
| `--json` | Output as JSON |

Output values support `{{params.*}}` and `{{machine.NAME.FIELD}}` template variables, resolved at display time.

### `forjar lock`

Generate a lock file from config without applying anything to machines. Resolves templates, computes BLAKE3 hashes for all desired-state resources, and writes the lock file. Use `--verify` in CI to assert that the committed lock matches the current config.

```bash
forjar lock -f forjar.yaml                  # generate lock files
forjar lock -f forjar.yaml --verify         # verify lock matches config (exit 1 on mismatch)
forjar lock -f forjar.yaml --json           # JSON output
```

| Flag | Description |
|------|-------------|
| `-f PATH` | Path to forjar.yaml (default: `forjar.yaml`) |
| `--state-dir PATH` | State directory to write lock files into (default: `state`) |
| `--verify` | Compare computed hashes against existing lock; exit 1 if mismatch |
| `--json` | Output as JSON |

### `forjar snapshot`

Save, list, and restore named state snapshots for safe rollbacks and checkpoint-based workflows.

#### Save a snapshot

```bash
forjar snapshot save pre-upgrade
forjar snapshot save production-2024-02-26
forjar snapshot save --state-dir /var/forjar/state backup-name
```

Copies the entire `state/` directory to `state/snapshots/<name>/`, preserving lock files and event logs for later restore.

#### List available snapshots

```bash
forjar snapshot list
forjar snapshot list --json
```

Shows all snapshots with creation timestamps. Sample output:

```
Snapshots:
  pre-upgrade           created 2024-02-26 14:32:15
  production-2024-02-26 created 2024-02-26 13:45:22
```

#### Restore a snapshot

```bash
forjar snapshot restore pre-upgrade
forjar snapshot restore production-2024-02-26 --state-dir /var/forjar/state
```

Replaces the current `state/` with the saved snapshot. The previous state is preserved in `state/snapshots/_previous/` for emergency recovery.

#### Delete a snapshot

```bash
forjar snapshot delete pre-upgrade
forjar snapshot delete old-backup --force
```

Removes a snapshot. Use `--force` to skip confirmation.

| Flag | Description |
|------|-------------|
| `--state-dir PATH` | State directory (default: `state`) |
| `--json` | Output as JSON |
| `--force` | Skip confirmation prompts |

**Use case**: Before a major config change, save a snapshot. If the apply fails or causes issues, restore the snapshot to revert state without re-running the previous converge.

`forjar lock` is useful in CI pipelines where you want to pre-compute and commit the expected lock file, then verify on each run that config and lock stay in sync — without executing any apply against real machines.

### `forjar schema`

Export the JSON Schema for `forjar.yaml` to stdout. The schema describes every valid field, type, and constraint for machines, resources, and policy configuration.

```bash
forjar schema
```

No arguments or flags are required. The schema is printed as JSON to stdout.

#### Use Cases

**Save to file for IDE integration:**

```bash
forjar schema > forjar-schema.json
```

**VS Code YAML extension** — add to `.vscode/settings.json`:

```json
{
  "yaml.schemas": {
    "./forjar-schema.json": "forjar.yaml"
  }
}
```

This gives you inline validation, field completion, and hover documentation as you edit `forjar.yaml`.

**CI validation** — validate config against the schema without connecting to machines:

```bash
forjar schema > /tmp/schema.json
# Use any JSON Schema validator
```

The schema covers the complete `ForjarConfig` structure: `version`, `name`, `description`, `params`, `machines`, `resources` (all 11 types with their fields), `policy`, `recipes`, and `includes`.

### Phase 67 — Advanced Graph Analysis & Fleet Monitoring (FJ-797→FJ-804)

**Orphan Resources** (FJ-797): Detect resources not part of any dependency chain.

```bash
forjar validate -f forjar.yaml --check-orphan-resources
forjar validate -f forjar.yaml --check-orphan-resources --json
```

**Machine Architecture Validation** (FJ-801): Verify machine arch fields are valid.

```bash
forjar validate -f forjar.yaml --check-machine-arch
forjar validate -f forjar.yaml --check-machine-arch --json
```

**Strongly Connected Components** (FJ-799): Find cycles using Tarjan's SCC algorithm.

```bash
forjar graph -f forjar.yaml --strongly-connected
forjar graph -f forjar.yaml --strongly-connected --json-output
```

**Dependency Matrix CSV** (FJ-803): Export dependency matrix as CSV.

```bash
forjar graph -f forjar.yaml --dependency-matrix-csv
forjar graph -f forjar.yaml --dependency-matrix-csv --json-output
```

**Apply Success Rate** (FJ-800): Show success/failure ratio per machine.

```bash
forjar status --state-dir state --apply-success-rate
forjar status --state-dir state --apply-success-rate --json
```

**Error Rate** (FJ-802): Show error rate per machine.

```bash
forjar status --state-dir state --error-rate
forjar status --state-dir state --error-rate --json
```

**Fleet Health Summary** (FJ-804): Aggregated fleet health overview.

```bash
forjar status --state-dir state --fleet-health-summary
forjar status --state-dir state --fleet-health-summary --json
```

### Phase 68 — Fleet Intelligence & Advanced Validation (FJ-805→FJ-812)

**Validate — resource health conflicts (FJ-805)**

```bash
forjar validate -f forjar.yaml --check-resource-health-conflicts
forjar validate -f forjar.yaml --check-resource-health-conflicts --json
```

**Validate — resource overlap detection (FJ-809)**

```bash
forjar validate -f forjar.yaml --check-resource-overlap
forjar validate -f forjar.yaml --check-resource-overlap --json
```

**Status — machine convergence history (FJ-806)**

```bash
forjar status --state-dir state --machine-convergence-history
forjar status --state-dir state --machine-convergence-history --machine web
```

**Status — drift history timeline (FJ-810)**

```bash
forjar status --state-dir state --drift-history
forjar status --state-dir state --drift-history --json
```

**Status — resource failure rate (FJ-812)**

```bash
forjar status --state-dir state --resource-failure-rate
forjar status --state-dir state --resource-failure-rate --json
```

**Graph — weighted dependency edges (FJ-807)**

```bash
forjar graph -f forjar.yaml --resource-weight
forjar graph -f forjar.yaml --resource-weight --json-output
```

**Graph — dependency depth per resource (FJ-811)**

```bash
forjar graph -f forjar.yaml --dependency-depth-per-resource
forjar graph -f forjar.yaml --dependency-depth-per-resource --json-output
```

**Apply — PagerDuty notifications (FJ-808)**

```bash
forjar apply -f forjar.yaml --notify-pagerduty <ROUTING_KEY>
```

### Phase 69 — Operational Insights & Governance (FJ-813→FJ-820)

**Validate — tag convention enforcement (FJ-813)**

```bash
forjar validate -f forjar.yaml --check-resource-tags
# Tag convention issues (2):
#   config-file — no tags assigned
#   data-dir — no tags assigned
```

**Status — last apply per machine (FJ-814)**

```bash
forjar status --state-dir state/ --machine-last-apply
# Last apply per machine:
#   web-server — 2026-02-28T10:30:00Z
```

**Graph — fan-in per resource (FJ-815)**

```bash
forjar graph -f forjar.yaml --resource-fanin
# Fan-in per resource:
#   base-packages — 3 dependents
#   config-file — 0 dependents
```

**Apply — Discord webhook notifications (FJ-816)**

```bash
forjar apply -f forjar.yaml --notify-discord-webhook <WEBHOOK_URL>
```

**Validate — state consistency check (FJ-817)**

```bash
forjar validate -f forjar.yaml --check-resource-state-consistency
# All resource states are consistent with their types.
```

**Status — fleet drift summary (FJ-818)**

```bash
forjar status --state-dir state/ --fleet-drift-summary
# Fleet drift summary:
#   web-server — 1/5 drifted (20.0%)
```

**Graph — isolated subgraphs (FJ-819)**

```bash
forjar graph -f forjar.yaml --isolated-subgraphs
# Isolated subgraphs (2):
#   Subgraph 1: config-file
#   Subgraph 2: data-dir
```

**Status — resource apply duration (FJ-820)**

```bash
forjar status --state-dir state/ --resource-apply-duration
# Average apply duration per resource type:
#   Package — 2.50s
#   File — 0.15s
```

### Phase 70 — Advanced Governance & Analytics (FJ-821→FJ-828)

**Validate — dependency completeness check (FJ-821)**

```bash
forjar validate -f forjar.yaml --check-resource-dependencies-complete
# All dependency targets exist.
```

**Status — machine resource health (FJ-822)**

```bash
forjar status --state-dir state/ --machine-resource-health
# Machine resource health:
#   web-server — converged: 4, failed: 1, drifted: 0
```

**Graph — dependency chain per resource (FJ-823)**

```bash
forjar graph -f forjar.yaml --resource-dependency-chain web-server
# Dependency chain for web-server:
#   base-packages
#     system-config
```

**Apply — MS Teams webhook notifications (FJ-824)**

```bash
forjar apply -f forjar.yaml --notify-teams-webhook <WEBHOOK_URL>
```

**Validate — machine connectivity check (FJ-825)**

```bash
forjar validate -f forjar.yaml --check-machine-connectivity
# All machine addresses look valid.
```

**Status — fleet convergence trend (FJ-826)**

```bash
forjar status --state-dir state/ --fleet-convergence-trend
# Fleet convergence trend:
#   web-server — 80.0% converged
```

**Graph — bottleneck resources (FJ-827)**

```bash
forjar graph -f forjar.yaml --bottleneck-resources
# Bottleneck resources (high fan-in + fan-out):
#   base-packages — fan-in: 3, fan-out: 2
```

**Status — resource state distribution (FJ-828)**

```bash
forjar status --state-dir state/ --resource-state-distribution
# Resource state distribution:
#   CONVERGED — 12
#   FAILED — 2
#   DRIFTED — 1
```

### Phase 71 — Compliance & Observability (FJ-829→FJ-836)

**Validate — resource naming pattern (FJ-829)**

```bash
forjar validate -f forjar.yaml --check-resource-naming-pattern "app"
# Resources not matching pattern 'app' (1):
#   inference-server
```

**Status — machine apply count (FJ-830)**

```bash
forjar status --state-dir state/ --machine-apply-count
# Apply counts per machine:
#   web-01 — 5 resources tracked
#   db-01 — 3 resources tracked
```

**Graph — critical dependency path (FJ-831)**

```bash
forjar graph -f forjar.yaml --critical-dependency-path
# Critical dependency path (length 3):
#   top → mid → base
```

**Validate — resource provider support (FJ-833)**

```bash
forjar validate -f forjar.yaml --check-resource-provider-support
# All resource types are supported by their providers.
```

**Status — fleet apply history (FJ-834)**

```bash
forjar status --state-dir state/ --fleet-apply-history
# Fleet apply history (most recent):
#   web-01 / nginx-config — 2026-02-28T10:30:00Z
```

**Graph — resource depth histogram (FJ-835)**

```bash
forjar graph -f forjar.yaml --resource-depth-histogram
# Resource depth histogram:
#   depth 0 — 3 ###
#   depth 1 — 2 ##
#   depth 2 — 1 #
```

**Status — resource hash changes (FJ-836)**

```bash
forjar status --state-dir state/ --resource-hash-changes
# Resource hashes (5 tracked):
#   web-01 / nginx-config — abc123...
```

### Phase 72 — Security & Fleet Insights (FJ-837→FJ-844)

**Validate — secret references (FJ-837)**

```bash
forjar validate -f forjar.yaml --check-resource-secret-refs
# No secret reference issues found.
```

**Status — machine uptime estimate (FJ-838)**

```bash
forjar status --state-dir state/ --machine-uptime-estimate
# Machine uptime estimates (by tracked resources):
#   web-01 — 5 resources with apply history
```

**Graph — resource coupling score (FJ-839)**

```bash
forjar graph -f forjar.yaml --resource-coupling-score
# Resource coupling scores:
#   app <-> base — score 3
```

**Validate — idempotency hints (FJ-841)**

```bash
forjar validate -f forjar.yaml --check-resource-idempotency-hints
# All resources have idempotency characteristics.
```

**Status — fleet resource type breakdown (FJ-842)**

```bash
forjar status --state-dir state/ --fleet-resource-type-breakdown
# Fleet resource type breakdown:
#   Package — 8
#   File — 5
#   Service — 3
```

**Graph — resource change frequency (FJ-843)**

```bash
forjar graph -f forjar.yaml --resource-change-frequency
# Estimated change frequency (by dependency impact):
#   base — score 3
#   app — score 1
```

**Status — resource convergence time (FJ-844)**

```bash
forjar status --state-dir state/ --resource-convergence-time
# Average convergence time per resource:
#   nginx-config — 1.23s
```

### Phase 73 — Drift Intelligence & Governance (FJ-845→FJ-852)

**Validate — dependency depth limit (FJ-845)**

```bash
forjar validate -f forjar.yaml --check-resource-dependency-depth 3
# All dependency chains within limit (3).
```

**Status — machine drift age (FJ-846)**

```bash
forjar status --state-dir state/ --machine-drift-age
# Machine drift age (drifted resource count):
#   web1 — 2 drifted resources
```

**Graph — resource impact score (FJ-847)**

```bash
forjar graph -f forjar.yaml --resource-impact-score
# Resource impact scores (dependents + depth):
#   ssl-cert — score 4
#   base-packages — score 2
```

**Validate — machine affinity (FJ-849)**

```bash
forjar validate -f forjar.yaml --check-resource-machine-affinity
# All resources have valid machine affinity.
```

**Status — fleet failed resources (FJ-850)**

```bash
forjar status --state-dir state/ --fleet-failed-resources
# No failed resources across fleet.
```

**Graph — resource stability score (FJ-851)**

```bash
forjar graph -f forjar.yaml --resource-stability-score
# Resource stability scores (higher = more stable):
#   nginx-config — score 13
#   base-packages — score 10
```

**Status — resource dependency health (FJ-852)**

```bash
forjar status --state-dir state/ --resource-dependency-health
# Resource dependency health:
#   web1 / nginx — 5 converged deps
```

#### Phase 74 — Predictive Analysis & Fleet Governance

**Validate — check resource drift risk (FJ-853)**

```bash
forjar validate -f forjar.yaml --check-resource-drift-risk
# Resource drift risk scores:
#   nginx-config — risk 3 (file, 1 dependent)
#   base-packages — risk 1 (package, 0 dependents)
```

**Status — machine resource age distribution (FJ-854)**

```bash
forjar status --state-dir state/ --machine-resource-age-distribution
# Machine resource age distribution:
#   web1 — 3 resources, oldest 45d, newest 2d
```

**Graph — resource dependency fanout (FJ-855)**

```bash
forjar graph -f forjar.yaml --resource-dependency-fanout
# Resource dependency fan-out:
#   base-packages — fan-out 4
#   nginx-config — fan-out 1
```

**Apply — notify with custom headers (FJ-856)**

```bash
forjar apply -f forjar.yaml --state-dir state/ \
  --notify-custom-headers "https://hooks.example.com/forjar|Authorization:Bearer tok123|X-Source:forjar"
```

**Validate — check resource tag coverage (FJ-857)**

```bash
forjar validate -f forjar.yaml --check-resource-tag-coverage
# Tag coverage: 3/5 resources tagged (60%)
#   Missing tags: db-config, log-rotate
```

**Status — fleet convergence velocity (FJ-858)**

```bash
forjar status --state-dir state/ --fleet-convergence-velocity
# Fleet convergence velocity:
#   web1 — 100% converged
#   db1 — 80% converged
```

**Graph — resource dependency weight (FJ-859)**

```bash
forjar graph -f forjar.yaml --resource-dependency-weight
# Resource dependency weights:
#   base-packages → nginx-config — weight 2
#   nginx-config → nginx-service — weight 1
```

**Status — resource failure correlation (FJ-860)**

```bash
forjar status --state-dir state/ --resource-failure-correlation
# Resource failure correlation:
#   nginx (Package) — failed on 2 machines: web1, web2
```

#### Phase 75 — Resource Lifecycle & Operational Intelligence

**Validate — check resource lifecycle hooks (FJ-861)**

```bash
forjar validate -f forjar.yaml --check-resource-lifecycle-hooks
# All lifecycle hook references are valid.
```

**Status — machine resource churn rate (FJ-862)**

```bash
forjar status --state-dir state/ --machine-resource-churn-rate
# Machine resource churn rate:
#   web1 — 5 resources tracked
#   db1 — 3 resources tracked
```

**Graph — resource dependency bottleneck (FJ-863)**

```bash
forjar graph -f forjar.yaml --resource-dependency-bottleneck
# Dependency bottlenecks (fan-in + fan-out):
#   base-packages — fan-in 0, fan-out 3, total 3
#   nginx-config — fan-in 1, fan-out 2, total 3
```

**Apply — notify with custom JSON (FJ-864)**

```bash
forjar apply -f forjar.yaml --state-dir state/ \
  --notify-custom-json "https://hooks.example.com|{\"status\":\"{{status}}\",\"config\":\"{{config}}\"}"
```

**Validate — check resource provider version (FJ-865)**

```bash
forjar validate -f forjar.yaml --check-resource-provider-version
# All provider versions are compatible.
```

**Status — fleet resource staleness (FJ-866)**

```bash
forjar status --state-dir state/ --fleet-resource-staleness
# Fleet resource staleness (oldest first):
#   web1 / nginx — last applied 2025-01-15T10:30:00Z
#   db1 / postgres — last applied 2025-02-01T14:00:00Z
```

**Graph — resource type clustering (FJ-867)**

```bash
forjar graph -f forjar.yaml --resource-type-clustering
# Resource type clusters:
#   Package — 3 resources: nginx, curl, base-packages
#   File — 2 resources: nginx-config, app-config
#   Service — 1 resource: nginx-service
```

**Status — machine convergence trend (FJ-868)**

```bash
forjar status --state-dir state/ --machine-convergence-trend
# Machine convergence trend:
#   web1 — 5/5 converged (100.0%)
#   db1 — 2/3 converged (66.7%)
```

#### Phase 76 — Capacity Planning & Configuration Analytics

**Validate — check resource naming convention (FJ-869)**

```bash
forjar validate -f forjar.yaml --check-resource-naming-convention
# All resources follow naming conventions.
```

**Status — machine capacity utilization (FJ-870)**

```bash
forjar status --state-dir state/ --machine-capacity-utilization
# Machine capacity utilization:
#   web1 — 5 resources
#   db1 — 3 resources
```

**Graph — resource dependency cycle risk (FJ-871)**

```bash
forjar graph -f forjar.yaml --resource-dependency-cycle-risk
# Dependency cycle risks:
#   app ↔ config — mutual depth 1
```

**Apply — notify with custom filter (FJ-872)**

```bash
forjar apply -f forjar.yaml --state-dir state/ \
  --notify-custom-filter "https://hooks.example.com|type:Package,status:Failed"
```

**Validate — check resource idempotency (FJ-873)**

```bash
forjar validate -f forjar.yaml --check-resource-idempotency
# All resources appear idempotent-safe.
```

**Status — fleet configuration entropy (FJ-874)**

```bash
forjar status --state-dir state/ --fleet-configuration-entropy
# Fleet configuration entropy (8 total resources):
#   Package — 4 (50.0%)
#   File — 3 (37.5%)
#   Service — 1 (12.5%)
```

**Graph — resource impact radius (FJ-875)**

```bash
forjar graph -f forjar.yaml --resource-impact-radius
# Resource impact radius (blast radius):
#   base-packages — impact radius 3
#   nginx-config — impact radius 1
#   nginx-service — impact radius 0
```

**Status — machine resource freshness (FJ-876)**

```bash
forjar status --state-dir state/ --machine-resource-freshness
# Machine resource freshness (oldest first):
#   web1 / nginx-config — last applied 2025-01-15T10:30:00Z
#   web1 / nginx — last applied 2025-02-01T14:00:00Z
```

#### Phase 77 — Operational Maturity & Compliance Automation

**Validate — check resource documentation (FJ-877)**

```bash
forjar validate -f forjar.yaml --check-resource-documentation
# Resources missing documentation:
#   data-dir
#   dev-tools
```

**Status — machine error budget (FJ-878)**

```bash
forjar status --state-dir state/ --machine-error-budget
# Machine error budget (failed / total):
#   web1 — 1/5 failed (20.0% error budget consumed)
```

**Graph — resource dependency health map (FJ-879)**

```bash
forjar graph -f forjar.yaml --resource-dependency-health-map
# Dependency health map:
#   base (no dependencies)
#   app → base
```

**Validate — check resource ownership (FJ-881)**

```bash
forjar validate -f forjar.yaml --check-resource-ownership
# Resources missing ownership (no tags or resource_group):
#   data-dir
#   dev-tools
```

**Status — fleet compliance score (FJ-882)**

```bash
forjar status --state-dir state/ --fleet-compliance-score
# Fleet compliance score: 80.0% (4/5 resources converged)
```

**Graph — resource change propagation (FJ-883)**

```bash
forjar graph -f forjar.yaml --resource-change-propagation
# Change propagation analysis (resources by impact depth):
#   base — propagation depth 2
#   middleware — propagation depth 1
```

**Status — machine mean time to recovery (FJ-884)**

```bash
forjar status --state-dir state/ --machine-mean-time-to-recovery
# Machine mean time to recovery:
#   web1 — event data present
```

#### Phase 78 — Automation Intelligence & Fleet Optimization

**Validate — check resource secret exposure (FJ-885)**

```bash
forjar validate -f examples/dogfood-packages.yaml --check-resource-secret-exposure
# No secret exposures detected.

forjar validate -f examples/dogfood-packages.yaml --check-resource-secret-exposure --json
# {"secret_exposures":[]}
```

**Status — machine resource dependency health (FJ-886)**

```bash
forjar status --state-dir state/ --machine-resource-dependency-health
# Machine resource dependency health:
#   web1 — 3/4 healthy
```

**Graph — resource dependency depth analysis (FJ-887)**

```bash
forjar graph -f examples/dogfood-packages.yaml --resource-dependency-depth-analysis
# Dependency depth analysis (deepest first):
#   tool-config — depth 1
#   data-dir — depth 0
#   dev-tools — depth 0
```

**Validate — check resource tag standards (FJ-889)**

```bash
forjar validate -f examples/dogfood-packages.yaml --check-resource-tag-standards
# All resource tags follow naming standards.
```

**Status — fleet resource type health (FJ-890)**

```bash
forjar status --state-dir state/ --fleet-resource-type-health
# Fleet resource type health:
#   File — 2/3 converged
#   Package — 5/5 converged
```

**Graph — resource dependency fan analysis (FJ-891)**

```bash
forjar graph -f examples/dogfood-packages.yaml --resource-dependency-fan-analysis
# Fan-in/fan-out analysis:
#   dev-tools — fan-in: 1, fan-out: 0
#   tool-config — fan-in: 0, fan-out: 1
#   data-dir — fan-in: 0, fan-out: 0
```

**Status — machine resource convergence rate (FJ-892)**

```bash
forjar status --state-dir state/ --machine-resource-convergence-rate
# Machine resource convergence rate:
#   web1 — 3/4 converged (75.0%)
```

#### Phase 79 — Security Hardening & Operational Insights

**Validate — check resource privilege escalation (FJ-893)**

```bash
forjar validate -f examples/dogfood-packages.yaml --check-resource-privilege-escalation
# No privilege escalation risks detected.
```

**Status — machine resource failure correlation (FJ-894)**

```bash
forjar status --state-dir state/ --machine-resource-failure-correlation
# Resource failure correlations:
#   nginx — failed on: web1, web2
```

**Graph — resource dependency isolation score (FJ-895)**

```bash
forjar graph -f examples/dogfood-packages.yaml --resource-dependency-isolation-score
# Dependency isolation scores (1.0 = fully isolated):
#   data-dir — 1.00
#   dev-tools — 0.50
#   tool-config — 0.50
```

**Validate — check resource update safety (FJ-897)**

```bash
forjar validate -f examples/dogfood-packages.yaml --check-resource-update-safety
# All resources can be safely updated.
```

**Status — fleet resource age distribution (FJ-898)**

```bash
forjar status --state-dir state/ --fleet-resource-age-distribution
# Fleet resource age distribution:
#   has_applied_at — 5 resources
#   no_applied_at — 2 resources
```

**Graph — resource dependency stability score (FJ-899)**

```bash
forjar graph -f examples/dogfood-packages.yaml --resource-dependency-stability-score
# Dependency stability scores (1.0 = most stable):
#   data-dir — 1.00
#   dev-tools — 1.00
#   tool-config — 0.50
```

**Status — machine resource rollback readiness (FJ-900)**

```bash
forjar status --state-dir state/ --machine-resource-rollback-readiness
# Machine rollback readiness:
#   web1 — partial (lock only, no snapshots)
```

#### Phase 80 — Operational Resilience & Configuration Intelligence

**Validate — check resource cross-machine consistency (FJ-901)**

```bash
forjar validate -f examples/dogfood-packages.yaml --check-resource-cross-machine-consistency
# No cross-machine inconsistencies found.
```

**Status — machine resource health trend (FJ-902)**

```bash
forjar status --state-dir state/ --machine-resource-health-trend
# Machine resource health trends:
#   web1 — current data only (no historical trend)
```

**Graph — resource dependency critical path length (FJ-903)**

```bash
forjar graph -f examples/dogfood-packages.yaml --resource-dependency-critical-path-length
# Critical path lengths (longest chain to root):
#   tool-config — 2
#   data-dir — 1
#   dev-tools — 1
```

**Validate — check resource version pinning (FJ-905)**

```bash
forjar validate -f examples/dogfood-packages.yaml --check-resource-version-pinning
# Resources without pinned versions:
#   dev-tools
```

**Status — fleet resource drift velocity (FJ-906)**

```bash
forjar status --state-dir state/ --fleet-resource-drift-velocity
# Fleet resource drift velocity:
#   web1 — 1/4 drifted (25.0%)
```

**Graph — resource dependency redundancy score (FJ-907)**

```bash
forjar graph -f examples/dogfood-packages.yaml --resource-dependency-redundancy-score
# Redundancy scores (higher = more redundant paths):
#   data-dir — 0.00
#   dev-tools — 0.00
#   tool-config — 0.00
```

**Status — machine resource apply success trend (FJ-908)**

```bash
forjar status --state-dir state/ --machine-resource-apply-success-trend
# Machine apply success trends:
#   web1 — event history available
```

#### Phase 81 — Predictive Analytics & Configuration Quality

**Validate — check resource dependency completeness (FJ-909)**

```bash
forjar validate -f examples/dogfood-packages.yaml --check-resource-dependency-completeness
# All dependency references are complete.
```

**Status — machine resource MTTR estimate (FJ-910)**

```bash
forjar status --state-dir state/ --machine-resource-mttr-estimate
# Machine MTTR estimates:
#   intel — no data
```

**Graph — resource dependency centrality score (FJ-911)**

```bash
forjar graph -f examples/dogfood-packages.yaml --resource-dependency-centrality-score
# Betweenness centrality scores:
#   data-dir — 0.000
#   dev-tools — 0.000
#   tool-config — 0.000
```

**Validate — check resource state coverage (FJ-913)**

```bash
forjar validate -f examples/dogfood-packages.yaml --check-resource-state-coverage
# Resources without explicit state:
#   dev-tools
#   tool-config
```

**Status — fleet resource convergence forecast (FJ-914)**

```bash
forjar status --state-dir state/ --fleet-resource-convergence-forecast
# No convergence forecast data available.
```

**Graph — resource dependency bridge detection (FJ-915)**

```bash
forjar graph -f examples/dogfood-packages.yaml --resource-dependency-bridge-detection
# Bridge edges (1):
#   tool-config → dev-tools
```

**Status — machine resource error budget forecast (FJ-916)**

```bash
forjar status --state-dir state/ --machine-resource-error-budget-forecast
# No error budget forecast data available.
```

**Validate — check resource rollback safety (FJ-917)**

```bash
forjar validate -f examples/dogfood-triggers.yaml --check-resource-rollback-safety
# Resources with rollback safety concerns:
#   app-service — triggers 1 other resources
#   monitoring — triggers 2 other resources
```

**Status — machine resource dependency lag (FJ-918)**

```bash
forjar status --state-dir state/ --machine-resource-dependency-lag
# No dependency lag data available.
```

**Graph — resource dependency cluster coefficient (FJ-919)**

```bash
forjar graph -f examples/dogfood-triggers.yaml --resource-dependency-cluster-coefficient
# Clustering coefficients:
#   app-config — 0.000
#   app-service — 0.000
#   monitoring — 0.000
```

**Validate — check resource config maturity (FJ-921)**

```bash
forjar validate -f examples/dogfood-tags.yaml --check-resource-config-maturity
# Resource configuration maturity scores:
#   db-config — 1/5
#   web-config — 1/5
```

**Status — fleet resource dependency lag (FJ-922)**

```bash
forjar status --state-dir state/ --fleet-resource-dependency-lag
# Fleet dependency lag: 0/0 resources converged (0.0% lagging)
```

**Graph — resource dependency modularity score (FJ-923)**

```bash
forjar graph -f examples/dogfood-triggers.yaml --resource-dependency-modularity-score
# Modularity score: 0.500
#   Community 0 — app-config, app-service, monitoring
```

**Status — machine resource config drift rate (FJ-924)**

```bash
forjar status --state-dir state/ --machine-resource-config-drift-rate
# No configuration drift rate data available.
```

#### Phase 83 — Advanced Graph Analytics & Fleet Observability (FJ-925→FJ-932)

```bash
forjar validate -f forjar.yaml --check-resource-dependency-ordering
# All resource dependencies are topologically valid.
```

```bash
forjar validate -f forjar.yaml --check-resource-tag-completeness
# Resources missing tags:
#   nginx-pkg
```

```bash
forjar status --state-dir state/ --machine-resource-convergence-lag
# No convergence lag data available.
```

```bash
forjar status --state-dir state/ --fleet-resource-convergence-lag
# Fleet convergence lag: 0 resources lagging
```

```bash
forjar status --state-dir state/ --machine-resource-dependency-depth
# No dependency depth data available.
```

```bash
forjar graph -f forjar.yaml --resource-dependency-diameter
# Graph diameter: 1
```

```bash
forjar graph -f forjar.yaml --resource-dependency-eccentricity
# Resource eccentricity:
#   app — eccentricity 1
#   base — eccentricity 0
```

#### Phase 84 — Compliance Analytics & Infrastructure Forecasting (FJ-933→FJ-940)

```bash
# FJ-933: Check resource naming standards (no spaces, lowercase start, no double underscores)
forjar validate -f forjar.yaml --check-resource-naming-standards
# All resource names follow naming conventions.

# FJ-934: Convergence velocity per machine (converged/total ratio)
forjar status --state-dir state --machine-resource-convergence-velocity
# Convergence velocity:
#   web — 0.8750
#   db — 1.0000

# FJ-935: Dependency graph edge density
forjar graph -f forjar.yaml --resource-dependency-density
# Graph density: 0.1667 (4 nodes, 1 edges)

# FJ-936: Route notifications by resource type
forjar apply -f forjar.yaml --notify-custom-routing "file:slack|package:email|service:pagerduty"

# FJ-937: Detect asymmetric dependency declarations
forjar validate -f forjar.yaml --check-resource-dependency-symmetry
# No asymmetric dependencies detected.

# FJ-938: Fleet-wide convergence velocity
forjar status --state-dir state --fleet-resource-convergence-velocity
# Fleet convergence velocity: 0.9375 (2 machines)

# FJ-939: Transitive reduction ratio in dependency graph
forjar graph -f forjar.yaml --resource-dependency-transitivity
# Transitivity: 0/3 edges redundant (ratio: 0.0000)

# FJ-940: Failure recurrence per machine
forjar status --state-dir state --machine-resource-failure-recurrence
# Failure recurrence:
#   web — 2 failed resources
```

#### Phase 85 — Advanced Compliance & Dependency Intelligence (FJ-941→FJ-948)

```bash
# FJ-941: Detect circular alias references
forjar validate -f forjar.yaml --check-resource-circular-alias
# No circular alias references detected.

# FJ-942: Drift frequency per machine
forjar status --state-dir state --machine-resource-drift-frequency
# Drift frequency:
#   web — 3 drifted resources

# FJ-943: Fan-out analysis (outgoing dependency edges)
forjar graph -f forjar.yaml --resource-dependency-fan-out
# Fan-out analysis (max: 3):
#   app — 3 outgoing
#   base — 0 outgoing

# FJ-944: Deduplicate notifications within a time window
forjar apply -f forjar.yaml --notify-custom-dedup-window "https://hooks.example.com|60"

# FJ-945: Warn when dependency depth exceeds threshold
forjar validate -f forjar.yaml --check-resource-dependency-depth-limit
# All dependency chains within depth limit (5).

# FJ-946: Fleet-wide drift frequency
forjar status --state-dir state --fleet-resource-drift-frequency
# Fleet drift frequency: 5 drifted resources across 2 machines

# FJ-947: Fan-in analysis (incoming dependency edges)
forjar graph -f forjar.yaml --resource-dependency-fan-in
# Fan-in analysis (max: 2):
#   base — 2 incoming
#   app — 0 incoming

# FJ-948: Apply duration trend per machine
forjar status --state-dir state --machine-resource-apply-duration-trend
# Apply duration trends:
#   web — avg 1.3000s
#   db — avg 0.8500s
```

#### Phase 86 — Resource Lifecycle & Configuration Maturity (FJ-949→FJ-956)

```bash
# FJ-949: Detect unused parameters
forjar validate -f forjar.yaml --check-resource-unused-params
# Unused parameters:
#   base_dir

# FJ-950: Convergence streak per machine
forjar status --state-dir state --machine-resource-convergence-streak
# Convergence streaks:
#   web — 2 consecutive converged

# FJ-951: Dependency path count
forjar graph -f forjar.yaml --resource-dependency-path-count
# Total dependency paths: 1 (2 nodes)

# FJ-952: Rate-limit notifications per channel
forjar apply -f forjar.yaml --notify-custom-rate-limit "https://hooks.example.com|10/min"

# FJ-953: Machine resource balance check
forjar validate -f forjar.yaml --check-resource-machine-balance
# Resource distribution is balanced (ratio: 0.0000).

# FJ-954: Fleet-wide convergence streak
forjar status --state-dir state --fleet-resource-convergence-streak
# Fleet convergence streak avg: 2.0000 (1 machines)

# FJ-955: Articulation points in dependency graph
forjar graph -f forjar.yaml --resource-dependency-articulation-points
# No articulation points found.

# FJ-956: Error distribution per machine
forjar status --state-dir state --machine-resource-error-distribution
# Error distribution:
#   web — 1 failed, 1 drifted
```

#### Phase 87 — Configuration Drift Analytics & Dependency Health (FJ-957→FJ-964)

```bash
# FJ-957: Content hash consistency check
forjar validate -f forjar.yaml --check-resource-content-hash-consistency
# All content hashes are consistent.

# FJ-958: Drift age per machine resource
forjar status --state-dir state --machine-resource-drift-age
# Drift ages:
#   web/config — 2.00h drifted

# FJ-959: Longest dependency path (critical chain)
forjar graph -f forjar.yaml --resource-dependency-longest-path
# Longest dependency path (2 hops):
#   c → b → a

# FJ-960: Exponential backoff for notification retries
forjar apply -f forjar.yaml --notify-custom-backoff "https://hooks.example.com|exponential"

# FJ-961: Dependency reference completeness
forjar validate -f forjar.yaml --check-resource-dependency-refs
# All dependency references are valid.

# FJ-962: Fleet-wide drift age aggregation
forjar status --state-dir state --fleet-resource-drift-age
# Fleet drift age: avg 2.00h across 1 drifted resources

# FJ-963: Strongly connected components
forjar graph -f forjar.yaml --resource-dependency-strongly-connected
# No strongly connected components found (DAG is acyclic).

# FJ-964: Recovery rate per machine
forjar status --state-dir state --machine-resource-recovery-rate
# Recovery rates:
#   web — 50.0% recovered
```

### `forjar watch`

Watch a config file for changes and automatically re-plan.

```bash
forjar watch -f <FILE> [--interval N] [--state-dir DIR] [--apply --yes]
```

| Flag | Default | Description |
|------|---------|-------------|
| `-f, --file` | `forjar.yaml` | Config file to watch |
| `--interval` | `2` | Polling interval in seconds |
| `--state-dir` | `state` | Directory for lock files |
| `--apply` | false | Auto-apply on change (requires `--yes`) |
| `--yes` | false | Confirm auto-apply (must be combined with `--apply`) |

Uses filesystem polling (no inotify dependency). On each detected change, forjar re-reads the config and prints an updated plan. Press `Ctrl-C` to stop.

**Watch and plan only (safe):**

```bash
forjar watch -f forjar.yaml
```

**Custom polling interval:**

```bash
forjar watch -f forjar.yaml --interval 5
```

**Auto-apply on change (requires both flags):**

```bash
forjar watch -f forjar.yaml --apply --yes
```

Both `--apply` and `--yes` are required for auto-apply. Passing `--apply` alone will error, preventing accidental unattended applies.

# Getting Started

## Installation

Build from source (requires Rust 1.85+):

```bash
git clone https://github.com/paiml/forjar.git
cd forjar
cargo install --path .
```

Verify:

```bash
forjar --help
```

You should see forjar's 74 subcommands: `init`, `validate`, `plan`, `apply`, `drift`, `status`, `history`, `destroy`, `import`, `show`, `graph`, `check`, `diff`, `fmt`, `lint`, `rollback`, `anomaly`, `trace`, `migrate`, `mcp`, `bench`, `state-list`, `state-mv`, `state-rm`, `output`, `policy`, `workspace`, `secrets`, `doctor`, `completion`, `lock`, `snapshot`, `schema`, `watch`, `explain`, `env`, `test`, `inventory`, `retry-failed`, `rolling`, `canary`, `audit`, `plan-compact`, `compliance`, `export`, `suggest`, `compare`, `lock-prune`, `env-diff`, `template`, `lock-info`, `lock-compact`, `lock-verify`, `lock-export`, `lock-gc`, `lock-diff`, `lock-merge`, `lock-rebase`, `lock-sign`, `lock-verify-sig`, `lock-compact-all`, `lock-audit-trail`, `lock-rotate-keys`, `lock-backup`, `lock-verify-chain`, `lock-stats`, `lock-audit`, `lock-compress`, `lock-defrag`, `lock-normalize`, `lock-validate`, `lock-verify-hmac`.

## Your First Project

```bash
forjar init my-infra
cd my-infra
```

This creates:
- `forjar.yaml` — configuration file (desired state)
- `state/` — directory for lock files and event logs

## Core Concepts

Before writing your first config, here are the key ideas behind forjar:

| Concept | Meaning |
|---------|---------|
| **Desired state** | What you declare in `forjar.yaml` — forjar converges machines to match |
| **Lock file** | BLAKE3-hashed record of what was actually applied, stored in `state/` |
| **Idempotency** | Apply only runs when desired state hash differs from lock file |
| **Jidoka** | Stops on first failure per machine, preserves partial state |
| **Provenance** | Every apply is logged to `state/{machine}/events.jsonl` |
| **DAG ordering** | Resources execute in dependency order (topological sort with alphabetical tie-breaking) |
| **Drift detection** | Compare live machine state against lock files to find unauthorized changes |

## Define a Machine

Edit `forjar.yaml`:

```yaml
version: "1.0"
name: my-infra

machines:
  web-server:
    hostname: web1
    addr: 192.168.1.100
    user: deploy
    ssh_key: ~/.ssh/id_ed25519

resources:
  base-packages:
    type: package
    machine: web-server
    provider: apt
    packages: [curl, htop, git]
```

Every forjar config needs:
- **version** — always `"1.0"` for now
- **name** — a human-readable project name
- **machines** — at least one target machine
- **resources** — at least one resource to manage

## Validate Before You Apply

Always validate your config before running it against real machines:

```bash
forjar validate -f forjar.yaml
```

Output:
```
Config valid: 1 machines, 1 resources (0 warnings)
```

Validation catches problems cheaply — no SSH connections needed:
- YAML syntax errors
- Missing required fields
- Invalid resource states
- Circular dependencies
- Unknown machine references

## Preview Changes

```bash
forjar plan -f forjar.yaml --state-dir state/
```

Output:
```
Planning: my-infra (1 resources)

web-server:
  + base-packages: install curl, htop, git

Plan: 1 to add, 0 to change, 0 to destroy, 0 unchanged.
```

The `+` symbol means "create" — this resource doesn't exist in the lock file yet.

Plan symbols:
- `+` Create (new resource)
- `~` Update (desired state changed since last apply)
- `-` Destroy (resource has `state: absent`)
- ` ` Unchanged (hash matches lock file)

## Apply

```bash
forjar apply -f forjar.yaml --state-dir state/
```

Forjar will:
1. SSH to the machine using the configured key
2. Run `apt-get install -y curl htop git`
3. Record the state in `state/web-server/state.lock.yaml`
4. Append events to `state/web-server/events.jsonl`

## Verify Idempotency

Run apply again:

```bash
forjar apply -f forjar.yaml --state-dir state/
```

Output:
```
web-server: 0 converged, 1 unchanged, 0 failed (0.0s)

Apply complete: 0 converged, 1 unchanged.
```

The BLAKE3 hash of the desired state matches the lock file — nothing to do.

## Check for Drift

```bash
forjar drift -f forjar.yaml --state-dir state/
```

If someone manually changes the machine (e.g., removes a package), drift detection will flag it. Use `--tripwire` in CI to exit non-zero on drift:

```bash
forjar drift -f forjar.yaml --state-dir state/ --tripwire
```

## Adding More Resources

Here's a more complete config with multiple resource types and dependencies:

```yaml
version: "1.0"
name: web-stack

machines:
  web1:
    hostname: web1
    addr: 10.0.0.1
    user: deploy
    ssh_key: ~/.ssh/id_ed25519

resources:
  # Install packages first
  nginx-pkg:
    type: package
    machine: web1
    provider: apt
    packages: [nginx]

  # Write config file after package is installed
  site-config:
    type: file
    machine: web1
    path: /etc/nginx/sites-enabled/mysite
    content: |
      server {
        listen 80;
        server_name example.com;
        root /var/www/html;
      }
    owner: root
    group: root
    mode: "0644"
    depends_on: [nginx-pkg]

  # Start service after config is in place
  nginx-svc:
    type: service
    machine: web1
    name: nginx
    state: running
    enabled: true
    restart_on: [site-config]
    depends_on: [site-config]
```

Key patterns:
- **`depends_on`** ensures ordering: nginx-pkg installs before site-config writes, which happens before nginx-svc starts
- **`restart_on`** tells the service to restart when the config file changes
- **`owner`/`group`/`mode`** set file permissions

### Lifecycle Hooks

Any resource can declare `pre_apply` and `post_apply` shell commands that run on the target machine before and after the main apply script:

```yaml
resources:
  site-config:
    type: file
    machine: web1
    path: /etc/nginx/sites-enabled/mysite
    content: |
      server { listen 80; server_name example.com; }
    pre_apply: "cp /etc/nginx/sites-enabled/mysite /tmp/mysite.bak"
    post_apply: "systemctl reload nginx"
    depends_on: [nginx-pkg]
```

- **`pre_apply`**: Runs before the resource is applied. If it exits non-zero, the resource is skipped entirely (the main apply script does not run). Use this to back up files, check preconditions, or acquire locks.
- **`post_apply`**: Runs after a successful apply. If it exits non-zero, the resource is marked as failed. Use this to restart services, send notifications, or run smoke tests.

## Visualize Dependencies

```bash
forjar graph -f forjar.yaml
```

Output (Mermaid format):
```
graph TD
  nginx-pkg --> site-config
  site-config --> nginx-svc
```

Paste into a GitHub markdown file or render with mermaid-cli.

## Filtering

Target a specific machine, resource, or tag:

```bash
# Plan for one machine only
forjar plan -f forjar.yaml -m web1

# Apply a single resource
forjar apply -f forjar.yaml -r nginx-pkg

# Apply only resources tagged "web"
forjar apply -f forjar.yaml -t web
```

To use tags, add them to your resources:

```yaml
resources:
  nginx-pkg:
    type: package
    machine: web1
    provider: apt
    packages: [nginx]
    tags: [web, core]
```

## Dry Run

Preview exactly what would happen without touching any machine:

```bash
forjar apply -f forjar.yaml --dry-run
```

## View State

See what forjar currently knows about your machines:

```bash
# Current lock file state
forjar status --state-dir state/

# Apply history
forjar history --state-dir state/ -n 5

# Full resolved config (recipes expanded, templates resolved)
forjar show -f forjar.yaml
```

## Audit Generated Scripts

For security review, write the scripts forjar would execute to a directory:

```bash
forjar plan -f forjar.yaml --output-dir /tmp/audit/
ls /tmp/audit/
```

This writes check, apply, and state_query scripts for every resource. Review them before running apply.

## Using Parameters

Pass runtime values without hardcoding them in the config:

```yaml
version: "1.0"
name: my-app
params:
  env: staging

machines:
  app1:
    hostname: app1
    addr: 10.0.0.1
resources:
  app-config:
    type: file
    machine: app1
    path: /etc/myapp/config.yaml
    content: "environment: {{params.env}}"
```

Override at runtime:

```bash
forjar apply -f forjar.yaml -p env=production
```

## Using Secrets

Reference secrets without storing them in YAML:

```yaml
resources:
  db-config:
    type: file
    machine: app1
    path: /etc/myapp/db.conf
    content: "password={{secrets.db-password}}"
```

Set the corresponding environment variable:

```bash
export FORJAR_SECRET_DB_PASSWORD="s3cret"
forjar apply -f forjar.yaml
```

Secret key normalization: `db-password` → `FORJAR_SECRET_DB_PASSWORD` (uppercase, hyphens become underscores).

## Cross-Architecture Support

Manage mixed fleets with architecture filtering:

```yaml
machines:
  x86-box:
    hostname: x86
    addr: 10.0.0.1
    arch: x86_64
  arm-box:
    hostname: arm
    addr: 10.0.0.2
    arch: aarch64

resources:
  # Only applies to x86_64 machines
  intel-microcode:
    type: package
    machine: [x86-box, arm-box]
    provider: apt
    packages: [intel-microcode]
    arch: [x86_64]
```

The `intel-microcode` resource only applies to `x86-box` — it's silently skipped on `arm-box`.

## Container Testing

You don't need real machines to test your configs. Use container transport to test locally with Docker:

```yaml
version: "1.0"
name: dev-test

machines:
  test-box:
    hostname: test-box
    addr: container
    transport: container
    container:
      runtime: docker
      image: ubuntu:22.04
      ephemeral: true         # destroy after apply
      init: true              # PID 1 reaping

resources:
  base-packages:
    type: package
    machine: test-box
    provider: apt
    packages: [curl, htop, git]

  welcome-msg:
    type: file
    machine: test-box
    path: /etc/motd
    content: "Welcome to the test box!"
    mode: "0644"
    depends_on: [base-packages]
```

Run it:

```bash
# Validate first
forjar validate -f forjar.yaml

# Apply to a fresh container
forjar apply -f forjar.yaml --state-dir state/

# Check state
forjar status --state-dir state/

# Re-apply — should be a no-op
forjar apply -f forjar.yaml --state-dir state/
```

With `ephemeral: true`, the container is created fresh and destroyed after each apply — giving you a clean environment every time.

## What Happens Under the Hood

When you run `forjar apply`, here's exactly what happens:

```
1. Parse YAML          → Load forjar.yaml, validate schema
2. Expand Recipes      → Resolve recipe resources into concrete resources
3. Resolve Templates   → Replace {{params.X}}, {{secrets.X}}, {{machine.X.Y}}
4. Build DAG           → Topological sort with alphabetical tie-breaking
5. Plan                → Compare BLAKE3 hashes: desired vs lock file
6. Generate Scripts    → Create check/apply/state_query shell scripts
7. Execute             → Pipe scripts to bash on each machine (local/SSH/container)
8. Record State        → Atomic write to state.lock.yaml + append events.jsonl
```

### Script Generation

For every resource, forjar generates three shell scripts:

| Script | Purpose | Example |
|--------|---------|---------|
| **check** | Verify preconditions | `dpkg -s curl > /dev/null 2>&1` |
| **apply** | Converge to desired state | `apt-get install -y curl htop git` |
| **state_query** | Capture live state for drift detection | `dpkg-query -W -f='${Package}=${Version}\n' curl htop git` |

All scripts begin with `set -euo pipefail` for strict error handling, and include automatic `sudo` detection:

```bash
SUDO="" ; [ "$(id -u)" -ne 0 ] && SUDO="sudo"
```

### Transport Selection

Forjar automatically selects how to execute scripts based on machine config:

| Machine Address | Transport | Command |
|-----------------|-----------|---------|
| `container` or `transport: container` | Container | `docker exec -i <name> bash` |
| `127.0.0.1` or `localhost` | Local | `bash` |
| Any other address | SSH | `ssh user@addr bash` |

Scripts are always piped to `stdin` — never passed as command-line arguments. This avoids shell metacharacter injection and argument length limits.

## Auditing Scripts Before Apply

For security-sensitive environments, inspect the generated scripts before running them:

```bash
# Write scripts to a directory for review
forjar plan -f forjar.yaml --output-dir /tmp/audit/

# Review the generated scripts
ls /tmp/audit/
# web-server/
#   base-packages.check.sh
#   base-packages.apply.sh
#   base-packages.state_query.sh
#   site-config.check.sh
#   site-config.apply.sh
#   ...
```

Every script is a plain shell file — no hidden behavior.

## bashrs Purification Overview

Forjar does not blindly execute shell commands. Every generated script passes through
[bashrs](https://crates.io/crates/bashrs), a Rust-native shell analysis library that
parses, lints, and optionally rewrites shell code before it reaches any machine.

### Why Shell Safety Matters

Configuration management tools generate shell scripts from user-supplied YAML. Without
validation, a mistyped path, an unquoted variable, or a missing error guard can cause
silent data loss or partial convergence. bashrs closes that gap by treating generated
shell as untrusted input and proving it safe before execution.

### Three Levels of Safety

Forjar's purifier (`src/core/purifier.rs`) exposes three functions, each stricter than
the last:

| Function | What It Does | When It Fails |
|----------|-------------|---------------|
| `validate_script()` | Runs the bashrs linter; fails on Error-severity diagnostics only | Syntax errors, unsafe constructs |
| `lint_script()` | Full linter pass; returns all diagnostics (errors and warnings) | Never fails -- returns a diagnostic list |
| `purify_script()` | Parse to AST, purify (quoting, injection prevention), reformat, then validate | Parse failures, purification errors, post-purification lint errors |

During `forjar apply`, every check, apply, and state_query script is validated through
`validate_script()` before being piped to the target machine. If validation fails, the
resource is marked as failed and execution halts (Jidoka).

### Running `forjar lint` for Script Diagnostics

The `forjar lint` command runs all standard config checks *and* generates every
resource's shell scripts, feeding each through the bashrs linter:

```bash
forjar lint -f forjar.yaml
```

Sample output:

```
  warn: bashrs: base-packages/apply [SC2086] Double quote to prevent globbing
  warn: bashrs: base-packages/apply [SC2059] Use %s with printf
  warn: bashrs script lint: 0 error(s), 2 warning(s) across 3 resources
No lint errors found.
```

Warnings are informational -- they do not block apply. Errors are blockers. To get
machine-readable output, pass `--json`:

```bash
forjar lint -f forjar.yaml --json
```

This produces a JSON object with a `warnings` array and a `warnings_count` field,
suitable for CI integration.

### Inspecting Purified Output

To see what bashrs does to a generated script, audit the scripts first, then run them
through the purifier example:

```bash
# 1. Write generated scripts to disk
forjar plan -f forjar.yaml --output-dir /tmp/audit/

# 2. Inspect a specific script
cat /tmp/audit/web-server/base-packages.apply.sh
```

The generated script will already begin with `set -euo pipefail` and include the
`SUDO` auto-detection preamble. bashrs validates that these guards are syntactically
correct and that no downstream commands bypass them.

## First Drift Detection

After your first successful `forjar apply`, you have a lock file recording what was
applied. This walkthrough shows how drift detection catches unauthorized changes.

### Step 1: Start with a Clean Apply

Apply your config to establish the baseline:

```bash
forjar apply -f forjar.yaml --state-dir state/
```

Confirm everything is converged:

```bash
forjar status --state-dir state/
```

At this point, the lock file in `state/{machine}/state.lock.yaml` contains a BLAKE3
hash of every managed resource's desired state.

### Step 2: Simulate a Manual Change

Suppose someone edits a managed file directly on the target machine. For a local or
container test, you can simulate this:

```bash
# If using container transport:
docker exec test-box sh -c 'echo "rogue line" >> /etc/nginx/sites-enabled/mysite'

# If using local transport (managing localhost):
echo "rogue line" >> /etc/nginx/sites-enabled/mysite
```

This modifies a file that forjar manages, without going through `forjar apply`.

### Step 3: Run Drift Detection

```bash
forjar drift -f forjar.yaml --state-dir state/
```

Output:

```
DRIFT DETECTED: 1 finding(s)

web-server:
  site-config (file):
    expected: blake3:7f83b1657ff1fc53b...
    actual:   blake3:a9c4e2d18f03bb7e1...
    path: /etc/nginx/sites-enabled/mysite

Summary: 1 drifted, 2 unchanged.
```

Forjar re-reads the file on the target machine, computes a fresh BLAKE3 hash, and
compares it to the hash stored in the lock file. The mismatch is reported with both
hashes so you can audit exactly what changed.

### Step 4: Understand the Report

Each drift finding includes:

| Field | Meaning |
|-------|---------|
| **Resource ID** | The resource name from your config (e.g., `site-config`) |
| **Type** | Resource type (`file`, `service`, `package`, etc.) |
| **Expected hash** | BLAKE3 hash recorded during the last successful apply |
| **Actual hash** | BLAKE3 hash computed from the live machine state |
| **Details** | Resource-specific context (file path, service name, etc.) |

For non-file resources (packages, services, mounts), forjar re-runs the
`state_query_script` on the machine and hashes its output. If someone manually stopped
a service or removed a package, the query output will differ from the stored hash.

### Step 5: Resolve the Drift

You have two options:

**Option A: Re-apply to restore desired state**

```bash
forjar apply -f forjar.yaml --state-dir state/
```

This overwrites the manual change with the declared desired state. The lock file hash
is updated to match.

**Option B: Accept the change and update config**

If the manual change was intentional, update `forjar.yaml` to reflect the new desired
state, then apply:

```bash
$EDITOR forjar.yaml   # update the resource definition
forjar apply -f forjar.yaml --state-dir state/
```

### Step 6: Automate with Tripwire Mode

For CI or cron-based monitoring, use `--tripwire` to exit non-zero when drift is
found:

```bash
forjar drift -f forjar.yaml --state-dir state/ --tripwire
echo $?   # 0 = no drift, 1 = drift detected
```

This integrates into any CI pipeline or monitoring system that checks exit codes.

## Local Development Workflow

A typical development cycle:

```bash
# 1. Initialize project
forjar init my-infra && cd my-infra

# 2. Edit config
$EDITOR forjar.yaml

# 3. Validate (catches errors without SSH)
forjar validate -f forjar.yaml

# 4. Preview changes
forjar plan -f forjar.yaml --state-dir state/

# 5. Visualize dependencies
forjar graph -f forjar.yaml

# 6. Apply
forjar apply -f forjar.yaml --state-dir state/

# 7. Verify idempotency
forjar apply -f forjar.yaml --state-dir state/

# 8. Check for drift later
forjar drift -f forjar.yaml --state-dir state/

# 9. Commit state for audit trail
git add state/ forjar.yaml && git commit -m "forjar: initial apply"
```

### Lint and Format

Keep your config clean:

```bash
# Format YAML consistently
forjar fmt -f forjar.yaml

# Check for style issues and common mistakes
forjar lint -f forjar.yaml
```

### Troubleshooting a Failed Apply

When a resource fails:

```bash
# See what failed
forjar status --state-dir state/

# Check event log for error details
forjar history --state-dir state/ -n 10

# Fix the config and retry — only the failed resource re-runs
forjar apply -f forjar.yaml --state-dir state/

# Force re-apply everything if needed
forjar apply -f forjar.yaml --state-dir state/ --force
```

## Project Structure

A complete forjar project looks like this:

```
my-infra/
  forjar.yaml               # Main config (desired state)
  recipes/                   # Reusable resource patterns
    web-server.yaml
    monitoring.yaml
  state/                     # Managed by forjar (commit to git)
    forjar.lock.yaml         # Global lock summary
    web-server/
      state.lock.yaml        # Per-machine resource hashes
      events.jsonl           # Append-only audit trail
    db-server/
      state.lock.yaml
      events.jsonl
```

| File | Purpose | Git? |
|------|---------|------|
| `forjar.yaml` | Desired state declaration | Yes |
| `recipes/*.yaml` | Reusable templates | Yes |
| `state/*.lock.yaml` | Lock files (current state) | Yes |
| `state/*/events.jsonl` | Audit log | Yes |

## Multi-Machine Example

A realistic config managing a web server and database:

```yaml
version: "1.0"
name: production

params:
  env: production

machines:
  web:
    hostname: web1
    addr: 10.0.0.1
    user: deploy
    ssh_key: ~/.ssh/id_ed25519
  db:
    hostname: db1
    addr: 10.0.0.2
    user: deploy
    ssh_key: ~/.ssh/id_ed25519

resources:
  # Web server
  web-packages:
    type: package
    machine: web
    provider: apt
    packages: [nginx, certbot]

  web-config:
    type: file
    machine: web
    path: /etc/nginx/sites-enabled/app
    content: |
      server {
        listen 80;
        server_name app.example.com;
        root /var/www/app;
      }
    owner: root
    mode: "0644"
    depends_on: [web-packages]

  web-service:
    type: service
    machine: web
    name: nginx
    state: running
    enabled: true
    restart_on: [web-config]
    depends_on: [web-config]

  # Database server
  db-packages:
    type: package
    machine: db
    provider: apt
    packages: [postgresql-16]

  db-service:
    type: service
    machine: db
    name: postgresql
    state: running
    enabled: true
    depends_on: [db-packages]

  # Deploy user on both machines
  deploy-user:
    type: user
    machine: [web, db]
    name: deploy
    shell: /bin/bash
    home: /home/deploy
    ssh_authorized_keys:
      - "ssh-ed25519 AAAA... deploy@laptop"
```

Apply:

```bash
# Preview all changes
forjar plan -f forjar.yaml --state-dir state/

# Apply to web only
forjar apply -f forjar.yaml --state-dir state/ -m web

# Apply everything
forjar apply -f forjar.yaml --state-dir state/

# Check for drift across the fleet
forjar drift -f forjar.yaml --state-dir state/
```

## Understanding the Apply Lifecycle

When you run `forjar apply`, the following steps happen in order:

### 1. Parse and Validate

Config files are loaded, YAML is parsed, and structural validation runs. This catches typos, missing fields, and invalid references immediately — before any machine is touched.

### 2. Template Resolution

All `{{params.key}}`, `{{secrets.key}}`, and `{{machine.name.field}}` templates are resolved to concrete values. Missing params or secrets cause an immediate error.

### 3. Recipe Expansion

Recipe resources are replaced with their expanded child resources. Namespacing prevents ID collisions (e.g., `web/nginx-conf`). Dependencies are rewritten to use namespaced IDs.

### 4. DAG Construction

A directed acyclic graph is built from `depends_on` edges. Cycles are detected and reported. Kahn's algorithm computes a topological execution order with alphabetical tie-breaking for determinism.

### 5. Script Generation

For each resource, three shell scripts are generated:
- **check**: Determines if the resource is already in the desired state
- **apply**: Converges the resource to the desired state
- **state_query**: Captures live state for drift detection

### 6. Transport and Execution

Scripts are piped to `bash` on the target machine via the appropriate transport (local, SSH, or container exec). Check runs first — if it returns 0 (already converged), apply is skipped.

### 7. State Recording

Results are written to the state directory:
- Per-machine lock files with resource hashes and status
- Global lock file summarizing the fleet
- Event log entries for auditing

### 8. Drift Detection (Optional)

After apply, or on a schedule, `forjar drift` re-checks each resource:
- **Files**: BLAKE3 hash of actual content vs. stored hash
- **Other types**: Re-run state_query and compare output hash

## Key Concepts

### Idempotency

Every forjar operation is idempotent. Running `apply` twice with the same config produces the same result. The check script prevents redundant work — if a file already has the right content, it's not rewritten.

### Content Hashing

Forjar uses BLAKE3 hashing for all integrity checks. The `hash` field in lock files represents the desired-state hash (computed from config fields). The `content_hash` field represents the actual file content on disk.

### Transport Abstraction

All three transports (local, SSH, container) share the same interface: pipe a shell script to `bash` stdin, capture stdout/stderr/exit_code. This means any resource type works on any transport without modification.

```
Local:     bash -c 'script'
SSH:       ssh user@host bash
Container: docker exec -i name bash
```

### State as Truth

The state directory is the single source of truth for what was last applied. Without state, forjar treats every resource as new and applies everything. With state, only changed resources are applied.

## Comparison with Other Tools

### Forjar vs Ansible

| Aspect | Forjar | Ansible |
|--------|--------|---------|
| Language | Rust (single binary) | Python + deps |
| Config format | YAML | YAML + Jinja2 |
| Agent required | No (bash only) | No (Python on target) |
| State tracking | BLAKE3 hash files | No built-in state |
| Drift detection | Yes (file + service) | No native drift |
| Speed | Fast (parallel BLAKE3) | Slower (Python overhead) |

### Forjar vs Terraform

| Aspect | Forjar | Terraform |
|--------|--------|-----------|
| Focus | Machine configuration | Cloud infrastructure |
| State | Local file-based | Remote state backends |
| Resources | Packages, files, services | Cloud APIs (AWS, GCP) |
| Transport | SSH / container / local | Provider APIs |
| Language | YAML | HCL |

### Forjar vs Chef/Puppet

| Aspect | Forjar | Chef/Puppet |
|--------|--------|-------------|
| Architecture | Agentless | Agent-based |
| State | BLAKE3 hashes | Agent catalog |
| DSL | YAML | Ruby / Puppet DSL |
| Binary size | ~5 MB | 100+ MB runtime |
| Learning curve | YAML + bash | Custom DSL |

## Trace Provenance

After any apply, forjar records trace spans (W3C-compatible) showing exactly what happened and when. Use `forjar trace` to audit:

```bash
# View all traces grouped by trace_id
forjar trace --state-dir state/

# Filter to a specific machine
forjar trace --state-dir state/ -m web-server

# JSON output for SIEM integration or scripting
forjar trace --state-dir state/ --json
```

Example output:

```
Trace b3a7f9e1 (3 spans):
  clock=1 → check:nginx-pkg          127µs
  clock=2 → apply:nginx-pkg          1.2s
  clock=3 → check:site-config        89µs
```

Each span records the resource, operation (check/apply), duration, and a Lamport logical clock for causal ordering. Traces are stored in `state/{machine}/trace.jsonl` and survive across sessions.

## Migrate Docker to Pepita

If you have Docker container resources and want to migrate to native kernel isolation (pepita), use `forjar migrate`:

```bash
# Preview migration (dry run — nothing changes)
forjar migrate -f forjar.yaml

# Pipe to a new file for review
forjar migrate -f forjar.yaml > forjar-migrated.yaml
```

The migrate command converts Docker resources to pepita resources:
- `docker` → `pepita` type
- `image` → overlay filesystem config
- `ports` → network namespace rules
- `volumes` → bind mount overlays
- `environment` → preserved as-is

Non-Docker resources pass through unchanged. Review the output before replacing your config.

## Performance Benchmarking

Use `forjar bench` to verify that your setup meets the spec §9 performance targets:

```bash
# Quick benchmark (default 1000 iterations)
forjar bench

# High-precision run
forjar bench --iterations 10000

# JSON output for CI tracking
forjar bench --json
```

Example output:

```
Forjar Performance Benchmarks (1000 iterations)

  Operation                         Average       Target
  --------------------------------------------------------
  validate (3m, 20r)                 62.1µs       < 10ms
  plan (3m, 20r)                     84.0µs         < 2s
  drift (100 resources)             356.0µs         < 1s
  blake3 hash (4KB)                   1.2µs        < 1µs
```

For deeper analysis with statistical confidence intervals, use Criterion benchmarks:

```bash
cargo bench
```

## FAQ

**Q: Can forjar manage cloud resources (EC2, S3)?**
A: No. Forjar manages machine configuration, not cloud infrastructure. Use Terraform for cloud resources and forjar for what runs on those machines.

**Q: Does forjar support Windows?**
A: Not currently. Forjar generates bash scripts and targets Unix-like systems. Windows support (PowerShell generation) may be considered in a future version.

**Q: Can I use forjar with Docker containers?**
A: Yes! Container transport (`transport: container`) lets you apply resources inside Docker or Podman containers. This is ideal for testing and CI.

**Q: How does forjar handle secrets?**
A: Secrets are never stored in config files. Use `{{secrets.key}}` templates that resolve from environment variables at apply time (`FORJAR_SECRET_KEY`).

**Q: What happens if an apply fails halfway?**
A: State is recorded per-resource. Successfully applied resources are marked as converged, failed ones as failed. Re-running apply only retries failed resources.

## Testing Resources

Use `forjar test` to run check scripts and see a summary table:

```bash
forjar test -f forjar.yaml
```

Output:

```
RESOURCE               TYPE  MACHINE  STATUS  DURATION
--------------------------------------------------------------------------
nginx-pkg              package  web     pass      0.003s
nginx-conf             file     web     pass      0.002s
nginx-svc              service  web     FAIL      0.001s
  exit 1
--------------------------------------------------------------------------
2 pass, 1 fail, 0 skip (0.006s)
```

JSON output for CI: `forjar test -f forjar.yaml --json`

## Apply Timing

See where time is spent during apply with `--timing`:

```bash
forjar apply -f forjar.yaml --timing
```

Output includes a timing breakdown:

```
Timing Breakdown
----------------------------------------
  Parse + resolve           0.001s
  Apply                     2.345s
----------------------------------------
  Total                     2.346s
```

## Unified Diff in Plan

When resources are being updated, `forjar plan` shows a unified diff:

```
local:
  ~ config: update (state changed)
    ---
    - host: staging.example.com
    + host: production.example.com
    - port: 8080
    + port: 443
    ---
```

## ASCII Dependency Graph

Use `forjar graph --format ascii` for a terminal-friendly view:

```bash
forjar graph -f forjar.yaml --format ascii
```

Output:

```
Dependency Graph

  * data-dir (file, gpu-box)
  * app-config (file, gpu-box) <- [data-dir]
  * app-service (service, gpu-box) <- [app-config]

3 resources in execution order.
```

Also available: `--format mermaid` (default) and `--format dot` (Graphviz).

## Resource Groups

Organize resources into groups for selective operations:

```yaml
resources:
  web-config:
    type: file
    machine: local
    path: /etc/app/web.conf
    content: "listen 8080"
    resource_group: web
  db-config:
    type: file
    machine: local
    path: /etc/app/db.conf
    content: "port 5432"
    resource_group: database
```

Apply only a specific group:

```bash
forjar apply --group web --yes         # Only web resources
forjar test --group database           # Only database resources
```

## Strict Validation

Extended validation catches issues before apply:

```bash
forjar validate -f forjar.yaml --strict
```

Strict mode checks: file paths are absolute, template variables resolve, no circular dependencies, and depends_on targets exist.

## Apply Retry

Retry failed resources with exponential backoff (useful for transient failures):

```bash
forjar apply --yes --retry 3     # Up to 3 retries (1s, 2s, 4s backoff)
```

## History Filtering

Filter history to recent events:

```bash
forjar history --since 24h       # Last 24 hours
forjar history --since 7d        # Last 7 days
```

## Targeted Planning

Plan a single resource and its transitive dependencies:

```bash
forjar plan --target app-config  # Plans app-config + its deps only
```

## Apply Confirmation

By default, apply prompts before making changes:

```
Apply 5 change(s) (3 create, 2 update, 0 destroy)? [y/N]
```

Use `--yes` to skip the prompt (CI/automation mode).

## Doctor Auto-Fix

Auto-fix common issues:

```bash
forjar doctor --fix              # Creates state dir, removes stale locks
```

## Parallel Apply

Override the config-level `parallel_resources` policy for a single run:

```bash
forjar apply -f forjar.yaml --parallel    # Force parallel wave execution
```

## Diff Single Resource

Focus diff output on a specific resource:

```bash
forjar diff --from state-v1 --to state-v2 --resource web-config
```

## Enriched Status JSON

Pass `-f` to include resource_group, tags, and depends_on in status output:

```bash
forjar status --state-dir state --json -f forjar.yaml
```

## Dry-Run JSON Plan

Get a machine-readable plan for CI integration:

```bash
forjar apply -f forjar.yaml --dry-run --json | jq '.changes[] | .action'
```

## Graph Filtering

Filter graph output to specific machines or resource groups:

```bash
forjar graph -f forjar.yaml --machine web         # Only web machine resources
forjar graph -f forjar.yaml --group frontend       # Only frontend group
```

## Validate JSON Output

Machine-readable validation results for CI/editor integration:

```bash
forjar validate -f forjar.yaml --json --strict
```

## Structured History

JSON history with summary counts and time filtering:

```bash
forjar history --state-dir state --json --since 24h
```

## Script Metadata Headers

Exported scripts include resource metadata in comment headers:

```bash
forjar plan -f forjar.yaml --output-dir scripts/
head -6 scripts/web-cfg.apply.sh
# forjar: web-cfg (my-project)
# machine: web-server
# type: file
# group: frontend
# tags: web, critical
# depends_on: base-packages
```

## Enhanced Apply JSON (CI Pipelines)

The `apply --json` output includes project name and total duration for CI:

```bash
forjar apply -f forjar.yaml --json --yes 2>/dev/null
# { "name": "home-lab", "total_duration_seconds": 1.23, "applied": 5, ... }
```

## Enriched Plan JSON

Plan JSON now includes resource metadata — group, tags, and dependencies:

```bash
forjar plan -f forjar.yaml --json
# Each change includes: resource_group, tags, depends_on
```

## Drift JSON with Machine Count

Drift JSON output includes `machines_checked` for CI dashboards:

```bash
forjar drift -f forjar.yaml --state-dir state --json
# { "machines_checked": 3, "drifted": 1, "clean": 2, ... }
```

## Status Summary Dashboard

One-line status for monitoring dashboards:

```bash
forjar status --state-dir state --summary
# home-lab: 12 converged, 0 failed, 1 drifted
```

## Per-Resource Timeout

Override global timeout for specific long-running applies:

```bash
# Global timeout is 30s, but allow 120s per resource
forjar apply -f forjar.yaml --timeout 30 --resource-timeout 120
```

## Check JSON for CI Gates

Machine-readable check results with pass/fail summary:

```bash
forjar check -f forjar.yaml --json
# { "name": "home-lab", "all_passed": true, "total": 5, "pass": 5, "fail": 0, ... }
```

## Environment JSON Debug

Full resolved environment for debugging CI issues:

```bash
forjar env --json
# { "config_name": "home-lab", "resolved_params": { "data_dir": "/mnt/data" },
#   "machine_names": ["gpu-box"], "resource_names": ["base-packages", ...] }
```

## Explain JSON for Tooling

Machine-readable resource detail for tooling integration:

```bash
forjar explain cfg --json
# { "resource": "cfg", "type": "file", "machine": "local",
#   "transport": "local", "apply_script": "...", "check_script": "..." }
```

## Rollback on Failure

Auto-restore previous state when any resource fails during apply:

```bash
forjar apply -f forjar.yaml --rollback-on-failure
# On failure: restores last-known-good lock files automatically
```

## Strict Validation Enhancements

Lint-grade validation catches unused params, missing descriptions, and duplicate tags:

```bash
forjar validate -f forjar.yaml --strict
# WARN: unused param 'old_port' (not referenced by any resource)
# WARN: missing project description
```

## Plan Cost Estimation

Show estimated change cost before applying — weighted by resource type, with destructive action warnings:

```bash
forjar plan -f forjar.yaml --cost
# Estimated cost: 12 units (3 packages × 3, 2 files × 1, 1 service × 3)
# WARNING: High destructive cost (15 units) — review carefully
```

## Max Parallel Execution

Cap concurrent resource execution per wave to prevent resource exhaustion:

```bash
forjar apply -f forjar.yaml --max-parallel 4
# Executes at most 4 resources concurrently per wave
```

## Live Status Watch

Live-updating status dashboard that refreshes on interval:

```bash
forjar status --state-dir state --watch 5
# Refreshes every 5 seconds (Ctrl+C to stop)
```

## Webhook Notifications

POST JSON results to a webhook URL after apply completes:

```bash
forjar apply -f forjar.yaml --notify https://hooks.example.com/forjar
# POSTs: { "name": "home-lab", "total_converged": 5, "total_failed": 0, "duration_ms": 1234 }
```

## Fleet Inventory

List all machines with connection status — SSH reachability probe for remote machines:

```bash
forjar inventory -f forjar.yaml
#   ● gpu-box (lambda) [192.168.50.100] — reachable via ssh (5 resources)
#   ● local (localhost) [127.0.0.1] — reachable via local (3 resources)
#   ✗ db-box (postgres) [10.0.0.50] — unreachable via ssh (2 resources)

forjar inventory -f forjar.yaml --json
# [{ "name": "gpu-box", "status": "reachable", "transport": "ssh", "resources": 5 }, ...]
```

## Rolling Deployment

Apply to N machines at a time — stop on first failure for safe fleet-wide updates:

```bash
forjar rolling -f forjar.yaml --batch-size 2
# Rolling deploy: 6 machines in 3 batch(es) of 2
# --- Batch 1/3: web-1, web-2 ---
# --- Batch 2/3: web-3, web-4 ---
# --- Batch 3/3: web-5, web-6 ---
```

## Canary Deployment

Apply to one machine first, then roll out to the rest:

```bash
forjar canary -f forjar.yaml --machine web-1
# === Canary Phase: applying to 'web-1' ===
# ✓ Canary 'web-1' succeeded.
# === Fleet Phase: applying to 5 remaining machine(s) ===

# Auto-proceed for CI (skip confirmation):
forjar canary -f forjar.yaml --machine web-1 --auto-proceed
```

## Retry Failed Resources

Re-run only resources that failed in the last apply — no re-running converged resources:

```bash
forjar retry-failed -f forjar.yaml
# Retrying 2 failed resource(s):
#   gpu-box → cuda-driver
#   gpu-box → model-download
# ✓ Retried 2 resource(s) successfully.
```

## Dry Expand Validation

Show the fully expanded config after template resolution — debug template issues without applying:

```bash
forjar validate -f forjar.yaml --dry-expand
# Outputs complete YAML with all {{params.key}} resolved
```

## Subset Apply

Apply only resources matching a glob pattern — fine-grained targeting:

```bash
forjar apply -f forjar.yaml --subset "web-*"
# Only applies resources whose ID matches web-* (e.g., web-config, web-service)
```

## Plan What-If

Show plan with hypothetical param override — preview changes without modifying config:

```bash
forjar plan -f forjar.yaml --what-if port=9090
# [what-if] Hypothetical params: port=9090
# Shows plan as if params.port were 9090
```

## Confirm Destructive

Require explicit confirmation for destroy actions — safety gate for production:

```bash
forjar apply -f forjar.yaml --confirm-destructive
# WARNING: 2 resource(s) will be DESTROYED. Use --yes to confirm.
# Blocks without --yes flag
```

## Stale Resource Detection

Find resources not updated in N days — identify abandoned infrastructure:

```bash
forjar status --state-dir state --stale 30
#   ⚠ gpu-box → old-model (not updated in 30+ days)
# 1 stale resource(s) found
```

## Lint Auto-Fix

Auto-fix common lint issues — sort resource keys for consistency:

```bash
forjar lint -f forjar.yaml --fix
# Wrote normalized config to forjar.yaml
```

## Audit Trail

View the full audit trail from event logs — who applied what, when:

```bash
forjar audit --state-dir state
forjar audit --state-dir state --machine gpu-box -n 50
forjar audit --state-dir state --json
```

## Pre-Apply Backup

Automatically snapshot state before apply for easy rollback:

```bash
forjar apply -f forjar.yaml --backup
# Creates snapshot "pre-apply-20260226-143022" before applying
```

## Network Diagnostics

Test SSH connectivity to all machines with latency reporting:

```bash
forjar doctor --network -f forjar.yaml
forjar doctor --network -f forjar.yaml --json
```

## Compact Plan Output

One-line-per-resource plan output for large configs:

```bash
forjar plan-compact -f forjar.yaml
# + base-packages    (create)
# ~ app-config       (update)
# - old-service      (destroy)
```

## Exclude Resources

Exclude resources matching a glob pattern from apply (inverse of --subset):

```bash
forjar apply -f forjar.yaml --exclude "test-*"
forjar apply -f forjar.yaml --exclude "*-staging"
```

## Health Score

Aggregate convergence health score (0-100) from lock file state:

```bash
forjar status --health --state-dir state
forjar status --health --state-dir state --json
```

## Sequential Execution

Force sequential resource execution for debugging ordering issues:

```bash
forjar apply -f forjar.yaml --sequential
```

## Diff-Only Preview

Show what would change without generating scripts (faster than --dry-run):

```bash
forjar apply -f forjar.yaml --diff-only
```

## Compliance Checks

Validate infrastructure against policy rules — file modes, owners, service configs:

```bash
forjar compliance -f forjar.yaml
forjar compliance -f forjar.yaml --json
```

## State Export

Export state to external formats for interoperability:

```bash
forjar export --state-dir state --format csv
forjar export --state-dir state --format terraform
forjar export --state-dir state --format ansible
forjar export --state-dir state --format csv -o state.csv
```

## Slack Notifications

Post apply results to Slack via webhook:

```bash
forjar apply -f forjar.yaml --notify-slack https://hooks.slack.com/services/...
```

## Impact Analysis

Show transitive dependents of a resource (what breaks if this changes):

```bash
forjar graph --affected base-packages -f forjar.yaml
```

## Drift Details

Show detailed drift report with per-resource status:

```bash
forjar status --drift-details --state-dir state
forjar status --drift-details --state-dir state --json
```

## Cost Limit

Abort apply if too many resources would change (safety guardrail):

```bash
forjar apply -f forjar.yaml --cost-limit 10
# Error: Cost limit exceeded: 15 changes planned, limit is 10
```

## Resource History

Show change history for a specific resource across all applies:

```bash
forjar history --resource base-packages --state-dir state
forjar history --resource app-config --state-dir state --json
```

## Script Preview

Show generated scripts before execution (audit what will run):

```bash
forjar apply -f forjar.yaml --preview
```

## Config Suggestions

Analyze config and suggest improvements:

```bash
forjar suggest -f forjar.yaml
forjar suggest -f forjar.yaml --json
```

## Config Comparison

Compare two config files and show differences:

```bash
forjar compare config-v1.yaml config-v2.yaml
forjar compare prod.yaml staging.yaml --json
```

## Convergence Timeline

Show resource convergence timeline with timestamps:

```bash
forjar status --timeline --state-dir state
forjar status --timeline --state-dir state --json
```

## Output Scripts

Write generated scripts to directory for manual review:

```bash
forjar apply -f forjar.yaml --output-scripts /tmp/review
ls /tmp/review/  # base-packages.sh, app-config.sh, ...
```

## Lock Pruning

Remove lock entries for resources no longer in config:

```bash
forjar lock-prune -f forjar.yaml --state-dir state         # dry-run
forjar lock-prune -f forjar.yaml --state-dir state --yes   # actually prune
```

## Environment Comparison

Compare environments (workspaces) for cross-environment drift:

```bash
forjar env-diff staging production --state-dir state
forjar env-diff dev staging --state-dir state --json
```

## Resume Failed Apply

Resume from last failed resource instead of re-running everything:

```bash
forjar apply -f forjar.yaml --resume
```

## Template Preview

Expand a recipe template to stdout without applying:

```bash
forjar template recipes/dev-tools.yaml --var user=noah --var shell=zsh
forjar template recipes/dev-tools.yaml --var user=noah --json
```

## Changes Since Commit

Show resources changed since a git commit:

```bash
forjar status --changes-since abc123 --state-dir state
```

## Critical Path Analysis

Highlight the longest dependency chain (bottleneck identification):

```bash
forjar graph --critical-path -f forjar.yaml
```

## Status Grouping

Group status output by dimension:

```bash
forjar status --summary-by machine --state-dir state
forjar status --summary-by type --state-dir state
forjar status --summary-by status --state-dir state --json
```

## Max Failures Override

Allow N failures before stopping (override jidoka for partial deploys):

```bash
forjar apply -f forjar.yaml --max-failures 3
```

## Prometheus Metrics

Expose resource metrics in Prometheus exposition format:

```bash
forjar status --prometheus --state-dir state
# forjar_resources_total 42
# forjar_resources_converged 40
# forjar_resources_failed 1
# forjar_resources_drifted 1
```

## Lock File Info

Show lock file metadata:

```bash
forjar lock-info --state-dir state
forjar lock-info --state-dir state --json
```

## Reverse Dependency Graph

Show what depends on each resource:

```bash
forjar graph --reverse -f forjar.yaml
```

## Expired Resources

Show resources whose lock entry is older than a duration:

```bash
forjar status --expired 7d --state-dir state
forjar status --expired 24h --state-dir state --json
```

## Exhaustive Validation

Deep validation of all cross-references, machine existence, and param usage:

```bash
forjar validate --exhaustive -f forjar.yaml
forjar validate --exhaustive --json -f forjar.yaml
```

## Resource Count

Quick dashboard metric — resource count by status:

```bash
forjar status --count --state-dir state
forjar status --count --json --state-dir state
```

## Email Notifications

Send apply results via email (requires sendmail):

```bash
forjar apply --notify-email admin@example.com -f forjar.yaml
```

## Graph Depth Limit

Limit graph traversal depth for focused visualization:

```bash
forjar graph --depth 2 -f forjar.yaml
forjar graph --depth 1 --format dot -f forjar.yaml
```

## Lock Compact

Compact event logs by removing historical entries:

```bash
forjar lock-compact --state-dir state          # dry-run
forjar lock-compact --state-dir state --yes    # actually compact
forjar lock-compact --state-dir state --json   # JSON output
```

## Skip Resources

Skip a specific resource during apply:

```bash
forjar apply --skip legacy-config -f forjar.yaml
```

## Status Output Format

Choose between table, JSON, or CSV output for status:

```bash
forjar status --format table --state-dir state
forjar status --format json --state-dir state
forjar status --format csv --state-dir state
```

## Policy File Validation

Validate config against external policy rules (OPA-style YAML):

```bash
forjar validate --policy-file policy.yaml -f forjar.yaml
forjar validate --policy-file policy.yaml --json -f forjar.yaml
```

## Status Anomalies

Detect anomalous resource states (failed, drifted, missing timestamps):

```bash
forjar status --anomalies --state-dir state
forjar status --anomalies --json --state-dir state
```

## Snapshot Before Apply

Create a named snapshot before apply:

```bash
forjar apply --snapshot-before v1-backup -f forjar.yaml
```

## Clustered Graph

Group resources by machine in graph output:

```bash
forjar graph --cluster -f forjar.yaml
forjar graph --cluster --format dot -f forjar.yaml
```

## Lock Verify

Verify lock file integrity (BLAKE3 checksums):

```bash
forjar lock-verify --state-dir state
forjar lock-verify --state-dir state --json
```

## Concurrency Limit

Set a global concurrency limit across all machines:

```bash
forjar apply --concurrency 4 -f forjar.yaml
```

## Diff From Snapshot

Compare current state against a named snapshot:

```bash
forjar status --diff-from v1-backup --state-dir state
forjar status --diff-from v1-backup --json --state-dir state
```

## Webhook Before Apply

POST to a webhook URL before apply starts:

```bash
forjar apply --webhook-before https://hooks.example.com/notify -f forjar.yaml
```

## Check Connectivity

Test SSH connectivity to all machines during validation:

```bash
forjar validate --check-connectivity -f forjar.yaml
forjar validate --check-connectivity --json -f forjar.yaml
```

## Resources by Type

Group status output by resource type:

```bash
forjar status --resources-by-type --state-dir state
forjar status --resources-by-type --json --state-dir state
```

## Rollback Snapshot

Auto-rollback to a named snapshot on apply failure:

```bash
forjar apply --rollback-snapshot v1-safe -f forjar.yaml
```

## Graph Orphans

Show resources with no dependencies and no dependents:

```bash
forjar graph --orphans -f forjar.yaml
```

## Lock Export

Export lock file data in alternative formats:

```bash
forjar lock-export --state-dir state --format json
forjar lock-export --state-dir state --format csv
forjar lock-export --state-dir state --format yaml
```

## Machines Only

Show only machine-level summary without resource details:

```bash
forjar status --machines-only --state-dir state
forjar status --machines-only --json --state-dir state
```

## Check Templates

Validate all template expressions resolve correctly:

```bash
forjar validate --check-templates -f forjar.yaml
forjar validate --check-templates --json -f forjar.yaml
```

## Stale Resources

Show resources whose lock entries haven't been updated recently:

```bash
forjar status --stale-resources --state-dir state
forjar status --stale-resources --json --state-dir state
```

## Multi-Tag Filter

Filter apply to resources matching specific tags:

```bash
forjar apply --tags networking --tags security -f forjar.yaml
forjar apply --tags database -f forjar.yaml --state-dir state
```

## Graph Stats

Show DAG statistics — node count, edge count, depth, and width:

```bash
forjar graph --stats -f forjar.yaml
forjar graph --stats --json -f forjar.yaml
```

## Lock GC

Garbage-collect orphaned lock entries not present in current config:

```bash
forjar lock-gc -f forjar.yaml --state-dir state
forjar lock-gc -f forjar.yaml --state-dir state --yes  # skip confirmation
forjar lock-gc -f forjar.yaml --state-dir state --json
```

## Log File

Write apply output to a log file:

```bash
forjar apply --log-file /var/log/forjar/apply.log -f forjar.yaml
```

## Health Threshold

Filter status to resources above a failure-rate threshold:

```bash
forjar status --health-threshold 50 --state-dir state
forjar status --health-threshold 25 --json --state-dir state
```

## Apply Comment

Attach an audit note to the apply run (stored in event log):

```bash
forjar apply --comment "deploy v2.1 hotfix" -f forjar.yaml
forjar apply --comment "scheduled maintenance" -f forjar.yaml --state-dir state
```

## Strict Dependencies

Verify that dependency ordering matches resource declaration order:

```bash
forjar validate --strict-deps -f forjar.yaml
forjar validate --strict-deps --json -f forjar.yaml
```

## Status JSON Lines

Output status as newline-delimited JSON (NDJSON) for stream processing:

```bash
forjar status --json-lines --state-dir state
forjar status --json-lines --machine gpu-box --state-dir state
```

## Only Changed

Apply only resources whose config hash changed since last apply:

```bash
forjar apply --only-changed -f forjar.yaml --state-dir state
```

## Graph JSON

Output dependency graph as a JSON adjacency list:

```bash
forjar graph --json -f forjar.yaml
```

## Lock Diff

Compare two lock file directories and show resource-level differences:

```bash
forjar lock-diff state-v1 state-v2
forjar lock-diff state-v1 state-v2 --json
```

## Pre-Script

Run a script before apply starts (pre-flight checks):

```bash
forjar apply --pre-script ./preflight.sh -f forjar.yaml
```

## Status Since

Show only resources changed within a time duration:

```bash
forjar status --since 1h --state-dir state
forjar status --since 7d --json --state-dir state
```

## Check Secrets

Scan config for hardcoded secrets or credentials:

```bash
forjar validate --check-secrets -f forjar.yaml
forjar validate --check-secrets --json -f forjar.yaml
```

## Status Export

Export status report to a file:

```bash
forjar status --export /tmp/report.json --state-dir state
forjar status --export /tmp/report.json --json --state-dir state
```

## Notify Webhook

POST structured apply results to any webhook URL:

```bash
forjar apply --notify-webhook https://hooks.example.com/forjar -f forjar.yaml
```

## Graph Highlight

Highlight a resource and its transitive dependencies in graph output:

```bash
forjar graph --highlight web-server -f forjar.yaml
forjar graph --highlight web-server --format dot -f forjar.yaml
```

## Lock Merge

Merge two lock file directories (right takes precedence on conflicts):

```bash
forjar lock-merge state-team-a state-team-b --output state-merged
forjar lock-merge state-team-a state-team-b --output state-merged --json
```

## Post Script

Run a script after apply completes:

```bash
forjar apply --post-script ./postflight.sh -f forjar.yaml
```

## Dry Run JSON

Output dry-run results as structured JSON for CI pipelines:

```bash
forjar apply --dry-run-json -f forjar.yaml --state-dir state
```

## Approval Required

Require explicit approval before destructive changes:

```bash
forjar apply --approval-required -f forjar.yaml
```

## Check Idempotency

Verify all resources produce idempotent scripts:

```bash
forjar validate --check-idempotency -f forjar.yaml
forjar validate --check-idempotency --json -f forjar.yaml
```

## Status Compact

Minimal one-line-per-machine output for large fleets:

```bash
forjar status --compact --state-dir state
forjar status --compact --json --state-dir state
```

## Canary Percent

Apply to N% of machines first, then the rest (gradual rollout):

```bash
forjar apply --canary-percent 25 -f forjar.yaml
```

## Graph Prune

Show graph with a resource and its subtree removed:

```bash
forjar graph --prune web-server -f forjar.yaml
forjar graph --prune web-server --format dot -f forjar.yaml
```

## Lock Rebase

Rebase lock file from one config version to another:

```bash
forjar lock-rebase old-state -f forjar.yaml --output new-state
forjar lock-rebase old-state -f forjar.yaml --output new-state --json
```

## Status Alerts

Show resources in alert state (failed, drifted, or stale):

```bash
forjar status --alerts --state-dir state
forjar status --alerts --json --state-dir state
```

## Environment Overlay

Apply using a named environment config overlay:

```bash
forjar apply --env-name staging -f forjar.yaml
forjar apply --env-name production -f forjar.yaml
```

## Check Drift Coverage

Verify all resources have drift detection configured:

```bash
forjar validate --check-drift-coverage -f forjar.yaml
forjar validate --check-drift-coverage --json -f forjar.yaml
```

## Diff Lock

Diff current lock against a saved lock snapshot:

```bash
forjar status --diff-lock old-state --state-dir state
```

## Graph Layers

Show graph organized by dependency layers (depth levels):

```bash
forjar graph --layers -f forjar.yaml
```

## Lock Sign

Cryptographically sign lock files with BLAKE3:

```bash
forjar lock-sign --state-dir state --key my-signing-key
forjar lock-sign --state-dir state --key my-signing-key --json
```

## Status Compliance

Check compliance against a named policy:

```bash
forjar status --compliance soc2 --state-dir state
forjar status --compliance hipaa --json --state-dir state
```

## Apply Batch Size

Process resources in batches for memory-bounded execution:

```bash
forjar apply -f forjar.yaml --batch-size 10
forjar apply -f forjar.yaml --batch-size 5 --state-dir state
```

## Validate Check Cycles Deep

Detect indirect circular dependencies via transitive closure:

```bash
forjar validate -f forjar.yaml --check-cycles-deep
forjar validate -f forjar.yaml --check-cycles-deep --json
```

## Status Histogram

Show resource status distribution as an ASCII histogram:

```bash
forjar status --histogram --state-dir state
forjar status --histogram --json --state-dir state
```

## Apply Notify Teams

Send apply results to Microsoft Teams webhook:

```bash
forjar apply -f forjar.yaml --notify-teams https://outlook.office.com/webhook/...
```

## Graph Critical Resources

Identify resources with the most dependents (bottleneck analysis):

```bash
forjar graph -f forjar.yaml --critical-resources
```

## Lock Verify Sig

Verify lock file signature against a signing key:

```bash
forjar lock-verify-sig --state-dir state --key my-signing-key
forjar lock-verify-sig --state-dir state --key my-signing-key --json
```

## Apply Abort on Drift

Abort apply if drift is detected before execution (safety gate):

```bash
forjar apply -f forjar.yaml --abort-on-drift
```

## Status Dependency Health

Show health score weighted by dependency position (risk analysis):

```bash
forjar status --dependency-health --state-dir state
forjar status --dependency-health --json --state-dir state
```

## Apply Dry Run Summary

Show a one-line summary of what would change per machine:

```bash
forjar apply -f forjar.yaml --dry-run-summary
```

## Validate Check Naming

Enforce kebab-case resource naming conventions:

```bash
forjar validate -f forjar.yaml --check-naming
forjar validate -f forjar.yaml --check-naming --json
```

## Status Top Failures

Show the most frequently failing resources:

```bash
forjar status --top-failures --state-dir state
forjar status --top-failures --json --state-dir state
```

## Apply Notify Discord

Send apply results to a Discord webhook:

```bash
forjar apply -f forjar.yaml --notify-discord https://discord.com/api/webhooks/...
```

## Graph Weight

Show edge weights based on dependency strength:

```bash
forjar graph -f forjar.yaml --weight
forjar graph -f forjar.yaml --weight --format dot
```

## Lock Compact All

Compact all machine lock files in one operation:

```bash
forjar lock-compact-all --state-dir state --yes
forjar lock-compact-all --state-dir state --yes --json
```

## Apply Rollback on Threshold

Auto-rollback if more than N resources fail:

```bash
forjar apply -f forjar.yaml --rollback-on-threshold 3
```

## Status Convergence Rate

Show convergence percentage across resources:

```bash
forjar status --convergence-rate --state-dir state
forjar status --convergence-rate --json --state-dir state
```

## Apply Metrics Port

Expose apply metrics on an HTTP port for Prometheus scraping:

```bash
forjar apply -f forjar.yaml --metrics-port 9090
```

## Validate Check Overlaps

Detect resources targeting the same path/port on the same machine:

```bash
forjar validate -f forjar.yaml --check-overlaps
forjar validate -f forjar.yaml --check-overlaps --json
```

## Status Drift Summary

Show one-line per-machine drift count and percentage:

```bash
forjar status --drift-summary --state-dir state
forjar status --drift-summary --json --state-dir state
```

## Apply Notify OpsGenie

Send apply alerts to OpsGenie:

```bash
forjar apply -f forjar.yaml --notify-opsgenie <api-key>
```

## Graph Subgraph

Extract and display a resource's dependency subgraph:

```bash
forjar graph -f forjar.yaml --subgraph my-resource
forjar graph -f forjar.yaml --subgraph my-resource --format dot
```

## Lock Audit Trail

Show full audit trail of lock file changes with timestamps:

```bash
forjar lock-audit-trail --state-dir state
forjar lock-audit-trail --state-dir state --machine gpu-box --json
```

## Apply Circuit Breaker

Pause apply after N consecutive failures:

```bash
forjar apply -f forjar.yaml --circuit-breaker 5
```

## Status Resource Age

Show age of each resource since last successful apply:

```bash
forjar status --resource-age --state-dir state
forjar status --resource-age --json --state-dir state
```

## Apply Require Approval

Require named approvers before apply proceeds:

```bash
forjar apply -f forjar.yaml --require-approval "alice,bob"
```

## Validate Check Limits

Enforce resource count limits per machine/type:

```bash
forjar validate -f forjar.yaml --check-limits
forjar validate -f forjar.yaml --check-limits --json
```

## Status SLA Report

Show SLA compliance based on convergence:

```bash
forjar status --sla-report --state-dir state
forjar status --sla-report --json --state-dir state
```

## Apply Notify Datadog

Send apply events to Datadog:

```bash
forjar apply -f forjar.yaml --notify-datadog <api-key>
```

## Graph Impact Radius

Show blast radius of changing a resource:

```bash
forjar graph -f forjar.yaml --impact-radius base-packages
```

## Lock Rotate Keys

Rotate all lock file signing keys:

```bash
forjar lock-rotate-keys --state-dir state --old-key old-key --new-key new-key
forjar lock-rotate-keys --state-dir state --old-key old --new-key new --json
```

## Apply Change Window

Restrict applies to defined maintenance windows:

```bash
forjar apply -f forjar.yaml --change-window "0 2 * * SAT"
```

## Status Compliance Report

Generate a full compliance report:

```bash
forjar status --compliance-report pci-dss --state-dir state
forjar status --compliance-report soc2 --json --state-dir state
```

### Canary Machine Deploy

Apply to a single machine first as a canary before the rest of the fleet:

```bash
forjar apply -f forjar.yaml --canary-machine gpu-box
```

### Complexity Check

Warn on resources with high dependency fan-out or fan-in:

```bash
forjar validate -f forjar.yaml --check-complexity
forjar validate -f forjar.yaml --check-complexity --json
```

### Mean Time To Recovery

Show MTTR per resource based on event log analysis:

```bash
forjar status --mttr --state-dir state
forjar status --mttr --machine gpu-box --json --state-dir state
```

### New Relic Notification

Send apply events to New Relic:

```bash
forjar apply -f forjar.yaml --notify-newrelic NRJS-your-key-here
```

### Dependency Matrix

Export resource dependency matrix as CSV or JSON:

```bash
forjar graph -f forjar.yaml --dependency-matrix
forjar graph -f forjar.yaml --dependency-matrix --json
```

### Lock Backup

Create a timestamped backup of all lock files:

```bash
forjar lock-backup --state-dir state
forjar lock-backup --state-dir state --json
```

### Apply Max Duration

Abort apply if it exceeds a time limit (seconds):

```bash
forjar apply -f forjar.yaml --max-duration 300
```

### Status Trend

Show status trend over the last N events:

```bash
forjar status --trend 20 --state-dir state
forjar status --trend 50 --machine gpu-box --json --state-dir state
```

### Grafana Notification

Send apply annotations to Grafana for dashboard integration:

```bash
forjar apply -f forjar.yaml --notify-grafana https://grafana.example.com/api/annotations
```

### Security Check

Scan for insecure permissions, ports, or user configurations:

```bash
forjar validate -f forjar.yaml --check-security
forjar validate -f forjar.yaml --check-security --json
```

### Failure Prediction

Predict next failure based on historical patterns:

```bash
forjar status --prediction --state-dir state
forjar status --prediction --machine gpu-box --json --state-dir state
```

### Resource Rate Limiting

Apply at most N resources per minute:

```bash
forjar apply -f forjar.yaml --rate-limit-resources 10
```

### Graph Hotspots

Highlight resources with most changes and failures:

```bash
forjar graph -f forjar.yaml --hotspots
```

### Checkpoint Interval

Save intermediate state during long applies for resumability:

```bash
forjar apply -f forjar.yaml --checkpoint-interval 60
```

### Capacity Planning

Show resource utilization vs limits per machine:

```bash
forjar status --capacity --state-dir state
forjar status --capacity --machine gpu-box --json --state-dir state
```

### VictorOps Notification

Send apply events to VictorOps/Splunk On-Call:

```bash
forjar apply -f forjar.yaml --notify-victorops your-api-key
```

### Deprecation Check

Warn on deprecated resource fields or types:

```bash
forjar validate -f forjar.yaml --check-deprecation
forjar validate -f forjar.yaml --check-deprecation --json
```

### Cost Estimate

Estimate resource cost based on type complexity:

```bash
forjar status --cost-estimate --state-dir state
forjar status --cost-estimate --json --state-dir state
```

### Blue/Green Deployment

Deploy with blue/green machine pairs:

```bash
forjar apply -f forjar.yaml --blue-green web-blue
```

### Graph Timeline

Show resource application order as ASCII timeline:

```bash
forjar graph -f forjar.yaml --timeline-graph
```

### Lock Chain Verification

Verify full chain of custody from lock signatures:

```bash
forjar lock-verify-chain --state-dir state
forjar lock-verify-chain --state-dir state --json
```

### Dry Run Cost

Show estimated changes without applying:

```bash
forjar apply -f forjar.yaml --dry-run-cost
```

### Staleness Report

Show resources not applied within a configurable window:

```bash
forjar status --staleness-report 7d --state-dir state
forjar status --staleness-report 30d --machine gpu-box --json --state-dir state
```

### Check Drift Risk

Score drift risk based on resource volatility:

```bash
forjar validate --check-drift-risk -f forjar.yaml
forjar validate --check-drift-risk -f forjar.yaml --json
```

### MS Teams Adaptive Cards

Send rich Adaptive Card notifications to Microsoft Teams:

```bash
forjar apply -f forjar.yaml --notify-msteams-adaptive https://outlook.office.com/webhook/...
```

### Progressive Rollout

Apply changes to a percentage of machines progressively:

```bash
forjar apply -f forjar.yaml --progressive 25
forjar apply -f forjar.yaml --progressive 50
```

### Approval Webhook

Require external approval via webhook before applying:

```bash
forjar apply -f forjar.yaml --approval-webhook https://api.example.com/approve
```

### Health Score

Get a composite health score (0-100) across all machines:

```bash
forjar status --health-score --state-dir state
forjar status --health-score --state-dir state --json
```

### Executive Summary

One-line per machine summary for dashboards:

```bash
forjar status --executive-summary --state-dir state
forjar status --executive-summary --state-dir state --json
```

### What-If Analysis

Simulate removing a resource and show transitive impact:

```bash
forjar graph --what-if base-packages -f forjar.yaml
forjar graph --what-if base-packages -f forjar.yaml --json
```

### Lock Stats

Show lock file statistics (sizes, ages, resource counts):

```bash
forjar lock-stats --state-dir state
forjar lock-stats --state-dir state --json
```

### Check Compliance

Validate resources against a compliance policy (CIS, SOC2, HIPAA):

```bash
forjar validate --check-compliance CIS -f forjar.yaml
forjar validate --check-compliance SOC2 -f forjar.yaml --json
forjar validate --check-compliance HIPAA -f forjar.yaml
```

### Incident Notification

POST incidents to PagerDuty/Opsgenie on apply failure:

```bash
forjar apply -f forjar.yaml --notify-incident ROUTING_KEY_123
```

### Audit Trail

Show full audit trail with who/what/when for each change:

```bash
forjar status --audit-trail --state-dir state
forjar status --audit-trail --machine gpu-box --json --state-dir state
```

### Sign-Off

Require named sign-off before apply proceeds:

```bash
forjar apply -f forjar.yaml --sign-off noah
```

### Blast Radius

Show all resources transitively affected by a change to a target:

```bash
forjar graph --blast-radius base-packages -f forjar.yaml
forjar graph --blast-radius base-packages -f forjar.yaml --json
```

### Lock Audit

Verify lock file integrity and detect tampering:

```bash
forjar lock-audit --state-dir state
forjar lock-audit --state-dir state --json
```

### Check Portability

Check resources for platform-specific assumptions:

```bash
forjar validate --check-portability -f forjar.yaml
forjar validate --check-portability -f forjar.yaml --json
```

### AWS SNS Notification

Publish apply events to AWS SNS topic:

```bash
forjar apply -f forjar.yaml --notify-sns arn:aws:sns:us-east-1:123456789:forjar-events
```

### OpenTelemetry Spans

POST OpenTelemetry spans for apply execution tracing:

```bash
forjar apply -f forjar.yaml --telemetry-endpoint https://otel.example.com/v1/traces
```

### Runbook Link

Attach runbook URL to apply for audit trail:

```bash
forjar apply -f forjar.yaml --runbook https://wiki.example.com/runbook/deploy
```

### Resource Graph

Show resource dependency graph from live state:

```bash
forjar status --resource-graph --state-dir state
forjar status --resource-graph --machine gpu-box --json --state-dir state
```

### Drift Velocity

Show drift rate over time:

```bash
forjar status --drift-velocity --state-dir state
forjar status --drift-velocity --machine gpu-box --json --state-dir state
```

### Change Impact

Show direct and indirect impact of changing a resource:

```bash
forjar graph --change-impact base-packages -f forjar.yaml
forjar graph --change-impact base-packages -f forjar.yaml --json
```

### Lock Compress

Compress old lock files by minifying YAML:

```bash
forjar lock-compress --state-dir state
forjar lock-compress --state-dir state --json
```

### Check Resource Limits

Validate resource counts don't exceed per-machine limits:

```bash
forjar validate --check-resource-limits -f forjar.yaml
forjar validate --check-resource-limits -f forjar.yaml --json
```

### GCP Pub/Sub Notification

Publish apply events to Google Cloud Pub/Sub:

```bash
forjar apply -f forjar.yaml --notify-pubsub projects/my-proj/topics/forjar-events
```

### Fleet Strategy

Fleet-wide rollout strategy (parallel, rolling, canary):

```bash
forjar apply -f forjar.yaml --fleet-strategy rolling
forjar apply -f forjar.yaml --fleet-strategy canary
```

### Pre-Check Script

Run validation script before apply proceeds:

```bash
forjar apply -f forjar.yaml --pre-check /usr/local/bin/pre-deploy.sh
```

### Fleet Overview

Aggregated fleet summary across all machines:

```bash
forjar status --fleet-overview --state-dir state
forjar status --fleet-overview --state-dir state --json
```

### Machine Health

Per-machine health details with resource breakdown:

```bash
forjar status --machine-health --state-dir state
forjar status --machine-health --machine gpu-box --json --state-dir state
```

### Resource Types Graph

Show graph colored/grouped by resource type:

```bash
forjar graph --resource-types -f forjar.yaml
forjar graph --resource-types -f forjar.yaml --json
```

### Lock Defrag

Defragment lock files (reorder resources alphabetically):

```bash
forjar lock-defrag --state-dir state
forjar lock-defrag --state-dir state --json
```

### Check Unused Resources

Detect resources not referenced by any dependency chain:

```bash
forjar validate --check-unused -f forjar.yaml
forjar validate --check-unused -f forjar.yaml --json
```

### AWS EventBridge Notification

Publish apply events to AWS EventBridge:

```bash
forjar apply -f forjar.yaml --notify-eventbridge custom-bus
```

### Dry Run Graph

Show execution graph without applying:

```bash
forjar apply -f forjar.yaml --dry-run-graph
```

### Post-Check Script

Run validation script after apply completes:

```bash
forjar apply -f forjar.yaml --post-check /usr/local/bin/verify.sh
```

### Config Drift

Compare running state against declared config:

```bash
forjar status --config-drift --state-dir state
forjar status --config-drift --machine gpu-box --json --state-dir state
```

### Convergence Time

Show average time to convergence per resource:

```bash
forjar status --convergence-time --state-dir state
forjar status --convergence-time --machine gpu-box --json --state-dir state
```

### Topological Levels

Show resources grouped by topological depth level:

```bash
forjar graph --topological-levels -f forjar.yaml
forjar graph --topological-levels -f forjar.yaml --json
```

### Lock Normalize

Normalize lock file format for consistency:

```bash
forjar lock-normalize --state-dir state
forjar lock-normalize --state-dir state --json
```

### Advanced Orchestration (FJ-590→FJ-597)

**Dependency validation** catches unresolved references before apply:

```bash
forjar validate --check-dependencies -f forjar.yaml
forjar validate --check-dependencies -f forjar.yaml --json
```

**Resource timeline** shows event log history:

```bash
forjar status --resource-timeline --state-dir state
forjar status --resource-timeline --state-dir state --json
```

**Error summary** aggregates failures across machines:

```bash
forjar status --error-summary --state-dir state
forjar status --error-summary --state-dir state --json
```

**Execution order** shows the flattened DAG sort:

```bash
forjar graph --execution-order -f forjar.yaml
forjar graph --execution-order -f forjar.yaml --json
```

**Lock validate** checks lock file integrity:

```bash
forjar lock-validate --state-dir state
forjar lock-validate --state-dir state --json
```

**Kafka notifications** stream apply events:

```bash
forjar apply -f forjar.yaml --notify-kafka my-topic
```

### Security Hardening & Audit (FJ-600→FJ-607)

**Permission validation** checks for insecure file modes and ownership:

```bash
forjar validate --check-permissions -f forjar.yaml
forjar validate --check-permissions -f forjar.yaml --json
```

**Security posture** shows security-relevant resource states:

```bash
forjar status --security-posture --state-dir state
forjar status --security-posture --state-dir state --json
```

**Security boundaries** highlights resources crossing security zones:

```bash
forjar graph --security-boundaries -f forjar.yaml
forjar graph --security-boundaries -f forjar.yaml --json
```

**HMAC verification** checks lock file signatures:

```bash
forjar lock-verify-hmac --state-dir state
forjar lock-verify-hmac --state-dir state --json
```

**Pre-flight checks** gate apply with a validation script:

```bash
forjar apply -f forjar.yaml --pre-flight "bash /path/to/check.sh"
```

## Next Steps

- [Configuration Reference](02-configuration.md) — Complete `forjar.yaml` schema
- [Resource Types](03-resources.md) — All 9 resource types with examples
- [Recipes](04-recipes.md) — Reusable parameterized infrastructure patterns
- [CLI Reference](06-cli.md) — Every command and flag
- [Troubleshooting](11-troubleshooting.md) — Common errors and fixes

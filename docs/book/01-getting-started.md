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

You should see forjar's subcommands: `init`, `validate`, `plan`, `apply`, `drift`, `status`, `history`, `show`, `graph`, `check`, `diff`, `fmt`, `lint`, `rollback`, `anomaly`.

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

## Next Steps

- [Configuration Reference](02-configuration.md) — Complete `forjar.yaml` schema
- [Resource Types](03-resources.md) — All 8 resource types with examples
- [Recipes](04-recipes.md) — Reusable parameterized infrastructure patterns
- [CLI Reference](06-cli.md) — Every command and flag
- [Troubleshooting](11-troubleshooting.md) — Common errors and fixes

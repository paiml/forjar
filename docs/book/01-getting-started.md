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

## Next Steps

- [Configuration Reference](02-configuration.md) — Complete `forjar.yaml` schema
- [Resource Types](03-resources.md) — All 8 resource types with examples
- [Recipes](04-recipes.md) — Reusable parameterized infrastructure patterns
- [CLI Reference](06-cli.md) — Every command and flag
- [Troubleshooting](11-troubleshooting.md) — Common errors and fixes

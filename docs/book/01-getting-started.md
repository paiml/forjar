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

## Your First Project

```bash
forjar init my-infra
cd my-infra
```

This creates:
- `forjar.yaml` — configuration file (desired state)
- `state/` — directory for lock files and event logs

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

## Preview Changes

```bash
forjar plan -f forjar.yaml
```

Output:
```
Planning: my-infra (1 resources)

web-server:
  + base-packages: install curl, htop, git

Plan: 1 to add, 0 to change, 0 to destroy, 0 unchanged.
```

## Apply

```bash
forjar apply -f forjar.yaml
```

Forjar will:
1. SSH to the machine
2. Run `apt-get install -y curl htop git`
3. Record the state in `state/web-server/state.lock.yaml`
4. Append events to `state/web-server/events.jsonl`

## Verify Idempotency

Run apply again:

```bash
forjar apply -f forjar.yaml
```

Output:
```
web-server: 0 converged, 1 unchanged, 0 failed (0.0s)

Apply complete: 0 converged, 1 unchanged.
```

The BLAKE3 hash of the desired state matches the lock file — nothing to do.

## Check for Drift

```bash
forjar drift --state-dir state
```

If someone manually changes the machine, drift detection will flag it.

## Key Concepts

- **Desired state**: What you declare in `forjar.yaml`
- **Lock file**: BLAKE3-hashed record of what was actually applied
- **Idempotency**: Apply only runs when hashes differ
- **Jidoka**: Stops on first failure, preserves partial state
- **Provenance**: Every apply is logged to `events.jsonl`

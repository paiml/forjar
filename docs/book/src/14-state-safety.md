# State Safety & Disaster Recovery

Forjar provides multiple layers of state protection to prevent the failure modes common in infrastructure-as-code tools. This chapter covers state safety invariants, disaster recovery mechanisms, and anti-pattern defenses.

## State Safety Invariants

Forjar enforces five state safety invariants (S1-S5):

| Invariant | Description |
|-----------|-------------|
| S1 | State files are always valid YAML with BLAKE3 integrity |
| S2 | Lock files are append-only (no destructive updates) |
| S3 | Every state mutation is logged to the event journal |
| S4 | Snapshots exist before destructive operations |
| S5 | Cross-stack references are validated before apply |

## Saga-Pattern Multi-Stack Apply

For multi-stack operations, forjar uses the saga pattern with compensating transactions:

```bash
forjar saga -f stack-a.yaml -f stack-b.yaml -f stack-c.yaml --state-dir state
```

If stack-b fails mid-apply:
1. Stack-b's partial changes are rolled back using the pre-apply snapshot
2. Stack-a (already completed) remains in its new state
3. Stack-c (not yet started) is skipped
4. A detailed failure report shows exactly what happened

See `cli/saga_coordinator.rs` for the implementation.

## Generational State Snapshots

Inspired by Nix's generation system, forjar maintains numbered state generations:

```bash
# List generations
forjar state generations --state-dir state

# Rollback to generation 3
forjar state rollback --generation 3
```

Each `forjar apply` creates a new generation. Generations are immutable — rolling back creates a new generation that restores old state rather than deleting history.

## Reversibility Classification

Every resource operation is classified by reversibility:

| Class | Description | Example |
|-------|-------------|---------|
| Reversible | Can be undone automatically | Package install/remove |
| Partially reversible | Requires manual cleanup | File content change (backup exists) |
| Irreversible | Cannot be undone | Database migration, `rm -rf` |

```bash
forjar plan -f config.yaml --classify-reversibility
```

Irreversible operations require explicit `--confirm-destructive` flag.

## State Integrity Verification

BLAKE3 checksums protect state files from corruption and tampering:

```bash
# Verify all state files
forjar lock-verify --state-dir state

# Sign state with ed25519 key
forjar lock-sign --state-dir state --key signing.key
```

Every `save_lock()` and `save_global_lock()` writes a `.b3` sidecar file. Pre-apply verification checks these before proceeding.

## Cross-Stack Staleness Detection

When consuming outputs from another stack via `forjar-state` data sources, forjar detects stale data:

```yaml
data:
  producer-state:
    type: forjar-state
    config: producer
    state_dir: ../producer/state
    outputs: [web_ip, db_ip]
    max_staleness: "24h"  # Warn if producer state is older than 24 hours
```

If the producer's last apply timestamp exceeds `max_staleness`, forjar emits a warning before proceeding. This prevents silent consumption of outdated infrastructure state.

## Event-Sourced State Reconstruction

The event journal (`state/intel/events.jsonl`) records every state mutation. Point-in-time recovery is possible by replaying events:

```bash
# Reconstruct state at a specific timestamp
forjar state reconstruct --at "2025-06-15T10:30:00Z" --machine web-01
```

This replays all events up to the given timestamp and rebuilds the lock file, enabling post-incident forensics and state recovery.

## Anti-Pattern Defenses

Forjar specifically defends against failure modes seen in other IaC tools:

### CDK Deadlock Prevention
Unlike CloudFormation's `UPDATE_ROLLBACK_FAILED` state, forjar's lock files never enter an unrecoverable state. The `--force-unlock` flag exists as an escape hatch, but the event journal ensures no state is lost.

### Terraform State Corruption
Forjar's BLAKE3 integrity verification detects state file corruption before apply. The `.b3` sidecar files provide tamper-evident checksums that would fail if state were modified outside of forjar.

### Ansible Partial Apply
Forjar's saga pattern ensures multi-stack operations are either fully completed or cleanly rolled back. Pre-apply snapshots guarantee a known-good state to return to.

## Convergence Budget Enforcement

The `convergence_budget` policy prevents runaway applies:

```yaml
policy:
  convergence_budget: 3  # Maximum apply attempts before giving up
```

If a resource fails to converge after the budget is exhausted, forjar stops retrying and reports the failure. This prevents infinite retry loops that waste resources.

## Pre-Apply Snapshots

Before every apply, forjar creates a state snapshot:

```bash
# Snapshots are automatic, but can be listed
forjar snapshot list --state-dir state

# Restore from a snapshot
forjar snapshot restore --id snap-20250615-103000
```

Snapshots capture the complete state directory (lock files, global lock, event journal) as a compressed archive.

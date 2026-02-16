# ADR-003: Jidoka (Stop-on-First-Failure) Policy

## Status
Accepted

## Context
When applying infrastructure changes, a resource may fail. The system must decide whether to:
- Continue applying remaining resources (Ansible default)
- Stop immediately and preserve partial state (Jidoka)
- Roll back all changes (transactional)

## Decision
Default to Jidoka: stop on first failure, preserve partial state in lock file.

## Consequences
- **Positive**: No cascading failures — a failed package install won't trigger dependent service restarts
- **Positive**: Partial state is preserved — operator can inspect, fix, and re-run
- **Positive**: Simple mental model — the lock file always reflects reality
- **Negative**: Partial state may leave the system in an inconsistent intermediate state
- **Negative**: No automatic rollback (by design — rollback is a new apply with previous config)

## Falsification
This decision would be wrong if:
1. Partial state causes more damage than cascading failures in practice
2. Users frequently need rollback and find "re-apply previous config" too slow
3. DAG-aware partial execution (skip unrelated branches) is needed — would require extending the failure policy

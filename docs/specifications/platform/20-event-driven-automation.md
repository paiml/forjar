# Event-Driven Automation

> Reactive rulebooks triggered by infrastructure events — file changes, process signals, webhooks, cron, metric thresholds.

**Status**: Implemented | **Date**: 2026-03-10 | **Spec IDs**: FJ-3100 through FJ-3109

---

## Motivation

Forjar currently operates in a pull model: `forjar apply` must be explicitly invoked. Competing tools (Ansible EDA, Salt reactor, Puppet orchestrator) offer event-driven automation where infrastructure converges reactively. For sovereign AI workloads — GPU fleet management, model serving, training pipeline recovery — reactive convergence is essential.

### Chain of Thought: Sovereign Stack Implementation

```
Problem: No reactive convergence capability.

STEP 1 — Event Sources (renacer integration)
  renacer provides syscall tracing (inotify, fanotify, process lifecycle).
  Reuse renacer's event hooks for file-change and process-exit detection.
  No external event broker needed — in-process channel stays sovereign.

STEP 2 — Daemon Lifecycle (duende-core)
  duende provides the Daemon trait, DaemonManager, and DaemonContext.
  `forjar watch` implements Daemon with graceful SIGTERM shutdown.
  DaemonManager provides restart policy (on_failure, always, never).
  DaemonMetrics gives RED metrics (rate, errors, duration) for event processing.
  Event channels use std::sync::mpsc (no tokio — stays synchronous).
  Events are typed: FileChanged, ProcessExited, CronFired, WebhookReceived, MetricThreshold.

STEP 3 — Rulebook Engine (forjar core, new module)
  Rulebooks are YAML — consistent with forjar's config-as-data philosophy.
  Pattern matching on event type + conditions → action list.
  Actions: apply (subset), destroy, notify, script (bashrs-validated).

STEP 4 — Script Safety (bashrs purification)
  All event handler scripts pass through bashrs I8 invariant before dispatch.
  No shell injection from event payloads — templated values are escaped.

STEP 5 — Quality Gates (pmat + certeza)
  pmat grades rulebook handler complexity (TDG).
  certeza validates event handler test coverage ≥ 95%.

Conclusion: Zero external dependencies. All event processing uses sovereign
stack components. No Kafka, no RabbitMQ, no cloud event services.
```

---

## Architecture

```
                    ┌─────────────────────────────────┐
                    │       forjar watch daemon         │
                    │  (duende-core Daemon trait)        │
                    │  (graceful shutdown + RED metrics) │
                    └──────────┬──────────────────────┘
                               │
          ┌────────────────────┼──────────────────────┐
          │                    │                       │
┌─────────▼──────┐  ┌─────────▼──────┐  ┌────────────▼────────┐
│  Event Sources │  │  Rulebook       │  │  Action Executor     │
│                │  │  Engine          │  │                      │
│ renacer hooks  │  │  Pattern match  │  │  forjar apply        │
│ cron scheduler │  │  Condition eval │  │  forjar destroy      │
│ webhook server │  │  Cooldown/dedup │  │  script (bashrs)     │
│ metric polling │  │  Rate limiting  │  │  notify (webhook)    │
└────────────────┘  └────────────────┘  └──────────────────────┘
```

### Event Sources

| Source | Sovereign Component | Mechanism |
|--------|-------------------|-----------|
| File change | renacer (inotify) | Kernel inotify/fanotify via renacer syscall hooks |
| Process exit | renacer (waitpid) | Process lifecycle tracking |
| Cron | forjar (thread-based) | In-process cron scheduler, no system crontab |
| Webhook | forjar (std TCP) | Lightweight HTTP listener on configurable port |
| Metric threshold | forjar (polling) | Periodic metric evaluation against thresholds |
| Daemon lifecycle | duende-core | `Daemon` trait, graceful shutdown, restart policy, RED metrics |

### Rulebook Format

```yaml
# forjar-rules.yaml
rulebooks:
  - name: gpu-recovery
    description: "Auto-recover GPU worker on OOM kill"
    events:
      - type: process_exit
        match:
          binary: "/usr/local/bin/realizar-serve"
          exit_code: 137  # OOM killed
    conditions:
      - "{{ machine.gpu_count > 0 }}"
    actions:
      - apply:
          file: gpu-worker.yaml
          subset: ["gpu-inference-service"]
          machine: "{{ event.machine }}"
    cooldown: 60s
    max_retries: 3

  - name: config-drift-repair
    events:
      - type: file_changed
        match:
          paths: ["/etc/nginx/nginx.conf", "/etc/systemd/system/*.service"]
    actions:
      - apply:
          file: forjar.yaml
          tags: ["config"]
    cooldown: 30s
```

---

## Spec IDs

| ID | Deliverable | Depends On |
|----|-------------|-----------|
| FJ-3100 | Event source abstraction trait + renacer file watcher | — |
| FJ-3101 | Rulebook YAML parser and pattern matcher | FJ-3100 |
| FJ-3102 | `forjar watch` daemon with graceful shutdown | FJ-3101 |
| FJ-3103 | Cron event source (in-process scheduler) | FJ-3100 |
| FJ-3104 | Webhook event source (hyper HTTP listener) | FJ-3100 |
| FJ-3105 | Metric threshold polling source | FJ-3100 |
| FJ-3106 | Cooldown, deduplication, rate limiting | FJ-3102 |
| FJ-3107 | Event log (append to events.jsonl) | FJ-3102 |
| FJ-3108 | `forjar rules validate` (bashrs + pmat quality check) | FJ-3101 |
| FJ-3109 | Integration tests: file change → reactive apply → convergence | FJ-3102 |

---

## Performance Targets

| Operation | Target | Mechanism |
|-----------|--------|-----------|
| Event detection latency | < 100ms | Kernel inotify (via renacer) |
| Rulebook evaluation | < 1ms | In-memory pattern match |
| Reactive apply (cached plan) | < 500ms | Subset apply skips unchanged resources |
| Webhook response | < 50ms | Acknowledge + async dispatch |

---

## Batuta Oracle Advice

**Recommendation**: batuta (primary orchestrator) for event dispatch patterns.
**Compute**: Scalar — no GPU needed for event processing.
**Supporting**: depyler for transpiling complex event handlers to optimized Rust.

## arXiv References

- [NSync: Automated IaC Reconciliation (2510.20211)](https://arxiv.org/abs/2510.20211) — API-trace-driven drift detection and reactive reconciliation
- [Savor et al. (2016) — Continuous Deployment at Facebook](https://arxiv.org/abs/2110.04008) — Event-driven deployment patterns at scale
- [AI-Augmented CI/CD Pipelines (2508.11867)](https://arxiv.org/abs/2508.11867) — Reactive pipeline triggers

---

## Falsification Criteria

| ID | Claim | Rejection Test |
|----|-------|---------------|
| F-3100-1 | Event detection < 100ms | Measure inotify latency on 10K watched paths; REJECT if p95 > 100ms |
| F-3100-2 | No event loss under load | Fire 1000 events/sec for 60s; REJECT if any event dropped |
| F-3100-3 | Cooldown prevents storms | Trigger same event 100x in 1s; REJECT if action fires > 1 time |
| F-3100-4 | bashrs validates all handler scripts | Inject semicolon-chain in handler; REJECT if bashrs doesn't flag it |
| F-3100-5 | Graceful shutdown preserves events | SIGTERM during event processing; REJECT if in-flight event lost |
| F-3100-6 | Zero external dependencies | Audit Cargo.toml; REJECT if any non-sovereign crate added for event bus |

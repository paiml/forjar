# The Forjar Book

Comprehensive documentation for forjar, the Rust-native Infrastructure as Code tool.

## Contents

1. [Getting Started](01-getting-started.md) — Installation, first project, basic workflow
2. [Configuration Reference](02-configuration.md) — Complete forjar.yaml schema
3. [Resource Types](03-resources.md) — Package, file, service, mount, user, docker, cron, network, pepita
4. [Recipes](04-recipes.md) — Reusable parameterized patterns
5. [Architecture](05-architecture.md) — Internals, DAG, hashing, transport, provable contracts
6. [CLI Reference](06-cli.md) — All commands and flags
7. [Cookbook](07-cookbook.md) — Real-world examples
8. [State Management](08-state-management.md) — Lock files, BLAKE3 hashing, drift detection, event logs
9. [Drift Detection & Tripwire](09-drift-and-tripwire.md) — Drift detection, auto-remediation, anomaly detection, event logs
10. [Testing & CI/CD Integration](10-testing-and-ci.md) — Validation pyramid, container testing, GitHub Actions, monitoring
11. [Troubleshooting](11-troubleshooting.md) — Common errors, SSH issues, state corruption, resource debugging
12. [Content-Addressed Store](12-store.md) — BLAKE3 content-addressed artifact storage
13. [Formal Verification & Provability](13-formal-verification.md) — Provable correctness, invariant checking, formal contracts
14. [State Safety & Disaster Recovery](14-state-safety.md) — Saga pattern, snapshots, rollback, lock signing
15. [DataOps & MLOps Pipelines](15-dataops-mlops.md) — Data pipelines, model training, GPU orchestration
16. [Agent Infrastructure & pforge](16-agent-infrastructure.md) — AI agent provisioning, pforge integration
17. [Operational Intelligence](17-operational-intelligence.md) — Metrics, observability, operational insights
18. [Supply Chain Security & Resilience](18-supply-chain-security.md) — Dependency verification, SBOM, provenance
19. [Competitive Positioning](19-competitive-positioning.md) — Forjar vs Terraform, Ansible, Nix, Pulumi

## Project Stats

| Metric | Value |
|--------|-------|
| CLI subcommands | 143 |
| Resource types | 15 (file, package, service, mount, user, group, cron, sysctl, directory, symlink, git-repo, docker-container, firewall-rule, apt-repo, shell) |
| Transport modes | 4 (local, SSH, container, pepita) |
| Tests | 9,571 at 95% line coverage |
| State integrity | BLAKE3 content-addressed state with git-native lock files |
| Binary | Single Rust binary, zero runtime dependencies |

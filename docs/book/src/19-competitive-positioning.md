# Competitive Positioning

## Overview

Forjar is a Rust-native Infrastructure as Code tool focused on OS-level configuration management with Terraform-grade state tracking. It ships as a single binary with zero runtime dependencies.

## Project Scale

| Metric | Value |
|--------|-------|
| Language | Rust (288K LOC) |
| CLI Commands | 143 subcommands |
| Resource Types | 15 |
| Transport Modes | 4 (local, SSH, container, pepita sandbox) |
| Tests | 9,571 at 95% line coverage |
| Example Configs | 103 recipes |

## Head-to-Head Comparison

| Dimension | Forjar | Terraform | Ansible | Nix | Pulumi |
|-----------|--------|-----------|---------|-----|--------|
| Language/Runtime | Rust single binary | Go | Python | Nix+C++ | Go+SDKs |
| Config Format | YAML+templates | HCL | YAML | Nix expression | General-purpose code |
| State | BLAKE3 YAML lock files (git-native) | JSON in S3/Consul | Stateless | Nix store | JSON in cloud backend |
| Transport | Local/SSH/Container/Pepita | Cloud APIs | SSH+WinRM | Local only | Cloud APIs |
| Drift Detection | Built-in (check, drift, tripwire) | plan (cloud-only) | None native | Declarative rebuild | preview (cloud-only) |
| Security | BLAKE3, age encryption, bashrs validation, Verus specs | State encryption, Vault | Ansible Vault | Reproducible builds | State encryption |
| Dependency Graph | DAG with cycle detection, graph --dot/--json | DAG with graph | Playbook ordering | Dependency closure | DAG with preview |

## Where Forjar Excels

1. **Single binary, zero dependencies** --- No Python, Go runtime, or cloud SDK needed. curl+install.
2. **Git-native state** --- Lock files are YAML in git. No remote state backend, no locking races.
3. **Built-in drift detection** --- `forjar check` and `forjar drift` detect changes without applying. Ansible has no equivalent.
4. **Security-first** --- BLAKE3 hashing, lock file HMAC signing/verification, age-encrypted secrets, bashrs shell validation, Verus formal specs.
5. **143 CLI commands** --- Full lifecycle: validate, plan, apply, check, drift, rollback, audit, export, compliance, suggest, prove, security-scan.
6. **Content-addressed store** --- Deduplicates across machines and applies, similar to Nix but with standard YAML.
7. **Multi-transport** --- Same config targets local, SSH, and container machines.

## Where Other Tools Excel

1. **Ecosystem breadth** --- Terraform has ~3,500 providers for every cloud API. Forjar targets OS-level resources only.
2. **Community** --- Terraform/Ansible have massive communities, documentation, consultants.
3. **Cloud-native** --- Terraform/Pulumi provision cloud infrastructure (VPCs, databases). Forjar operates at the OS layer.
4. **IDE tooling** --- Terraform has HCL language servers and extensions. Forjar has an MCP server but no IDE plugins yet.

## Strategic Positioning

Forjar occupies a distinct niche: OS-level configuration management with formal verification guarantees. The natural deployment is Terraform/Pulumi for cloud provisioning + Forjar for machine configuration --- replacing the Terraform+Ansible pattern with stronger state tracking and drift detection at the OS layer.

## Resource Types

Forjar supports 15 resource types:

1. file
2. package
3. service
4. mount
5. user
6. group
7. cron
8. sysctl
9. directory
10. symlink
11. git-repo
12. docker-container
13. firewall-rule
14. apt-repo
15. shell

## Transport Modes

- **Local** --- Direct execution on the host machine.
- **SSH** --- Remote execution via SSH (key or agent auth).
- **Container** --- Execute inside Docker/Podman containers.
- **Pepita** --- Sandboxed execution with namespace isolation, overlayfs, seccomp BPF.

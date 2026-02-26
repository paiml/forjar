# Forjar Examples

## Runnable Examples

Run any example with `cargo run --example <name>`.

| Example | Description |
|---------|-------------|
| `anomaly_detection` | ML-inspired anomaly detection: ADWIN windowing, isolation scoring, EWMA z-score |
| `arch_filtering` | Cross-architecture filtering — skip mismatched resources in multi-arch fleets |
| `blake3_hashing` | BLAKE3 content-addressed hashing for strings and files |
| `codegen_scripts` | Generate shell scripts for all resource types |
| `container_transport` | Container transport lifecycle: parse, validate, plan (no Docker required) |
| `drift_detection` | Detect unauthorized changes to managed files via hash comparison |
| `event_logging` | Provenance event log: record and replay apply events for auditing |
| `full_stack_deploy` | Full-stack deployment: packages, files, users, services, cron, firewall, Docker |
| `mcp_server` | MCP tool registry introspection (9 tools via pforge) |
| `multi_machine` | Multi-machine orchestration with cost-based scheduling |
| `parse_and_plan` | Parse config, resolve DAG, produce execution plan |
| `recipe_expansion` | Recipe loading, input validation, and expansion |
| `resource_scripts` | All 9 resource types x 3 script types (check, apply, state_query) |
| `shell_purifier` | bashrs shell purification: validation, linting, safety levels |
| `state_management` | Lock files, global locks, resource state tracking |
| `template_resolution` | Template resolution: params, machine refs, resource templates |
| `trace_provenance` | W3C-compatible trace spans with Lamport logical clocks |
| `user_management` | User creation, SSH keys, groups, shell config |
| `validation` | Config validation with multi-error collection and reporting |

## Dogfood Configs

Validate any config with `cargo run -- validate -f examples/<config>`.

| Config | Resource Types | What It Exercises |
|--------|---------------|-------------------|
| `dogfood-container.yaml` | file | Container transport, state hashing, lock persistence |
| `dogfood-packages.yaml` | package, file | Package codegen, cross-resource dependencies |
| `dogfood-phase2.yaml` | user, file, cron | User management, cron scheduling, base64 source transfer |
| `dogfood-service.yaml` | service, file | Systemd lifecycle, restart_on triggers, enabled/disabled |
| `dogfood-mount.yaml` | mount | NFS, bind, tmpfs mounts; fstab management; absent cleanup |
| `dogfood-network.yaml` | network | UFW firewall rules; allow/deny/absent; CIDR filtering |
| `dogfood-pepita.yaml` | pepita | Kernel isolation: cgroups, netns, overlay, seccomp |
| `dogfood-migrate.yaml` | docker, package | Docker containers, migration to pepita |
| `dogfood-recipe.yaml` | package, recipe | Recipe expansion, composite resources |
| `dogfood-crossarch.yaml` | file | Multi-machine, architecture filtering (x86_64/aarch64) |
| `dogfood-tags.yaml` | file | Resource tagging, tag-filtered operations |
| `dogfood-secrets.yaml` | file | Template interpolation with `{{params.*}}` |
| `dogfood-hooks.yaml` | file | Pre/post apply hooks, lifecycle callbacks |
| `dogfood-conditions.yaml` | file, package | Conditional resources (`when:` field), expression evaluation |
| `dogfood-iteration.yaml` | file | Resource iteration: `count:` ({{index}}), `for_each:` ({{item}}) |
| `dogfood-outputs.yaml` | file | Output values: `outputs:` block, template resolution, `--json` |
| `dogfood-policies.yaml` | file, package | Policy-as-code: require/deny/warn rules, plan-time enforcement |
| `dogfood-data.yaml` | file | Data sources: `data:` block, file/command/dns types, `{{data.*}}` templates |
| `dogfood-triggers.yaml` | file, service | General-purpose triggers: force re-apply when dependencies change |

## Supporting Assets

| Path | Used By | Description |
|------|---------|-------------|
| `files/app-entrypoint.sh` | `dogfood-phase2.yaml` | Source file for base64 transfer testing (`source:` field) |
| `recipes/web-server.yaml` | `dogfood-recipe.yaml`, `recipe_expansion.rs` | Reusable recipe with typed inputs for recipe expansion testing |

## Quick Start

```bash
# Run all examples
for ex in $(cargo run --example 2>&1 | grep '^\s' | awk '{print $1}'); do
  cargo run --example "$ex"
done

# Validate all dogfood configs
for f in examples/dogfood-*.yaml; do
  cargo run -- validate -f "$f"
done
```

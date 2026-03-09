# Contract Coverage, GC Sweep & Registry Push

Falsification coverage for FJ-1351/1352 (contract coverage and scaffolding), FJ-1365 (GC sweep execution), and E14 (OCI registry push commands).

## Contract Coverage (FJ-1351)

Check which kernel contracts are implemented for a model architecture:

```rust
use forjar::core::store::contract_coverage::{coverage_report, ContractStatus};

let report = coverage_report("llama", &required, &registry, &available);
assert_eq!(report.coverage_pct, 33.3);
assert_eq!(report.contracts["softmax-v1"], ContractStatus::Implemented);
assert_eq!(report.contracts["matmul-v1"], ContractStatus::Partial);
```

### Contract Status Logic

| Registry Status | File Exists | Result |
|----------------|-------------|--------|
| "implemented" | Yes | `Implemented` |
| "implemented" | No | `Missing` |
| "partial" | Any | `Partial` |
| Not in registry | Any | `Missing` |

## Contract Scaffolding (FJ-1352)

Generate YAML stubs for missing contracts:

```rust
use forjar::core::store::contract_scaffold::{scaffold_contracts, write_stubs};

let stubs = scaffold_contracts(&missing_requirements, "author-name");
let written = write_stubs(&stubs, output_dir)?;
// Skips files that already exist (no overwrite)
```

## GC Sweep Execution (FJ-1365)

Bridges mark-and-sweep analysis to actual filesystem deletion:

```rust
use forjar::core::store::gc_exec::{sweep, sweep_dry_run, dir_size};

// Dry run first
let dry = sweep_dry_run(&gc_report, store_dir);
println!("Would free {} bytes", dry.iter().map(|d| d.size_bytes).sum::<u64>());

// Execute sweep with path traversal protection
let result = sweep(&gc_report, store_dir)?;
println!("Freed {} bytes, {} entries removed", result.bytes_freed, result.removed.len());
```

### Safety Features

- Path traversal validation (prevents `blake3:../../etc` attacks)
- Journal logging before deletion (for recovery)
- Continues on partial failure (collects per-entry errors)

## Registry Push Commands (E14)

OCI Distribution Spec v1.1 compliant push operations:

```rust
use forjar::core::store::registry_push::*;

// Generate curl commands for OCI push protocol
let head = head_check_command("ghcr.io", "org/app", "sha256:abc");
let init = upload_initiate_command("ghcr.io", "org/app");
let complete = upload_complete_command(url, "sha256:abc", "/tmp/blob");
let manifest = manifest_put_command("ghcr.io", "org/app", "v1.0", "/tmp/manifest.json");
```

## Mutation Scripts

Generate mutation test scripts for drift detection validation:

```rust
use forjar::core::store::mutation_runner::{mutation_script, applicable_operators};

let ops = applicable_operators("file"); // DeleteFile, ModifyContent, ChangePermissions, CorruptConfig
let script = mutation_script(MutationOperator::DeleteFile, "nginx-conf");
// rm -f "${FORJAR_SANDBOX:-}/etc/forjar/nginx-conf"
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_contract_gc_registry.rs` | 31 | ~310 |

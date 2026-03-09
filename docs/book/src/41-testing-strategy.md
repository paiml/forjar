# Testing Strategy & Defect Analysis

Forjar implements a multi-level testing framework for infrastructure recipes, plus defect analysis lints that detect common IaC anti-patterns.

## Resource Coverage Model (FJ-2605)

Six levels of testing maturity, each subsuming the previous:

| Level | Name | Requirement |
|-------|------|-------------|
| L0 | No tests | Resource untested |
| L1 | Unit tested | Codegen script and planner action verified |
| L2 | Behavior spec | YAML `.spec.yaml` with verify commands |
| L3 | Convergence tested | Apply → verify → reapply → verify in sandbox |
| L4 | Mutation tested | All applicable mutations detected |
| L5 | Preservation tested | Pairwise preservation with co-located resources |

```rust
use forjar::core::types::{CoverageLevel, CoverageReport, ResourceCoverage};

let entries = vec![
    ResourceCoverage { resource_id: "nginx-pkg".into(), level: CoverageLevel::L4, resource_type: "package".into() },
    ResourceCoverage { resource_id: "config".into(), level: CoverageLevel::L3, resource_type: "file".into() },
];
let report = CoverageReport::from_entries(entries);
println!("Min: {}, Avg: {:.1}", report.min_level, report.avg_level);
assert!(report.meets_threshold(CoverageLevel::L3));
```

## Behavior Specs (FJ-2602)

YAML-based assertions for expected system state:

```yaml
name: nginx web server
config: examples/nginx.yaml
machine: web-1
behaviors:
  - name: nginx is installed
    resource: nginx-pkg
    state: present
    verify:
      command: "dpkg -l nginx | grep -q '^ii'"
      exit_code: 0
      retries: 3
      retry_delay_secs: 2

  - name: config valid
    resource: nginx-conf
    verify:
      command: "nginx -t"
      exit_code: 0

  - name: idempotent apply
    type: convergence
    convergence:
      second_apply: noop
      state_unchanged: true
```

Verify commands support: exit code, stdout match, stderr substring, file existence, port open checks, and retry logic.

## Mutation Testing (FJ-2604)

Eight mutation operators per resource type:

| Operator | Applies To | Effect |
|----------|-----------|--------|
| DeleteFile | file | Remove managed file |
| ModifyContent | file | Change file content |
| ChangePermissions | file | Alter mode/owner |
| StopService | service | Stop via systemctl |
| RemovePackage | package | Uninstall package |
| KillProcess | service | Kill process |
| UnmountFilesystem | mount | Unmount filesystem |
| CorruptConfig | file | Partial config modification |

Grade scale: A (≥90%), B (≥80%), C (≥60%), F (<60%).

## Design by Contract (FJ-2200-2203)

Four-tier verification:

1. **Runtime contracts** — `debug_assert!` on critical-path invariants
2. **Kani harnesses** — Bounded model checking on real production functions
3. **Verus proofs** — Formal idempotency/convergence/termination proofs
4. **Structural enforcement** — Handler hash invariant via `debug_assert_eq`

```rust
use forjar::core::types::{HandlerAuditReport, HashInvariantCheck};

let report = HandlerAuditReport {
    checks: vec![
        HashInvariantCheck::pass("pkg", "package", "blake3:abc"),
        HashInvariantCheck::fail("cron", "cron", "blake3:a", "blake3:b", "schedule hash"),
    ],
    exemptions: vec![],
};
println!("{}/{} passed", report.pass_count(), report.checks.len());
```

## Defect Analysis Lints (FJ-3000-3040)

### FJ-3000: Semicolon Chain Detection

Semicolons mask exit codes — `cmd1 ; cmd2` runs cmd2 even if cmd1 fails.

```yaml
# BAD: semicolon masks failures
resources:
  build:
    type: task
    command: "cd /app ; make install"

# GOOD: && fails fast
resources:
  build:
    type: task
    command: "cd /app && make install"
```

### FJ-3030: Nohup LD_LIBRARY_PATH

Nohup with absolute binary paths may fail at runtime if shared libraries are in non-standard paths.

```yaml
# FLAGGED: no LD_LIBRARY_PATH
command: "nohup /opt/cuda/bin/train &"

# SAFE: LD_LIBRARY_PATH set
command: "LD_LIBRARY_PATH=/opt/cuda/lib nohup /opt/cuda/bin/train &"
```

### FJ-3040: Nohup + Sleep + Health Check

The `nohup ... & sleep N; curl` pattern has race windows. Use task_mode service with health_check instead.

## Falsification

```bash
cargo run --example testing_defects_falsification
```

Key invariants verified:
- Coverage levels L0 < L1 < ... < L5 (strict ordering)
- Behavior reports correctly aggregate pass/fail counts
- Mutation score grade is monotonically non-decreasing
- Semicolon detection respects quoting (single/double)
- Nohup lint flags absolute paths without LD_LIBRARY_PATH
- Handler audit report tracks pass/fail/exempt counts

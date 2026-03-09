# Generation Diffs, Tasks, Mutations & Observability

Falsification coverage for FJ-2003, FJ-2700, FJ-2605, FJ-2604, and FJ-2301.

## Generation Diffs (FJ-2003)

Cross-generation resource comparison with set-diff semantics:

```rust
use forjar::core::types::*;

let diffs = diff_resource_sets(&from_set, &to_set);
let gen_diff = GenerationDiff { gen_from: 12, gen_to: 15, machine: "intel".into(), resources: diffs };
assert_eq!(gen_diff.added_count(), 1);
assert!(gen_diff.has_changes());
println!("{}", gen_diff.format_summary());
```

Builder API: `ResourceDiff::added()`, `::modified()`, `::removed()`, `::unchanged()` with `.with_hashes()` and `.with_detail()`.

## GPU Scheduling (FJ-2703)

Multi-GPU parallel scheduling with CUDA device assignment:

```rust
use forjar::core::types::GpuSchedule;

let schedule = GpuSchedule::round_robin(&["train", "eval", "infer"], 4);
assert_eq!(schedule.cuda_visible_devices("train"), Some("0".into()));
assert!(!schedule.fully_utilized());
```

## Barrier Tasks (FJ-2704)

Multi-machine synchronization barriers:

```rust
use forjar::core::types::BarrierTask;

let mut barrier = BarrierTask::new("sync", vec!["gpu-1".into(), "gpu-2".into()]);
barrier.mark_complete("gpu-1");
assert!(!barrier.is_satisfied());  // still waiting for gpu-2
```

## Coverage Levels (FJ-2605)

Five-level testing maturity model (L0-L5):

```rust
use forjar::core::types::*;

let report = CoverageReport::from_entries(entries);
assert!(report.meets_threshold(CoverageLevel::L2));
println!("{}", report.format_report());
```

## Mutation Testing (FJ-2604)

Infrastructure mutation operators with scoring:

```rust
use forjar::core::types::*;

let report = MutationReport::from_results(results);
assert_eq!(report.score.grade(), 'A');  // >=90% detected
println!("{}", report.format_summary());
```

Eight operators: DeleteFile, ModifyContent, ChangePermissions, StopService, RemovePackage, KillProcess, UnmountFilesystem, CorruptConfig.

## Observability (FJ-2301)

Verbosity levels, log filtering, truncation, and structured output:

```rust
use forjar::core::types::*;

let v = VerbosityLevel::from_count(2);  // -vv
assert!(v.shows_scripts());

let filter = LogFilter::for_machine("intel");
assert!(filter.has_criteria());

let trunc = LogTruncation::default();  // 8KB first + 8KB last
assert!(trunc.should_truncate(20_000));

let path = RunLogPath::new("state", "intel", "r-abc");
assert_eq!(path.resource_log("pkg", "apply"), "state/intel/runs/r-abc/pkg.apply.log");
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_diff_task_coverage.rs` | 38 | ~355 |
| `falsification_mutation_observability.rs` | 37 | ~330 |

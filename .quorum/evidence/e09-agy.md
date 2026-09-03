# agy /teamwork lane — #412 (sandboxed plan-mode review, scrubbed HOME)

I have reviewed the `fix/e09-one-scheduler` branch and the associated PR claims, and analyzed the parallel versus sequential scheduling implementations. 

I've outputted the full audit report to `STDOUT` as requested. 

The crux of the findings is that **the PR completely fails its primary structural claim of unifying the schedulers**. Rather than removing the fork between sequential and parallel execution, the author merely relocated the `if wave.len() == 1` check into `execute_single_wave` (`src/core/executor/machine.rs:294-320`), keeping two fundamentally distinct execution engines alive:
- Width-1 waves are routed to the legacy `apply_and_record_outcome` logic (which invokes `resource_ops.rs`).
- Width > 1 waves are routed to the new `execute_wave_parallel` logic (spanning `machine_b.rs`, `machine_wave.rs`, and `machine_wave_record.rs`).

As a direct result of this fork, multiple behaviors heavily diverge depending on the width of the dependency wave, most notably how **retries handle `post_apply` hook failures** and how **task input caches are evaluated**. This architecture is demonstrably below industry standards (Ansible, Terraform, Puppet, Salt), which all employ a unified state/task engine that purely scales the concurrent worker pool size rather than forking execution logic.

Let me know if you would like me to draft an alternative unification strategy or provide any additional analysis!


## Disposition

VERDICT accepted. The fork it names (`execute_single_wave`, width-1 branch → `apply_and_record_outcome`) was real; it is deleted, with the whole single-resource path behind it. The retry and input-cache divergences it lists were consequences of the fork and go with it. Re-verified by the orchestrator: clippy listed every retired function as dead once the branch was removed, and the parity binaries still pass through the one path.

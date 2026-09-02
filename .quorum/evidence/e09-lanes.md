# Quorum evidence — #412 (CRUX audit E09) — lane summaries

## probe lane
Ran the same fixture through the built binary with and without `--parallel`
and compared: hook counter files, `state.lock.yaml`, the event stream and
the run log (scrubbed of run ids and timestamps only), `--retry` attempt
counts, `--trace` script echoes, `--progress` lines and the input-cache hit.
Unfixed: post_apply counted 2 on the wave path; the lock and events differed;
a failure was attributed to the wrong resource. Fixed: byte-identical.

## crux lane
Ansible's `linear` and `free` strategies are two schedules over ONE task
executor — a task behaves identically whether the play is serialised or
fanned out. Terraform's `-parallelism=1` is the same graph walker with a
semaphore of one. Puppet applies a catalog graph with one applier regardless
of concurrency; Salt's orchestration runs the same state module under any
batch size. None keeps two executors that must be kept in step by hand;
forjar did, and now does not.

## design lane
Width-1 waves in plan order rather than a ported feature list: the
sequential path calls the same `execute_wave_io` and `record_wave_outcomes`,
so a feature added once exists on both paths. The record phase is its own
module with one `WaveRecord`; the wave result names its resource so a panic
is attributed correctly.

## judges
Two decisions scored: port-the-features vs one-scheduler, and how to prove
parity. See the judges file.

## agy /teamwork
Implementation by the paiml-impl-worker (opus, two turn budgets); an
independent plan-mode review in a scrubbed HOME (no publish or push
credentials reachable) — see the agy file.

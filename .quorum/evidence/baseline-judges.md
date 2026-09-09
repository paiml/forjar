# Quorum evidence — PMAT-219 — adjudicated claims

## CONFIRMED

1. [ownership] THE MACHINE THAT OWNS THE FILE ANSWERS — a remote baseline is read from that machine, and the writer no longer hashes this host on its behalf.
- evidence: the branch that decided it sat at src/core/executor/helpers.rs:113 at the merge base, reading through the transport only for `is_container_transport()`. Measured before the fix: the recorded `content_hash` was byte-for-byte `hash_file` of the controller's own file at the declared path, which is why drift's actual was right and its expected was a third file.

2. [honesty] ABSENT BEATS WRONG — a baseline that could not be read is not recorded.
- evidence: `locked_file_target` at src/tripwire/drift/file.rs:171 returns `None` without a `content_hash`, so the file path declines to judge rather than producing a confident verdict about a file this host never saw. The case is at src/core/executor/tests_baseline_transport.rs:96, a file this branch adds.

3. [no-regression] LOCAL IS UNCHANGED — a local machine still records the hash of the file it manages, with the same digest identity it always did.
- evidence: the case at src/core/executor/tests_baseline_transport.rs:113 passes both before and after, which is what makes it a guard rather than a demonstration. The two digest forms were measured to agree for ordinary text and to differ for an empty file and for non-UTF-8 bytes; keeping `hash_file` is what protects those two kinds of existing local baseline.

## REFUTED

4. [predicate] TWO DEFINITIONS OF LOCAL, DISAGREEING ON PEPITA — the fix used `machine_is_local`, which excludes a container and not a namespace.
- corrected: all three lanes found it. `transport::controller_answers_for` is now the one definition, called by the writer and delegated to by drift's `reads_the_controller` at src/tripwire/drift/file.rs:251, which resolves at the merge base as the second, stricter definition. The pepita case at src/core/executor/tests_baseline_transport.rs:140 fails when the predicate is reverted and the other five stay green. The hole `machine_is_local` still has for its other four callers is filed as forjar#495 rather than widened here.

5. [protocol] TWO READERS, DISAGREEING ON `__DIR__` — the writer's plain `cat` disagreed with drift's directory protocol.
- corrected: one lane found it, grounding asserted, and it holds. A file whose entire content is the literal `__DIR__` was digested as that string by the writer and as an `ls -la` listing by the reader. Both call `remote_path_digest` now, re-exported rather than opening the module. A behavioural test would need a reachable non-local machine, which this suite deliberately lacks, so the invariant is pinned at the source level and the receipt says so instead of implying coverage it does not have.

6. [scope] AN UNRELATED COMMENT — `mod.rs` carried a rewrite the ticket did not ask for.
- corrected: it existed only to free two lines in a file sitting exactly on the 500-line limit. Reverted, and the module declaration moved into `tests_edge_details.rs` via `#[path]`, which costs `mod.rs` nothing. Three lanes named it and none of them accepted the reason, correctly: a line budget is not a licence to edit unrelated prose.

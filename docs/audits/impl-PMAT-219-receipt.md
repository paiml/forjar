# Implementation receipt — PMAT-219 — the baseline a lock recorded for a file on a remote machine was a hash of the CONTROLLER's copy of that path, so drift compared a correct actual against a third file, permanently

verdict: DONE — the baseline is read from the machine that owns the file, through the same reader drift uses and behind the same predicate drift uses, with the local arm preserved for the two digest cases that genuinely differ. Closes forjar#485, whose mechanism the operator supplied after my first close asserted the wrong one.

## Identity

| field | value |
|---|---|
| ticket | PMAT-219 (kind: code) |
| issue | forjar#485, refiled by the operator with the mechanism |
| branch | PMAT-218-details-content-hash-asks-the-machine |
| base | 3a8b415e |
| model gate | `model=opus class=opus decision=admit basis=file` |

## I closed this issue on a mechanism I had not measured

My first close said #485 was #488: that drift fell to a lock-only arm and hashed the local filesystem, so the two hashes were of two different computers' files. Plausible, and wrong. The operator's four measurements contradicted it — `Actual` was correct, so the read path was fine, and #488's fix landing did not make it go away.

The defect is on the **write** side and it is one branch:

```rust
let hash = if machine.is_container_transport() {
    transport::exec_script(machine, &format!("cat '{path}'"))   // ask the machine
} else {
    hasher::hash_file(std::path::Path::new(path)).ok()          // hash THIS host
};
```

A container was read through the transport; **everything else, including plain SSH, hashed the controller** at the declared path. A resource declaring `/home/<user>/.bashrc` recorded the workstation's `.bashrc` as its baseline. That is forjar#305's root cause, fixed on the read side and left here.

It also explains what a write-once field would not: the field is not frozen, it is recomputed every apply from a file that never changes, which is why the stored value matched neither the declaration nor the live file. The operator's 4-of-4 discriminator is the tell — a resource whose path also exists on the controller gets a wrong hash, one whose path does not gets `None` and converges.

## Three review lanes refused the first fix, and all three were right

| finding | lanes | outcome |
|---|---|---|
| `machine_is_local` excludes a container but **not** a pepita namespace, while drift's predicate excludes both — so apply would hash the controller while drift asked the namespace | 1, 2, 3 | **fixed**: one shared `transport::controller_answers_for`, which drift now delegates to |
| the migration argument was too broad | 1, 2 | **measured and narrowed**, below |
| an unrelated comment rewrite in `mod.rs` | 1, 2, 3 | **reverted**; it existed only to free two lines, and the module declaration moved to `tests_edge_details.rs` via `#[path]` |
| the `__DIR__` protocol collision | 1 | **fixed**: one shared reader |
| a genuinely local machine with a non-loopback addr | 3 | addressed by the shared predicate: whatever it answers, both sides answer it |
| stale remote entries self-heal on one apply; the 192.0.2.1 test is bounded by `ConnectTimeout=5` | 2 | affirmed, no change |

### The migration argument, measured

I claimed the local arm had to keep `hash_file` because `hash_string_or_sentinel` is "a different digest identity". Two lanes said that is false for ordinary files. They were right, and no lane could run it, so I did:

```
ordinary        file=f92ca07e3206 str=f92ca07e3206 same=true
no_trailing_nl  file=6437b3ac3846 str=6437b3ac3846 same=true
empty           file=af1349b9f5f9 str=d70cbc1aa622 same=false
non_utf8        file=2a7c022c5f18 str=6329f2bdda5d same=false
```

The asymmetry buys exactly two things: an empty file keeps its real digest instead of the sentinel, and a non-UTF-8 file keeps its raw-byte digest instead of one taken after `from_utf8_lossy`. Every existing local baseline of those two kinds would otherwise flip to false drift. That is a smaller claim than the one I wrote, and it is the one the code now states, with the numbers in it.

## The invariant this settled on

Apply writes the baseline that drift reads. They must agree about **which machine answers** and about **how the bytes are read**. Both had two implementations and both pairs disagreed:

- predicate: `machine_is_local` (apply) vs `reads_the_controller` (drift) — differing on pepita
- reader: `cat '{path}'` (apply) vs the `__DIR__` protocol (drift) — differing on a directory and on a file whose content is literally `__DIR__`

There is now one of each, and the second is re-exported rather than the module being opened.

## Verification, all my own runs

| command | result |
|---|---|
| `cargo test --lib baseline_transport` | 6 passed |
| `cargo test --workspace` | **exit 0, 311 binaries** |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets --all-features -- -D warnings` | exit 0 |

Mutations, each killing exactly its own case: the predicate reverted to `machine_is_local` fails the pepita case alone; the fix reverted entirely fails the two original cases and leaves the local one green.

No lane ran cargo — the delegate forbade it after measuring 213G of build output against 220G free — so every number here is an orchestrator rerun.

## What the fleet should expect

The fix corrects what gets **written**. Existing lock entries keep their wrong `content_hash` until the next apply on that machine, so false drift persists for one cycle and then clears. The operator's close condition is the right one and still holds: grep the lock, not the drift output.

## Gaps

- `machine_is_local` keeps its pepita hole for its other four callers, in the build-I/O probe, the pre-plan probe, output verification and the exec dispatch. Filed as **forjar#495** with the call sites named, rather than widened here under code this ticket has not measured.
- Reading a file through a shell is lossy for non-UTF-8 bytes. That limit is now symmetric between the two sides rather than one-sided, but it is still a limit and is named in the code.
- No behavioural test covers the `__DIR__` collision, because it needs a reachable non-local machine. The invariant is pinned at the source level instead, and the receipt says so rather than implying coverage it does not have.

IMPL-PMAT-219-RECEIPT-END

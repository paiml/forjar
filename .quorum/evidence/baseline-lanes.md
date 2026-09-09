# Quorum evidence — PMAT-219 — lane rulings

Three sandboxed review lanes on the diff at `836554f6`, each in its own clone, base pinned by SHA rather than by name because local `main` was behind `origin/main` and a `--shared` clone would have shown the lanes an extra commit. Verdicts 3 of 3 FAIL, `partial=false`.

## The finding all three shared

`machine_is_local` excludes a container and **not** a pepita namespace:

```rust
!machine.is_container_transport() && (addr == "127.0.0.1" || addr == "localhost" || is_local_addr(addr))
```

while drift's `reads_the_controller` excludes both, and says why: a pepita machine commonly declares a loopback address and its files live inside the namespace. So the first fix left apply hashing the controller for a pepita machine while drift asked the namespace — the original defect surviving one transport over, with the two sides now permanently disagreed rather than merely both wrong.

`machine_is_local`'s own doc-comment is the argument against what it was being used for: *"One definition, one place — the exec path and the probe path must never disagree about what 'local' means."* There were two definitions and they disagreed.

Fixed by making one: `transport::controller_answers_for`, called by the baseline writer and delegated to by `reads_the_controller`. Its other four callers are left alone and filed as forjar#495 rather than widened under code this ticket has not measured.

## The finding two lanes shared, which measurement narrowed rather than dismissed

The first fix justified keeping `hash_file` on the local arm by claiming `hash_string_or_sentinel` is "a different digest identity". Lanes 1 and 2 said that is false for ordinary files. Neither could run anything — the delegate forbade cargo after measuring 213G of build output against 220G free — so the orchestrator measured it:

```
ordinary        file=f92ca07e3206 str=f92ca07e3206 same=true
no_trailing_nl  file=6437b3ac3846 str=6437b3ac3846 same=true
empty           file=af1349b9f5f9 str=d70cbc1aa622 same=false
non_utf8        file=2a7c022c5f18 str=6329f2bdda5d same=false
```

The lanes were right about the broad claim and the narrow one survives: the asymmetry buys an empty file its real digest instead of the sentinel, and a non-UTF-8 file its raw bytes instead of a lossy decode. Those are exactly the local baselines that would otherwise flip to false drift. The code now states the narrow claim with the numbers in it.

## The single-lane finding that mattered most

Lane 1, grounding `asserted` and correct: the writer used a plain `cat '{path}'` while drift uses `if [ -d ]; then echo __DIR__; else cat; fi` and digests `ls -la` on seeing that marker. The two disagree on a directory, and on a file whose entire content is the literal `__DIR__` — a permanent, confident mismatch on a resource that is perfectly converged.

Both sides call one reader now. Its lossiness is shared too, which is the honest outcome: reading a file through a shell loses non-UTF-8 bytes, and that limit is now symmetric where before only one side had it.

## The one all three shared that was purely mine

`src/core/executor/mod.rs` carried a rewritten comment about `--force` and `--refresh`. It existed only to free two lines for a module declaration in a file sitting exactly on the 500-line limit. Reverted; the declaration moved to `tests_edge_details.rs` via `#[path]`, which needs no room in `mod.rs` at all.

## What the lanes disagreed about

Lane 2 answered claim 5 in the negative — the two hash forms are never cross-compared — and lane 1 gave two concrete paths where they are. Lane 1 was right about the `__DIR__` one. That disagreement was the sharpest thing in the round and a reduced consensus would have shown neither side.

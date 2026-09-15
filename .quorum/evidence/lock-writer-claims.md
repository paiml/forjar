# PMAT-565 — the claims put to the round

The branch makes the per-machine lock name the binary that WROTE it, on
every write, and keep the binary that created it: `state::save_lock`
serialises `stamped_for_write(lock)` — `generator` := this binary,
`created_by` := the value replaced, once — and `forjar lock --restamp` walks
a state dir and writes every stale lock through it in one run. Measured
before: four fleet locks under one 1.30.0 binary said 1.1.1, 1.13.1, 1.27.0
and 1.10.0 (paiml/infra#605, third signature).

Three lanes, read-only, sandboxed, against self-contained clones of the PR
worktree at `a0070731`, diffed against the base branch
`PMAT-564-drift-declines-on-empty-scope`, dispatched in one message with
`--not-before` pinned and `out_dir` keyed by ticket AND session id. Models
named in the brief (gemini-3.1-pro-high twice, gemini-3.7-flash-high once;
gemini-3.8-flash-high had 503'd all day).

The questions, identical to every lane:

1. Every writer: is there any path that writes a per-machine lock WITHOUT
   `save_lock` — a bare `serde_yaml_ng::to_string` + `fs::write`?
2. `created_by` semantics: a fresh lock, an empty generator, `lock-audit`'s
   `starts_with("forjar")`.
3. Compatibility: an older forjar parsing the file; the `.b3` sidecar under
   restamp; encrypted `.yaml.age` locks.
4. What `lock --restamp` walks and what it misses — nested state dirs,
   `forjar.lock.yaml`, `.yaml.age` — and whether that is stated.
5. The contract, CHANGELOG and book: quote any false sentence; do the
   falsifiers' mutations turn their tests red?
6. The 137 scripted `created_by: None` insertions: any misplaced into a
   GlobalLock or StackStamp literal?

The acceptance command every lane was told to run:
`falsification_lock_names_its_writer`, `cli::lock_restamp core::state`,
`falsification_contract_citations_resolve`.

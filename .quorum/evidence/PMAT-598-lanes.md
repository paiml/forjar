# Lanes — tool-requiring jobs on bare metal (PMAT-598)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

Two families per the standing ruling: 2 agy + 1 claude. Read-only lanes, diff
pasted inline, no repository, shell, network or credential, no build. The Claude
lane could additionally run read-only `git show` and query the runner API.

| round | head | agy gemini-3.1-pro-high | agy gemini-3.1-pro-low | claude sonnet |
|---|---|---|---|---|
| 1 | 10ff8755 | FAIL conv-bc46dd79 | FAIL conv-afbce85a | FAIL |
| 2 | cf9990b6 | FAIL conv-7d37392b | FAIL conv-c4b2e500 | PASS |
| 3 | 62f42d1d | PASS conv-3607d864 | PASS conv-36b1a491 | FAIL |

## What moved the diff, by round

- **1** — all three: the test declared one mutation while the commit claimed four.
  Claude: stress.yml and mutation.yml run unfiltered lib tests on the same label
  set; and `yoga-build` is ephemeral and not registered today.
- **2** — Claude and agy-low independently: the substring detector missed
  `cargo test --locked --lib`, bare `cargo test`, `--workspace`, `--all-targets`,
  `cargo nextest run`. agy-high: a filter after `--` was over-pinned. agy-low:
  mutation 4 reddens two tests, not "exactly its own arm".
- **3** — Claude: libtest value flags after `--` (`--skip slow`) were read as
  filters, a false negative introduced by round 2's fix. agy-high: a stale
  function count in the commit message.

## A lane was wrong, and it matters which way

Round 3's Claude lane reported `cargo test --lib && cargo test --doc` as
classified correctly. Adding it as an asserted case failed immediately: the
tokenizer read `&&` as a positional filter. So the round that found the value-flag
defect also asserted a false negative was fine. Lanes are right about what they
can compute from the text and can be wrong about a trace they report — the
defence is to turn every trace into an asserted case.

## Lane errors

agy-low's round-3 response carried a valid verdict object followed by 2194 bytes
of trailing text; decoded with `raw_decode`, it is a PASS, not a no-verdict.
Round 2's agy-high refuted C6 because its brief carried the diff without the
commit messages the claim referred to — a gap in the brief, fixed for round 3.

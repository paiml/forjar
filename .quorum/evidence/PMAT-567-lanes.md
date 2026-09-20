# Lanes — lint toolchain override (PMAT-567)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

Two families per the standing ruling: **2 agy + 1 claude**. Lanes are read-only
over the diff pasted inline — no repository, no shell, no network, no credential,
no build.

| round | head | lane 1 (agy) | lane 2 (agy) | lane 3 (claude) |
|---|---|---|---|---|
| 1 | 949ca86c | gemini-3.1-pro-high FAIL | gemini-3.1-pro-low FAIL | sonnet PASS |

## What each lane was right about

- **gemini-3.1-pro-high** — `cut -d- -f1` false-passes a beta toolchain
  (`1.93.0-beta.1` reduces to `1.93.0`); and under `set -euo pipefail` a broken
  rustup aborts the script before the named refusal can print.
- **gemini-3.1-pro-low** — `rustup override unset` removes the override for the
  CURRENT directory only, so one set on a parent survives it; and date-pinned
  channels (`nightly-2026-01-01`) reduce to `nightly`.
- **claude sonnet** — a `channel = "stable"` pin never matches a resolved
  version, so the comparison false-FAILS on the most ordinary pin there is; and
  `toolchain_step()` sliced three steps wide while its name claimed one.

Two lanes reached the same conclusion by different counterexamples, which is why
the repair replaced the approach rather than patching the parser.

## The finding no lane made, which the mutation did

Re-running the mutation the test itself declared showed the test was VACUOUS:
`step.contains("rustup override unset")` is satisfied by the step's own comments
and by its error message, so replacing the real command left it green. Found by
running the address rather than trusting it.

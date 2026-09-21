# Lanes — the MSRV job left a toolchain override behind (PMAT-566)

THIS TABLE IS THE ONLY PLACE ROUNDS ARE COUNTED AND HEADS ARE NAMED.

2 agy + 1 claude. The Claude lane was permitted one write: a scratch directory for
an empirical precedence test, removed afterwards.

| round | head | agy gemini-3.1-pro-high | agy gemini-3.1-pro-low | claude sonnet |
|---|---|---|---|---|
| 1 | 71376ed1 | NO-VERDICT, then re-run: FAIL conv-48905a36 | FAIL conv-1696552f | PASS |

## The precedence, measured rather than recited

The Claude lane set a directory override to 1.91.0 in a scratch directory whose
rust-toolchain.toml named 1.95.0: `rustup show active-toolchain` reported 1.91.0
"(directory override for …)". With `RUSTUP_TOOLCHAIN=1.97.1` added it reported
1.97.1 "(overridden by environment variable RUSTUP_TOOLCHAIN)". And with
RUSTUP_TOOLCHAIN naming an UNINSTALLED toolchain, `rustup override unset` still
exited 0 — which is what the msrv step order needs, since it unsets before it
installs.

## Lane error, and what it cost

agy-high's first run returned a SUCCESS envelope with an empty response: it tried
to run a command to test the precedence itself, and headless sandbox mode denied
it. Counted as no review, never as a pass. Re-run once with the brief telling it
not to attempt tools; it returned a verdict. NOT re-run with
`--dangerously-skip-permissions` — a lane with that flag has run `cargo publish`
on this fleet.

## Side effect, reversed

The Claude lane's probe ran `rustup toolchain list` with RUSTUP_TOOLCHAIN naming
1.70.0, and rustup auto-installed it. The lane reported that rather than acting
on it; the toolchain was uninstalled afterwards, on a box at 93% disk.

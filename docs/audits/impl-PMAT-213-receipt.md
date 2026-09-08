# Implementation receipt — PMAT-213 — the cargo provider's install action repaired PATH and its check did not, so forjar installed a crate and then reported it missing for ever, re-bootstrapping rustup on every apply

verdict: DONE — one shared, idempotent PATH prelude is emitted by all three cargo sites (install action, `package_check`, drift observable), proven by a test that EXECUTES the emitted check against a toolchain reachable only through `$CARGO_HOME/bin` and killed by two mutations. Closes #489.

## Identity

| field | value |
|---|---|
| ticket | PMAT-213 (kind: code) |
| issue | #489 |
| branch | PMAT-213-cargo-check-path, merged into release-1.27.0-fleet-p0 |
| worker | `paiml-impl-worker` (opus) in its own worktree; every claim below re-run by the orchestrator |

## The asymmetry, and that forjar creates the condition itself

The install action repairs its own environment (`export PATH="$HOME/.cargo/bin:$PATH"` inside its bootstrap guard). `package_check` and the drift observable did not, and both ask `cargo install --list` and `command -v <bin>`.

On a host where cargo is not on the NON-INTERACTIVE PATH that is fatal in a quiet way: forjar installs the crate, then cannot see it, for ever. The operator measured it on yoga — `~/.cargo/bin/rg` present and executable, `rg --version` reporting ripgrep 15.1.0, `.crates.toml` valid and listing it, `cargo install --list` listing it, and forjar reporting `missing:ripgrep`, likewise `bat` and `fd-find`.

forjar creates that host state itself. It runs `rustup-init -y --no-modify-path`, which is the right choice — do not edit the user's shell files — but it means the only thing putting cargo on PATH is the install action's own local export. Ubuntu's stock `~/.bashrc` returns at line 8 when not interactive and rustup appends its env line at the bottom, so a non-interactive `ssh host 'command -v cargo'` finds nothing.

The second symptom has the same cause: the install action's `command -v cargo ||` guard kept missing, so every apply re-ran the rustup installer over a toolchain that was already there.

## What changed

`cargo::path_prelude()` is now the single emitter, used by all three sites:

```sh
case ":${PATH:-}:" in
  *":${CARGO_HOME:-$HOME/.cargo}/bin:"*) ;;
  *) export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:${PATH:-}" ;;
esac
```

Idempotent (three invocations prepend once), safe under `set -u` via `${PATH:-}`, and it honours `CARGO_HOME` the way the neighbouring `_CARGO_BIN` and `_CRATES_TOML` already do. No shell rc file is touched and rustup keeps `--no-modify-path`.

The bootstrap guard needed no explicit change and that was CONFIRMED BY EXECUTION rather than by reading: the emitted prologue was run with cargo present only under `$CARGO_HOME/bin`, printing `GUARD-DID-NOT-FIRE` with the prelude and `GUARD-FIRED-RUSTUP-RERUN` with the prelude lines deleted.

## Falsification

`tests/falsification_cargo_check_sees_what_install_wrote.rs`, nine cases. Two of them EXECUTE the emitted scripts against a stub toolchain; the rest assert on comment-stripped text anchored to a `PATH="` assignment naming cargo's bin directory.

Two mutations, both killed: `path_prelude()` returning empty (7 of 9 fail, leading message `forjar reported a crate it installed as missing ... stdout: missing:ripgrep`), and `path_prelude()` returning the repair as a COMMENT (the same 7 fail, including `every_repair_is_code_and_not_a_comment`).

That second mutation is deliberate. This repository shipped a shape gate one day earlier whose assertion was satisfied by the comment explaining it — RULE 8 of `tests/falsification_release_workflow_shape.rs`, caught by a merge-review quorum and fixed in #484. The worker was briefed with that failure by name and built the defence into every text assertion here.

## Consequence worth naming

The prelude PREPENDS, exactly as the install action does, so it shadows a cargo an operator put on PATH by other means. That is deliberate — the check and the apply must resolve the same cargo, which is the ticket's whole thesis — and it broke one pre-existing test whose fixture put a fake cargo on PATH without setting `CARGO_HOME`. That test now sets `CARGO_HOME` to its own fixture and moved to `tests/falsification_cargo_observable_binary_status.rs`, because `src/resources/tests_package_b.rs` stood at 537 lines and the file-health ratchet forbids growth above 500. It is 471 lines now.

## Orchestrator verification

`cargo test --test falsification_cargo_check_sees_what_install_wrote` re-run by the orchestrator: 9 passed. Full suite on the merged release branch: 304 test binaries, 0 failures.

IMPL-PMAT-213-RECEIPT-END

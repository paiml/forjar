# CLI Reference Appendix

This appendix documents subcommands not yet covered in the main
[CLI Reference](06-cli.md). Each entry is a minimal stub: purpose and usage.
A CI parity test (`tests/doc_cli_parity.rs`) ensures every subcommand in the
`Commands` enum appears somewhere in this book, so new commands must be added
here (or in a full chapter) before they can merge.

## Build Semantics

### forjar make

Build the named goals and their transitive `depends_on` prerequisites, and
nothing else — what `make <goal>` means.

```bash
forjar make [GOALS...] [-f forjar.yaml] [-n] [-B] [-j <N>] [-p KEY=VALUE] [--yes]
```

The goal closure is downward-closed, so a targeted build can never run against
an unconverged prerequisite. This is what distinguishes it from `apply -r`,
which is exact-match with no closure (make's `-o` semantics), and from
`--subset`/`--exclude`, whose patterns can cut a resource out from under a
dependent.

With no goals it is equivalent to `forjar apply`. An unknown goal is an error
listing the known targets, never a silent no-op.

Flags mirror make: `-n` dry run, `-B` always-make, `-j` parallel jobs.

Targets marked `phony: true` name an ACTION rather than a file. They are
excluded from bulk `apply`/`plan` and run unconditionally when named as a goal,
which is what keeps a repeated `forjar apply` idempotent.

### forjar import-makefile

Import a single-makefile, non-recursive build into a forjar config.

```bash
forjar import-makefile [MAKEFILE] [-o <OUTPUT>] [-m <MACHINE>]
```

Runs `make -p --trace -n -B` and joins the two streams it produces: the parsed
database (structure, with unexpanded recipes) and the trace (expanded commands).
Emits one `type: task` resource per target, with `output_artifacts` for file
targets, `phony: true` for `.PHONY` members, `task_inputs` for source
prerequisites, and `depends_on` for prerequisites that are themselves targets —
including order-only (`| dir`) prerequisites, which become edges and never
inputs.

Each logical recipe line is emitted inside its own subshell, reproducing make's
per-line shell isolation, so a `cd` on one line does not affect the next.

**It refuses rather than mistranslates.** Recursive make, `.ONESHELL`,
double-colon rules, VPATH, and GNU make older than 4.0 (which macOS still ships)
are detected and reported, and nothing is written. Review the generated config
before applying it: recipes are make's own expansion, and forjar injects
`set -euo pipefail` where make sets no shell options.

### forjar lsp

Run the forjar.yaml language server on stdio, for editor integration.

```bash
forjar lsp
```

Speaks LSP over stdin/stdout with `Content-Length` framing: diagnostics from the
same validator `forjar validate` uses, plus completion for resource types and
fields. Point your editor's LSP client at this command for `forjar.yaml` files.

## Config Analysis & Composition

### forjar stack-diff

Unified stack diff — compare two configs (resources, machines, params).

```bash
forjar stack-diff <FILE1> <FILE2> [--json]
```

### forjar config-merge

Merge two forjar config files into one.

```bash
forjar config-merge <FILE_A> <FILE_B> [-o <OUTPUT>] [--allow-collisions]
```

### forjar extract

Extract resources matching tag, group, or glob into a sub-config.

```bash
forjar extract -f forjar.yaml [--tags <TAG>] [--group <GROUP>] [--glob "web-*"] [-o <OUTPUT>]
```

### forjar query

Infrastructure query — search resources across config and state.

```bash
forjar query -f forjar.yaml [--pattern <P>] [--type <T>] [--machine <M>] [--tag <T>] [--live] [--json]
```

### forjar preservation

Preservation checking — verify resources that must never be destroyed.

```bash
forjar preservation -f forjar.yaml [--json]
```

## State & Lock Integrity

### forjar reseal

Regenerate BLAKE3 integrity sidecars from current lock contents (use when a
lock file and its `.b3` sidecar diverge).

```bash
forjar reseal [--file <LOCK> | --all | --machine <NAME>] [--dry-run]
```

### forjar generation

Manage state generations (Nix-style numbered snapshots): list, garbage-collect,
and diff.

```bash
forjar generation <list|gc|diff> [OPTIONS]
```

### forjar state-backend

Remote state backend operations — inspect backend keys.

```bash
forjar state-backend [--state-dir state] [--prefix <PREFIX>] [--json]
```

## Policy & Supply Chain

### forjar policy-coverage

Policy rule coverage analysis — which rules fire against the current config.

```bash
forjar policy-coverage -f forjar.yaml [--json]
```

### forjar policy-install

Install a compliance pack (e.g., `cis-ubuntu-22`, `nist-800-53`, `soc2`, `hipaa`).

```bash
forjar policy-install <PACK> [--output-dir policies] [--json]
```

### forjar sign

Recipe signing — sign or verify a recipe file (optionally post-quantum dual
signing).

```bash
forjar sign <RECIPE> [--verify] [--signer <ID>] [--pq] [--json]
```

## Multi-Stack Orchestration

### forjar multi-apply

Multi-config apply ordering — analyze cross-stack dependencies and report the
correct apply order.

```bash
forjar multi-apply -f <CONFIG> -f <CONFIG2> [--json]
```

### forjar stack-graph

Stack dependency graph across multiple config files.

```bash
forjar stack-graph -f <CONFIG> -f <CONFIG2> [--json]
```

### forjar parallel-apply

Parallel multi-stack apply with a bounded worker pool.

```bash
forjar parallel-apply -f <CONFIG> -f <CONFIG2> [--max-parallel 4] [--json]
```

## Agents & Registries

### forjar agent

Pull agent / hybrid push-pull enforcement — one-shot push by default, daemon
loop with `--pull`.

```bash
forjar agent -f forjar.yaml [--pull] [--interval 60] [--auto-apply] [--json]
```

### forjar agent-registry

Agent recipe registry — list agent recipes by category.

```bash
forjar agent-registry [--registry-dir <DIR>] [--category <CAT>] [--json]
```

### forjar catalog-list

Service catalog listing — browse the service catalog by category.

```bash
forjar catalog-list [--catalog-dir <DIR>] [--category <CAT>] [--json]
```

## Environments

### forjar environments

Manage named environments (dev, staging, prod): list, diff, rollback, and
history.

```bash
forjar environments <list|diff|rollback|history> [OPTIONS]
```

## Plugins

### forjar plugin

Manage WASM resource plugins: list, verify, init, install, build, run, remove.

```bash
forjar plugin <list|verify|init|install|build|run|remove> [OPTIONS]
```

## Provisioning & Distribution

### forjar iso-export

ISO distribution export — export config and state for offline installation.

```bash
forjar iso-export -o <OUTPUT_DIR> -f forjar.yaml [--include-binary] [--json]
```

### forjar import-brownfield

Brownfield state import — scan an existing machine and import discovered
resources into state.

```bash
forjar import-brownfield [-m localhost] [-s package -s file -s service] [-o <OUTPUT>] [--json]
```

### forjar dist

Generate distribution artifacts (installer script, Homebrew formula,
cargo-binstall metadata, Nix flake, GitHub Action, deb/rpm specs).

```bash
forjar dist [--installer|--homebrew|--binstall|--nix|--github-action|--deb|--rpm|--all] [-o <OUT>] [--json]
```

**Checksum resolution.** Artifacts that embed real SHA-256 checksums
(`--homebrew`, `--nix`) need a release to pin against:

```bash
# Pin a release tag — checksums are fetched from the GitHub release
forjar dist --homebrew --version v1.6.1

# Or resolve checksums offline from a local SHA256SUMS-format file
forjar dist --nix --version v1.6.1 --checksums-file dist/SHA256SUMS
```

**Verification (FJ-3607).** Validate the generated installer before you ship it:

```bash
# Tier 1 (static): generate to a temp dir and check the installer with
# `sh -n`, bashrs lint, required-snippet presence, and download-URL structure
forjar dist --installer --verify

# Tier 2 (runtime): actually RUN the installer inside ubuntu (gnu) and
# alpine (musl) containers against a locally-staged tarball, asserting the
# binary lands in install_dir and version_cmd succeeds. Requires
# Docker/Podman and implies --verify; cleanly skips when no runtime is found.
forjar dist --installer --verify-containers
```

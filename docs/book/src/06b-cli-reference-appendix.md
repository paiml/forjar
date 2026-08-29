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

## The Unified Verb Surface

### forjar verb

Every capability forjar exposes on **more than one** transport, in one place.

```bash
forjar verb list                 # the surface, one name per line
forjar verb list --json          # with descriptions, read_only, timeouts
forjar verb schema plan          # a verb's input and output JSON Schema
forjar verb call validate --json '{"path":"forjar.yaml"}'
forjar verb serve --port 8737    # the same surface over HTTP
```

The twelve verbs — `validate`, `plan`, `drift`, `lint`, `graph`, `show`,
`status`, `trace`, `anomaly`, `remediate`, `audit`, `workspace` — are
declared **once**, in `src/verb/registry.rs`, and the CLI, MCP and HTTP
transports each render that one declaration. Adding a verb is one row. There is
no second list to keep in step, which is the defect this replaced: the same nine
tools were previously written out four times in `src/mcp/registry.rs`, and only
one of those four copies was reachable in production. Run `forjar verb list` for
the set this binary actually ships; a list typed into a document is the drift
the registry exists to prevent.

`policy-coverage` was on this list and was **withdrawn**. The unified
calculation derives a rule's identity from its `message:` when the rule declares
no `id:`, so two such rules sharing a message collapse into one and a rule that
never ran is reported as having run — in the one report whose job is to say what
is *not* covered. The leaf is back in `Pending` citing
[paiml/forjar#369][fj369], and `forjar policy-coverage` is still the way to ask.
Honest debt beats a tool that answers wrongly on every transport at once.

[fj369]: https://github.com/paiml/forjar/issues/369

A verb's MCP name is its own name with `forjar_` prefixed and any hyphen folded
to an underscore — `policy-coverage` would publish as `forjar_policy_coverage`.
Both spellings are derived from the one row; no verb on the surface today
carries a hyphen.

All twelve are **read-only**, and that is a property of the surface rather than
a coincidence of which verbs it happens to hold:
`tests/falsification_verb_readonly_surface.rs` fails if any row declares
`Effects::Mutating`. It is published, not assumed — `verb list --json` reports
`read_only` per verb and MCP publishes the same value as `readOnlyHint`, both
derived from one field so they cannot disagree. An agent may call any forjar
verb unattended without risking a change to a machine.

`workspace` is the one verb that unifies part of a subcommand group: `workspace
list` and `workspace current` read, so they are on the surface; `workspace new`,
`select` and `delete` write, so they are not.

It REPORTS a selection; it does not impose one. The active workspace is joined
onto the state dir by the CLI commands that take `--workspace` (`apply`, `plan`,
`drift`, `lock`) and by nothing on the verb surface — a verb called with the
same `path` reads `<config dir>/state`, not `<config dir>/state/<active>`. The
report therefore carries `workspace_state_dir`, the directory the selection
designates, so a caller that wants the CLI's view can pass it as the next verb's
`state_dir`. Closing the gap itself is [paiml/forjar#367][fj367].

[fj367]: https://github.com/paiml/forjar/issues/367

Read-only means it does not run what the **config** declares, either. That was
not true before 1.21.1 (forjar#372): planning executed the config's own
`ambient_inputs` commands, shelled out to `sops`/`op` for
`secrets.provider`, and ran `output_equivalence` normalisers — so pointing an
agent at an untrusted repository executed whatever that repository declared, on
a tool advertising `readOnlyHint: true`. The verb surface now plans over a
config with those three keys stripped and reports what it skipped:

```json
{
  "to_create": 1, "lock_relative": true,
  "unattended_skipped": [
    "build: 1 ambient_inputs command(s) not executed; staleness from ambient state not checked",
    "secrets: provider 'sops' not invoked; `{{secrets.*}}` left unresolved"
  ],
  "disclosure": "this surface never executes what a config declares, so 2 …"
}
```

`unattended_skipped` is always present, empty when there was nothing to skip —
that is the case where the unattended plan and `forjar plan` compute the same
thing. **The CLI is unchanged**: `forjar plan` still probes ambient inputs and
still resolves `sops` secrets, because the operator who typed it chose that
config. The distinction is the caller, not the feature.

**This is not all 193 subcommands, and it does not claim to be.** Every CLI leaf
is accounted for in `src/verb/partition.rs` as exactly one of `Unified`,
`CliOnly` (with a written reason) or `Pending` (with an issue). The partition is
total and enforced: a new subcommand that names no bucket fails the build.

#### forjar verb serve

Serves the same verbs over HTTP:

| route | |
|---|---|
| `GET /healthz` | liveness |
| `GET /v1/verbs` | the surface, identical to `verb list --json` |
| `GET /v1/verbs/{name}/schema` | that verb's schemas |
| `POST /v1/verbs/{name}` | invoke, JSON body as params |

```bash
forjar verb serve --port 8737 &
curl -s localhost:8737/v1/verbs
curl -s -X POST -d '{"path":"forjar.yaml"}' localhost:8737/v1/verbs/validate
```

A verb returns **byte-identical** output over HTTP and the CLI; both render
through one function, and a test invokes the same verb over both surfaces and
compares the bytes.

It binds `127.0.0.1` and has **no authentication**, so `--bind` on a routable
address is a deliberate choice and prints a warning. Because every verb is
read-only, exposure leaks configuration rather than granting control — a real
distinction, and not a reason to relax it.

`forjar rules serve` is a different thing entirely: an HMAC-authenticated
*inbound webhook receiver*. It accepts events; it does not expose forjar's
capability set, so it is not part of this surface.

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

Policy rule coverage analysis. Two questions, one report:

- **which resources any rule is SCOPED to** — `coverage_percent`, `uncovered`
- **what the rules then SAID** — `rules_triggered`, `untriggered_rules`,
  `clean_resources`

The distinction is the point. A resource no rule scopes to is *clean* because
nothing ever looked at it, and a report that printed only "N clean" would call
such a config compliant. `uncovered` is the list of resources that answer holds
vacuously for.

```bash
forjar policy-coverage -f forjar.yaml [--json]
```

`--json` prints `core::policy_coverage::compute_coverage` verbatim — the value,
not a projection of it, so a renderer cannot reshape the answer. There is one
calculation behind this command and no second one anywhere.

This command is **CLI-only**. A `policy-coverage` verb shipped briefly on the
unified surface and was withdrawn: rule identity is derived from `message:` for
a rule that declares no `id:`, so two such rules sharing a message collapse and
the satisfied one disappears from `untriggered_rules` instead of being listed.
See [paiml/forjar#369][fj369-pc]. Until that is fixed the wrong answer is
reachable from one place rather than from every transport.

[fj369-pc]: https://github.com/paiml/forjar/issues/369

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

### forjar remediate

Compute the corrections your config's own `policies:` block determines, and
print the corrected document.

```bash
forjar remediate -f forjar.yaml [--policy-id SEC-MODE]... [--json]
```

**It never writes.** The corrected document goes to stdout and the summary to
stderr, so `forjar remediate > forjar.new.yaml` is the write and you perform it
after reading the diff. That diff is short: the correction replaces the byte
range of one scalar, and every other byte — comments, quote style, key order,
blank lines — is copied through unchanged.

**The value comes from your policy, never from forjar.** Only `assert` rules
determine a fix, because only an `assert` names the value a field must have:

| rule type | says | fix |
|-----------|------|-----|
| `assert`  | field must EQUAL X | write X |
| `deny` / `warn` | field must NOT equal X | none — X is what to avoid |
| `require` | field must be set | none — no value is named |
| `limit`   | a list must stay in bounds | none — no scalar to set |

Everything else is reported in `remaining_violations` with the reason, which is
usually the more useful half of the output. Forjar also refuses, rather than
guesses, when the value it would edit is written in flow style, is a block
scalar, is an anchor or alias, appears twice, or does not match the value the
parser resolved — the last of which means it came from an `includes:` file, a
recipe or a `{{template}}`, so editing the literal would not change anything.

Remediation reads inline `policies:` only. A project whose rules come from a
compliance pack gets a `scope_note` saying so rather than a silent zero.

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

## `forjar codegen`

Emit the shell a resource *generates*, resolved exactly as `apply` would resolve
it — templates expanded, secrets substituted.

```bash
forjar codegen -f machines/lambda-labs/forjar.yaml -r media-backup --phase apply
forjar codegen -f forjar.yaml -r root-disk-budget --phase state-query
```

`--phase` is `apply` (default), `check`, `state-query`, or `reaper`.

Most resource types describe state directly; a few — `disk_budget`, `backup_sync`
— have a *synthesised shell script* as their real payload. That artifact cannot
be reviewed, debugged, or dogfooded unless you can get at it, and reading the
handler's source is not the same thing as reading what it emits for your config.

To preview a `disk_budget` reclaim, emit the **reaper** rather than the apply
script:

```bash
forjar codegen -f forjar.yaml -r root-disk-budget --phase reaper > /tmp/reaper.sh
sh /tmp/reaper.sh    # lists what it WOULD reclaim; deletes nothing
```

The reaper deletes only when `FORJAR_BUDGET_EXECUTE=1` is set, which is granted
in exactly two places: the generated systemd unit, and the pass `forjar apply`
runs. Run by hand it inspects, and says `mode=dry-run` on its start and
completion lines.

`--phase apply` emits the **installer**, not a preview: it writes the reaper and
its units, grants the reclaim opt-in, and (for `sudo: true`) re-elevates through
`sudo bash`. The recipe once documented here — `--phase apply` piped to `sh`
with `FORJAR_BUDGET_DRY_RUN=1` — reclaimed 1.5 TB, because that variable is read
by the reaper on the target and survives neither `sudo` (`env_reset`) nor `ssh`
(no `SendEnv`). For the same reason, `forjar apply` now refuses to run at all
when `FORJAR_BUDGET_DRY_RUN` is set and the scope holds a `disk_budget`: it
cannot honour the request and will not pretend to. To preview an apply, use
`forjar apply --dry-run` (forjar#334).

## `forjar verify`

Regenerate a resource's declared outputs into a **scratch tree** and report
whether they still reproduce.

```bash
forjar verify                      # every resource in forjar.yaml
forjar verify -r apr-build         # one resource
forjar verify --tag stack-tools    # a tagged subset
forjar verify --json               # machine-readable, for CI gating
forjar verify --keep-scratch       # leave the scratch tree to inspect a mismatch
forjar verify --check-declared-inputs   # fail on a read outside task_inputs
```

`--check-declared-inputs` (GH-244) runs the resource twice: once from a full
copy of `working_dir`, once from a tree containing only the glob-expanded
`task_inputs`. If the full run reproduces and the declared-only run does not,
the resource read something it never declared and the verdict is
`UndeclaredInput`. The early return matters: a non-deterministic generator
fails BOTH runs and stays `Diverged`, because blaming the declaration for a
generator's own instability is the misdiagnosis this replaces.

It sees reads of files inside the project tree. It cannot see a read of
`/usr/share/fonts` or a tool version, because those exist in the scratch tree
too — declare those with `ambient_inputs` instead.

Exits non-zero if any resource diverged or failed to regenerate, so it gates
cleanly in CI.

The defining property is a **negative** one: `verify` never writes a declared
output path. That is why there is no `--fix`, `--restore` or `--write` flag —
a flag that relaxed the guarantee would make it conditional on every caller
remembering not to pass it, which is not a guarantee at all. The restriction is
enforced structurally in `core::verify` rather than by convention.

Use it to answer "would this still build the same thing?" without disturbing the
artifact you are asking about — the question `apply` cannot answer, because
answering it is the same act as changing it.

## `forjar dogfood`

Exercise forjar's generated artifacts against **real external tools and real
on-disk shapes**, and fail when reality disagrees with what the code assumes.

```bash
forjar dogfood          # human-readable
forjar dogfood --json   # machine-readable, for release receipts
```

This is not a second test suite. Unit tests are written by the same person as
the code, so a fixture can only ever confirm the assumption it was built from —
which is how three releases in two days each shipped a bug that 12,904 passing
tests, a five-gate clean room and a 19-check CI run all missed:

- `backup_sync` read rclone's `--combined` status characters inverted, so files
  that were **not** backed up left the coverage denominator and a backup missing
  data reported *higher* coverage than one with everything. The test stub emitted
  whichever characters the author believed in.
- `disk_budget` required both `CACHEDIR.TAG` and `.rustc_info.json` on a cargo
  target dir. Across a real 4.6 TB tree, **zero of sixteen** marker-bearing
  directories had the pair. The fixture had both because the author believed both
  were present.

So each exercise invokes the actual tool (`rclone check --combined`, not a stub
of it), builds the layouts that really occur on disk, and executes emitted shell
under `bash` — the interpreter every forjar transport actually uses. **A missing
external tool is a failure, not a skip:** dogfooding a resource built on a tool's
output format, without that tool, proves nothing.

Coverage is declared by an exhaustive match over `ResourceType`, so a new
resource type **fails to compile** until its dogfood status is stated, and
`NotApplicable` requires a written reason that prints on every run:

```
PASS  disk_budget   detection rule correct on all 4 real shapes: repo root,
                    per-arch, registry (excluded), cc source dir (excluded)
PASS  backup_sync   rclone v1.75.0: --combined characters confirmed = * + -

not exercised (16 types):
  package     mutates system packages
  docker      requires a docker daemon
  ...
```

That property is the point. The previous shell-script gate covered only `file`
resources and still reported success while two new resource types shipped
broken — a gate that can quietly stop covering things is worse than none,
because it reports GO with authority.

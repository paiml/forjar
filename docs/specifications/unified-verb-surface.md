# forjar Unified Verb Surface (FVS)

**Status:** proposed · **Refs:** paiml/forjar#288 · **Supersedes:** nothing
**Sibling prior art:** `rmedia/docs/specifications/unified-verb-surface.md` (UVS),
`paiml-mcp-agent-toolkit` three-transport declaration.

## 1. The problem, measured

Walked from the shipped 1.16.0 binary on 2026-08-22:

| surface | count |
|---|---|
| CLI leaves | **193** (160 top-level, 10 of which carry 43 nested subcommands) |
| MCP tools | **9** |
| HTTP | none — `core::webhook_server` exists but is reachable from no subcommand |

**Updated 2026-08-22, after implementation:** HTTP is now a declared transport.
`forjar verb serve` exposes every verb over `GET /v1/verbs`,
`GET /v1/verbs/{name}/schema` and `POST /v1/verbs/{name}`, reusing
`core::webhook_http::read_request` rather than a second parser — that code
already answers an oversized head with 431, refuses `Transfer-Encoding` with 501
instead of mis-framing it, and checks `Content-Length` against the body cap
*before* buffering. Writing a second parser would mean getting all of that right
twice, with only one of the two exercised by the webhook tests.

`rules serve` (#205) is deliberately NOT that transport. It is an
HMAC-authenticated inbound webhook receiver: it accepts events, it does not
expose forjar's capability set, so it is bucketed `CliOnly` with that reason.
Declaring it a transport would assert parity between `forjar plan` and an event
endpoint, which is not a meaningful equality.

So 184 capabilities exist on exactly one transport. That is the gap this spec is
about, and the first thing it must do is **stop pretending the gap will close**.

Three defects make the current split worse than the count suggests:

* `src/mcp/registry.rs` declares the same 9 tools **four times** — `export_schema()`,
  `build_registry()`, `build_forge_config()`, and again inside `serve()`. The literal
  `forjar_validate` appears 4× in one file. Adding a 10th tool means editing four
  places and the compiler will not notice if you edit three.
* `src/main.rs::classify_exit_code` chooses the **process exit code by substring-matching
  the error prose**. Observed live: an I8 bashrs-validation failure whose message begins
  `transport error:` exits **4** — the connection code a CI script retries — for a
  deterministic failure that can never succeed on retry.
* MCP publishes **zero tool annotations**. An agent cannot tell a read-only verb from a
  mutating one.

## 2. Non-goal, stated first

**FVS does not promise to unify 193 leaves.** A migration promise that large is
unfalsifiable, and the sibling that wrote this spec first has not kept it: rmedia
registered 34 verbs, proved parity over its derived tree, and **still ships its
hand-written `Commands` enum** — `rmedia-cli/src/main.rs:172,174` routes only `Mcp` and
`Serve` through `cli_registry()`. Its CLI parity leg is an in-process claim.

What FVS promises instead is an **enforced three-way partition** in which every one of
the 193 leaves is in exactly one bucket, with a written reason:

* `Unified` — on every declared transport, parity- and invariance-gated.
* `CliOnly` — deliberately CLI-shaped (`completions`, `doctor`, `version`, anything whose
  output is a terminal affordance). Requires a one-line reason.
* `Pending` — belongs on the unified surface and is not there yet. Requires an issue link.

`Pending` is the debt ledger. It may only shrink. **The partition is total: a new CLI
leaf that names no bucket fails the build.** That is the property that makes the negative
half of this design falsifiable, and it is the half the judge panel identified as this
approach's weakest point — an exclusion list that is green by construction proves nothing.

## 3. Equations

### FVS-1 — surface parity over the unified set
```
names(CLI_derived) = names(MCP tools/list) = names(manifest)
```
Each side computed the way a **client** sees it: the CLI's by walking the derived clap
tree from the shipped binary, MCP's from a real `tools/list` response, the manifest's
from the generated file. Reading the registry three times proves nothing.

### FVS-2 — params are validated before invoke
```
invoke(v, p) reached  ⟹  p validated against v.input_schema
```

### FVS-3 — success implies the output schema
```
invoke(v, p) = Ok(out)  ⟹  out validated against v.output_schema
```

### FVS-4 — the error taxonomy is total
```
∀ e: classify(e) ∈ {Validation, Connection, Partial, Drift, Other}
```
From a **variant**, never from prose. Exit-code *values* are a public contract and do not
change: 3 Validation, 4 Connection, 2 Partial, 10 Drift, 1 Other.

### FVS-5 — transport invariance
```
∀ v ∈ Unified, ∀ case: bytes(CLI(v,case)) = bytes(MCP(v,case))
```
with the MCP params **derived from the CLI argv through the adapter**, so this compares
adapters rather than two hand-written inputs that happen to agree.

### FVS-6 — renderer fidelity (novel; not in rmedia's UVS)
```
∀ v ∈ Unified: bytes(stdout of legacy leaf) = bytes(stdout of derived leaf)
```
Both sides spawned from the shipped binary on the same fixture. This is what lets the
193-leaf CLI cut over **one verb at a time** instead of in a single commit, and it is
precisely what rmedia lacks — its `cli_surface.rs` (1,028 lines) measures a distance
nobody can close because `VerbSpec` has no per-verb human renderer while `src/cli/`
carries **3,351 `println!` calls**.

FVS-6 is empty on the day it ships and grows one entry per migrated verb. That is a
stated weakness, not a hidden one: it means FVS-6 gates nothing initially, and the
partition in §2 is what carries the design until it does.

## 4. Reachability is designed in, not tested in

The derived tree ships as a real subcommand — `forjar verb list|call|<name>` — and is
user-reachable from day one. Every conformance test spawns `CARGO_BIN_EXE_forjar`.

This is not a style preference. rmedia's four-way parity suite was **green for the entire
period `mcp::serve_stdio` and `http::serve` had no caller from `main.rs`**: the transports
agreed with each other perfectly while being unreachable from the process entry point.
Agreement cannot falsify reachability. A library-level parity test is a test of the
library, not a gate on the release.

## 5. HTTP: absent, and its absence asserted

`[package.metadata.transports]` declares exactly `cli` and `mcp`. A falsifier asserts from
the spawned binary that `--help` advertises no `serve`, and that the metadata key set is
exactly those two. Three-way parity for a transport with no consumer is a third surface to
keep green for nobody; adding it later is one declared key and one feature flag, and the
gate follows automatically.

## 6. What this does not fix

* The 184 `Pending`/`CliOnly` leaves keep their current behaviour, including their
  `println!` output and their prose-classified errors, until individually migrated.
* forjar's MCP transport is `pforge_runtime` → `pmcp`, which does not exit on stdin EOF
  (paiml/pforge#18). FVS does not replace the transport; that is a separate decision with
  its own cost.

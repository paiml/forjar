# Provable IaC + copia-as-a-library — forjar design spec

Status: DRAFT v2 (quorum-validated 2026-07-04 — 6 lenses + synthesis; must-fixes folded in)
Owner: PAIML Engineering
Supersedes/extends: `forjar-spec.md`, the 11 `contracts/*.yaml`, `ForjarExecution.tla`, `ForjarDependencyGraph.als`

## What `forjar prove` is — and is NOT (honest framing)

`forjar prove` is **config validation raised to the proof ladder** — the honest analog
of `terraform validate` with a *machine-checked validator*, NOT `terraform plan` +
Sentinel + "safe to apply." Every invariant below is a pure function of the config `C`;
proving them means the **declaration is internally consistent and structurally safe**,
not that applying it leaves a healthy machine. The remote shell that performs the
actual mutation is **outside the trusted computing base** and outside every proof.

Three-state result vocabulary (never collapse them):
- **PROVED** — a Lean theorem transfers to *this* instance (e.g. I1 acyclic ⇒ a finite apply order exists).
- **CHECKED** — a Kani-verified falsifier ran within a bound that covers this instance and found nothing.
- **UNKNOWN** — an opaque/impure resource or an out-of-bound input the analysis cannot see through.

A config with any `exec`/`command`/`script`/`cron` resource **cannot** report I3/I5/I6/I7
as PROVED — those downgrade to CHECKED/UNKNOWN and the receipt shows the mix
("18/23 resources L4-idempotent, 5 exec resources L2-asserted, 5 UNANALYZED for I3").
A gate that green-lights vacuously is worse than no gate.

## Vision

Terraform/Ansible/Pulumi *plan and apply* infrastructure. forjar will additionally
**prove** it. The differentiator: before forjar touches a machine, `forjar prove
machines/<m>/forjar.yaml` emits a **pv-style L1–L5 provability report over the
infrastructure declaration itself** — not just the tool's code. An IaC file that
proves to L4 is one whose apply is guaranteed acyclic, deterministic, idempotent,
conflict-free, and convergent, with the guarantees bound to the *real* executor.

This spec covers three threads of one initiative:

1. **copia as a library** — de-vendor `src/copia`, redesign large-file provisioning around the published `copia 0.2` crate.
2. **L1–L5 contracts for forjar's core Rust** — climb the 11 existing L3 contracts to L5 (Lean + bindings) and add contracts for the core provable subsystems.
3. **Provable IaC** — a new `forjar prove` capability that applies the L1–L5 ladder to a `forjar.yaml`.

The three are ordered by dependency: (3) *reuses* the semantic contracts hardened in (2); (1) is independent and lands first as the smallest, highest-confidence change.

---

## Thread 1 — copia as a library

### Current state
`src/copia/` (552 LOC) is a self-contained, **fixed-block BLAKE3** delta module: it
hashes each 4 KiB block and compares block *i* local vs block *i* remote. It also
emits remote SSH scripts (`signature_script`, `patch_script`, `full_transfer_script`)
that forjar's own `transport` runs on the target. Call site: `copia_apply_file`
(`src/core/executor/helpers.rs`), reached from `resource_ops` / `machine_wave` when
`use_copia` and the source file exceeds 1 MiB.

### copia 0.2 surface (published, standalone leaf crate)
`copia::{BlockSignature, Signature, SignatureTable, Delta, DeltaOp, DeltaStats,
RollingChecksum, StrongHash, Sync, SyncBuilder, SyncConfig, SyncStats}`. copia's
delta is **rolling-checksum (rsync-style)** — it finds matches at *any* byte offset,
so it handles insertions/deletions, strictly stronger than forjar's fixed-block scheme.

### Decision: transport-invoked applier on the RECEIVER (quorum-corrected)
**The v1 claim "forjar gains the rolling-checksum engine for free" was false.** copia's
weak rolling checksum (custom Adler, MOD 65521) has to be computed over the *remote*
basis, and no coreutils tool (`b3sum`/`sha256sum`/`cksum`) computes it — so a pure-shell
applier gets copia's harder byte-offset wire format with **zero** rolling benefit. The
spec also conflated two different things: *shelling out on the sender* (correctly
rejected — bypasses forjar's transport) vs *a transport-invoked applier on the receiver*
(correct, and the only path to real rolling delta — this is rsync's own server-side model).

**Decision: Option A — the comprehensive fix (receiver-side copia applier).** forjar's
transport stages the target-arch `copia` static-musl binary on the receiver (copia's
release already builds x86_64 + aarch64 musl statics), then orchestrates copia's existing
`signature` → `delta` → `patch` subcommands over forjar's own transport: (1) run
`copia signature <remote-path>` on the receiver to get the rolling signature; (2) compute
`copia::Delta::compute` locally against the source; (3) stream the delta and run
`copia patch` on the receiver to reconstruct. Real rolling-offset matching, ONE tested
applier, and forjar's transport still owns the connection/provenance (it *invokes* copia
on the receiver — it does not shell out to copia on the sender). The type-only variant is
explicitly rejected: it delivers no rolling benefit and barely uses copia.

Binary staging: forjar detects receiver arch (`uname -m` via transport), stages the
matching `copia-<arch>-musl` static (content-addressed, cached under a forjar state dir,
re-staged only on hash miss). Targets without a musl static (busybox routers) fall back to
forjar's existing full-transfer path — documented, not silently degraded.

`copia = { version = "=0.2.0" }` **exact-pinned** (a `cargo update` to copia 0.3 must not
silently change `Delta::compute` under an already-proven forjar I5). The staged binary
version is asserted to match the linked crate version at stage time.

### Applier hardening (mandatory, both options) — the remote applier must:
1. **Verify whole-file integrity BEFORE the rename**: hash `$TMPFILE`, compare to the
   `Delta` strong checksum, abort+cleanup on mismatch (a 32-bit weak checksum makes
   collisions real — verify is more mandatory, not less).
2. **Set owner/group/mode on `$TMPFILE` before the atomic rename** — this fleet provisions
   cargo credentials / TLS keys at `0600`; a chmod-after-rename leaves a world-readable window.
3. `trap 'rm -f "$TMPFILE"' EXIT` in patch + full-transfer scripts (interrupted transfer = ENOSPC litter).
4. `printf '%s'` not `echo`, and shell-quote-escape every interpolated path/owner/group/mode
   (satisfies the Thread-2 no-injection contract, which the current `echo '<b64>'` violates).
5. Single-source `block_size`, threaded to both the remote signature script and local
   `Delta::compute`, asserted-equal on parse.
6. Re-check basis size+hash between the two SSH phases (signature → patch) — TOCTOU.
7. Stream large literals over transport **stdin**, not argv (`ARG_MAX`/`E2BIG` on mostly-changed 1 MiB+ files).

Regression bar: `src/copia/tests.rs` semantics preserved; `forjar apply` byte-identical on a
>1 MiB source; interrupted provision is complete-or-noop (atomic); full suite + clean-room green.

---

## Thread 2 — L1–L5 contracts for forjar's core Rust

Same honest split we used for copia: **prove the provable core, test the I/O.**
Scope = `src/core/` (executor, apply, DAG, recipe), `src/transport/`, `src/resources/`,
`src/tripwire/`, and the copia glue — **not** the 220 KLOC `src/cli/` surface.

1. **Climb the existing 11 L3 contracts to L5** — each currently has Kani (L3) but 0 Lean, 0 bindings. Add a Lean 4 proof (L4) for each provable obligation and a verified `binding.yaml` entry (L5). Where an obligation is genuinely I/O (e.g. actually writing state to disk), keep it as a falsification test and narrow `proof_obligations` to the provable core — never a fake Lean proof.
2. **Add contracts for uncovered core subsystems** — transport safety (no command injection in generated scripts), resource-conflict-freedom, recipe expansion termination, tripwire append-only monotonicity, copia-glue delta correctness (`Delta::compute` reconstructs the source). ~8–12 new contracts, each L1→L5.
3. Enforce via `make contracts` (validate + `lean lean/*.lean` + `pv proof-status --binding`) exactly as copia does.

---

## Thread 3 — Provable IaC (`forjar prove`)

### The key idea
forjar's 11 contracts already formalize its apply **semantics in general** (apply is
idempotent, plans are deterministic, the DAG orders correctly). Provable IaC
**instantiates** those proven semantics on a **specific** `forjar.yaml`: it proves the
config-level facts that, combined with the code-level theorems, guarantee a safe apply.

### The invariants
Defined over the **post-expansion** resource set `R` and edges `E` — i.e. the
recipe-expanded, variable-interpolated, topologically-ordered **canonical plan** that
`apply` actually runs, NOT the raw YAML (proving over raw YAML proves I1/I3/I6 over the
wrong graph). Enforcement class: **HARD** = a falsified invariant blocks apply; **ADV** =
advisory (semantic, exec-fragile, reused from code proofs).

| # | Invariant | Meaning (honest) | Class | Ceiling |
|---|-----------|------------------|-------|---------|
| I1 | **DAG acyclicity** | `(R,E)` has no cycle → a finite apply **order exists** (NOT "apply terminates" — per-recipe termination is the halting problem) | HARD | L4 Lean (acyclic⇒toposort) + L3 Kani(checker) |
| I2 | **Dependency completeness** | every edge targets a declared resource (no dangling `needs:`) | HARD | L2 falsification |
| I3 | **Conflict-freedom** | no two resources write an **overlapping target namespace** (path prefix/recursive-dir subsumption, nvram key, package) — subsumption, not string equality | HARD | L3 Kani (proven total per-namespace normalizer) |
| I4 | **Plan determinism** | `plan(C, observed)` is a pure function of `(C, observed)` — **stated as "deterministic given observed-snapshot `<hash>`"**, not "reproducible" | ADV | L4 (reuses `plan-apply-equivalence-v1` + `recipe-determinism-v1`) |
| I5 | **Idempotency** | split: **plan-idempotence** (converged lock + matching hash ⇒ NoOp — provable) vs **effect-idempotence** (UNKNOWN for exec/script unless a `creates`/`not_if`/`only_if` guard is present) | ADV | L4 declarative / L2 exec |
| I6 | **Protected-resource safety** | no resource destroys/overwrites a `protected` one; ordering respects protection; **UNMARKED destroy/replace is flagged** (blast radius) | HARD | L3 |
| I7 | **Convergence** | I1 ∧ I5 ⇒ apply reaches declared state in ≤ \|R\| steps **under the single-pass premise** (unsound if notify/handlers/triggers re-run resources — stated explicitly) | ADV | L4 |
| I8 | **Crash recoverability** | re-apply from an arbitrary partial state (crash at resource *k*, ENOSPC — a real fleet incident) is safe/idempotent; adopt Terraform's persist-partial / no-rollback / resume model and prove resume-after-*k* is safe | HARD | L3 + L4 |
| I9 | **Input purity / pinning** | every external input (package version, file/URL/git source, nvram value) is version-locked or content-hashed; reject impure interpolations (`$(date)`, random, hostname) that realize a different machine next week | HARD | L2 falsification + L3 checker |

Additional layers (NOT among the universal invariants, but part of `prove`):
- **Policy-as-code (Sentinel/OPA analog)** — user-defined, org-mutable rules (`contracts/policies/*.yaml`, rego-like) evaluated against the expanded plan (e.g. "no SSH ingress from 0.0.0.0/0"). This is what makes `prove` a *gate* rather than a linter; kept separate from the universal I1–I9.
- **Fleet/global invariants** (`make prove-fleet` builds ONE cross-config DAG, not a per-file loop): overlay-IP uniqueness (10.42.0.x), single DHCP authority, hostname uniqueness, no two machines owning one shared target, cross-machine ordering. A per-file loop is structurally blind to these.

### How each climbs the ladder
- **L1** — I1–I9 declared as equations in `contracts/provable-iac-v1.yaml`, over an abstract model of the **expanded plan** (resources, normalized targets, edges, protection/pinning flags, resource-kind).
- **L2** — `forjar prove` runs **decision-procedure checkers** on the concrete parsed+expanded plan: cycle detection (I1), dangling-dep (I2), target-namespace-collision (I3), protection/blast-radius (I6), resume-safety (I8), impurity scan (I9). A checker that fires *falsifies* that invariant for that file.
- **L3** — Kani proves the **checkers themselves** correct over bounded graphs ("cycle-detector returns false ⇒ a valid toposort exists"). A CHECKED result only transfers when the verified bound ≥ the config's node/edge count — otherwise the receipt states the bound and flags it.
- **L4** — **reuse, don't rebuild.** Do NOT add a third free-floating formal model. `forjar prove` **generates TLC/Alloy instances** (CONSTANTS `R,E` from the actual expanded config) and runs the **existing `ForjarExecution.tla` / `ForjarDependencyGraph.als`** as the L3/L4 engine, plus the Thread-2 Lean theorems for I1 (acyclic⇒order) and the reused `idempotent-apply`/`plan-apply-equivalence`/`recipe-determinism` for I4/I5.
- **L5 — the central fix: bind prove to apply through a content-hashed PLAN ARTIFACT.**
  `prove` emits the canonical expanded/interpolated/toposorted plan and hashes it.
  `forjar apply` **consumes that exact artifact by hash**, re-validates that observed
  remote state has not drifted since the proof (**fail-closed** on observed-hash
  mismatch), and refuses otherwise — the only escape is an explicit, receipt-writing
  `--force`. The gate is **in-process** (not a bypassable git hook). This is what makes
  "L5 proves the REAL apply path" true rather than theater: it closes the prove→apply
  TOCTOU, forces prove and apply onto the *same* graph, and scopes the L5 binding to the
  bound entry points (fresh apply, resume, `--target`, drift-repair). The wave-parallel
  executor gets a **refinement proof** that it honors the sequential toposort; unbound
  entry points are listed as out-of-scope, not silently covered.

### UX (three-state, scoped verdict — no "safe to apply")
```
$ forjar prove machines/intel/forjar.yaml
Proving machines/intel/forjar.yaml — expanded plan: 23 resources (18 declarative, 5 exec), 5 edges
plan-hash: b3:7f2a… (apply must consume this exact plan)

  Inv  Invariant              Class  Result
  I1   dag-acyclicity         HARD   PROVED    (acyclic; apply order length 23)
  I2   dependency-complete    HARD   CHECKED   (5/5 edges resolve)
  I3   conflict-freedom       HARD   CHECKED   (18 declarative disjoint; 5 exec UNANALYZED)
  I4   plan-determinism       ADV    PROVED    (given observed-snapshot b3:91c…)
  I5   idempotency            ADV    PROVED    plan-idem 18/18; effect-idem 5 exec UNKNOWN (no guards)
  I6   protected-safety       HARD   CHECKED   (0 violations; 2 unmarked replaces FLAGGED)
  I7   convergence            ADV    PROVED    (≤23 steps; single-pass — no handlers present)
  I8   crash-recoverability   HARD   CHECKED   (resume-after-k safe for 18 declarative)
  I9   input-purity           HARD   CHECKED   (23/23 pinned; 0 impure interpolations)
  policy (org rules)          —      2/2 pass

  VERDICT: structurally safe — no cycle, no dangling dep, no target collision, no
           protection violation, all inputs pinned; planner deterministic given
           observed-snapshot b3:91c…. 5 exec resources UNANALYZED for effect —
           operational safety NOT implied. Apply gate: PASS (plan-hash pinned).
  Receipt: .forjar/proofs/intel-<ts>.json   (secrets redacted)
```
`--json`; `--min-level`; non-zero exit if any HARD invariant is falsified (advisory ones warn).

### Trusted computing base (documented boundary)
Everything past `ssh target sh -c '…'` — the actual mutation on the remote — is
**unmodeled and outside every proof**. `prove` guarantees the *declaration* is
consistent and structurally safe and that *apply consumes the proven plan*; it does not
and cannot prove the remote shell, the target OS, or that a syntactically-valid systemd
unit boots. Receipts operate on expanded/interpolated plans and therefore **redact
secrets** before being written to the infra repo.

### Deliverables
- `forjar prove <file> [--json] [--min-level L4]` command (in `src/cli/`, logic in `src/core/prove/`).
- `contracts/provable-iac-v1.yaml` (L1–L5) + `lean/ProvableIac.lean`.
- Per-config receipts under `.forjar/proofs/` (reproducible artifacts, infra-repo policy).
- `make prove-fleet` — prove every `machines/*/forjar.yaml` in infra as a nightly lane.

---

## Phasing
- **Phase 1** (small, high-confidence): Thread 1 — copia as a library. Lands first.
- **Phase 2** (large, parallelizable): Thread 2 — climb 11 + add core contracts to L5.
- **Phase 3** (novel, flagship): Thread 3 — `forjar prove`, reusing Phase-2 proofs.

Each phase: quorum-validated design (≥3 world-class systems), provable contract shipped in the same PR, clean-room + `/dogfood` before any release.

## Quorum validation targets (per feature)
- **copia integration**: rsync/librsync, Terraform provisioners, Ansible synchronize.
- **Provable IaC**: Terraform plan/`terraform validate` + Sentinel/OPA policy proofs, Pulumi CrossGuard, TLA+/Alloy model checking (forjar already ships both), Kubernetes admission control, Nix derivation purity.

## Quorum verdict (2026-07-04) + residual risks

Panel of 6 lenses + synthesis. Verdict: **architecture sound, assurance spine novel and
endorsed; the v1 draft was FLAWED on the copia boundary and OVERCLAIMED provable-IaC by
a full layer.** All must-fixes above are folded into this v2. Endorsed strengths kept:
the L2→L3→L5 assurance spine (run the real checker → Kani-prove the checker → bind to the
executor), honest ceiling discipline, "prove the core / test the I/O," genuine reuse of
the already-bound `plan-apply-equivalence`/`idempotent-apply`/`recipe-determinism` contracts,
and making `observed` an explicit argument of `plan()`.

Residual risks to watch while building:
- **copia version drift** — mitigated by `=0.2.0` exact-pin; re-prove I5 on any bump.
- **Gate bypassability** — the in-process plan-hash gate (not a git hook) is the fix; until apply refuses an unproven/`--force`-less plan, `prove` is advisory only.
- **Receipt secret leakage** — expanded plans carry interpolated secrets; receipts MUST redact before write (they live in the infra repo).
- **Closed-world assumption** — I5/I7 hold only if the target is unchanged between applies; a cron/package-editable box can invalidate a prior proof. State the assumption in the receipt.
- **Cross-machine failure domains** — `machine_wave` applies per-machine over independent crash/partition boundaries; the shipped `.tla` is a single global model — the fleet pass (I-fleet) must model independent domains.
- **Trigger/handler re-application** — I7's single-pass premise is unsound where notify/handlers exist (`binding.yaml` already notes timer-restart re-runs). Detect handlers → drop I7 to UNKNOWN.
- **HashMap iteration nondeterminism** — confirm every plan path uses `IndexMap`/`BTreeMap`/sorted order; make deterministic tie-break a CHECKED invariant, not a note.
- **L5 binding completeness** — prove the binding covers EVERY apply entry point (fresh, resume, `--target`, drift-repair, wave concurrency), not just the happy path.

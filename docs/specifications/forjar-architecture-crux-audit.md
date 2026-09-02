# forjar CRUX Audit — Architecture, Performance, Competitive Research

**CRUX** — Competitive Research Unified with eXamination of architecture and performance.

| | |
|---|---|
| Audited version | 1.24.0 @ `f0cbf635` |
| Date | 2026-09-02 |
| Method | 9 parallel research lenses, read-only, then synthesis |
| Findings | 118 (7 critical, 35 high, 44 medium, 22 low, 16 strengths) |
| Evidence quality | **all 9 lenses reported `measured`** — numbers taken on this box against the built binary, not inferred |
| Competitors surveyed | Ansible, Salt, Chef, pyinfra, cdist · Terraform/OpenTofu, Pulumi, CDKTF, Crossplane · Nix, Guix, bootc/OSTree, Ignition, Kairos · Sigstore, in-toto/SLSA, SOPS, Vault |

> **How to read this.** Every claim below is anchored to a `file:line` or a measurement. Where a
> shipped doc or README claim is contradicted by measured behaviour, the audit says so plainly and
> cites both. This is deliberate: the project's own strongest habit is self-correction against
> measurement (#236, #239, #241, #249, `3a51a92c`), and this document is written to feed that habit.

---

## 1. Executive summary

forjar is an agentless, brownfield-first IaC tool that compiles declarative YAML to auditable shell,
executes it over SSH, and keeps a persisted two-hash lock (desired + observed) with fleet-wide drift
detection. **That combination is genuinely unmatched**: no agentless competitor persists a drift
baseline, none lets you print the exact bytes that will execute, and none ships mutation testing of
infrastructure declarations or 35 machine-readable contracts with Kani/Lean/TLA+ bindings. The
controller is fast and linear — 10K resources validate in 0.27 s, plan against a 5.5 MB lock in
0.32 s at 120 MB RSS.

**The three things that matter most, in order:**

1. **Convergence is decided by a 35-field allowlist over a 124-field resource** (E01). Editing a
   `github_release` tag, a user's `uid` or `ssh_authorized_keys`, a GPU `driver_version`, a model
   `checksum`, or a task's `working_dir`/`timeout` produces a **byte-identical lock hash**, so the
   planner returns NoOp and apply reports `unchanged`. Measured: two 6-resource configs differing in
   11 identity-bearing fields produced `diff`-identical lock files while their generated apply
   scripts differed. This is silent non-convergence — the failure mode `hashing.rs:140-142` itself
   calls "the worst available failure mode".

2. **The performance claims for no-op apply are false on any remote fleet** (E02). The default
   `policy.tripwire=true` pre-apply drift gate runs one **fresh-handshake** SSH session per locked
   resource, sequentially across machines, *before* ControlMaster is started 20 lines later.
   Measured: 306 ms fresh vs 6.7 ms multiplexed — a 45× per-session penalty. For 100×50 that is
   ~25–50 minutes of pure SSH setup before any work. Both `forjar-spec.md §9` ("< 500ms, no shell
   exec") and `forjar-platform-spec.md:73` ("zero remote I/O") are contradicted.

3. **Three signature verifiers verify nothing** (E03). `sign --verify`, `sign --pq --verify` and
   `lock-verify-hmac` compare a BLAKE3 digest and never read the signature field. Measured: setting
   `signature` to `"deadbeef"` and `signer` to `"root@prod"` still yields `"valid": true`. A CI gate
   built on these is green by construction — worse than absent, because the output says "valid".

The strategic finding is narrower and more interesting than any single bug: **forjar's declared
capability ladder has outrun its executed behaviour in exactly the places that are hardest to
falsify from outside** — the store is not on the apply path (`store: true` changes the *score*, not
the script), the derivation sandbox emits commands that cannot run (`seccomp-bpf`, `forjar-hash-dir`
— neither exists), the tamper-evident event chain has no production caller. Meanwhile the places
that *are* falsifiable from outside — transport safety, verb read-only-ness, drift census, failure
text — are of unusually high quality. The remedy is not more features; it is **subtraction and
honest labelling** (E10, E14), which the project has already done well twice (GH-211's inert-flag
refusal, `plan`'s `lock_relative: true` disclosure).

---

## 2. Strengths — where forjar leads the field

These are measured, not marketing. An audit that only lists weaknesses is not credible.

**Category-unique**

- **Persisted two-hash lock + drift census.** Desired-config hash *and* host-observed live hash,
  with `drift` reporting its denominator ("inspected 28 of 62") and skip reasons by category. No
  agentless competitor has a persisted drift baseline; Salt needs a master, Ansible has no drift
  concept at all.
- **Post-apply host verification.** A resource is recorded Converged only if the host's own check
  script agrees (`output_verify.rs`), so "exit 0 but nothing happened" is caught at apply time.
  Ansible, Salt, Chef and pyinfra all trust the module return.
- **Compile-to-shell auditability** *(qualified — see E07)*. `forjar codegen -r <id> --phase apply`
  prints the exact bytes that will execute. pyinfra and cdist compile to shell but neither exposes the
  artifact this directly. Review correctly notes the tension: the derivation-sandbox path emits
  `seccomp-bpf` and `forjar-hash-dir`, **neither of which exists**, so auditability of *that* path
  shows you a script that cannot run. The property holds for the 20 shipping resource handlers, which
  is where it is claimed; it does not hold for the unreachable store path.
- **Formal spine.** 35 contracts with proof obligations and explicit `scope_boundary` sections
  naming what is *not* proven, 32 Kani harnesses, 7 Lean files, TLA+ and Alloy models, 231
  `falsification_*` files. Nobody else in this category ships this.
- **Mutation testing of infrastructure declarations** (`forjar test --mutations`, L4) and pairwise
  preservation (L5). Molecule and Test Kitchen only do converge + idempotence + verify.
- **Brownfield by design.** Converges an existing Ubuntu host over SSH with no reimage and no agent.
  NixOS, bootc, Kairos and CoreOS all require installing their image first.

**Engineering quality**

- **Codegen dispatch is one compile-time-exhaustive table** — an unrouted `ResourceType` is a
  compile error, and check/apply/state-query symmetry is *structural*, not tested.
- **Reflection-based completeness guards** for `known_fields` (both directions), resolver field
  coverage, and `ResourceType::ALL` (derived from serde's own unknown-variant error). Most IaC
  codebases have nothing like this. *(The audit's top finding is precisely that the two lists which
  decide convergence were the ones left unguarded.)*
- **Transport is a true funnel.** bashrs I8 gate + stdin-isolation wrapper applied once in
  `exec_script_tracked` for local/ssh/container/pepita alike. Scripts ship on **stdin, never argv** —
  invisible in `ps`, no ARG_MAX ceiling, never persisted on target.
- **Timeout handling is unusually careful**: process-group kill gated on `worker_done` to close the
  PID-reuse race, child reaped on stdin EPIPE, worker joined before return.
- **`webhook_sig.rs` is textbook** — RFC 4231 vectors and openssl-generated literals as external
  oracles, constant-time verify, method+path+timestamp bound in, replay window with nonce. Better
  than most of the field.
- **Test corpus exceeds production** (212K vs 209K lines; 13,456 + 4,345 test fns) with anti-vacuity
  guards like `the_wave_path_is_actually_taken`.
- **Institutionalised honesty** *(qualified — see E10)*. UNIMPLEMENTED flags *refuse* with a non-zero
  exit naming all of them at once rather than silently no-op'ing (GH-211). Adversarial review is right
  that this is a **mitigation, not a virtue**: 61 declared-and-unimplemented flags is a broken surface,
  and refusing them well is the correct handling of a defect rather than a feature. Counted here only
  because most tools in this category silently no-op instead; `plan` prints "did not contact any machine";
  the security spec states outright that the `.b3` sidecar does not resist a malicious writer.
- **Comments cite the measurement and issue that motivated the line** ("320 of 329 locked file
  resources carried NO content_hash", "CI lanes hit 15m01s"), making the codebase auditable from
  source without tribal knowledge.

---

## 3. Enhancement candidates

15 candidates. Each has a **falsifiable success criterion** — if it cannot be measured or asserted,
it is not ready to be a ticket.

> **Severity is not priority, and all 7 criticals are here.** Adversarial review charged that the
> synthesis "silently dropped 2 critical findings" because only 5 candidates were P0. That is
> incorrect and worth stating precisely: the 7 research-critical findings map to **E01–E07**. Two of
> them (E06 store-not-on-apply-path, E07 derivation sandbox) are ranked **P1 despite critical
> severity**, because both are *dormant* — they mislead a score and a status table, but no
> configuration a user writes today executes the broken path. E01–E05 all corrupt or expose something
> during a normal `apply`. That is the ranking rule; disagreement with it is legitimate, but nothing
> was dropped. Raw per-lens findings: workflow journal `wf_1b69442c-9f8/journal.jsonl`, extracted to
> `scratchpad/crux/lens*.json` (9 files, 118 findings) — not committed, since it is 268 KB of
> agent transcript, but reproducible via the method in §6.

### E01 · Hash the whole resource, not a 35-field allowlist · `architecture` · **P0** · L · risk high

**Problem.** `hash_desired_state` covers 35 of 124 fields. 74 direct fields are unhashed, including
`uid`, `groups`, `ssh_authorized_keys`, `tag`, `repo`, `binary`, `driver_version`, `cuda_version`,
`checksum`, `script`, `stages`, `timeout`, `working_dir`, `sudo`, all `budget_*`/`backup.*`.
`determine_present_action` returns NoOp iff `rl.hash == hash_desired_state(resource)`. The same
defect class has been fixed piecemeal **at least five times** (FJ-127, FJ-035, GH-206, #390, FJ-036).

**Evidence.** Measured: two 6-resource configs differing only in tag/binary/uid/ssh_authorized_keys/
driver_version/cuda_version/checksum/quantization/working_dir/timeout/sudo → `state.lock.yaml` files
`diff`-identical (all 6 blake3 hashes equal), while `codegen --phase apply` diffs show `RELEASE_URL`,
`useradd --uid`, `apt-get install nvidia-driver-*`, `CHECKSUM MISMATCH` and `cd`/`timeout` lines all
differ. `planner/hashing.rs:26-41,76-130`; `planner/mod.rs:361-366`; `executor/resource_ops.rs:189`.

**Proposal.** Replace the allowlist with a canonical serialisation of the **resolved** Resource minus
an explicit denylist of non-identity fields (`depends_on`, `machine`, `tags`, `when`, `count`,
`for_each`, `arch`, `resource_group`, `lifecycle`, `triggers`, `phony`). Alternatively hash the three
*generated scripts* per type the way FJ-036 already does for `disk_budget` — codegen is pure and
cheap. Add the reflection guard the other three lists have. Gate behind a schema bump + `forjar reseal`.

**Competitive.** Terraform hashes the whole resource block; a changed attribute is always a diff.
Pulumi diffs the full input bag. forjar is alone in maintaining a hand-written subset.

**Success criterion.** A property test that walks `serde_yaml_ng::to_value(Resource::default())`,
sets each string/list field to a sentinel, and asserts the hash changes unless the field is on the
denylist — **must fail on current `main`, pass after**. Plus: the 11-field config pair above must
produce differing lock hashes.

---

### E02 · Start ControlMaster before the drift gate, and parallelise it · `performance` · **P0** · M · risk low

**Problem.** With default `policy.tripwire=true`, every apply (including a no-op second apply and a
`-r single-resource` apply) runs `detect_drift_full` over every locked resource of every target
machine — sequentially across machines, unscoped by resource/tag filter, and **before**
`apply_machine` starts ControlMaster. Every query is a full handshake.

**Evidence.** Measured to localhost: fresh `ssh … bash <<<true` median **306 ms** (5 samples);
multiplexed median **6.7 ms** (10 samples) — 45×. `apply_preflight.rs:89` → `apply_drift.rs:63-67`
(`for (machine_name, lock) in &locks`) → `machine.rs:78` starts the master afterwards. The same
40-call drift scan drops **13.61 s → 1.00 s** when the socket is present.

**Proposal.** Hoist a per-machine ControlMaster RAII guard to the top of `cmd_apply`, before the
gate. Run the gate with the same `std::thread::scope` fan-out `forjar drift` already uses
(`cli/drift.rs:220`). Scope the gate by the apply's resource/tag filters.

**Competitive.** Ansible enables `ControlPersist` by default for exactly this reason. forjar already
has the mechanism and simply orders it wrong.

**Success criterion.** A CI lane that counts `ssh` process spawns for a no-op apply against a local
sshd: **≤ 2 per machine** (currently ~1 per locked resource). Plus a criterion bench asserting no-op
apply on 3 machines × 20 resources completes under the spec's stated 500 ms.

---

### E03 · Make signature verification real, or delete it · `security` · **P0** · M · risk medium

**Problem.** `sign --verify` and `sign --pq --verify` compare only the file's BLAKE3 to
`blake3_hash` in the sidecar; the `signature`/`classical_sig`/`pq_sig`/`signer` fields are never read.
`lock-verify-hmac` counts any lock with a sig file as verified without comparing anything, and looks
for the sig at a path `lock-sign` does not write.

**Evidence.** `recipe_signing.rs:75` `let valid = current_hash == sig.blake3_hash;`;
`pq_signing.rs:83` same; `lock_audit.rs:174-206` `// In production, compare against stored HMAC with
key; verified += 1;`. Measured: `sed`-ing signature to `"deadbeef"` and signer to `"root@prod"` →
`sign --verify --json` returns `"valid": true, "signer": "root@prod"`; `--pq --verify` → "both
signatures valid".

**Proposal.** Either implement real signing (ed25519-dalek or age-style X25519 identity, DSSE
envelope, trusted-keys file, optional `apply --require-signed`), **or** delete `sign --pq`, rename
`sign` → `digest`, and delete `lock-verify-hmac` (its honest twin `lock-verify-sig` exists).
Subtraction is the cheaper correct answer.

**Competitive.** Sigstore/cosign binds an OIDC identity to an artifact via a real signature plus a
transparency log. in-toto/SLSA require verifiable attestations. A free-text `signer` nobody checks is
below the floor for this category.

**Success criterion.** Falsification test: mutate one byte of the signature field, assert verify
**fails** with non-zero exit. Must be red on current `main`.

---

### E04 · Redact secrets from run logs before they reach git · `security` · **P0** · S · risk low

**Problem.** `run_capture::capture_output` writes the executed script to `<res>.<action>.log`,
`.json` (field `script`) and `<res>.script`. The executor resolves `{{secrets.*}}` into the resource
*before* codegen, so a secret in `content:` lands in those files. `redact_secrets` exists but has
**no production caller**. `git_commit_state` runs `git add state`.

**Evidence.** `run_capture.rs:66-91`, `resource_ops.rs:493-498`, `template.rs:96-101`,
`apply_helpers.rs:111`. `platform/10-security-model.md:135-150` claims "Secrets are never stored in
state files" and shows a redaction the executor does not perform.

**Proposal.** Thread resolved secret values into `capture_exec_output`, apply `redact_secrets` to
script/stdout/stderr before writing. Add `state/*/runs/` to the `forjar init` gitignore. Add a
`sensitive: true` resource field that suppresses transcript capture entirely.

**Competitive.** Ansible `no_log: true`, Chef `sensitive true`, Salt `show_changes: False` — all a
decade old.

**Success criterion.** Test: apply a file resource whose `content` is `ENC[age,…]`, then grep the
entire `state/` tree for the plaintext — **zero matches**. Red on current `main`.

---

### E05 · Fix drift on the agent-facing surface (MCP/HTTP/`verb call`) · `correctness` · **P0** · S · risk low

**Problem.** `DriftHandler` has `config.machines[name]` in hand but calls `drift::detect_drift(&lock)`
= `detect_drift_reported(lock, None)`. With `machine == None`, file drift falls to `check_file_drift`,
which hashes the **controller's** filesystem. Non-file resources are census-skipped as
`NoConfigLoaded` even though the config *was* loaded, and `DriftOutput` has no census field.

**Evidence.** `mcp/handlers.rs:218`, `drift/mod.rs:54-56`, `drift/file.rs:217-230`,
`mcp/types.rs:108-120`. Measured: machine `web` at 203.0.113.9 (TEST-NET), lock `content_hash` set to
the *controller's* file hash → `verb call drift` returned `{"drifted": false, "findings": []}` in
0.016 s **without contacting the host**.

**Proposal.** Pass `Some(machine)` and resolved resources to `detect_drift_full_reported`; add
`census` to `DriftOutput`.

**Competitive.** This is forjar#305's root cause — documented as fixed in `file.rs`, still live on
every agent-facing transport.

**Success criterion.** Falsification test pointing the verb at a TEST-NET address: must **not** answer
`drifted: false` without either a transport error or an `unchecked` entry.

---

### E06 · Put the store on the apply path, or stop scoring it · `competitive-gap` · **P1** · XL · risk high

**Problem.** `Resource.store` is read in exactly one place — a report field. No file under
`executor/`, `planner/` or `resources/` imports `core::store`. Two identical apt resources, one with
`store: true`, produce **byte-identical** apply scripts while scoring 68 vs 38.

**Evidence.** `model_card.rs:66` is the only reader; `grep -rln 'core::store' src/core/executor
src/core/planner src/resources` → empty. Measured: `ripgrep.apply.sh == ripgrep-plain.apply.sh`
(both `apt-get update && apt-get install -y 'ripgrep=14.1.0-1'`).

**Proposal.** Pick one integration and make it real: for `type: package` with `provider: cargo|uv`
and `store: true`, emit store lookup by `pin_hash` → cache pull → `store-import` on miss → install
from `<entry>/content/`. Those two providers already stage into `$STAGING`. **Until then, remove
`has_store`/`has_sandbox` from `repro_score::compute_score`** so the number only counts what executes.

**Competitive.** In Nix, `nix build` *cannot* produce an output outside the store. That is the entire
gap: forjar's apt still resolves against a live mirror at apply time.

**Success criterion.** Two configs identical but for `store: true` must produce **differing** apply
scripts, and the stored one must install from a store path. If deferred: the score for two
byte-identical scripts must be **equal**.

---

### E07 · Delete the derivation sandbox plan and delegate to pepita · `architecture` · **P1** · L · risk medium

**Problem.** `plan_sandbox_build` emits step 1 `unshare … -- /bin/true` (namespace exits
immediately), step 5 `seccomp-bpf --deny …` (**no such binary**), step 6 `nsenter --target $PID`
(`$PID` never bound), step 8 `forjar-hash-dir` (**not a shipped binary** — Cargo.toml has one
`[[bin]]`), step 9 `mv $out <store>/HASH/content` with the literal string `HASH`. The "output hash"
is a composite of input path *strings* plus script text, never a hash of `$out`.

**Evidence.** `sandbox_exec.rs:113,170,185,206,214`; `sandbox_run.rs:103`; `which seccomp-bpf
forjar-hash-dir` → not found; only `examples/` import these paths; the one test passes `dry_run: true`.

**Proposal.** Delete the shell-text plan for steps 1/5/6/8/9 and delegate to the **working** namespace
sandbox that already exists: `transport::pepita::ensure_namespace` + `exec_in_namespace`, then
`content::content_hash($out)` in Rust, then `atomic_move_to_store` + `seal_output`. The fix is a
delegation, not a rewrite. Downgrade `phase-f` status to 🔧 until it passes.

**Success criterion.** One integration test gated on `unshare` availability that builds
`echo hi > $out/x` and asserts a store entry exists with `output_hash == hash_directory(content)`.

---

### E08 · Collapse three sessions per resource into one · `performance` · **P1** · L · risk medium

**Problem.** A converged resource costs three SSH sessions (check → apply → verify → state-query),
and the apply script **already contains the check it re-runs**. The parallel wave path parallelises
only the apply script; verification and state-query are serialized afterwards. Every file resource is
probed twice per drift scan.

**Evidence.** Fleet model at 100×50 from measured per-session costs. `--refresh` runs each check
script **twice** before the executor, without timeout or multiplexing, then a third time inside it.

**Proposal.** Emit one combined script per resource that runs check, apply-if-needed, and state-query,
returning a structured verdict on a single stream. The `verdict.rs` marker protocol already exists and
is deliberately non-short-circuiting.

**Competitive.** Ansible pipelining collapses module transfer + execution into one connection.

**Success criterion.** Benchmark counting SSH sessions for a 50-resource converged apply: **≤ 1 per
resource + 1 per machine**, down from 3.

---

### E09 · Give the parallel and sequential executors one implementation · `architecture` · **P1** · M · risk medium

**Problem.** Two schedulers have drifted: five features exist only in the sequential path, plus a
thread-panic arm that misattributes failure to index 0, and `post_apply` hooks execute **twice per
resource** on the wave path (untested).

**Evidence.** Lens 1 (`arch:core`). Related to #393/#394, already fixed this session — but the
structural cause (two code paths) remains.

**Proposal.** Make the sequential path a wave scheduler with width 1. One implementation, one set of
features.

**Success criterion.** A parity test that runs the same fixture through both paths and asserts
identical lock contents, event streams and hook invocation counts. Red today on `post_apply`.

---

### E10 · Cut the CLI surface — 163 subcommands, 61 unimplemented flags · `operability` · **P1** · L · risk medium

**Problem.** 163 top-level subcommands / 200 leaves, 33 `lock-*` variants, 1,728 flags, of which
**61 are declared and unimplemented**. A third of `apply`'s flags are advertised but refuse at
runtime. Only 12 of 200 leaves are on the unified verb surface. This is an order of magnitude larger
than any competitor.

**Evidence.** Lens 3. Terraform has ~15 top-level commands; Ansible has ~10 binaries.

**Proposal.** Subtraction. Fold the 33 `lock-*` into `forjar lock <verb>`. Delete the 61
unimplemented flags outright rather than refusing them at runtime (GH-211's refusal was the right
*interim* answer; deletion is the right final one). Publish a deprecation window.

**Success criterion.** `forjar --help` leaf count **< 100**, declared-but-unimplemented flag count
**= 0**, asserted by the existing inert-flag reflection guard.

---

### E11 · Add a facts model · `competitive-gap` · **P1** · XL · risk medium

**Problem.** There is no facts model. `when:`, templates and providers **cannot see the target host** —
no OS, version, architecture, memory, mounted filesystems, or installed package set. Inventory is a
flat static machine map with no groups, group_vars or dynamic sources.

**Evidence.** Lens 6 (`crux:config-mgmt`). Ansible `setup`/`ansible_facts`, Salt grains and pillars,
Chef Ohai, and pyinfra `host.fact.*` are all core to their conditional model.

**Proposal.** A `forjar facts` gather phase caching a typed fact bundle per machine in the state dir,
exposed as `{{facts.os.family}}` etc. and usable in `when:`. Gate on a TTL; make staleness explicit.

**Success criterion.** A config with `when: facts.os.family == "debian"` that converges on Debian and
skips on RHEL, measured against two container targets.

---

### E12 · Make plan able to consult the host · `competitive-gap` · **P2** · L · risk medium

**Problem.** `plan`, `--dry-run` and `check` are **lock-relative** and cannot consult a host.
Terraform refreshes by default; Ansible `--check`, Salt `test=True`, Chef why-run and pyinfra `--dry`
all evaluate against the live target.

**Evidence.** Lens 6, Lens 7. forjar is honest about this (`plan` prints "did not contact any
machine", JSON carries `lock_relative: true`) — the honesty is a strength; the limitation is still a
gap.

**Proposal.** `plan --refresh` that runs state-query before diffing, reusing the drift machinery, with
the ControlMaster fix from E02 so it is affordable.

**Success criterion.** Drift a file on the host, then `plan --refresh` must show it as a change
**without** an intervening `apply` or `drift` run.

---

### E13 · One trust root instead of 16 verify paths · `security` · **P0** · L · risk high

> **Escalated P2 → P0 by adversarial review.** The key-on-argv finding is not a hygiene issue: it is
> credential disclosure to every local user. Verified independently at
> `src/cli/commands/lock_core_args.rs:187-189` — `--key: String` documented as "path to key file **or
> inline**", so an inline secret lands in `ps` output. `LockVerifySigArgs` (`:203-205`) has the same
> shape. This outranks E04, which requires an attacker to already have repo access.

**Problem.** 16 distinct verify/auth paths, 5 MAC/KDF constructions, **0 asymmetric signatures**.
Lock signing is `blake3(content||key)` with the key **on argv** (visible in `ps`), the formula
duplicated in four places, and a false help string. Operator authorization is a self-asserted string.
The state-at-rest MAC uses an unstretched passphrase key beside age's scrypt.

**Evidence.** Lens 9 (`crux:security-supply-chain`).

**Proposal.** One trust root: age/X25519 identities for signing, one KDF, keys from file or agent —
never argv. Fold the 5 MAC constructions into one. Depends on E03.

**Success criterion.** `grep` for key material in `argv` across the CLI = 0 sites; a single
`trust::verify()` entry point with all 16 paths routed through it.

---

### E14 · Stop shipping claims the code does not keep · `testing` · **P2** · M · risk low

**Problem.** `prove` renders UNKNOWN as `[PASS]` and counts it in "N/N proofs passed". `provenance`
emits an unsigned, non-conformant attestation labelled **"SLSA Level 3"**. The tamper-evident event
chain (`tripwire::chain`) has **no production caller** and `append_event` never chains. `moved` blocks
rename state in memory only and are never persisted, while cookbook §34 says the opposite. FAR archive
round-trip loses all content and `archive verify` verifies no bytes.

**Evidence.** Lenses 3, 8, 9. Each is individually small; together they are the audit's structural
theme.

**Proposal.** A `claims.yaml` enumerating every user-visible capability claim with the test that
falsifies it, wired into CI — an extension of the existing contract system to *documentation*.
Downgrade or delete every claim without a binding.

**Success criterion.** Two mechanical assertions, neither relying on a human applying a tag:
(a) `prove` exits **non-zero** if any proof is UNKNOWN, and UNKNOWN never renders as `[PASS]` —
falsifiable today by feeding it an unproven obligation; (b) every capability noun in
`contracts/binding.yaml` resolves to a test id that exists and passes, asserted by walking the
bindings file rather than by scanning prose. *(Review correctly flagged the original marker-based
criterion as unfalsifiable — it depended on humans tagging claims.)*

---

### E15 · Type the error boundary · `architecture` · **P3** · XL · risk medium

**Problem.** The typed error taxonomy is adopted at **1 of ~1,542** `Result<_, String>` sites. Exit
codes are derived by **prose-matching error strings** — `dispatch` returns `Result<(), String>` for
all 163 commands. 173 direct `state.lock.yaml` reads in the CLI bypass `state::load_lock` and swallow
corrupt locks.

**Evidence.** Lens 1, Lens 3.

**Proposal.** Incremental: make `dispatch` return `ForjarError`, convert the exit-code mapping to an
exhaustive match, and route the 173 direct lock reads through `state::load_lock`. The
`into_untyped` `debug_assert` already exists to catch regressions.

**Success criterion.** Zero prose-matching in exit-code derivation (assert by grep); direct
`state.lock.yaml` read sites outside `state/` = 0.

---

## 4. Rejected ideas

Recorded so a reviewer can see what was considered and why it lost.

| Idea | Why rejected |
|---|---|
| **Rewrite as an agent/daemon architecture** for fleet performance | Agentless-over-SSH is forjar's central differentiator against Salt and Chef, and the measured bottleneck (E02, E08) is *ordering and session count*, not the transport model. Fixing ControlMaster ordering recovers 45× without giving up brownfield. |
| **Adopt HCL or a real programming language** for config (Pulumi-style) | The YAML + compile-to-shell pipeline is what makes `codegen --phase apply` auditable, and auditability is a top-3 strength. A general-purpose language would destroy the property that the executed artifact is inspectable ahead of time. |
| **Full Nix parity — make everything go through the store** | E06 as written is already XL. Full parity requires re-expressing every artifact in a build language, which is the exact cost Nix pays and forjar's "import once, own forever" pitch was designed to avoid. Make *one* provider real first, then decide. |
| **Add a plugin ABI for third-party resource types** | `ResourceType` is a 21-variant enum that 237 files depend on; a plugin ABI before E01/E15 would freeze the current flat-struct design as public API. Sequence it after the resource model is fixed. |
| **Ship a hosted state backend / remote locking** (Terraform Cloud–style) | The per-checkout PID lock is a real limitation, but a hosted backend contradicts the single-static-binary, no-daemon property and adds an operational dependency. A shared advisory lock over SSH is the cheaper 80%. |

---

## 5. Open questions

1. **What is the real fleet size?** The 100×50 model is derived from measured per-session costs, not
   observed on a real fleet. If the largest actual deployment is 5 machines, E02/E08 drop below E11.
2. **Is the store strategic or vestigial?** E06 offers two exits (make it real, or stop scoring it).
   That is a product decision the audit cannot make.
3. **Does anyone use `sign --pq`?** If not, E03's delete path is strictly better than its implement
   path.
4. **Is `moved` persistence a bug or unimplemented?** Cookbook §34 documents behaviour the code does
   not have; which side is authoritative changes whether this is a fix or a feature.
5. **How much of the 7.6K lines of core with no in-crate consumer is dead** vs. public API surface
   consumed by the cookbook workspace? Not determinable from this repo alone.

---

## 6. Method and reproducibility

Nine read-only research lenses ran in parallel (`arch:core`, `arch:resources-transport`,
`arch:tripwire-cli-mcp`, `perf:state-hashing`, `perf:execution`, `crux:config-mgmt`, `crux:iac`,
`crux:declarative-os`, `crux:security-supply-chain`), each required to return structured findings with
`file:line` or measured evidence and to report strengths as well as defects. All nine self-reported
evidence quality `measured`. Raw per-lens output is preserved in the workflow journal.

**Adversarial review.** This document was reviewed by an independent agent (`agy --mode plan`) tasked
with reproducing 4 of the 7 criticals from source and attacking the prioritisation, the omissions, the
success criteria and the strengths. Verdict: **SOUND-WITH-CORRECTIONS**. E01, E02, E03 and E05 were
each independently confirmed against the code (`hashing.rs`, `planner/mod.rs:362-366`,
`apply_preflight.rs:89-96`, `machine.rs:78`, `recipe_signing.rs:76`, `lock_audit.rs:184-188`,
`mcp/handlers.rs:218`). Four corrections were accepted and applied: E13 escalated to P0, E14's success
criterion replaced with a mechanical one, and two strengths qualified. One charge was rejected with
evidence — see the severity-vs-priority note in §3.

**Known limits of this audit.** No lens executed `apply` against a real remote fleet — remote numbers
are modelled from measured per-session costs. Competitive claims about Ansible/Terraform/Nix internals
come from training knowledge and cited doc sections, not from running those tools here. The synthesis
step was performed in the main loop after the synthesis agent hit a session limit; candidate
prioritisation is therefore one judgement, not a panel — **E01–E15 have not yet survived adversarial
review**, which is the next stage of this pipeline.

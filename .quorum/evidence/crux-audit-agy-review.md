## Goal Description
Perform an adversarial review of the `docs/specifications/forjar-architecture-crux-audit.md` CRUX audit for the forjar 1.24.0 project.

## User Review Required
Please review the adversarial audit verdict (SOUND-WITH-CORRECTIONS) and the specific findings below. If you approve, I will consider the task complete and generate the walkthrough artifact.

## Proposed Changes

### Verdict: SOUND-WITH-CORRECTIONS

The audit is largely accurate in its mechanical claims (which I have verified against the code), but its synthesis suffers from dropped criticals, misprioritised security vulnerabilities, and overly generous "strengths".

### 1. Verified Critical Claims (Spot-Checks)
The fundamental mechanical claims of the audit are **correct and sound**:
*   **E01 (Hash the whole resource):** Confirmed. `src/core/planner/hashing.rs` only collects ~35 fields. Critical fields like `uid` (defined in `src/core/types/resource.rs:110`) are entirely absent from the `collect_core_fields` and `collect_phase2_fields` functions. `src/core/planner/mod.rs:362-366` strictly returns `PlanAction::NoOp` on hash equality.
*   **E02 (ControlMaster before drift gate):** Confirmed. `src/cli/apply_preflight.rs:89-96` calls `check_pre_apply_drift`, which sequentially loops over machines doing full SSH handshakes. The multiplexing `ControlMaster` is not started until later in `src/core/executor/machine.rs:78` (`transport::ssh::start_control_master`).
*   **E03 (Fake signature verification):** Confirmed. `src/cli/recipe_signing.rs:76` checks `current_hash == sig.blake3_hash` without inspecting `sig.signature`. Even worse, `src/cli/lock_audit.rs:184-188` increments `verified += 1` purely if `sig_path.exists()` without validating the HMAC.
*   **E05 (MCP DriftHandler passes None):** Confirmed. `src/mcp/handlers.rs:218` calls `drift::detect_drift(&lock_data)`, skipping the machine context and falling back to hashing the controller's local filesystem.

### 2. Prioritisation Attacks
The prioritisation panel failed on security and operational reality:
*   **E13 is P0, not P2:** The audit notes in E13 that lock signing uses `blake3(content||key)` with the **key on argv**. `argv` is visible to all users on a machine via `ps`. Exposing cryptographic keys in `ps` is a catastrophic security vulnerability and should heavily outrank E04 (which is P0).
*   **E10 (CLI Surface) is P0/P1 but framed poorly:** Shipping 61 unimplemented flags is a broken product surface. Framing this as a simple "operability" fix (P1) underestimates the user trust damage.

### 3. Missing Findings
*   **Dropped Criticals:** The executive summary claims "118 findings (7 critical...)". However, the candidates list (E01-E15) only contains **5 P0s** (E01-E05). The synthesis has silently dropped 2 critical findings. The "workflow journal" containing the raw severity list is missing from the repository, making it impossible to audit the omissions, which is a major transparency failure for an audit that prides itself on "falsifiable success criteria".

### 4. Success Criteria Checks
*   Most are concrete and falsifiable.
*   **E14 (Stop shipping claims the code does not keep)** is behaviorally unfalsifiable. The success criterion is "CI job asserting every claim... with a `[verified]` marker has a passing falsification test". This relies on humans correctly applying the `[verified]` tag, rather than mechanically asserting all claims.

### 5. Rejected Section and Strengths Checks
The audit grades forjar far too generously on its strengths:
*   **"Institutionalised honesty" is a false strength:** The audit praises the CLI for returning an error on unimplemented flags instead of silently no-op'ing. While better than silence, shipping 61 unimplemented flags (a third of the apply surface) is vaporware. It should not be listed as a strength.
*   **"Compile-to-shell auditability" is contradicted:** The audit praises how the tool prints the exact bytes that will execute. But E07 points out that the derivation sandbox emits commands that do not even exist (`seccomp-bpf`, `forjar-hash-dir`). Generating shell scripts containing non-existent binaries defeats the purpose of auditability.

## Verification Plan
N/A - This is a read-only review task. No code changes are being proposed.

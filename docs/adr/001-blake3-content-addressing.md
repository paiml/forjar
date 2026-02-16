# ADR-001: BLAKE3 Content-Addressed State

## Status
Accepted

## Context
Infrastructure state needs a deterministic, collision-resistant fingerprint to detect drift and enable idempotent operations. Options considered:
- SHA-256: Standard but slow (no SIMD acceleration in pure Rust)
- SHA-3: Newer standard but slower than SHA-256 for our use case
- BLAKE3: Fastest cryptographic hash, pure Rust, tree-hashing for parallelism

## Decision
Use BLAKE3 for all state hashing. Format: `"blake3:{hex}"`.

## Consequences
- **Positive**: Microsecond drift detection via hash comparison (no API calls)
- **Positive**: Deterministic — same input always produces same hash
- **Positive**: Single dependency (`blake3` crate), pure Rust, no C bindings
- **Negative**: Not NIST-approved (BLAKE3 is based on BLAKE2/ChaCha, not SHA family)
- **Negative**: Lock files are not interoperable with SHA-256-based tools

## Falsification
This decision would be wrong if:
1. BLAKE3 produces collisions on real infrastructure state (extremely unlikely, 256-bit output)
2. A compliance requirement mandates NIST-approved hashes (would require migration path)
3. Hash computation becomes a bottleneck (measure with `cargo bench`)

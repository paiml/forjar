# Nix-Compatible Reproducible Package Management — Specification Index

**Version**: 0.1.0-spec
**Status**: Active
**Author**: Noah Gift / Pragmatic AI Labs
**Ticket Range**: FJ-1300–FJ-1399

---

## Status Legend

| Icon | Meaning |
|------|---------|
| ✅ | Complete — types, tests, and example coverage |
| 🔧 | Partial — types done, execution/integration missing |
| 🔲 | Spec only — not yet implemented |

---

## Table of Contents

| Phase | File | Tickets | Status | Summary |
|-------|------|---------|--------|---------|
| **A** | [Store Model](phase-a-store.md) | FJ-1300–1304 | ✅ | Path derivation, metadata, profiles, YAML integration, references |
| **B** | [Purity](phase-b-purity.md) | FJ-1305–1309 | ✅ | 4-level purity model, monotonicity, static analysis, closure tracking |
| **C** | [Closures & Locking](phase-c-closure.md) | FJ-1310–1314 | ✅ | Lock file format, CLI commands, tripwire integration |
| **D** | [Repro Scoring](phase-d-scoring.md) | FJ-1315–1319 | ✅ | Build sandboxing, isolation levels, preset profiles |
| **E** | [Cache & GC](phase-e-cache.md) | FJ-1320–1329 | 🔧 | SSH cache, substitution, GC — types ✅ / execution bridges ✅ / CLI wiring 🔲 |
| **F** | [Derivations](phase-f-derivation.md) | FJ-1330–1344 | 🔧 | Universal provider import, derivation model — types ✅ / execution bridges ✅ / CLI wiring 🔲 |
| **G** | [FAR Archive](phase-g-far.md) | FJ-1345–1349 | ✅ | FAR binary format, encode/decode, streaming, chunking |
| **H** | [Import/Convert](phase-h-convert.md) | FJ-1345–1349 | 🔧 | Conversion ladder, reproducibility score — types ✅ / --apply bridge ✅ / CLI wiring 🔲 |
| **I** | [Security & Auditability](phase-i-security.md) | FJ-1356 | ✅ | Secret scanning, regex detection, encrypted value validation |
| **J** | [Performance Guarantees](phase-j-benchmarks.md) | FJ-1355 | ✅ | Criterion benchmarks for all store operations |
| **K** | [Bash Provability](phase-k-bash.md) | FJ-1357 | ✅ | I8 invariant enforcement at all exec entry points |
| **L** | [Execution Layer](phase-l-execution.md) | FJ-1358–1365 | 🔧 | Execution bridges: provider, GC, pins, cache, convert, diff/sync, sandbox |

---

## Architecture

```
/var/forjar/store/
├── <blake3-hash>/
│   ├── meta.yaml       # Provenance, input hashes, timestamps
│   └── content/        # Build output (files, dirs)
└── .gc-roots/          # Symlinks to live store paths
```

## Key Invariants

- **Store**: Write-once (hash = identity). Atomic creation via temp-dir + rename.
- **Purity**: Monotonicity — resource purity ≥ max(dep_levels).
- **Derivations**: Input immutability (read-only bind mounts). Output isolation ($out).
- **Lock file**: Completeness (all inputs pinned). Freshness (stale hashes detected).
- **Security**: No plaintext secrets in config. All sensitive values must use `ENC[age,...]`.
- **Bash (I8)**: No raw shell execution — all shell is bashrs-validated before transport.

## References

See individual phase files for inline citations. Full bibliography in the original specification.

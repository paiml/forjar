# Phase G: FAR Archive (FJ-1346–FJ-1349)

**Status**: ✅ Complete
**Implementation**: `src/core/store/far.rs`, `src/core/store/chunker.rs`, `src/core/store/kernel_far.rs`

---

## 1. Archive Layout (FJ-1346)

Forjar's sovereign package format — FAR (Forjar ARchive). BLAKE3 verified streaming, content-defined chunking, built-in provenance.

Advantages over NAR (2003): BLAKE3 verified streaming (vs SHA256), content-defined chunking for delta transfers, manifest-first metadata access, zstd compression, chunk-level resume, inline provenance.

Layout:
```
magic (12B) → manifest length (u64 LE) → manifest (YAML, zstd)
→ chunk table (per-chunk: blake3 hash + offset + length)
→ chunks 0..N (zstd, CDC boundaries ~64KB) → signature (age identity over manifest hash)
```

Zstd/age are transitive via `age v0.11`; BLAKE3 is direct. **Zero new crates.**

## 2. Manifest Schema (FJ-1347)

```yaml
schema: "1.0"
name: "ripgrep"
version: "14.1.0"
arch: "x86_64"
store_hash: "blake3:abc123..."
tree_hash: "blake3:def456..."
recipe_hash: "blake3:789abc..."
input_closure: ["blake3:aaa...", "blake3:bbb..."]
provenance:
  sandbox_level: "full"
  builder_identity: "forjar@build-01"
  built_at: "2026-03-02T10:00:00Z"
files:
  - path: "bin/rg"
    size: 5242880
    mode: "0755"
    hash: "blake3:eee..."
    chunks: [0,1,2,3,4]
```

## 3. Streaming and Chunking (FJ-1348–FJ-1349)

BLAKE3 tree hashing [11]: 256KB chunks hashed independently, combined in binary tree — parallel verification, partial verification, incremental transfer, resume. Content-defined chunking at ~64KB boundaries — only changed chunks transfer on version updates.

Implementation note: Rabin fingerprinting is the classical CDC algorithm [12] but FastCDC (USENIX ATC 2016) achieves 3–10x higher throughput at equal deduplication ratios. Recent evaluation [13] shows CDC algorithm choice significantly impacts throughput-dedup tradeoff. Phase H should benchmark FastCDC vs Rabin before committing.

## 4. CLI

```bash
forjar archive pack <store-hash>     # pack store entry into .far
forjar archive unpack <file.far>     # unpack .far into store
forjar archive inspect <file.far>    # print manifest without unpacking
forjar archive verify <file.far>     # verify chunk hashes + signature
```

---

## References

- [11] L. Champine, "Streaming Merkle Proofs within Binary Numeral Trees," IACR ePrint 2021/038, 2021
- [12] M. O. Rabin, "Fingerprinting by Random Polynomials," TR-15-81, Harvard, 1981
- [13] M. Gregoriadis et al., "A Thorough Investigation of CDC Algorithms for Data Deduplication," arXiv:2409.06066, 2024

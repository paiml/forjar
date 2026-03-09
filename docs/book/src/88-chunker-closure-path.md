# Content Chunking, Input Closures & Store Paths

Falsification coverage for FJ-1347 (content chunking), FJ-1307 (input closures), and FJ-1300 (store paths).

## Content Chunking (FJ-1347)

Fixed 64KB BLAKE3-hashed chunks with binary Merkle tree hashing:

```rust
use forjar::core::store::chunker::{chunk_bytes, reassemble, tree_hash, CHUNK_SIZE};

// CHUNK_SIZE = 65536 (64KB)
let data = vec![42u8; CHUNK_SIZE * 3 + 100];
let chunks = chunk_bytes(&data);
assert_eq!(chunks.len(), 4);

// Each chunk has a 32-byte BLAKE3 hash
assert_eq!(chunks[0].hash.len(), 32);

// Lossless reassembly
assert_eq!(reassemble(&chunks), data);

// Merkle tree hash is deterministic and content-sensitive
let tree = tree_hash(&chunks);
assert_eq!(tree.len(), 32);
assert_ne!(tree_hash(&chunk_bytes(b"a")), tree_hash(&chunk_bytes(b"b")));
```

### Edge Cases

| Input | Chunks | Behavior |
|-------|--------|----------|
| Empty | 0 | `chunk_bytes(b"")` returns empty vec |
| < 64KB | 1 | Single chunk with full data |
| Exactly 64KB | 1 | Single chunk, no remainder |
| 64KB + 1 | 2 | Second chunk has 1 byte |

## Input Closures (FJ-1307)

Transitive dependency graph walking with cycle protection:

```rust
use forjar::core::store::closure::{input_closure, closure_hash, all_closures, ResourceInputs};

// Build dependency graph
let mut graph = BTreeMap::new();
graph.insert("libc".into(), ResourceInputs {
    input_hashes: vec!["blake3:libc-src".into()],
    depends_on: vec![],
});
graph.insert("curl".into(), ResourceInputs {
    input_hashes: vec!["blake3:curl-src".into()],
    depends_on: vec!["libc".into()],
});

// Transitive closure includes all upstream inputs
let closure = input_closure("curl", &graph);
assert!(closure.contains(&"blake3:libc-src".to_string()));
assert!(closure.contains(&"blake3:curl-src".to_string()));

// Deterministic BLAKE3 hash of closure
let hash = closure_hash(&closure);
assert!(hash.starts_with("blake3:"));
```

### Graph Properties

- **Diamond dependencies**: Deduplicated (4 unique hashes from diamond with shared base)
- **Missing resources**: Traversal stops silently at unknown nodes
- **Cycles**: Protected via visited set; `a -> b -> a` terminates correctly

## Store Paths (FJ-1300)

Deterministic content-addressed store paths:

```rust
use forjar::core::store::path::{store_path, store_entry_path, STORE_BASE};

// STORE_BASE = "/var/lib/forjar/store"

// Deterministic: same inputs always produce same hash
let h1 = store_path("recipe1", &["in1", "in2"], "x86_64", "apt");
let h2 = store_path("recipe1", &["in1", "in2"], "x86_64", "apt");
assert_eq!(h1, h2);

// Order-independent: inputs are sorted internally
let h3 = store_path("recipe1", &["in2", "in1"], "x86_64", "apt");
assert_eq!(h1, h3);

// Sensitive to recipe, arch, and provider
assert_ne!(store_path("r", &[], "x86_64", "apt"),
           store_path("r", &[], "aarch64", "apt"));

// Entry path strips blake3: prefix
let entry = store_entry_path("blake3:abcdef");
assert_eq!(entry, "/var/lib/forjar/store/abcdef");
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_chunker_closure_path.rs` | 27 | ~255 |

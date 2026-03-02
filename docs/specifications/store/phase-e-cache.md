# Phase E: Cache & GC (FJ-1320–FJ-1329)

**Status**: 🔧 Partial — types ✅ / SSH execution 🔲
**Implementation**: `src/core/store/cache.rs`, `src/core/store/substitution.rs`, `src/core/store/gc.rs`

---

## 1. Cache Transport (FJ-1320)

SSH-only. Sovereign — no HTTP client crate, no tokens, no TLS certificates. Uses forjar's existing SSH transport. HTTP-based package registries are a documented attack surface (339 malicious packages found in npm/PyPI/RubyGems [9], 107 unique supply chain attack vectors [10]). SSH transport eliminates the registry attack class but introduces its own surface (key management, agent forwarding). This is a design position, not an empirically proven security improvement.

```yaml
cache:
  sources:
    - type: ssh
      host: cache.internal
      user: forjar
      path: /var/forjar/cache
    - type: local
      path: /var/forjar/store
```

## 2. Substitution Protocol (FJ-1322)

Compute store hash from input closure → check local store → check SSH cache sources → build from scratch (sandbox if configured) → store result, optionally push to cache.

## 3. CLI (FJ-1323–FJ-1324)

```bash
forjar cache list                    # list local store entries
forjar cache push <remote>           # push local store to SSH remote
forjar cache pull <hash>             # pull specific entry from cache
forjar cache verify                  # verify all store entries (re-hash)
```

## 4. Garbage Collection (FJ-1325–FJ-1327)

**GC roots** (FJ-1325): current profile symlink, profile generations (keep last N), lock file pins, `.gc-roots/` symlinks.

**Mark-and-sweep** (FJ-1326): walk roots, follow `references` in `meta.yaml`, mark as live. Unreachable entries are dead.

**CLI** (FJ-1327): `forjar store gc` (delete unreachable), `--dry-run`, `--older-than 90d`, `--keep-generations 5`. GC is never automatic.

## 5. Remaining Work

| Gap | Status | Description |
|-----|--------|-------------|
| SSH cache pull/push | 🔲 | Actual `scp`/`rsync` transport for cache entries |
| GC sweep | 🔲 | Actual `rm -rf` of unreachable store entries |
| Substitution network | 🔲 | SSH probing of remote cache sources |

---

## References

- [9] R. Duan et al., "Towards Measuring Supply Chain Attacks on Package Managers," arXiv:2002.01139, 2020
- [10] P. Ladisa et al., "Taxonomy of Attacks on Open-Source Software Supply Chains," arXiv:2204.04008, 2022

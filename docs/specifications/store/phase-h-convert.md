# Phase H: Import/Convert (FJ-1345–FJ-1349)

**Status**: ✅ Complete — types + --apply execution + CLI wiring
**Implementation**: `src/core/store/convert.rs`, `src/core/store/convert_exec.rs`, `src/core/store/pin_resolve.rs`

---

## 1. Conversion Ladder

The five-step conversion ladder progressively moves recipes from impure to pure:

1. Add `version:` pins to all packages → Constrained → Pinned
2. Add `store: true` to cacheable resources → Enables store
3. Generate `forjar.inputs.lock.yaml` → Pins all inputs
4. Add `sandbox:` blocks → Pinned → Pure
5. Replace imperative hooks with declarative resources → Full purity

`forjar convert --reproducible` automates steps 1-3.

## 2. Cookbook Recipes (FJ-1330–FJ-1332)

| Recipe | Description |
|--------|-------------|
| 63 | Version-pinned apt with store |
| 64 | Cargo sandbox + input lock |
| 65 | Multi-machine SSH cache deploy |
| 66 | Reproducibility score CI gate |
| 67 | Profile generation rollback |

## 3. Implementation Status

| Component | Status | Implementation |
|-----------|--------|----------------|
| `--apply` mode | ✅ | `convert_exec::apply_conversion()` — backup, YAML modify, lock gen, atomic write |
| Version resolution | ✅ | `pin_resolve::resolve_all_pins()` — per-provider CLI queries (apt, cargo, nix, uv) |
| CLI wiring | ✅ | `forjar convert --reproducible --apply`, `forjar pin --check` |

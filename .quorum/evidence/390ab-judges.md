# Judge scores — #390-A/B capture placement

| option | race safety | code moved | verdict |
|---|---|---|---|
| capture inside the spawn closure | poor — `ensure_run_dir` is not synchronised and N threads would race the same run dir | none | rejected |
| **carry the script out, capture in Phase 3** | **safe — Phase 3 already serialises** | **a signature widening** | **chosen** |
| write to a per-thread temp dir and merge | safe | most | rejected — invents a second layout to reconcile |

The chosen option costs a signature change rippling into machine_b.rs, which is
precisely why the defect survived this long; it introduces no new concurrency.

//! Per-source `data:` resolution for `drift`.

/// Resolve each `data:` source on its own. A failing source must not abort
/// drift, and must not stop the sources after it from resolving (the bulk
/// resolver returns at the first failure, which left a healthy later source
/// reported as "unknown data source"). Each failure is named on stderr.
pub(super) fn resolve_sources_individually(cfg: &mut crate::core::types::ForjarConfig) {
    let keys: Vec<String> = cfg.data.keys().cloned().collect();
    for key in keys {
        let mut one = cfg.clone();
        one.data.retain(|k, _| *k == key);
        match crate::core::resolver::resolve_data_sources(&mut one) {
            Ok(()) => {
                let param = format!("__data__{key}");
                if let Some(v) = one.params.get(&param) {
                    cfg.params.insert(param, v.clone());
                }
            }
            Err(e) => eprintln!("warning: data source '{key}': {e}"),
        }
    }
}

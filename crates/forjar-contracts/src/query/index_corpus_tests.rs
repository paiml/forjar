//! The `ContractIndex` tests that need aprender's contract corpus (#452).
//!
//! Split out of `index.rs` for one mechanical reason: that file sits at the
//! repo's 500-line ceiling and the pre-commit ratchet forbids growth, so the
//! `ignore` attributes could not be added in place. Nothing else changed —
//! same bodies, same assertions, one module deeper in the test path.
//!
//! Every test here indexes `<crate>/../../contracts` and then asserts on
//! `softmax-kernel-v1` / `rmsnorm-kernel-v1`. forjar vendors this crate but not
//! aprender's corpus, so those stems are absent and the assertions fail on a
//! fact about the vendoring rather than about `ContractIndex`. See VENDORED.md.

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    #[cfg_attr(
        not(feature = "aprender-corpus"),
        ignore = "needs aprender's kernel corpus in contracts/ (softmax/rmsnorm); forjar ships an IaC corpus (#452)"
    )]
    fn index_from_contracts_dir() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts");
        let index = ContractIndex::build_from_directory(&dir).unwrap();
        assert!(index.entries.len() > 10, "Should index many contracts");
        assert!(index.get_by_stem("softmax-kernel-v1").is_some());
    }

    #[test]
    #[cfg_attr(
        not(feature = "aprender-corpus"),
        ignore = "needs aprender's kernel corpus in contracts/ (softmax/rmsnorm); forjar ships an IaC corpus (#452)"
    )]
    fn bm25_ranks_relevant_first() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts");
        let index = ContractIndex::build_from_directory(&dir).unwrap();
        let results = index.bm25_search("softmax numerical stability");
        assert!(!results.is_empty());
        // Top result should be related to softmax/cross-entropy (both reference softmax)
        let top = &index.entries[results[0].0];
        assert!(
            top.corpus_text.to_lowercase().contains("softmax"),
            "Top result corpus should mention softmax, got stem={}",
            top.stem,
        );
    }

    #[test]
    #[cfg_attr(
        not(feature = "aprender-corpus"),
        ignore = "needs aprender's kernel corpus in contracts/ (softmax/rmsnorm); forjar ships an IaC corpus (#452)"
    )]
    fn literal_search_finds_match() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts");
        let index = ContractIndex::build_from_directory(&dir).unwrap();
        let matches = index.literal_search("RMSNorm", false);
        assert!(!matches.is_empty());
    }

    #[test]
    #[cfg_attr(
        not(feature = "aprender-corpus"),
        ignore = "needs aprender's kernel corpus in contracts/ (softmax/rmsnorm); forjar ships an IaC corpus (#452)"
    )]
    fn regex_search_finds_patterns() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts");
        let index = ContractIndex::build_from_directory(&dir).unwrap();
        let matches = index.regex_search(r"(?i)softmax|log.softmax").unwrap();
        assert!(!matches.is_empty());
    }

    #[test]
    #[cfg_attr(
        not(feature = "aprender-corpus"),
        ignore = "needs aprender's kernel corpus in contracts/ (softmax/rmsnorm); forjar ships an IaC corpus (#452)"
    )]
    fn pagerank_produces_valid_scores() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts");
        let index = ContractIndex::build_from_directory(&dir).unwrap();
        let scores = index.pagerank(20, 0.85);
        // scores is a HashMap keyed by stem — duplicate stems collapse to one entry
        let unique_stems: std::collections::HashSet<_> =
            index.entries.iter().map(|e| &e.stem).collect();
        assert_eq!(scores.len(), unique_stems.len());
        // All scores should be positive
        for s in scores.values() {
            assert!(*s > 0.0, "PageRank should be positive");
        }
        // Softmax should rank relatively high (many things depend on it)
        let softmax = scores.get("softmax-kernel-v1").unwrap();
        #[allow(clippy::cast_precision_loss)]
        let mean = scores.values().sum::<f64>() / scores.len() as f64;
        assert!(
            *softmax >= mean,
            "softmax ({softmax:.4}) should be >= mean ({mean:.4})"
        );
    }

    #[test]
    #[cfg_attr(
        not(feature = "aprender-corpus"),
        ignore = "needs aprender's kernel corpus in contracts/ (softmax/rmsnorm); forjar ships an IaC corpus (#452)"
    )]
    fn from_directory_uses_cache() {
        // Build from a temp dir to avoid contaminating the real .pv cache
        let tmp = std::env::temp_dir().join("pv_from_dir_cache_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        // Copy a few contracts to temp
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts");
        for name in &["softmax-kernel-v1.yaml", "rmsnorm-kernel-v1.yaml"] {
            let content = std::fs::read_to_string(src.join(name)).unwrap();
            std::fs::write(tmp.join(name), content).unwrap();
        }

        // First call builds + caches
        let idx1 = ContractIndex::from_directory(&tmp).unwrap();
        assert!(idx1.entries.len() >= 2);

        // Second call should hit cache
        let idx2 = ContractIndex::from_directory(&tmp).unwrap();
        assert_eq!(idx1.entries.len(), idx2.entries.len());

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(tmp.parent().unwrap().join(".pv"));
    }
}

//! FJ-3406: Plugin hot-reload via BLAKE3 hash check.
//!
//! Caches loaded plugin manifests and detects when .wasm files change
//! by comparing BLAKE3 hashes before each invocation.

use crate::core::types::PluginManifest;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A cached plugin entry with its BLAKE3 hash at load time.
#[derive(Debug, Clone)]
pub struct CachedPlugin {
    /// Parsed manifest.
    pub manifest: PluginManifest,
    /// Path to the WASM module.
    pub wasm_path: PathBuf,
    /// BLAKE3 hash of the .wasm file when it was last loaded.
    pub loaded_hash: String,
    /// Timestamp of last successful load (monotonic counter).
    pub load_generation: u64,
}

/// Plugin cache with BLAKE3-based hot-reload detection.
#[derive(Debug)]
pub struct PluginCache {
    plugins: HashMap<String, CachedPlugin>,
    generation: u64,
}

impl Default for PluginCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginCache {
    /// Create an empty plugin cache.
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            generation: 0,
        }
    }

    /// Insert or update a plugin in the cache.
    pub fn insert(&mut self, name: &str, manifest: PluginManifest, wasm_path: PathBuf) {
        let hash = compute_file_hash(&wasm_path).unwrap_or_default();
        self.generation += 1;
        self.plugins.insert(
            name.to_string(),
            CachedPlugin {
                manifest,
                wasm_path,
                loaded_hash: hash,
                load_generation: self.generation,
            },
        );
    }

    /// Check if a plugin needs reload (WASM file hash has changed).
    pub fn needs_reload(&self, name: &str) -> ReloadCheck {
        let entry = match self.plugins.get(name) {
            Some(e) => e,
            None => return ReloadCheck::NotCached,
        };

        let current_hash = match compute_file_hash(&entry.wasm_path) {
            Some(h) => h,
            None => return ReloadCheck::FileGone,
        };

        if current_hash == entry.loaded_hash {
            ReloadCheck::UpToDate
        } else {
            ReloadCheck::Changed {
                old_hash: entry.loaded_hash.clone(),
                new_hash: current_hash,
            }
        }
    }

    /// Get a cached plugin if it exists and is up-to-date.
    /// Returns None if the plugin needs reload or is not cached.
    pub fn get_if_current(&self, name: &str) -> Option<&CachedPlugin> {
        if matches!(self.needs_reload(name), ReloadCheck::UpToDate) {
            self.plugins.get(name)
        } else {
            None
        }
    }

    /// Remove a plugin from the cache.
    pub fn remove(&mut self, name: &str) -> bool {
        self.plugins.remove(name).is_some()
    }

    /// List all cached plugin names.
    pub fn cached_names(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    /// Number of cached plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Clear all cached plugins.
    pub fn clear(&mut self) {
        self.plugins.clear();
    }

    /// Get the current generation counter.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Scan all cached plugins and return names of those needing reload.
    pub fn stale_plugins(&self) -> Vec<String> {
        self.plugins
            .iter()
            .filter(|(name, _)| !matches!(self.needs_reload(name), ReloadCheck::UpToDate))
            .map(|(name, _)| name.clone())
            .collect()
    }
}

/// Result of checking whether a plugin needs reload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadCheck {
    /// Plugin is up-to-date, no reload needed.
    UpToDate,
    /// Plugin WASM file has changed.
    Changed {
        /// Hash when the plugin was loaded.
        old_hash: String,
        /// Current hash on disk.
        new_hash: String,
    },
    /// Plugin is not in the cache.
    NotCached,
    /// WASM file no longer exists on disk.
    FileGone,
}

impl ReloadCheck {
    /// Whether the plugin should be reloaded.
    pub fn should_reload(&self) -> bool {
        !matches!(self, Self::UpToDate)
    }
}

/// Compute the BLAKE3 hash of a file, returning None if the file cannot be read.
pub fn compute_file_hash(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(blake3::hash(&bytes).to_hex().to_string())
}

/// Resolve, verify, and cache a plugin (combining loader + cache).
///
/// If the plugin is already cached and up-to-date, returns the cached entry.
/// Otherwise, resolves and verifies the plugin, then caches it.
pub fn resolve_cached(
    cache: &mut PluginCache,
    plugin_dir: &Path,
    plugin_name: &str,
) -> Result<CachedPlugin, String> {
    // Check if we have a current cached version
    if let Some(cached) = cache.get_if_current(plugin_name) {
        return Ok(cached.clone());
    }

    // Resolve and verify from disk
    let resolved = crate::core::plugin_loader::resolve_and_verify(plugin_dir, plugin_name)?;
    cache.insert(
        plugin_name,
        resolved.manifest.clone(),
        resolved.wasm_path.clone(),
    );

    Ok(CachedPlugin {
        manifest: resolved.manifest,
        wasm_path: resolved.wasm_path,
        loaded_hash: cache.plugins.get(plugin_name).unwrap().loaded_hash.clone(),
        load_generation: cache.generation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_wasm(dir: &TempDir, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.path().join(format!("{name}.wasm"));
        std::fs::write(&path, content).unwrap();
        path
    }

    fn test_manifest(name: &str) -> PluginManifest {
        PluginManifest {
            name: name.to_string(),
            version: "0.1.0".into(),
            description: Some("test".into()),
            abi_version: 1,
            wasm: format!("{name}.wasm"),
            blake3: String::new(),
            permissions: Default::default(),
            schema: None,
        }
    }

    #[test]
    fn cache_insert_and_get() {
        let dir = TempDir::new().unwrap();
        let path = write_wasm(&dir, "test", b"wasm bytes");
        let mut cache = PluginCache::new();

        cache.insert("test", test_manifest("test"), path);
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());
        assert!(cache.get_if_current("test").is_some());
    }

    #[test]
    fn cache_detects_change() {
        let dir = TempDir::new().unwrap();
        let path = write_wasm(&dir, "mod", b"original");
        let mut cache = PluginCache::new();

        cache.insert("mod", test_manifest("mod"), path.clone());
        assert_eq!(cache.needs_reload("mod"), ReloadCheck::UpToDate);

        // Modify the file
        std::fs::write(&path, b"modified content").unwrap();
        let check = cache.needs_reload("mod");
        assert!(check.should_reload());
        match check {
            ReloadCheck::Changed { old_hash, new_hash } => {
                assert_ne!(old_hash, new_hash);
            }
            _ => panic!("expected Changed"),
        }
    }

    #[test]
    fn cache_not_cached() {
        let cache = PluginCache::new();
        assert_eq!(cache.needs_reload("nonexistent"), ReloadCheck::NotCached);
        assert!(cache.needs_reload("nonexistent").should_reload());
    }

    #[test]
    fn cache_file_gone() {
        let dir = TempDir::new().unwrap();
        let path = write_wasm(&dir, "gone", b"data");
        let mut cache = PluginCache::new();

        cache.insert("gone", test_manifest("gone"), path.clone());
        std::fs::remove_file(&path).unwrap();
        assert_eq!(cache.needs_reload("gone"), ReloadCheck::FileGone);
    }

    #[test]
    fn cache_remove() {
        let dir = TempDir::new().unwrap();
        let path = write_wasm(&dir, "rem", b"data");
        let mut cache = PluginCache::new();

        cache.insert("rem", test_manifest("rem"), path);
        assert!(cache.remove("rem"));
        assert!(!cache.remove("rem"));
        assert!(cache.is_empty());
    }

    #[test]
    fn cache_clear() {
        let dir = TempDir::new().unwrap();
        let mut cache = PluginCache::new();
        cache.insert("a", test_manifest("a"), write_wasm(&dir, "a", b"1"));
        cache.insert("b", test_manifest("b"), write_wasm(&dir, "b", b"2"));
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn cache_generation_increments() {
        let dir = TempDir::new().unwrap();
        let mut cache = PluginCache::new();
        assert_eq!(cache.generation(), 0);

        cache.insert("a", test_manifest("a"), write_wasm(&dir, "a", b"1"));
        assert_eq!(cache.generation(), 1);

        cache.insert("b", test_manifest("b"), write_wasm(&dir, "b", b"2"));
        assert_eq!(cache.generation(), 2);
    }

    #[test]
    fn cached_names() {
        let dir = TempDir::new().unwrap();
        let mut cache = PluginCache::new();
        cache.insert(
            "alpha",
            test_manifest("alpha"),
            write_wasm(&dir, "alpha", b"a"),
        );
        cache.insert(
            "beta",
            test_manifest("beta"),
            write_wasm(&dir, "beta", b"b"),
        );

        let mut names = cache.cached_names();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn stale_plugins_after_modification() {
        let dir = TempDir::new().unwrap();
        let path_a = write_wasm(&dir, "fresh", b"unchanged");
        let path_b = write_wasm(&dir, "stale", b"will change");
        let mut cache = PluginCache::new();

        cache.insert("fresh", test_manifest("fresh"), path_a);
        cache.insert("stale", test_manifest("stale"), path_b.clone());

        std::fs::write(&path_b, b"new content").unwrap();

        let stale = cache.stale_plugins();
        assert_eq!(stale, vec!["stale"]);
    }

    #[test]
    fn get_if_current_returns_none_on_change() {
        let dir = TempDir::new().unwrap();
        let path = write_wasm(&dir, "check", b"original");
        let mut cache = PluginCache::new();

        cache.insert("check", test_manifest("check"), path.clone());
        assert!(cache.get_if_current("check").is_some());

        std::fs::write(&path, b"changed").unwrap();
        assert!(cache.get_if_current("check").is_none());
    }

    #[test]
    fn compute_file_hash_works() {
        let dir = TempDir::new().unwrap();
        let path = write_wasm(&dir, "hash", b"test data");
        let hash = compute_file_hash(&path).unwrap();
        assert_eq!(hash.len(), 64); // BLAKE3 hex is 64 chars
    }

    #[test]
    fn compute_file_hash_missing() {
        assert!(compute_file_hash(Path::new("/nonexistent/file.wasm")).is_none());
    }

    #[test]
    fn reload_check_should_reload() {
        assert!(!ReloadCheck::UpToDate.should_reload());
        assert!(ReloadCheck::NotCached.should_reload());
        assert!(ReloadCheck::FileGone.should_reload());
        assert!(ReloadCheck::Changed {
            old_hash: "a".into(),
            new_hash: "b".into(),
        }
        .should_reload());
    }

    #[test]
    fn default_cache() {
        let cache = PluginCache::default();
        assert!(cache.is_empty());
        assert_eq!(cache.generation(), 0);
    }
}

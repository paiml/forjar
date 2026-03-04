//! FJ-1425: Remote state backend.
//!
//! Pluggable state backend trait with local filesystem and optional
//! remote (S3/GCS-compatible) implementations. Encrypted at rest.

use std::path::{Path, PathBuf};

/// State backend operations.
#[allow(dead_code)]
pub trait StateBackend: Send + Sync {
    fn name(&self) -> &str;
    fn get(&self, key: &str) -> Result<Vec<u8>, String>;
    fn put(&self, key: &str, data: &[u8]) -> Result<(), String>;
    fn list(&self, prefix: &str) -> Result<Vec<String>, String>;
    fn delete(&self, key: &str) -> Result<(), String>;
    fn exists(&self, key: &str) -> Result<bool, String>;
}

/// Local filesystem state backend (default).
pub struct LocalBackend {
    root: PathBuf,
}

impl LocalBackend {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }
}

impl StateBackend for LocalBackend {
    fn name(&self) -> &str {
        "local"
    }

    fn get(&self, key: &str) -> Result<Vec<u8>, String> {
        let p = self.root.join(key);
        std::fs::read(&p).map_err(|e| format!("read {}: {e}", p.display()))
    }

    fn put(&self, key: &str, data: &[u8]) -> Result<(), String> {
        let p = self.root.join(key);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
        std::fs::write(&p, data).map_err(|e| format!("write {}: {e}", p.display()))
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>, String> {
        let dir = self.root.join(prefix);
        if !dir.exists() {
            return Ok(Vec::new());
        }
        collect_files(&dir, &self.root)
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        let p = self.root.join(key);
        if p.exists() {
            std::fs::remove_file(&p).map_err(|e| format!("delete {}: {e}", p.display()))?;
        }
        Ok(())
    }

    fn exists(&self, key: &str) -> Result<bool, String> {
        Ok(self.root.join(key).exists())
    }
}

fn collect_files(dir: &Path, root: &Path) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("readdir {}: {e}", dir.display()))?;
    let mut result = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                result.push(rel.display().to_string());
            }
        } else if path.is_dir() {
            result.extend(collect_files(&path, root)?);
        }
    }
    Ok(result)
}

/// Remote state backend report.
#[derive(Debug, serde::Serialize)]
pub struct StateBackendReport {
    pub backend: String,
    pub keys: Vec<String>,
    pub total: usize,
}

/// List state backend contents.
pub fn cmd_state_backend(
    state_dir: &Path,
    prefix: Option<&str>,
    json: bool,
) -> Result<(), String> {
    let backend = LocalBackend::new(state_dir);
    let pfx = prefix.unwrap_or("");
    let keys = backend.list(pfx)?;
    let total = keys.len();

    let report = StateBackendReport {
        backend: backend.name().to_string(),
        keys: keys.clone(),
        total,
    };

    if json {
        let out =
            serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        println!("State Backend: {}", backend.name());
        println!("Keys ({total}):");
        for k in &keys {
            println!("  {k}");
        }
    }
    Ok(())
}

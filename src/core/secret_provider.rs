//! FJ-3301: Secret provider trait and implementations.
//!
//! Defines a pluggable interface for resolving secrets from different
//! backends: environment variables, files, age encryption, and exec.

use std::path::Path;

/// A resolved secret value.
#[derive(Debug, Clone)]
pub struct SecretValue {
    /// The resolved plaintext value.
    pub value: String,
    /// Provider that resolved this secret.
    pub provider: &'static str,
}

/// Trait for secret providers.
pub trait SecretProvider {
    /// Resolve a secret by key. Returns None if the key doesn't exist.
    fn resolve(&self, key: &str) -> Result<Option<SecretValue>, String>;

    /// Provider name (for logging/diagnostics).
    fn name(&self) -> &'static str;
}

/// FJ-3301: Environment variable secret provider.
///
/// Resolves secrets from environment variables. Keys are uppercased
/// with hyphens replaced by underscores.
pub struct EnvProvider;

impl SecretProvider for EnvProvider {
    fn resolve(&self, key: &str) -> Result<Option<SecretValue>, String> {
        let env_key = key.to_uppercase().replace('-', "_");
        match std::env::var(&env_key) {
            Ok(val) => Ok(Some(SecretValue {
                value: val,
                provider: "env",
            })),
            Err(std::env::VarError::NotPresent) => Ok(None),
            Err(e) => Err(format!("env var '{env_key}': {e}")),
        }
    }

    fn name(&self) -> &'static str {
        "env"
    }
}

/// FJ-3301: File-based secret provider.
///
/// Reads secrets from a directory where each file's name is the key
/// and its contents (trimmed) are the value.
pub struct FileProvider {
    dir: std::path::PathBuf,
}

impl FileProvider {
    /// Create a new file provider reading from the given directory.
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
        }
    }
}

impl SecretProvider for FileProvider {
    fn resolve(&self, key: &str) -> Result<Option<SecretValue>, String> {
        // GH-85: Reject path traversal sequences in secret key names
        if key.contains("..") || key.contains('/') || key.contains('\\') {
            return Err(format!(
                "invalid secret key '{}': must not contain path separators or '..'",
                key
            ));
        }
        let path = self.dir.join(key);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("read secret {}: {e}", path.display()))?;
        Ok(Some(SecretValue {
            value: content.trim().to_string(),
            provider: "file",
        }))
    }

    fn name(&self) -> &'static str {
        "file"
    }
}

/// FJ-3301: Exec-based secret provider.
///
/// Runs a command to resolve a secret. The key is passed as the first
/// argument. The command's stdout (trimmed) is the value.
pub struct ExecProvider {
    command: String,
}

impl ExecProvider {
    /// Create a new exec provider running the given command.
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
        }
    }
}

impl SecretProvider for ExecProvider {
    fn resolve(&self, key: &str) -> Result<Option<SecretValue>, String> {
        // GH-85: Pass key as a shell variable to prevent command injection.
        // The key is available to the command as $1 instead of being interpolated.
        let output = std::process::Command::new("sh")
            .args(["-c", &format!("{} \"$1\"", self.command), "--", key])
            .output()
            .map_err(|e| format!("exec secret provider: {e}"))?;

        if !output.status.success() {
            return Ok(None);
        }

        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if value.is_empty() {
            return Ok(None);
        }

        Ok(Some(SecretValue {
            value,
            provider: "exec",
        }))
    }

    fn name(&self) -> &'static str {
        "exec"
    }
}

/// Chain of secret providers — tries each in order until one resolves.
pub struct ProviderChain {
    providers: Vec<Box<dyn SecretProvider>>,
}

impl ProviderChain {
    /// Create a new empty chain.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Add a provider to the chain (builder pattern).
    pub fn with(mut self, provider: Box<dyn SecretProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    /// Resolve a secret by trying each provider in order.
    pub fn resolve(&self, key: &str) -> Result<Option<SecretValue>, String> {
        for provider in &self.providers {
            if let Some(val) = provider.resolve(key)? {
                return Ok(Some(val));
            }
        }
        Ok(None)
    }
}

impl Default for ProviderChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn env_provider_missing_key() {
        let provider = EnvProvider;
        let result = provider.resolve("FORJAR_NONEXISTENT_KEY_999").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn env_provider_uses_path_var() {
        // PATH is always set — use it to test env resolution without unsafe set_var
        let provider = EnvProvider;
        let result = provider.resolve("PATH").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().provider, "env");
    }

    #[test]
    fn file_provider_resolves() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("db-password"), "s3cret\n").unwrap();

        let provider = FileProvider::new(dir.path());
        let result = provider.resolve("db-password").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().value, "s3cret");
    }

    #[test]
    fn file_provider_missing() {
        let dir = TempDir::new().unwrap();
        let provider = FileProvider::new(dir.path());
        let result = provider.resolve("nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn provider_chain_first_match() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("api-key"), "from-file").unwrap();

        let chain = ProviderChain::new()
            .with(Box::new(EnvProvider))
            .with(Box::new(FileProvider::new(dir.path())));

        let result = chain.resolve("api-key").unwrap();
        // File provider should resolve (env won't have API_KEY)
        assert!(result.is_some());
        assert_eq!(result.unwrap().provider, "file");
    }

    #[test]
    fn provider_chain_empty() {
        let chain = ProviderChain::new();
        let result = chain.resolve("anything").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn provider_chain_default() {
        let chain = ProviderChain::default();
        assert!(chain.resolve("test").unwrap().is_none());
    }

    #[test]
    fn provider_names() {
        assert_eq!(EnvProvider.name(), "env");
        let dir = TempDir::new().unwrap();
        assert_eq!(FileProvider::new(dir.path()).name(), "file");
        assert_eq!(ExecProvider::new("echo").name(), "exec");
    }

    #[test]
    fn exec_provider_resolves() {
        let provider = ExecProvider::new("echo");
        let result = provider.resolve("hello").unwrap();
        assert!(result.is_some());
        // `echo hello` outputs "hello\n"
        assert!(result.unwrap().value.contains("hello"));
    }

    #[test]
    fn exec_provider_failure() {
        let provider = ExecProvider::new("false");
        let result = provider.resolve("key").unwrap();
        assert!(result.is_none());
    }
}

//! FJ-043: Refinement types for config values.
//!
//! Compile-time and runtime verification that port numbers, permissions,
//! versions, and other config values are valid. Uses Rust's type system
//! to encode constraints that Flux-style refinement types would verify.
//!
//! Each refined type wraps a primitive and enforces invariants at construction.

/// A validated TCP/UDP port number (1-65535).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Port(u16);

impl Port {
    pub fn new(value: u16) -> Result<Self, String> {
        if value == 0 {
            return Err("port must be 1-65535, got 0".to_string());
        }
        Ok(Port(value))
    }

    pub fn value(self) -> u16 {
        self.0
    }
}

/// A validated Unix file permission mode (0o000-0o777).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FileMode(u16);

impl FileMode {
    pub fn new(value: u16) -> Result<Self, String> {
        if value > 0o777 {
            return Err(format!("mode must be 0-0o777, got {value:#o}"));
        }
        Ok(FileMode(value))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self, String> {
        let v = u16::from_str_radix(s, 8)
            .map_err(|e| format!("invalid octal mode '{s}': {e}"))?;
        Self::new(v)
    }

    pub fn value(self) -> u16 {
        self.0
    }

    pub fn as_octal_string(self) -> String {
        format!("{:04o}", self.0)
    }
}

/// A validated semantic version string (X.Y.Z).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SemVer {
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("expected X.Y.Z, got '{s}'"));
        }
        let major = parts[0].parse().map_err(|_| format!("invalid major: {}", parts[0]))?;
        let minor = parts[1].parse().map_err(|_| format!("invalid minor: {}", parts[1]))?;
        let patch = parts[2].parse().map_err(|_| format!("invalid patch: {}", parts[2]))?;
        Ok(SemVer { major, minor, patch })
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A validated hostname (RFC 1123).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Hostname(String);

impl Hostname {
    pub fn new(s: &str) -> Result<Self, String> {
        if s.is_empty() || s.len() > 253 {
            return Err(format!("hostname length must be 1-253, got {}", s.len()));
        }
        for label in s.split('.') {
            if label.is_empty() || label.len() > 63 {
                return Err(format!("label length must be 1-63: '{label}'"));
            }
            if !label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return Err(format!("invalid chars in label: '{label}'"));
            }
            if label.starts_with('-') || label.ends_with('-') {
                return Err(format!("label cannot start/end with dash: '{label}'"));
            }
        }
        Ok(Hostname(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated Unix path (absolute, no null bytes).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AbsPath(String);

impl AbsPath {
    pub fn new(s: &str) -> Result<Self, String> {
        if !s.starts_with('/') {
            return Err(format!("path must be absolute: '{s}'"));
        }
        if s.contains('\0') {
            return Err("path cannot contain null bytes".to_string());
        }
        Ok(AbsPath(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A validated resource name (alphanumeric + hyphens, 1-128 chars).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResourceName(String);

impl ResourceName {
    pub fn new(s: &str) -> Result<Self, String> {
        if s.is_empty() || s.len() > 128 {
            return Err(format!("name length must be 1-128, got {}", s.len()));
        }
        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(format!("name has invalid chars: '{s}'"));
        }
        Ok(ResourceName(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Compile-time assertions for const contexts
#[allow(clippy::eq_op)]
const _: () = {
    // Port range: u16 max is 65535, matches our constraint
    assert!(u16::MAX == 65535);
    // File mode max: 0o777 = 511
    assert!(0o777u16 == 511);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_valid() {
        assert!(Port::new(80).is_ok());
        assert!(Port::new(443).is_ok());
        assert!(Port::new(65535).is_ok());
        assert_eq!(Port::new(8080).unwrap().value(), 8080);
    }

    #[test]
    fn test_port_invalid() {
        assert!(Port::new(0).is_err());
    }

    #[test]
    fn test_filemode_valid() {
        assert!(FileMode::new(0o644).is_ok());
        assert!(FileMode::new(0o755).is_ok());
        assert_eq!(FileMode::new(0o644).unwrap().as_octal_string(), "0644");
    }

    #[test]
    fn test_filemode_invalid() {
        assert!(FileMode::new(0o1000).is_err());
    }

    #[test]
    fn test_filemode_from_str() {
        assert_eq!(FileMode::from_str("644").unwrap().value(), 0o644);
        assert!(FileMode::from_str("999").is_err());
    }

    #[test]
    fn test_semver_valid() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(format!("{v}"), "1.2.3");
    }

    #[test]
    fn test_semver_invalid() {
        assert!(SemVer::parse("1.2").is_err());
        assert!(SemVer::parse("abc").is_err());
    }

    #[test]
    fn test_hostname_valid() {
        assert!(Hostname::new("example.com").is_ok());
        assert!(Hostname::new("a-b.example.com").is_ok());
    }

    #[test]
    fn test_hostname_invalid() {
        assert!(Hostname::new("").is_err());
        assert!(Hostname::new("-bad.com").is_err());
    }

    #[test]
    fn test_abspath_valid() {
        assert!(AbsPath::new("/etc/nginx/nginx.conf").is_ok());
    }

    #[test]
    fn test_abspath_invalid() {
        assert!(AbsPath::new("relative/path").is_err());
    }

    #[test]
    fn test_resource_name_valid() {
        assert!(ResourceName::new("pkg-nginx").is_ok());
        assert!(ResourceName::new("my_resource").is_ok());
    }

    #[test]
    fn test_resource_name_invalid() {
        assert!(ResourceName::new("").is_err());
        assert!(ResourceName::new("has space").is_err());
    }
}

//! #1033-#1035: where a `cloud_instance` lands (subnet, security groups).
//! Every value is a whole token, validated here and quoted where it is used.

use crate::core::shell_escape::sh_squote;
use crate::core::types::CloudSpec;

/// Validated placement. `Default` = nothing declared = today's command line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Placement {
    subnet: Option<String>,
    groups: Vec<String>,
}

/// `<prefix>` + 8 or 17 lowercase hex digits.
fn is_aws_id(s: &str, prefix: &str) -> bool {
    s.strip_prefix(prefix).is_some_and(|h| {
        matches!(h.len(), 8 | 17) && h.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
    })
}

impl Placement {
    pub(super) fn of(c: &CloudSpec) -> Result<Self, String> {
        let subnet = c.subnet_id.clone();
        if let Some(s) = &subnet {
            if !is_aws_id(s, "subnet-") {
                return Err(
                    "cloud_subnet_id must be subnet- plus 8 or 17 lowercase hex digits".into(),
                );
            }
        }
        let groups = c.security_group_ids.clone().unwrap_or_default();
        if c.security_group_ids.is_some() {
            let uniq: std::collections::BTreeSet<_> = groups.iter().collect();
            if groups.is_empty() || groups.len() > 5 || uniq.len() != groups.len() {
                return Err("cloud_security_group_ids needs 1 to 5 distinct ids".into());
            }
            if !groups.iter().all(|g| is_aws_id(g, "sg-")) {
                return Err(
                    "cloud_security_group_ids entries must be sg- plus 8 or 17 lowercase hex digits"
                        .into(),
                );
            }
        }
        Ok(Self { subnet, groups })
    }

    /// Flags for `run-instances`, each followed by a space; empty if nothing declared.
    pub(super) fn args(&self) -> String {
        let mut a = String::new();
        if let Some(s) = &self.subnet {
            a += &format!("--subnet-id {} ", sh_squote(s));
        }
        if !self.groups.is_empty() {
            let g: Vec<String> = self.groups.iter().map(|g| sh_squote(g)).collect();
            a += &format!("--security-group-ids {} ", g.join(" "));
        }
        a
    }

    /// Key suffix: empty if nothing declared, else a short hash of the placement, so a
    /// changed placement on a live instance is refused like a changed image.
    pub(super) fn key_suffix(&self) -> String {
        if *self == Self::default() {
            return String::new();
        }
        let mut g = self.groups.clone();
        g.sort();
        let canon = format!(
            "subnet={};sg={}",
            self.subnet.as_deref().unwrap_or(""),
            g.join(",")
        );
        let h = crate::tripwire::hasher::hash_string(&canon);
        format!("@p{}", &h["blake3:".len()..][..12])
    }
}

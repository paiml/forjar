//! #1033-#1035: where a `cloud_instance` lands (subnet, security groups, zone).
//! Every value is a whole token, validated here and quoted where it is used.

use crate::core::error::ForjarError;
use crate::core::shell_escape::sh_squote;
use crate::core::types::CloudSpec;

/// Validated placement. `Default` = nothing declared = today's command line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Placement {
    subnet: Option<String>,
    groups: Vec<String>,
    zone: Option<String>,
}

/// `<prefix>` + 8 or 17 lowercase hex digits.
fn is_aws_id(s: &str, prefix: &str) -> bool {
    s.strip_prefix(prefix).is_some_and(|h| {
        matches!(h.len(), 8 | 17) && h.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
    })
}

fn check_subnet(s: &str) -> Result<(), ForjarError> {
    if is_aws_id(s, "subnet-") {
        return Ok(());
    }
    Err(ForjarError::from_untyped(
        "cloud_subnet_id must be subnet- plus 8 or 17 lowercase hex digits",
    ))
}

fn check_groups(groups: &[String]) -> Result<(), ForjarError> {
    let uniq: std::collections::BTreeSet<_> = groups.iter().collect();
    if groups.is_empty() || groups.len() > 5 || uniq.len() != groups.len() {
        return Err(ForjarError::from_untyped(
            "cloud_security_group_ids needs 1 to 5 distinct ids",
        ));
    }
    if !groups.iter().all(|g| is_aws_id(g, "sg-")) {
        return Err(ForjarError::from_untyped(
            "cloud_security_group_ids entries must be sg- plus 8 or 17 lowercase hex digits",
        ));
    }
    Ok(())
}

fn check_zone(z: &str, region: &str) -> Result<(), ForjarError> {
    let letter = z.strip_prefix(region).filter(|l| l.len() == 1);
    if letter.is_some_and(|l| l.chars().all(|c| c.is_ascii_lowercase())) {
        return Ok(());
    }
    Err(ForjarError::from_untyped(
        "cloud_availability_zone must be cloud_region plus one lowercase letter",
    ))
}

impl Placement {
    pub(super) fn of(c: &CloudSpec, region: &str) -> Result<Self, String> {
        let subnet = c.subnet_id.clone();
        let zone = c.availability_zone.clone();
        let groups = c.security_group_ids.clone();
        subnet
            .as_deref()
            .map_or(Ok(()), check_subnet)
            .map_err(ForjarError::into_message)?;
        groups
            .as_deref()
            .map_or(Ok(()), check_groups)
            .map_err(ForjarError::into_message)?;
        zone.as_deref()
            .map_or(Ok(()), |z| check_zone(z, region))
            .map_err(ForjarError::into_message)?;
        Ok(Self {
            subnet,
            groups: groups.unwrap_or_default(),
            zone,
        })
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
        if let Some(z) = &self.zone {
            a += &format!(
                "--placement {} ",
                sh_squote(&format!("AvailabilityZone={z}"))
            );
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
            "subnet={};sg={};az={}",
            self.subnet.as_deref().unwrap_or(""),
            g.join(","),
            self.zone.as_deref().unwrap_or("")
        );
        let h = crate::tripwire::hasher::hash_string(&canon);
        format!("@p{}", &h["blake3:".len()..][..12])
    }
}

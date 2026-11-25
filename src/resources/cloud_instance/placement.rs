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
    zones: Vec<String>,
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

fn resolve_zones(c: &CloudSpec, region: &str) -> Result<Vec<String>, ForjarError> {
    match (&c.availability_zone, &c.availability_zones) {
        (Some(_), Some(_)) => Err(ForjarError::from_untyped(
            "cannot set both cloud_availability_zone and cloud_availability_zones",
        )),
        (None, None) => Ok(Vec::new()),
        (Some(z), None) => {
            check_zone(z, region)?;
            Ok(vec![z.clone()])
        }
        (None, Some(zs)) => {
            if zs.is_empty() {
                return Err(ForjarError::from_untyped(
                    "cloud_availability_zones must not be empty",
                ));
            }
            let uniq: std::collections::BTreeSet<&str> = zs.iter().map(String::as_str).collect();
            if uniq.len() != zs.len() {
                return Err(ForjarError::from_untyped(
                    "cloud_availability_zones entries must be distinct",
                ));
            }
            for z in zs {
                check_zone(z, region)?;
            }
            Ok(zs.clone())
        }
    }
}

impl Placement {
    pub(super) fn of(c: &CloudSpec, region: &str) -> Result<Self, String> {
        let subnet = c.subnet_id.clone();
        let groups = c.security_group_ids.clone();
        subnet
            .as_deref()
            .map_or(Ok(()), check_subnet)
            .map_err(ForjarError::into_message)?;
        groups
            .as_deref()
            .map_or(Ok(()), check_groups)
            .map_err(ForjarError::into_message)?;
        let zones = resolve_zones(c, region).map_err(ForjarError::into_message)?;
        if subnet.is_some() && zones.len() > 1 {
            return Err("cloud_subnet_id is one AZ; it cannot combine with multiple cloud_availability_zones".into());
        }
        Ok(Self {
            subnet,
            groups: groups.unwrap_or_default(),
            zones,
        })
    }

    pub(super) fn zones(&self) -> &[String] {
        &self.zones
    }

    /// Network flags (`--subnet-id`, `--security-group-ids`) without `--placement`, followed by a space.
    pub(super) fn net_args(&self) -> String {
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

    /// `--placement` flag for a single zone, followed by a space; empty when `None`.
    pub(super) fn args_for_zone(zone: Option<&str>) -> String {
        zone.map_or_else(String::new, |z| {
            format!(
                "--placement {} ",
                sh_squote(&format!("AvailabilityZone={z}"))
            )
        })
    }

    /// Flags for `run-instances`, each followed by a space; empty if nothing declared.
    pub(super) fn args(&self) -> String {
        let mut a = self.net_args();
        a += &Self::args_for_zone(self.zones.first().map(String::as_str));
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
            self.zones.join(",")
        );
        let h = crate::tripwire::hasher::hash_string(&canon);
        format!("@p{}", &h["blake3:".len()..][..12])
    }
}

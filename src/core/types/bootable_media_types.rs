//! Refs #713: `bootable_media` declaration fields and the validated form.
//!
//! An install stick is the Ubuntu Server ISO written raw to a USB disk, plus a
//! NoCloud seed (`user-data`, `meta-data`, optional extras) on a FAT partition
//! appended after the ISO's own partitions. It was written by hand-rolled shell
//! with no plan before the write and no read-back after it. This type makes it
//! declared state.
//!
//! Every refusal in [`BootableMedia::new`] is a property of the DECLARATION,
//! so it fires at `forjar validate` / `plan` time, before any byte is written.
//! The properties of the DISK (removable/USB, nothing mounted) can only be
//! asked of the box, and are asserted by the generated scripts.

use indexmap::IndexMap;

/// The only directory a `media_device` may name.
pub const MEDIA_BY_ID_PREFIX: &str = "/dev/disk/by-id/";

/// Default size of the seed partition, MiB.
pub const DEFAULT_SEED_MIB: u32 = 16;

/// Bounds on the seed partition: FAT16 with 2 KiB clusters needs at least
/// 4085 clusters (~8 MiB); 64 MiB keeps the in-memory image small.
pub const MIN_SEED_MIB: u32 = 9;
/// See [`MIN_SEED_MIB`].
pub const MAX_SEED_MIB: u32 = 64;

/// `bootable_media` YAML fields. Flattened into `Resource`, so they stay top
/// level (`media_device:`), while the fields live with the type that reads them.
///
/// Every field skips serialising when unset. `hash_desired_state` serialises the
/// whole `Resource`, so a field that serialised as `~` on every resource would
/// move every recorded lock hash on every machine (a fleet-wide re-converge,
/// `test_golden_hash_pinned_value`) for a type almost no config declares.
/// A resource that sets them hashes them like any other field.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MediaSpec {
    /// Whole-disk `/dev/disk/by-id/usb-*` path. Nothing else is accepted.
    #[serde(
        rename = "media_device",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub device: Option<String>,

    /// Absolute path of the ISO on the target machine.
    #[serde(rename = "media_iso", default, skip_serializing_if = "Option::is_none")]
    pub iso: Option<String>,

    /// Pinned sha256 of the whole ISO file (64 lowercase hex).
    #[serde(
        rename = "media_iso_sha256",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub iso_sha256: Option<String>,

    /// Pinned size of the ISO in bytes. With it, `plan` can name the exact
    /// sectors it will write without reading the target.
    #[serde(
        rename = "media_iso_bytes",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub iso_bytes: Option<u64>,

    /// Seed files: name -> content (rendered through params/templates).
    /// Must contain `user-data` and `meta-data`.
    #[serde(
        rename = "media_seed",
        default,
        skip_serializing_if = "IndexMap::is_empty"
    )]
    pub seed: IndexMap<String, String>,

    /// `armed` (label `CIDATA`: the installer consumes it) or `disarmed`
    /// (label `SCRIPTS`: the stick boots the plain ISO). Default `armed`.
    #[serde(
        rename = "media_seed_state",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub seed_state: Option<String>,

    /// Seed partition size in MiB (default 16).
    #[serde(
        rename = "media_seed_mib",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub seed_mib: Option<u32>,
}

/// Whether the installer will consume the seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedState {
    /// Label `CIDATA`.
    Armed,
    /// Label `SCRIPTS`.
    Disarmed,
}

impl SeedState {
    /// The FAT volume label this state writes.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Armed => "CIDATA",
            Self::Disarmed => "SCRIPTS",
        }
    }
}

/// A `bootable_media` declaration that passed every declaration-level refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootableMedia {
    /// `/dev/disk/by-id/usb-…`, whole disk.
    pub device: String,
    /// ISO path on the target.
    pub iso: String,
    /// Pinned sha256 (lowercase hex).
    pub iso_sha256: String,
    /// Pinned ISO size in bytes.
    pub iso_bytes: u64,
    /// Seed files, in declaration order.
    pub seed: Vec<(String, String)>,
    /// Armed or disarmed.
    pub state: SeedState,
    /// Seed partition size in MiB.
    pub seed_mib: u32,
}

/// Is `p` a whole-disk USB by-id path?
///
/// Refuses `/dev/sdX`, `/dev/nvme…`, every other `/dev/disk/by-*` family, a
/// by-id name that is not `usb-`, a `-partN` partition link, and any path with
/// a further `/` or a `..` in it.
///
/// # Errors
///
/// Returns the reason the path is refused.
pub fn check_by_id_path(p: &str) -> Result<(), String> {
    let Some(name) = p.strip_prefix(MEDIA_BY_ID_PREFIX) else {
        return Err(format!(
            "media_device '{p}' is not a {MEDIA_BY_ID_PREFIX} path: a /dev/sdX name moves between boots \
             and can name the system disk, so only a stable by-id link is accepted"
        ));
    };
    if name.is_empty() || name.contains('/') || name.contains("..") {
        return Err(format!("media_device '{p}' is not a single by-id entry"));
    }
    if !name.starts_with("usb-") {
        return Err(format!(
            "media_device '{p}' is not a usb- by-id link: only USB media can be declared"
        ));
    }
    if is_partition_link(name) {
        return Err(format!(
            "media_device '{p}' names a partition: declare the whole disk"
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.' | '+'))
    {
        return Err(format!(
            "media_device '{p}' has characters a by-id link never carries"
        ));
    }
    Ok(())
}

fn is_partition_link(name: &str) -> bool {
    match name.rfind("-part") {
        Some(i) => {
            let tail = &name[i + 5..];
            !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

/// Is `name` a seed file name the FAT writer and the installer both accept?
fn check_seed_name(name: &str) -> Result<(), String> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if ok {
        Ok(())
    } else {
        Err(format!(
            "media_seed name '{name}' must be 1-64 chars of [A-Za-z0-9._-], not starting with '.'"
        ))
    }
}

fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}

/// The ISO path, its pinned sha256 and its pinned size.
fn iso_pin(spec: &MediaSpec) -> Result<(String, String, u64), String> {
    let iso = spec
        .iso
        .clone()
        .ok_or_else(|| "bootable_media requires media_iso".to_string())?;
    if !iso.starts_with('/') || iso.contains("..") {
        return Err(format!("media_iso '{iso}' must be an absolute path"));
    }
    let iso_sha256 = spec.iso_sha256.clone().ok_or_else(|| {
        "bootable_media requires media_iso_sha256: an unpinned ISO is refused".to_string()
    })?;
    if !is_sha256_hex(&iso_sha256) {
        return Err("media_iso_sha256 must be 64 lowercase hex characters".to_string());
    }
    let iso_bytes = spec
        .iso_bytes
        .ok_or_else(|| "bootable_media requires media_iso_bytes".to_string())?;
    if iso_bytes < 64 * 1024 || !iso_bytes.is_multiple_of(512) {
        return Err(format!(
            "media_iso_bytes {iso_bytes} is not an ISO size (at least 64 KiB, whole 512-byte sectors)"
        ));
    }
    Ok((iso, iso_sha256, iso_bytes))
}

/// Seed files, name to content, in declaration order.
type SeedFiles = Vec<(String, String)>;

/// The seed files, its armed/disarmed state and its partition size.
fn seed_of(spec: &MediaSpec) -> Result<(SeedFiles, SeedState, u32), String> {
    for required in ["user-data", "meta-data"] {
        if !spec.seed.contains_key(required) {
            return Err(format!("media_seed must contain '{required}'"));
        }
    }
    for name in spec.seed.keys() {
        check_seed_name(name)?;
    }
    let state = match spec.seed_state.as_deref().unwrap_or("armed") {
        "armed" => SeedState::Armed,
        "disarmed" => SeedState::Disarmed,
        other => {
            return Err(format!(
                "media_seed_state '{other}' must be 'armed' or 'disarmed'"
            ))
        }
    };
    let seed_mib = spec.seed_mib.unwrap_or(DEFAULT_SEED_MIB);
    if !(MIN_SEED_MIB..=MAX_SEED_MIB).contains(&seed_mib) {
        return Err(format!(
            "media_seed_mib {seed_mib} must be within {MIN_SEED_MIB}..={MAX_SEED_MIB}"
        ));
    }
    let seed: SeedFiles = spec
        .seed
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let total: u64 = seed.iter().map(|(_, v)| v.len() as u64).sum();
    if total > u64::from(seed_mib) * 1024 * 1024 / 2 {
        return Err(format!(
            "media_seed is {total} bytes: more than half of a {seed_mib} MiB partition"
        ));
    }
    Ok((seed, state, seed_mib))
}

impl BootableMedia {
    /// Validate a declaration.
    ///
    /// # Errors
    ///
    /// Returns the first refusal: a non-by-id device, a missing or relative
    /// ISO, an unpinned or malformed sha256, a missing size, a seed without
    /// `user-data`/`meta-data`, a bad seed name, an unknown seed state, or a
    /// seed that does not fit its partition.
    pub fn new(spec: &MediaSpec) -> Result<Self, String> {
        let device = spec
            .device
            .clone()
            .ok_or_else(|| "bootable_media requires media_device".to_string())?;
        check_by_id_path(&device)?;
        let (iso, iso_sha256, iso_bytes) = iso_pin(spec)?;
        let (seed, state, seed_mib) = seed_of(spec)?;
        Ok(Self {
            device,
            iso,
            iso_sha256,
            iso_bytes,
            seed,
            state,
            seed_mib,
        })
    }

    /// First sector of the seed partition: the ISO end rounded up to 1 MiB.
    pub const fn seed_start_sector(&self) -> u64 {
        let mib = 1024 * 1024;
        self.iso_bytes.div_ceil(mib) * mib / 512
    }

    /// Size of the seed partition in 512-byte sectors.
    pub const fn seed_sectors(&self) -> u64 {
        self.seed_mib as u64 * 2048
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEV: &str = "/dev/disk/by-id/usb-SSK_Port_able_SSD_256_ABCDEFA88120-0:0";

    fn spec() -> MediaSpec {
        let mut seed = IndexMap::new();
        seed.insert("user-data".to_string(), "#cloud-config\n".to_string());
        seed.insert("meta-data".to_string(), "instance-id: t\n".to_string());
        MediaSpec {
            device: Some(DEV.to_string()),
            iso: Some("/srv/iso/u.iso".to_string()),
            iso_sha256: Some("a".repeat(64)),
            iso_bytes: Some(3 * 1024 * 1024 * 1024 + 7 * 2048),
            seed,
            seed_state: None,
            seed_mib: None,
        }
    }

    #[test]
    fn a_usb_by_id_whole_disk_is_accepted() {
        assert!(check_by_id_path(DEV).is_ok());
        assert!(BootableMedia::new(&spec()).is_ok());
    }

    #[test]
    fn dev_sdx_is_refused() {
        for p in [
            "/dev/sdb",
            "/dev/sda",
            "/dev/nvme0n1",
            "/dev/disk/by-path/pci-0:usb-0:1",
        ] {
            let e = check_by_id_path(p).expect_err(p);
            assert!(e.contains("by-id"), "{p}: {e}");
        }
    }

    #[test]
    fn a_partition_link_and_a_non_usb_by_id_are_refused() {
        assert!(check_by_id_path(&format!("{DEV}-part4")).is_err());
        assert!(check_by_id_path("/dev/disk/by-id/ata-Samsung_SSD_870").is_err());
        assert!(check_by_id_path("/dev/disk/by-id/nvme-eui.0025").is_err());
        assert!(check_by_id_path("/dev/disk/by-id/usb-../../sda").is_err());
    }

    #[test]
    fn an_unpinned_iso_is_refused() {
        let mut s = spec();
        s.iso_sha256 = None;
        assert!(BootableMedia::new(&s).unwrap_err().contains("unpinned"));
        s.iso_sha256 = Some("ABC".to_string());
        assert!(BootableMedia::new(&s).is_err());
    }

    #[test]
    fn a_seed_without_user_data_is_refused() {
        let mut s = spec();
        s.seed.shift_remove("user-data");
        assert!(BootableMedia::new(&s).unwrap_err().contains("user-data"));
    }

    #[test]
    fn seed_state_maps_to_its_label() {
        let mut s = spec();
        assert_eq!(BootableMedia::new(&s).unwrap().state.label(), "CIDATA");
        s.seed_state = Some("disarmed".to_string());
        assert_eq!(BootableMedia::new(&s).unwrap().state.label(), "SCRIPTS");
        s.seed_state = Some("maybe".to_string());
        assert!(BootableMedia::new(&s).is_err());
    }

    #[test]
    fn seed_starts_on_the_first_mib_boundary_after_the_iso() {
        let m = BootableMedia::new(&spec()).unwrap();
        assert_eq!(m.seed_start_sector() * 512 % (1024 * 1024), 0);
        assert!(m.seed_start_sector() * 512 >= m.iso_bytes);
        assert!(m.seed_start_sector() * 512 - m.iso_bytes < 1024 * 1024);
    }
}

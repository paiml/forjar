//! A deterministic FAT16 image writer for the NoCloud seed partition.
//!
//! Why in Rust and not `mkfs.vfat` + `mcopy` on the target: mtools is not on
//! the fleet (measured on lambda-labs 2026-10-14: `mcopy not found`), and a
//! filesystem built on the box carries a fresh volume id and timestamps, so its
//! bytes can never be predicted. An image built here is a pure function of the
//! declaration — label, file names, file bytes, size — which is what lets the
//! completion check compare the whole partition, and every file at its exact
//! on-disk extent, against bytes known before anything was written.
//!
//! Layout: 512-byte sectors, 4 sectors per cluster, one reserved sector, two
//! FATs, a 512-entry root directory. Every file lives in the root directory
//! with a VFAT long name, and is stored in contiguous clusters. Timestamps are
//! the FAT epoch (1980-01-01 00:00). The volume label is written both in the
//! boot sector and as the root directory's label entry, which is the one
//! `blkid` reports.

use sha2::{Digest, Sha256};

/// Bytes per sector.
pub const SECTOR: usize = 512;
const SECTORS_PER_CLUSTER: usize = 4;
const CLUSTER: usize = SECTOR * SECTORS_PER_CLUSTER;
const RESERVED: usize = 1;
const NUM_FATS: usize = 2;
const ROOT_ENTRIES: usize = 512;
const ROOT_SECTORS: usize = ROOT_ENTRIES * 32 / SECTOR;
/// FAT date for 1980-01-01.
const FAT_EPOCH_DATE: u16 = 0x0021;

/// Where one file's bytes live inside the partition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extent {
    /// File name as declared.
    pub name: String,
    /// Byte offset from the start of the partition.
    pub offset: u64,
    /// Length in bytes.
    pub len: u64,
}

/// A built FAT16 partition image.
#[derive(Debug, Clone)]
pub struct FatImage {
    /// The whole partition, `total_sectors * 512` bytes.
    pub bytes: Vec<u8>,
    /// One extent per file, in declaration order.
    pub extents: Vec<Extent>,
}

impl FatImage {
    /// sha256 of the whole partition image, lowercase hex.
    pub fn sha256(&self) -> String {
        hex_sha256(&self.bytes)
    }
}

/// Lowercase hex sha256.
pub fn hex_sha256(b: &[u8]) -> String {
    let d = Sha256::digest(b);
    d.iter().map(|x| format!("{x:02x}")).collect()
}

struct Geometry {
    fat_sectors: usize,
    data_start: usize,
    clusters: usize,
}

fn geometry(total_sectors: usize) -> Result<Geometry, String> {
    let mut fat_sectors = 1;
    loop {
        let data_start = RESERVED + NUM_FATS * fat_sectors + ROOT_SECTORS;
        let clusters = total_sectors.saturating_sub(data_start) / SECTORS_PER_CLUSTER;
        let need = ((clusters + 2) * 2).div_ceil(SECTOR);
        if need <= fat_sectors {
            // FAT16 is defined by its cluster count, not its name field.
            if !(4085..=65524).contains(&clusters) {
                return Err(format!(
                    "{total_sectors} sectors gives {clusters} clusters: not a FAT16 volume"
                ));
            }
            return Ok(Geometry {
                fat_sectors,
                data_start,
                clusters,
            });
        }
        fat_sectors = need;
    }
}

/// The 8.3 short name for the `n`th file, unique by construction (`~n`).
fn short_name(name: &str, n: usize) -> [u8; 11] {
    let (base, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i + 1..]),
        _ => (name, ""),
    };
    let clean = |s: &str| -> Vec<u8> {
        s.bytes()
            .filter(|b| *b != b'.')
            .map(|b| {
                let u = b.to_ascii_uppercase();
                if u.is_ascii_alphanumeric() || u == b'-' || u == b'_' {
                    u
                } else {
                    b'_'
                }
            })
            .collect()
    };
    let tail = format!("~{n}");
    let keep = 8 - tail.len();
    let mut b: Vec<u8> = clean(base).into_iter().take(keep).collect();
    if b.is_empty() {
        b.push(b'F');
    }
    b.extend_from_slice(tail.as_bytes());
    let mut out = [b' '; 11];
    out[..b.len()].copy_from_slice(&b);
    for (i, c) in clean(ext).into_iter().take(3).enumerate() {
        out[8 + i] = c;
    }
    out
}

fn lfn_checksum(short: &[u8; 11]) -> u8 {
    short.iter().fold(0u8, |s, &c| {
        (s >> 1).wrapping_add((s & 1) << 7).wrapping_add(c)
    })
}

/// VFAT long-name entries for `name`, in on-disk order (highest sequence first).
fn lfn_entries(name: &str, short: &[u8; 11]) -> Vec<[u8; 32]> {
    let mut units: Vec<u16> = name.encode_utf16().collect();
    if !units.len().is_multiple_of(13) {
        units.push(0);
        while !units.len().is_multiple_of(13) {
            units.push(0xFFFF);
        }
    }
    let count = units.len() / 13;
    let sum = lfn_checksum(short);
    let mut out = Vec::with_capacity(count);
    for seq in (1..=count).rev() {
        let chunk = &units[(seq - 1) * 13..seq * 13];
        let mut e = [0u8; 32];
        e[0] = seq as u8 | if seq == count { 0x40 } else { 0 };
        e[11] = 0x0F;
        e[13] = sum;
        let slots = (1..11)
            .step_by(2)
            .chain((14..26).step_by(2))
            .chain((28..32).step_by(2));
        for (u, off) in chunk.iter().zip(slots) {
            e[off..off + 2].copy_from_slice(&u.to_le_bytes());
        }
        out.push(e);
    }
    out
}

fn short_entry(short: &[u8; 11], attr: u8, cluster: u16, size: u32) -> [u8; 32] {
    let mut e = [0u8; 32];
    e[..11].copy_from_slice(short);
    e[11] = attr;
    if attr != 0x08 {
        e[16..18].copy_from_slice(&FAT_EPOCH_DATE.to_le_bytes());
        e[18..20].copy_from_slice(&FAT_EPOCH_DATE.to_le_bytes());
        e[24..26].copy_from_slice(&FAT_EPOCH_DATE.to_le_bytes());
    }
    e[26..28].copy_from_slice(&cluster.to_le_bytes());
    e[28..32].copy_from_slice(&size.to_le_bytes());
    e
}

fn label11(label: &str) -> [u8; 11] {
    let mut l = [b' '; 11];
    for (i, b) in label.bytes().take(11).enumerate() {
        l[i] = b.to_ascii_uppercase();
    }
    l
}

fn boot_sector(img: &mut [u8], g: &Geometry, total: usize, label: &[u8; 11], vol_id: u32) {
    let b = &mut img[..SECTOR];
    b[0..3].copy_from_slice(&[0xEB, 0x3C, 0x90]);
    b[3..11].copy_from_slice(b"FORJAR  ");
    b[11..13].copy_from_slice(&(SECTOR as u16).to_le_bytes());
    b[13] = SECTORS_PER_CLUSTER as u8;
    b[14..16].copy_from_slice(&(RESERVED as u16).to_le_bytes());
    b[16] = NUM_FATS as u8;
    b[17..19].copy_from_slice(&(ROOT_ENTRIES as u16).to_le_bytes());
    if total < 65536 {
        b[19..21].copy_from_slice(&(total as u16).to_le_bytes());
    } else {
        b[32..36].copy_from_slice(&(total as u32).to_le_bytes());
    }
    b[21] = 0xF8;
    b[22..24].copy_from_slice(&(g.fat_sectors as u16).to_le_bytes());
    b[24..26].copy_from_slice(&32u16.to_le_bytes());
    b[26..28].copy_from_slice(&64u16.to_le_bytes());
    b[36] = 0x80;
    b[38] = 0x29;
    b[39..43].copy_from_slice(&vol_id.to_le_bytes());
    b[43..54].copy_from_slice(label);
    b[54..62].copy_from_slice(b"FAT16   ");
    b[510] = 0x55;
    b[511] = 0xAA;
}

/// Build the partition image.
///
/// # Errors
///
/// Returns `Err` when the size is not a FAT16 volume, the files do not fit, or
/// the root directory overflows.
pub fn build(
    label: &str,
    files: &[(String, Vec<u8>)],
    total_sectors: usize,
) -> Result<FatImage, String> {
    let g = geometry(total_sectors)?;
    let mut img = vec![0u8; total_sectors * SECTOR];
    let l11 = label11(label);
    // The volume id is a function of the content, so the image is too.
    let mut h = Sha256::new();
    h.update(label.as_bytes());
    for (n, c) in files {
        h.update(n.as_bytes());
        h.update([0]);
        h.update(c);
        h.update([0]);
    }
    let d = h.finalize();
    let vol_id = u32::from_le_bytes([d[0], d[1], d[2], d[3]]);
    boot_sector(&mut img, &g, total_sectors, &l11, vol_id);

    let mut fat = vec![0u16; g.clusters + 2];
    fat[0] = 0xFFF8;
    fat[1] = 0xFFFF;
    let mut dir: Vec<[u8; 32]> = vec![short_entry(&l11, 0x08, 0, 0)];
    let mut extents = Vec::with_capacity(files.len());
    let mut next = 2usize;
    for (i, (name, content)) in files.iter().enumerate() {
        let short = short_name(name, i + 1);
        let n_clusters = content.len().div_ceil(CLUSTER);
        if next + n_clusters > g.clusters + 2 {
            return Err(format!("seed file '{name}' does not fit the partition"));
        }
        let first = if n_clusters == 0 { 0 } else { next };
        for k in 0..n_clusters {
            let c = next + k;
            fat[c] = if k + 1 == n_clusters {
                0xFFFF
            } else {
                (c + 1) as u16
            };
        }
        let offset = (g.data_start + (next - 2) * SECTORS_PER_CLUSTER) * SECTOR;
        if n_clusters > 0 {
            img[offset..offset + content.len()].copy_from_slice(content);
        }
        extents.push(Extent {
            name: name.clone(),
            offset: offset as u64,
            len: content.len() as u64,
        });
        next += n_clusters;
        dir.extend(lfn_entries(name, &short));
        dir.push(short_entry(
            &short,
            0x20,
            first as u16,
            content.len() as u32,
        ));
    }
    if dir.len() > ROOT_ENTRIES {
        return Err("seed has too many files for the root directory".to_string());
    }
    for copy in 0..NUM_FATS {
        let base = (RESERVED + copy * g.fat_sectors) * SECTOR;
        for (i, v) in fat.iter().enumerate() {
            img[base + 2 * i..base + 2 * i + 2].copy_from_slice(&v.to_le_bytes());
        }
    }
    let root = (RESERVED + NUM_FATS * g.fat_sectors) * SECTOR;
    for (i, e) in dir.iter().enumerate() {
        img[root + 32 * i..root + 32 * i + 32].copy_from_slice(e);
    }
    Ok(FatImage {
        bytes: img,
        extents,
    })
}

/// Runs of consecutive non-zero sectors: `(first_sector, sector_count)`.
///
/// The writer zeroes the whole partition first, then writes only these, so a
/// 16 MiB partition holding 14 KiB of seed costs a few KiB of script.
pub fn nonzero_runs(img: &[u8]) -> Vec<(usize, usize)> {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for (i, s) in img.chunks(SECTOR).enumerate() {
        if s.iter().all(|b| *b == 0) {
            continue;
        }
        match runs.last_mut() {
            Some((start, len)) if *start + *len == i => *len += 1,
            _ => runs.push((i, 1)),
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> Vec<(String, Vec<u8>)> {
        vec![
            (
                "user-data".into(),
                b"#cloud-config\nautoinstall:\n  version: 1\n".to_vec(),
            ),
            ("meta-data".into(), b"instance-id: t\n".to_vec()),
            ("install.sh".into(), vec![b'x'; 5000]),
            ("empty".into(), Vec::new()),
        ]
    }

    #[test]
    fn the_image_is_a_pure_function_of_the_declaration() {
        let a = build("CIDATA", &seed(), 32768).unwrap();
        let b = build("CIDATA", &seed(), 32768).unwrap();
        assert_eq!(a.sha256(), b.sha256());
        let c = build("SCRIPTS", &seed(), 32768).unwrap();
        assert_ne!(
            a.sha256(),
            c.sha256(),
            "the label state must change the bytes"
        );
    }

    #[test]
    fn every_file_sits_at_its_extent() {
        let img = build("CIDATA", &seed(), 32768).unwrap();
        for ((_, content), e) in seed().iter().zip(&img.extents) {
            let o = e.offset as usize;
            assert_eq!(
                &img.bytes[o..o + e.len as usize],
                content.as_slice(),
                "{}",
                e.name
            );
        }
    }

    #[test]
    fn the_boot_sector_carries_the_label_and_signature() {
        let img = build("CIDATA", &seed(), 32768).unwrap();
        assert_eq!(&img.bytes[43..54], b"CIDATA     ");
        assert_eq!(&img.bytes[54..62], b"FAT16   ");
        assert_eq!(&img.bytes[510..512], &[0x55, 0xAA]);
    }

    #[test]
    fn short_names_are_unique_and_lfn_checksums_match() {
        let a = short_name("user-data", 1);
        let b = short_name("user-data2", 2);
        assert_ne!(a, b);
        assert_eq!(&a, b"USER-D~1   ");
        assert_eq!(&short_name("install.sh", 3), b"INSTAL~3SH ");
        let l = lfn_entries("user-data", &a);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0][13], lfn_checksum(&a));
        assert_eq!(l[0][0], 0x41);
    }

    #[test]
    fn a_size_outside_fat16_is_refused() {
        assert!(build("CIDATA", &seed(), 4096).is_err());
    }

    #[test]
    fn nonzero_runs_cover_every_nonzero_byte() {
        let img = build("CIDATA", &seed(), 32768).unwrap();
        let mut rebuilt = vec![0u8; img.bytes.len()];
        for (s, n) in nonzero_runs(&img.bytes) {
            let r = s * SECTOR..(s + n) * SECTOR;
            rebuilt[r.clone()].copy_from_slice(&img.bytes[r]);
        }
        assert_eq!(rebuilt, img.bytes);
    }
}

//! `bootable_media`: an install stick as declared, linted, read-back-proven state.
//!
//! # Problem
//!
//! The stick that installs a fleet box was a hand ritual: `dd` an ISO, carve a
//! FAT partition, `mount`, `cp` a seed, `fatlabel` it CIDATA. The forjar config
//! that existed for it (`stick-forjar.yaml`) was a `task` that mounted the
//! CIDATA partition and `cmp`'d three files: it proved the seed was on *some*
//! FAT partition, said nothing about the ISO, and named the stick by a path
//! nothing checked was a USB disk. Its seed's `early-commands` and
//! `late-commands` are root shell that never met bashrs.
//!
//! # Model
//!
//! ```yaml
//! install-stick:
//!   type: bootable_media
//!   machine: localhost
//!   sudo: true
//!   media_device: /dev/disk/by-id/usb-SSK_Port_able_SSD_256_ABCDEFA88120-0:0
//!   media_iso: /srv/iso/ubuntu-26.04.1-live-server-amd64.iso
//!   media_iso_sha256: <64 hex>
//!   media_iso_bytes: <exact size>
//!   media_seed_state: armed          # CIDATA; `disarmed` relabels it SCRIPTS
//!   media_seed:
//!     user-data: "{{ secrets.t2_user_data }}"
//!     meta-data: "instance-id: t2-7\n"
//!     install.sh: "#!/bin/sh\n..."
//! ```
//!
//! - **device**: `/dev/disk/by-id/usb-*` only. `/dev/sdX` is refused at
//!   validate; at apply the disk must be removable or USB-attached, a whole
//!   disk, and hold nothing mounted or claimed (holders).
//! - **ISO**: path + pinned sha256 + exact size. The stick's ISO region is
//!   bytes `[32768, iso_bytes)`, the ISO9660 system area excluded: that area
//!   is the partition table forjar rewrites to append the seed partition.
//! - **seed**: a FAT16 image built in Rust (deterministic, no mtools), written
//!   as partition `start = iso end rounded up to 1 MiB`, `seed_mib` long.
//! - **lint**: every shell fragment of the seed goes through bashrs (I8) at
//!   validate, so a bad `early-command` fails `forjar plan` (`seed_lint`).
//! - **check**: reads the disk back: ISO region sha256, the partition entry,
//!   the label bytes, a `cmp` of every seed file at its extent, and the whole
//!   partition sha256. `--refresh` is `unchanged` only when all of it matches.
//!
//! `FORJAR_MEDIA_ROOT` prefixes the device, `/sys/class/block` and
//! `/proc/self/mounts`, so the tests run the generated scripts against an
//! image FILE with a fake sysfs. Unset (production) it is empty and the
//! device must be a block device.

use crate::core::shell_escape::sh_squote;
use crate::core::types::{BootableMedia, Resource};
use crate::resources::verdict::{assert_that, check_script_from, exit_if_diverged, flag_init};
use base64::Engine;

pub mod fat;
pub mod seed_lint;

#[cfg(test)]
mod tests;

/// Bytes at the head of the ISO excluded from the region check: the ISO9660
/// system area, which holds the (hybrid) partition table forjar rewrites.
pub const SYSTEM_AREA_BYTES: u64 = 32768;

/// GPT partition type of the seed partition (Microsoft basic data, i.e. FAT).
const GPT_BASIC_DATA: &str = "EBD0A0A2-B9E5-4433-87C0-68B6B72699C7";

/// A validated declaration, its lint verdict, and the FAT image it implies.
pub struct Media {
    /// The declaration.
    pub spec: BootableMedia,
    /// The seed partition, byte for byte.
    pub image: fat::FatImage,
}

/// Validate, lint and build.
///
/// # Errors
///
/// Returns the declaration refusal, the bashrs refusal of a seed fragment, or
/// a seed the FAT writer cannot place.
pub fn media_of(resource: &Resource) -> Result<Media, String> {
    if let Some(st) = resource.state.as_deref().filter(|s| *s != "present") {
        return Err(format!(
            "state '{st}' is not supported: forjar writes a stick, it never wipes one"
        ));
    }
    let spec = BootableMedia::new(&resource.media)?;
    seed_lint::lint(&spec.seed)?;
    let files: Vec<(String, Vec<u8>)> = spec
        .seed
        .iter()
        .map(|(k, v)| (k.clone(), v.clone().into_bytes()))
        .collect();
    let sectors = usize::try_from(spec.seed_sectors()).map_err(|e| e.to_string())?;
    let image = fat::build(spec.state.label(), &files, sectors)?;
    Ok(Media { spec, image })
}

fn reject(msg: &str) -> String {
    format!("echo {} >&2\nexit 1", sh_squote(&format!("ERROR: {msg}")))
}

fn b64(data: &[u8]) -> String {
    let s = base64::engine::general_purpose::STANDARD.encode(data);
    let mut out = String::with_capacity(s.len() + s.len() / 76 + 1);
    for chunk in s.as_bytes().chunks(76) {
        out.push_str(std::str::from_utf8(chunk).unwrap_or_default());
        out.push('\n');
    }
    out
}

/// Shell shared by check, apply and state_query: names, sysfs facts, and the
/// region hash function. Read-only.
fn prelude(m: &Media) -> String {
    let s = &m.spec;
    format!(
        "FJ_ROOT=\"${{FORJAR_MEDIA_ROOT:-}}\"\n\
         DEV=\"$FJ_ROOT\"{dev}\n\
         ISO={iso}\n\
         ISO_BYTES={bytes}\n\
         SEED_START={start}\n\
         SEED_SECTORS={sectors}\n\
         SEED_MIB={mib}\n\
         SYS=\"$FJ_ROOT/sys/class/block\"\n\
         MOUNTS=\"$FJ_ROOT/proc/self/mounts\"\n\
         NAME=''\n\
         if [ -e \"$DEV\" ]; then NAME=$(basename \"$(readlink -f \"$DEV\")\"); fi\n\
         FJ_BLOCK=1\n\
         if [ -z \"$FJ_ROOT\" ] && [ ! -b \"$DEV\" ]; then FJ_BLOCK=0; fi\n\
         FJ_USB=0\n\
         FJ_WHOLE=0\n\
         FJ_BUSY=0\n\
         FJ_SIZE=0\n\
         if [ -n \"$NAME\" ] && [ -d \"$SYS/$NAME\" ]; then\n\
         \x20 if [ \"$(cat \"$SYS/$NAME/removable\" 2>/dev/null)\" = 1 ]; then FJ_USB=1; fi\n\
         \x20 FJ_REAL=$(readlink -f \"$SYS/$NAME\")\n\
         \x20 FJ_REAL=${{FJ_REAL#\"$FJ_ROOT\"}}\n\
         \x20 case \"$FJ_REAL\" in */usb*) FJ_USB=1 ;; esac\n\
         \x20 if [ ! -e \"$SYS/$NAME/partition\" ]; then FJ_WHOLE=1; fi\n\
         \x20 FJ_SIZE=$(cat \"$SYS/$NAME/size\" 2>/dev/null || echo 0)\n\
         \x20 FJ_PARTS=\" /dev/$NAME \"\n\
         \x20 if [ -n \"$(ls -A \"$SYS/$NAME/holders\" 2>/dev/null)\" ]; then FJ_BUSY=1; fi\n\
         \x20 for FJ_P in \"$SYS/$NAME/$NAME\"*; do\n\
         \x20   [ -e \"$FJ_P\" ] || continue\n\
         \x20   FJ_PARTS=\"$FJ_PARTS/dev/${{FJ_P##*/}} \"\n\
         \x20   if [ -n \"$(ls -A \"$FJ_P/holders\" 2>/dev/null)\" ]; then FJ_BUSY=1; fi\n\
         \x20 done\n\
         \x20 if awk -v l=\"$FJ_PARTS\" 'index(l, \" \" $1 \" \") {{ f = 1 }} END {{ exit !f }}' \"$MOUNTS\" 2>/dev/null; then FJ_BUSY=1; fi\n\
         fi\n\
         fj_region() {{\n\
         \x20 dd if=\"$1\" iflag=skip_bytes,count_bytes skip={sys} count=$((ISO_BYTES - {sys})) bs=4M status=none 2>/dev/null | sha256sum | cut -d' ' -f1\n\
         }}\n\
         fj_seed_sha() {{\n\
         \x20 dd if=\"$DEV\" bs=1M skip=$((SEED_START / 2048)) count=\"$SEED_MIB\" status=none 2>/dev/null | sha256sum | cut -d' ' -f1\n\
         }}\n",
        dev = sh_squote(&s.device),
        iso = sh_squote(&s.iso),
        bytes = s.iso_bytes,
        start = s.seed_start_sector(),
        sectors = s.seed_sectors(),
        mib = s.seed_mib,
        sys = SYSTEM_AREA_BYTES,
    )
}

/// The read-back assertions, in order. Each prints one stable marker.
fn assertions(m: &Media) -> Vec<String> {
    let s = &m.spec;
    let label = s.state.label();
    let mut out = vec![
        assert_that(
            "[ -n \"$NAME\" ] && [ \"$FJ_BLOCK\" = 1 ] && [ \"$FJ_USB\" = 1 ] && [ \"$FJ_WHOLE\" = 1 ]",
            "media=usb-disk",
            "media=refused:not-a-removable-usb-whole-disk",
        ),
        assert_that(
            &format!(
                "[ -f \"$ISO\" ] && [ \"$(stat -c %s \"$ISO\")\" = \"$ISO_BYTES\" ] && [ \"$(sha256sum < \"$ISO\" | cut -d' ' -f1)\" = {} ]",
                sh_squote(&s.iso_sha256)
            ),
            "iso=pinned",
            "iso=mismatch",
        ),
        assert_that(
            "[ -n \"$NAME\" ] && [ -f \"$ISO\" ] && [ \"$(fj_region \"$DEV\")\" = \"$(fj_region \"$ISO\")\" ]",
            "iso_region=match",
            "iso_region=drift",
        ),
        assert_that(
            "[ -n \"$NAME\" ] && sfdisk -d \"$DEV\" 2>/dev/null | grep -Eq \"start= *$SEED_START, size= *$SEED_SECTORS,\"",
            "seed_partition=present",
            "seed_partition=drift",
        ),
        assert_that(
            &format!(
                "[ -n \"$NAME\" ] && [ \"$(dd if=\"$DEV\" iflag=skip_bytes,count_bytes skip=$((SEED_START * 512 + 43)) count=11 bs=512 status=none 2>/dev/null)\" = {} ]",
                sh_squote(&format!("{label:<11}"))
            ),
            &format!("seed_label={label}"),
            "seed_label=drift",
        ),
    ];
    for e in &m.image.extents {
        let content = s
            .seed
            .iter()
            .find(|(k, _)| *k == e.name)
            .map(|(_, v)| v.as_bytes())
            .unwrap_or_default();
        out.push(assert_that(
            &format!(
                "[ -n \"$NAME\" ] && base64 -d <<'FORJAR_EOF' | cmp -s -n {len} - \"$DEV\" 0 $((SEED_START * 512 + {off}))\n{data}FORJAR_EOF",
                len = e.len,
                off = e.offset,
                data = b64(content),
            ),
            &format!("seed_file={}:match", e.name),
            &format!("seed_file={}:drift", e.name),
        ));
    }
    out.push(assert_that(
        &format!(
            "[ -n \"$NAME\" ] && [ \"$(fj_seed_sha)\" = {} ]",
            sh_squote(&m.image.sha256())
        ),
        "seed_partition_sha256=match",
        "seed_partition_sha256=drift",
    ));
    out
}

/// Read-only check: exit 0 only when every read-back assertion holds.
pub fn check_script(resource: &Resource) -> String {
    match media_of(resource) {
        Ok(m) => format!(
            "set -u\n{}{}",
            prelude(&m),
            check_script_from(&assertions(&m))
        ),
        Err(e) => reject(&e),
    }
}

/// Stable markers only (drift hashes stdout); always exits 0.
pub fn state_query_script(resource: &Resource) -> String {
    match media_of(resource) {
        Ok(m) => format!(
            "set -u\n{}{}\n{}\nexit 0\n",
            prelude(&m),
            flag_init(),
            assertions(&m).join("\n")
        ),
        Err(e) => reject(&e),
    }
}

fn refuse_unless(cond: &str, msg: &str) -> String {
    format!(
        "if\n  {cond}\nthen\n  :\nelse\n  echo {} >&2\n  exit 1\nfi\n",
        sh_squote(&format!("ERROR: bootable_media: {msg}"))
    )
}

/// Write the ISO (when its region or the seed entry differs) and the seed
/// partition (when its sha256 differs), then prove it by reading back.
pub fn apply_script(resource: &Resource) -> String {
    let m = match media_of(resource) {
        Ok(m) => m,
        Err(e) => return reject(&e),
    };
    let s = &m.spec;
    let mut out = format!("set -eu\n{}", prelude(&m));
    out.push_str(&refuse_unless(
        "[ -n \"$NAME\" ]",
        "media_device does not exist",
    ));
    out.push_str(&refuse_unless(
        "[ \"$FJ_BLOCK\" = 1 ]",
        "media_device is not a block device",
    ));
    out.push_str(&refuse_unless(
        "[ \"$FJ_WHOLE\" = 1 ]",
        "media_device is a partition, not a whole disk",
    ));
    out.push_str(&refuse_unless(
        "[ \"$FJ_USB\" = 1 ]",
        "media_device is neither removable nor USB-attached; refusing to write it",
    ));
    out.push_str(&refuse_unless(
        "[ \"$FJ_BUSY\" = 0 ]",
        "media_device holds a mounted or claimed filesystem; refusing to write it",
    ));
    out.push_str(&refuse_unless(
        "[ \"$FJ_SIZE\" -ge $((SEED_START + SEED_SECTORS + 34)) ]",
        "media_device is too small for the ISO and the seed partition",
    ));
    out.push_str(&refuse_unless(
        &format!(
            "[ -f \"$ISO\" ] && [ \"$(stat -c %s \"$ISO\")\" = \"$ISO_BYTES\" ] && [ \"$(sha256sum < \"$ISO\" | cut -d' ' -f1)\" = {} ]",
            sh_squote(&s.iso_sha256)
        ),
        "media_iso does not match media_iso_sha256 / media_iso_bytes",
    ));
    out.push_str(&format!(
        "FJ_WRITE_ISO=0\n\
         if [ \"$(fj_region \"$DEV\")\" != \"$(fj_region \"$ISO\")\" ]; then FJ_WRITE_ISO=1; fi\n\
         if ! sfdisk -d \"$DEV\" 2>/dev/null | grep -Eq \"start= *$SEED_START, size= *$SEED_SECTORS,\"; then FJ_WRITE_ISO=1; fi\n\
         if [ \"$FJ_WRITE_ISO\" = 1 ]; then\n\
         \x20 echo 'forjar: writing the ISO and appending the seed partition'\n\
         \x20 dd if=\"$ISO\" of=\"$DEV\" bs=4M conv=fsync,notrunc status=none\n\
         \x20 FJ_LABEL=$(sfdisk -d \"$DEV\" 2>/dev/null | sed -n 's/^label: *//p')\n\
         \x20 case \"$FJ_LABEL\" in\n\
         \x20   gpt)\n\
         \x20     sfdisk -q --relocate gpt-bak-std \"$DEV\" >/dev/null\n\
         \x20     printf 'start=%s, size=%s, type={gpt}, name=\"forjar-seed\"\\n' \"$SEED_START\" \"$SEED_SECTORS\" | sfdisk -q --append --wipe never --wipe-partitions never \"$DEV\" >/dev/null\n\
         \x20     ;;\n\
         \x20   dos)\n\
         \x20     printf 'start=%s, size=%s, type=c\\n' \"$SEED_START\" \"$SEED_SECTORS\" | sfdisk -q --append --wipe never --wipe-partitions never \"$DEV\" >/dev/null\n\
         \x20     ;;\n\
         \x20   *)\n\
         \x20     echo 'ERROR: bootable_media: media_iso carries no gpt or dos partition table' >&2\n\
         \x20     exit 1\n\
         \x20     ;;\n\
         \x20 esac\n\
         fi\n\
         if [ \"$(fj_seed_sha)\" != {sha} ]; then\n\
         \x20 echo 'forjar: writing the seed partition'\n\
         \x20 dd if=/dev/zero of=\"$DEV\" bs=1M seek=$((SEED_START / 2048)) count=\"$SEED_MIB\" conv=fsync,notrunc status=none\n",
        gpt = GPT_BASIC_DATA,
        sha = sh_squote(&m.image.sha256()),
    ));
    for (sector, count) in fat::nonzero_runs(&m.image.bytes) {
        let a = sector * fat::SECTOR;
        let b = a + count * fat::SECTOR;
        out.push_str(&format!(
            "  base64 -d <<'FORJAR_EOF' | dd of=\"$DEV\" bs=512 seek=$((SEED_START + {sector})) conv=notrunc status=none\n{}FORJAR_EOF\n",
            b64(&m.image.bytes[a..b])
        ));
    }
    out.push_str("  sync\nfi\n");
    out.push_str(&flag_init());
    out.push('\n');
    out.push_str(&assertions(&m).join("\n"));
    out.push('\n');
    out.push_str(&exit_if_diverged(
        "bootable_media: the stick does not read back as declared after apply",
    ));
    out.push('\n');
    out
}

/// The plan line: the exact device and every region it will write.
pub fn plan_line(resource_id: &str, resource: &Resource) -> String {
    match media_of(resource) {
        Ok(m) => {
            let s = &m.spec;
            let names: Vec<&str> = s.seed.iter().map(|(k, _)| k.as_str()).collect();
            format!(
                "{resource_id}: write {dev}: ISO region bytes {sys}..{end} from {iso} (sha256 {sha}); \
                 seed partition {label} at sector {start}, {sectors} sectors ({mib} MiB), \
                 appended after the ISO's own partitions; seed files: {files}",
                dev = s.device,
                sys = SYSTEM_AREA_BYTES,
                end = s.iso_bytes,
                iso = s.iso,
                sha = &s.iso_sha256[..12],
                label = s.state.label(),
                start = s.seed_start_sector(),
                sectors = s.seed_sectors(),
                mib = s.seed_mib,
                files = names.join(", "),
            )
        }
        Err(e) => format!("{resource_id}: REFUSED: {e}"),
    }
}

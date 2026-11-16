//! #1128-#1130: root volume, optional data volume and volume tags for `cloud_instance`.
//! Types are an allow-list, sizes and rates are range-checked integers, so no
//! declared value can carry shell text. A declared volume is part of the
//! instance key: changing it on a live instance is refused, never a second launch.

use crate::core::shell_escape::sh_squote;
use crate::core::types::CloudSpec;

const TYPES: [&str; 2] = ["gp2", "gp3"];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Volume {
    device: &'static str,
    gb: u32,
    kind: String,
    iops: Option<u32>,
    throughput: Option<u32>,
}

/// Validated storage. `Default` = nothing declared = today's command line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Storage {
    volumes: Vec<Volume>,
    tag_volumes: bool,
}

fn range(name: &str, v: u32, lo: u32, hi: u32) -> Result<(), String> {
    if (lo..=hi).contains(&v) {
        return Ok(());
    }
    Err(format!("{name} must be {lo} to {hi}"))
}

fn kind_of(name: &str, t: &Option<String>) -> Result<String, String> {
    let t = t.as_deref().unwrap_or("gp3");
    if TYPES.contains(&t) {
        return Ok(t.to_string());
    }
    Err(format!("{name} must be gp2 or gp3"))
}

fn root(c: &CloudSpec) -> Result<Option<Volume>, String> {
    let Some(gb) = c.root_volume_gb else {
        return match c.root_volume_type {
            None => Ok(None),
            Some(_) => Err("cloud_root_volume_type needs cloud_root_volume_gb".into()),
        };
    };
    range("cloud_root_volume_gb", gb, 1, 16384)?;
    let kind = kind_of("cloud_root_volume_type", &c.root_volume_type)?;
    Ok(Some(Volume {
        device: "/dev/sda1",
        gb,
        kind,
        iops: None,
        throughput: None,
    }))
}

fn data(c: &CloudSpec) -> Result<Option<Volume>, String> {
    let Some(gb) = c.data_volume_gb else {
        let stray = c.data_volume_type.is_some()
            || c.data_volume_iops.is_some()
            || c.data_volume_throughput.is_some();
        return if stray {
            Err("cloud_data_volume_* needs cloud_data_volume_gb".into())
        } else {
            Ok(None)
        };
    };
    range("cloud_data_volume_gb", gb, 1, 16384)?;
    let kind = kind_of("cloud_data_volume_type", &c.data_volume_type)?;
    let (iops, throughput) = (c.data_volume_iops, c.data_volume_throughput);
    if kind != "gp3" && (iops.is_some() || throughput.is_some()) {
        return Err("cloud_data_volume_iops and _throughput need type gp3".into());
    }
    if let Some(i) = iops {
        range("cloud_data_volume_iops", i, 3000, 16000)?;
        if i > gb.saturating_mul(500) {
            return Err("cloud_data_volume_iops is at most 500 per GiB".into());
        }
    }
    if let Some(t) = throughput {
        range("cloud_data_volume_throughput", t, 125, 1000)?;
        if t > iops.unwrap_or(3000) / 4 {
            return Err(
                "cloud_data_volume_throughput is at most iops / 4 (3000 iops if unset)".into(),
            );
        }
    }
    Ok(Some(Volume {
        device: "/dev/sdf",
        gb,
        kind,
        iops,
        throughput,
    }))
}

impl Storage {
    pub(super) fn of(c: &CloudSpec, tags: &[String]) -> Result<Self, String> {
        let volumes: Vec<Volume> = [root(c)?, data(c)?].into_iter().flatten().collect();
        let tag_volumes = c.volume_tags == Some(true);
        if tag_volumes && tags.is_empty() {
            return Err("cloud_volume_tags needs at least one entry in tags".into());
        }
        Ok(Self {
            volumes,
            tag_volumes,
        })
    }

    fn mapping(v: &Volume) -> String {
        let mut ebs = format!("VolumeSize={},VolumeType={}", v.gb, v.kind);
        if let Some(i) = v.iops {
            ebs += &format!(",Iops={i}");
        }
        if let Some(t) = v.throughput {
            ebs += &format!(",Throughput={t}");
        }
        sh_squote(&format!(
            "DeviceName={},Ebs={{{ebs},DeleteOnTermination=true}}",
            v.device
        ))
    }

    /// `run-instances` flag, followed by a space; empty if no volume is declared.
    pub(super) fn args(&self) -> String {
        if self.volumes.is_empty() {
            return String::new();
        }
        let m: Vec<String> = self.volumes.iter().map(Self::mapping).collect();
        format!("--block-device-mappings {} ", m.join(" "))
    }

    /// Second tag specification (leading space) with the declared tags; empty unless asked.
    /// `forjar-key` and `forjar-owner` stay on the instance, so teardown finds only instances.
    pub(super) fn volume_tagspec(&self, tags: &[String]) -> String {
        if !self.tag_volumes {
            return String::new();
        }
        let t: Vec<String> = tags
            .iter()
            .filter_map(|t| t.split_once('='))
            .map(|(k, v)| format!("{{Key={k},Value={v}}}"))
            .collect();
        format!(
            " {}",
            sh_squote(&format!("ResourceType=volume,Tags=[{}]", t.join(",")))
        )
    }

    /// Key suffix: empty if no volume is declared, else a short hash of the volumes.
    pub(super) fn key_suffix(&self) -> String {
        if self.volumes.is_empty() {
            return String::new();
        }
        let canon: Vec<String> = self
            .volumes
            .iter()
            .map(|v| {
                format!(
                    "{},{},{},{:?},{:?}",
                    v.device, v.gb, v.kind, v.iops, v.throughput
                )
            })
            .collect();
        let h = crate::tripwire::hasher::hash_string(&canon.join(";"));
        format!("@s{}", &h["blake3:".len()..][..12])
    }
}

//! #1128-#1130: root volume, optional data volume and volume tags for `cloud_instance`.
//! Types are an allow-list, sizes and rates are range-checked integers, so no
//! declared value can carry shell text. A declared volume is part of the
//! instance key: changing it on a live instance is refused, never a second launch.

use crate::core::shell_escape::sh_squote;
use crate::core::types::CloudSpec;

const TYPES: [&str; 2] = ["gp2", "gp3"];
/// Marker device of the root volume: the real name is read from the image at apply time.
const ROOT: &str = "<root>";

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
        device: ROOT,
        gb,
        kind,
        iops: None,
        throughput: None,
    }))
}

/// The gp3 limits here are the older, lower AWS figures (checked 2026-11-16): deliberately
/// conservative refusals; a larger valid request is refused, never passed through.
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

    fn has_root(&self) -> bool {
        self.volumes.iter().any(|v| v.device == ROOT)
    }

    /// The `--image-id` value: today's quoted literal, or the validated `$img` when a
    /// root volume needs the image's own root device name.
    pub(super) fn image_arg(&self, ami: &Option<String>) -> String {
        if self.has_root() {
            return "\"$img\"".into();
        }
        sh_squote(&super::ami::image_id(ami))
    }

    /// Shell run after the gate and owner checks and BEFORE `run-instances`: sets `img`
    /// (the resolved image id) and `rootdev` (that image's `RootDeviceName`), or EXITS
    /// NON-ZERO with REFUSED. Empty when no root volume is declared. Every reply must be
    /// a whole token (`ami-` + 8 or 17 hex; `/dev/` + 2 to 12 of a-z0-9); anything else,
    /// `None` and a failed call included, refuses before any launch.
    pub(super) fn root_lookup(&self, region: &str, ami: &Option<String>) -> String {
        if !self.has_root() {
            return String::new();
        }
        let id = super::ami::image_id(ami);
        let get = match id.strip_prefix("resolve:ssm:") {
            Some(p) => format!(
                "aws ssm get-parameter --region {region} --name {} --query Parameter.Value --output text",
                sh_squote(p)
            ),
            None => format!("echo {}", sh_squote(&id)),
        };
        let fail = |what: &str| {
            format!(
                "echo {} >&2; exit 1",
                sh_squote(&format!("REFUSED: {what}"))
            )
        };
        format!(
            "img=\"$({get})\" || {{ {f1}; }}\n\
             case \"$img\" in ami-*) h=\"${{img#ami-}}\";; *) h='!';; esac\n\
             case \"$h\" in *[!0-9a-f]*) h='!';; esac\n\
             if [ \"${{#h}}\" -ne 8 ] && [ \"${{#h}}\" -ne 17 ]; then {f2}; fi\n\
             rootdev=\"$(aws ec2 describe-images --region {region} --image-ids \"$img\" --query 'Images[0].RootDeviceName' --output text)\" || {{ {f3}; }}\n\
             case \"$rootdev\" in /dev/*) h=\"${{rootdev#/dev/}}\";; *) h='!';; esac\n\
             case \"$h\" in *[!a-z0-9]*) h='!';; esac\n\
             if [ \"${{#h}}\" -lt 2 ] || [ \"${{#h}}\" -gt 12 ]; then {f4}; fi\n",
            f1 = fail("image id lookup failed; nothing was launched"),
            f2 = fail("image id lookup returned something that is not a whole AMI id"),
            f3 = fail("root device lookup failed; nothing was launched"),
            f4 = fail("root device lookup returned something that is not a device name; nothing was launched"),
        )
    }

    fn mapping(v: &Volume) -> String {
        let mut ebs = format!("VolumeSize={},VolumeType={}", v.gb, v.kind);
        if let Some(i) = v.iops {
            ebs += &format!(",Iops={i}");
        }
        if let Some(t) = v.throughput {
            ebs += &format!(",Throughput={t}");
        }
        let tail = format!("Ebs={{{ebs},DeleteOnTermination=true}}");
        if v.device == ROOT {
            // the validated `rootdev`, spliced between two single-quoted halves
            return format!(
                "'DeviceName='\"$rootdev\"{}",
                sh_squote(&format!(",{tail}"))
            );
        }
        sh_squote(&format!("DeviceName={},{tail}", v.device))
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

    /// Check fragment for a live instance (`$ids` set): do its volumes carry every declared tag?
    /// A marker line, never a failed check (apply cannot retag a live instance). None if undeclared.
    pub(super) fn tag_check(&self, region: &str, tags: &[String], key: &str) -> Option<String> {
        if !self.tag_volumes {
            return None;
        }
        let by_tag: String = tags
            .iter()
            .filter_map(|t| t.split_once('='))
            .map(|(k, v)| format!(" 'Name=tag:{k},Values={v}'"))
            .collect();
        let q = |f: &str| {
            format!("aws ec2 describe-volumes --region {} --filters \"$vf\"{f} --query 'length(Volumes)' --output text", sh_squote(region))
        };
        let say = |m: &str| format!("echo {}", sh_squote(&format!("{m}:{key}")));
        Some(format!(
            "if [ -n \"$ids\" ]; then\n\
             vf=\"Name=attachment.instance-id,Values=$(echo \"$ids\" | tr ' ' ',')\"\n\
             va=\"$({all})\" || va=''; vt=\"$({tagged})\" || vt=''\n\
             case \"$va\" in ''|*[!0-9]*) va=x;; esac; case \"$vt\" in ''|*[!0-9]*) vt=x;; esac\n\
             if [ \"$va\" = x ] || [ \"$vt\" = x ]; then {bad}\n\
             elif [ \"$va\" = \"$vt\" ]; then {ok}\n\
             else {drift}; fi\n\
             fi",
            all = q(""),
            tagged = q(&by_tag),
            bad = say("volume-tags-unreadable"),
            ok = say("volume-tags-match"),
            drift = say("volume-tags-drift"),
        ))
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

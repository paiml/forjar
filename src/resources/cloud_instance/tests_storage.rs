//! #1128-#1130 falsifiers: root volume, data volume, volume tags (asserted on the fake aws log).

use super::tests::{gated, res, Sandbox};
use super::*;
use crate::core::types::CloudSpec;

/// Today's `apply_script(gated("/t"))` from the run-instances call on, captured from
/// origin/main (56cd6f54) before this change: a literal, not a re-derivation.
const TODAY_TAIL: &str = r##"aws ec2 run-instances --region 'eu-south-2' --count 1 --instance-type 'c7a.xlarge' --image-id 'resolve:ssm:/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64' --instance-market-options 'MarketType=spot,SpotOptions={MaxPrice=0.20,SpotInstanceType=one-time,InstanceInterruptionBehavior=terminate}' --instance-initiated-shutdown-behavior terminate --metadata-options 'HttpTokens=required,HttpPutResponseHopLimit=1,HttpEndpoint=enabled' --user-data '#!/bin/sh
shutdown -h +60
export FORJAR_IMAGE='"'"'123456789012.dkr.ecr.eu-south-2.amazonaws.com/img001@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef'"'"'
' --tag-specifications 'ResourceType=instance,Tags=[{Key=forjar-key,Value=ci-box-a@eu-south-2@0123456789ab},{Key=forjar-owner,Value=ci-box-a@eu-south-2},{Key=pool,Value=spot-rmedia-pr}]'
"##;

/// Today's launch tail, now followed by the id-only query and the result handling.
fn today_tail() -> String {
    format!(
        "{}{}{}",
        TODAY_TAIL.trim_end(),
        launch::CAPTURE,
        launch::POST
    )
}

fn run_line(r: &Resource) -> String {
    let sb = Sandbox::new();
    sb.open_gate();
    let mut r = r.clone();
    r.cloud.kill_switch = gated(sb.path()).cloud.kill_switch;
    r.cloud.parity_gate = gated(sb.path()).cloud.parity_gate;
    r.cloud.budget_cap_usd_day = Some("5".into());
    let (rc, out) = sb.run_with(&apply_script(&r), "", Some(""));
    assert_eq!(rc, 0, "{out}");
    // the user-data argument spans lines, so keep everything from the launch call on
    let log = sb.log();
    log.find("ec2 run-instances").map_or(String::new(), |i| {
        log[i..]
            .trim_end()
            .trim_end_matches(" --query Instances[0].InstanceId --output text")
            .to_string()
    })
}

fn with(f: impl Fn(&mut CloudSpec)) -> Resource {
    let mut r = res();
    f(&mut r.cloud);
    r
}

fn good_data(c: &mut CloudSpec) {
    c.data_volume_gb = Some(800);
    c.data_volume_iops = Some(6000);
    c.data_volume_throughput = Some(500);
}

#[test]
fn nothing_declared_is_byte_identical_to_today() {
    let s = apply_script(&gated(std::path::Path::new("/t")));
    assert!(s.ends_with(&today_tail()), "{s}");
    assert_eq!(s.matches("block-device-mappings").count(), 0);
    assert!(!key_of(&spec_of(&res()).unwrap()).contains("@s"));
    // tags alone (volume_tags unset or false) never add a volume tag specification
    let mut r = gated(std::path::Path::new("/t"));
    r.cloud.volume_tags = Some(false);
    assert!(apply_script(&r).ends_with(&today_tail()));
}

#[test]
fn root_volume_emits_mapping_gp3_40() {
    let l = run_line(&with(|c| c.root_volume_gb = Some(40)));
    assert!(
        l.contains("--block-device-mappings DeviceName=/dev/xvda,Ebs={VolumeSize=40,VolumeType=gp3,DeleteOnTermination=true} --user-data"),
        "{l}"
    );
    let l = run_line(&with(|c| {
        c.root_volume_gb = Some(8);
        c.root_volume_type = Some("gp2".into());
    }));
    assert!(
        l.contains("VolumeSize=8,VolumeType=gp2,DeleteOnTermination=true}"),
        "{l}"
    );
    let s = apply_script(&gated(std::path::Path::new("/t")));
    assert!(!s.contains("sda1") && !s.contains("describe-images"));
}

#[test]
fn data_volume_mapping_800_6000_500() {
    let l = run_line(&with(|c| {
        c.root_volume_gb = Some(40);
        good_data(c);
    }));
    let want = "--block-device-mappings DeviceName=/dev/xvda,Ebs={VolumeSize=40,VolumeType=gp3,DeleteOnTermination=true} DeviceName=/dev/sdf,Ebs={VolumeSize=800,VolumeType=gp3,Iops=6000,Throughput=500,DeleteOnTermination=true} --user-data";
    assert!(l.contains(want), "{l}");
    let l = run_line(&with(|c| c.data_volume_gb = Some(100)));
    assert!(
        l.contains(
            "DeviceName=/dev/sdf,Ebs={VolumeSize=100,VolumeType=gp3,DeleteOnTermination=true}"
        ),
        "{l}"
    );
    assert!(!l.contains("Iops") && !l.contains("xvda"), "{l}");
    let s = apply_script(&gated(std::path::Path::new("/t")));
    assert!(!s.contains("sdf"));
}

#[test]
fn tagspec_has_volume_resource_with_declared_tags() {
    let l = run_line(&with(|c| c.volume_tags = Some(true)));
    assert!(
        l.ends_with(" ResourceType=volume,Tags=[{Key=pool,Value=spot-rmedia-pr}]"),
        "{l}"
    );
    assert_eq!(l.matches("ResourceType=volume").count(), 1, "{l}");
    let vol = l.rsplit_once("ResourceType=volume").unwrap().1;
    assert!(!vol.contains("forjar-"), "{l}");
    assert!(
        l.contains("ResourceType=instance,Tags=[{Key=forjar-key"),
        "{l}"
    );
    // quoted in the generated script
    let mut r = gated(std::path::Path::new("/t"));
    r.cloud.volume_tags = Some(true);
    let tail = format!("{}{}{}", "--tag-specifications 'ResourceType=instance,Tags=[{Key=forjar-key,Value=ci-box-a@eu-south-2@0123456789ab},{Key=forjar-owner,Value=ci-box-a@eu-south-2},{Key=pool,Value=spot-rmedia-pr}]' 'ResourceType=volume,Tags=[{Key=pool,Value=spot-rmedia-pr}]'", launch::CAPTURE, launch::POST);
    assert!(apply_script(&r).ends_with(&tail));
}

fn refused(f: impl Fn(&mut CloudSpec), why: &str) {
    let r = with(f);
    let e = spec_of(&r).expect_err(why);
    assert!(e.contains("cloud_"), "{why}: {e}");
    let s = apply_script(&r);
    assert!(s.contains("REFUSED") || s.contains("exit 1"), "{why}: {s}");
    assert!(!s.contains("run-instances"), "{why}");
}

#[test]
fn bad_volume_declarations_are_refused_at_parse_and_codegen() {
    refused(|c| c.root_volume_gb = Some(0), "root 0");
    refused(|c| c.root_volume_gb = Some(16385), "root 16385");
    refused(
        |c| c.root_volume_type = Some("gp3".into()),
        "type without size",
    );
    for bad in [
        "io1",
        "GP3",
        "gp3 ",
        "gp3'",
        "gp3;x",
        "{{p.t}}",
        "",
        "gp3,Iops=1",
        "st1",
    ] {
        refused(
            |c| {
                c.root_volume_gb = Some(8);
                c.root_volume_type = Some(bad.into());
            },
            bad,
        );
        refused(
            |c| {
                c.data_volume_gb = Some(8);
                c.data_volume_type = Some(bad.into());
            },
            bad,
        );
    }
    refused(|c| c.data_volume_gb = Some(0), "data 0");
    refused(|c| c.data_volume_gb = Some(16385), "data 16385");
    refused(|c| c.data_volume_iops = Some(3000), "iops without size");
    refused(
        |c| c.data_volume_throughput = Some(125),
        "tput without size",
    );
    refused(
        |c| c.data_volume_type = Some("gp3".into()),
        "dtype without size",
    );
    let sized = |c: &mut CloudSpec| c.data_volume_gb = Some(800);
    refused(
        |c| {
            sized(c);
            c.data_volume_iops = Some(2999);
        },
        "iops low",
    );
    refused(
        |c| {
            sized(c);
            c.data_volume_iops = Some(16001);
        },
        "iops high",
    );
    refused(
        |c| {
            sized(c);
            c.data_volume_throughput = Some(124);
        },
        "tput low",
    );
    refused(
        |c| {
            sized(c);
            c.data_volume_throughput = Some(1001);
            c.data_volume_iops = Some(16000);
        },
        "tput high",
    );
    refused(
        |c| {
            sized(c);
            c.data_volume_throughput = Some(800);
        },
        "tput over iops/4",
    );
    refused(
        |c| {
            c.data_volume_gb = Some(5);
            c.data_volume_iops = Some(3000);
        },
        "iops over 500/GiB",
    );
    refused(
        |c| {
            sized(c);
            c.data_volume_type = Some("gp2".into());
            c.data_volume_iops = Some(3000);
        },
        "gp2 iops",
    );
    refused(
        |c| {
            sized(c);
            c.data_volume_type = Some("gp2".into());
            c.data_volume_throughput = Some(125);
        },
        "gp2 tput",
    );
    let mut r = res();
    r.tags.clear();
    r.cloud.volume_tags = Some(true);
    assert!(spec_of(&r).unwrap_err().contains("cloud_volume_tags"));
}

#[test]
fn limits_are_inclusive() {
    for f in [
        (|c: &mut CloudSpec| c.root_volume_gb = Some(1)) as fn(&mut CloudSpec),
        |c| c.root_volume_gb = Some(16384),
        |c| {
            c.data_volume_gb = Some(16384);
            c.data_volume_iops = Some(16000);
            c.data_volume_throughput = Some(1000);
        },
        |c| {
            c.data_volume_gb = Some(6);
            c.data_volume_iops = Some(3000);
            c.data_volume_throughput = Some(125);
        },
        |c| {
            c.data_volume_gb = Some(1);
            c.data_volume_type = Some("gp2".into());
        },
    ] {
        assert!(spec_of(&with(f)).is_ok());
    }
}

#[test]
fn a_changed_volume_is_part_of_the_key_and_is_refused_live() {
    let base = key_of(&spec_of(&res()).unwrap());
    let key = |f: fn(&mut CloudSpec)| key_of(&spec_of(&with(f)).unwrap());
    let ks = [
        key(|c| c.root_volume_gb = Some(40)),
        key(|c| c.root_volume_gb = Some(41)),
        key(|c| {
            c.root_volume_gb = Some(40);
            c.root_volume_type = Some("gp2".into());
        }),
        key(good_data),
        key(|c| {
            good_data(c);
            c.data_volume_iops = Some(6001);
        }),
        key(|c| {
            good_data(c);
            c.data_volume_throughput = Some(400);
        }),
        key(|c| {
            good_data(c);
            c.data_volume_gb = Some(801);
        }),
        key(|c| {
            c.root_volume_gb = Some(800);
            c.data_volume_gb = None;
        }),
    ];
    let set: std::collections::BTreeSet<_> = ks.iter().chain([&base]).collect();
    assert_eq!(set.len(), ks.len() + 1, "{ks:?}");
    assert!(ks.iter().all(|k| k.starts_with(&base) && k.contains("@s")));
    // volume tags are not part of the key
    assert_eq!(key(|c| c.volume_tags = Some(true)), base);
    // live instance with the old key: new volume size refuses, never launches
    let sb = Sandbox::new();
    sb.open_gate();
    let old = key_of(&spec_of(&with(|c| c.root_volume_gb = Some(40))).unwrap());
    let fx = format!("i-00a1b2c3 {old} ci-box-a@eu-south-2 sir-0own\n");
    let mut r = gated(sb.path());
    r.cloud.root_volume_gb = Some(80);
    let (rc, out) = sb.run_with(&apply_script(&r), "", Some(&fx));
    assert_ne!(rc, 0, "{out}");
    assert!(out.contains("REFUSED"), "{out}");
    assert!(!sb.log().contains("run-instances"), "{}", sb.log());
    r.cloud.root_volume_gb = Some(40);
    let (rc, out) = sb.run_with(&apply_script(&r), "", Some(&fx));
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("present"), "{out}");
}

fn root_40() -> Resource {
    with(|c| c.root_volume_gb = Some(40))
}

/// Run a gated root-volume apply against a fake aws whose root-device reply is set by `f`.
fn root_run(r: &Resource, f: impl Fn(&Sandbox)) -> (i32, String, String) {
    let sb = Sandbox::new();
    sb.open_gate();
    f(&sb);
    let mut r = r.clone();
    r.cloud.kill_switch = gated(sb.path()).cloud.kill_switch;
    r.cloud.parity_gate = gated(sb.path()).cloud.parity_gate;
    r.cloud.budget_cap_usd_day = Some("5".into());
    let (rc, out) = sb.run_with(&apply_script(&r), "", Some(""));
    (rc, out, sb.log())
}

#[test]
fn root_device_is_read_from_the_image_before_run_instances() {
    let s = apply_script(&gated(std::path::Path::new("/t")));
    assert!(!s.contains("describe-images"));
    let mut r = gated(std::path::Path::new("/t"));
    r.cloud.root_volume_gb = Some(40);
    let s = apply_script(&r);
    let at = s.find("describe-images").expect("lookup emitted");
    assert!(at < s.find("aws ec2 run-instances").unwrap(), "{s}");
    assert!(s.contains("--query 'Images[0].RootDeviceName' --output text"));
    assert!(
        !s.contains("/dev/sda1") && s.contains("\"$rootdev\""),
        "{s}"
    );
    // default image: resolved through SSM first, then the same id is launched
    assert!(
        s.contains("aws ssm get-parameter --region 'eu-south-2'"),
        "{s}"
    );
    assert!(s.contains("--image-id \"$img\" "), "{s}");
}

#[test]
fn root_device_comes_from_the_image_not_a_constant() {
    for dev in ["/dev/xvda", "/dev/sda1", "/dev/nvme0n1"] {
        let (rc, out, log) = root_run(&root_40(), |sb| sb.set_root_device(&format!("{dev}\n")));
        assert_eq!(rc, 0, "{dev}: {out}");
        assert!(
            log.contains(&format!("DeviceName={dev},Ebs={{VolumeSize=40")),
            "{log}"
        );
        let (d, r) = (log.find("describe-images"), log.find("run-instances"));
        assert!(d.unwrap() < r.unwrap(), "{log}");
        assert!(log.contains("--image-id ami-0123456789abcdef0"), "{log}");
    }
}

#[test]
fn pinned_image_is_looked_up_directly_without_ssm() {
    let (rc, out, log) = root_run(
        &with(|c| {
            c.root_volume_gb = Some(40);
            c.ami = Some("ami-0123abcd".into());
        }),
        |_| {},
    );
    assert_eq!(rc, 0, "{out}");
    assert!(
        log.contains("describe-images --region eu-south-2 --image-ids ami-0123abcd"),
        "{log}"
    );
    assert!(!log.contains("ssm"), "{log}");
}

#[test]
fn bad_or_failed_root_device_lookup_refuses_and_never_launches() {
    for bad in [
        "None\n",
        "\n",
        "",
        "/dev/\n",
        "/dev/a\n",
        "sda1\n",
        "/dev/SDA1\n",
        "/dev/sda1;x\n",
        "/dev/abcdefghijklm\n",
        "/dev/xv da\n",
        "/dev/xvda\n/dev/sdb\n",
        "/dev/xvda/\n",
        "\"/dev/xvda\"\n",
    ] {
        let (rc, out, log) = root_run(&root_40(), |sb| sb.set_root_device(bad));
        assert_ne!(rc, 0, "{bad:?}: {out}");
        assert!(out.contains("REFUSED"), "{bad:?}: {out}");
        assert!(!log.contains("run-instances"), "{bad:?}: {log}");
    }
    let (rc, out, log) = root_run(&root_40(), |sb| sb.fail_images());
    assert_ne!(rc, 0, "{out}");
    assert!(
        out.contains("REFUSED") && !log.contains("run-instances"),
        "{out}{log}"
    );
    let (rc, out, log) = root_run(&root_40(), |sb| sb.fail_ssm());
    assert_ne!(rc, 0, "{out}");
    assert!(
        out.contains("REFUSED") && !log.contains("describe-images"),
        "{out}{log}"
    );
    assert!(!log.contains("run-instances"), "{log}");
}

#[test]
fn data_volume_alone_needs_no_image_lookup() {
    let (rc, out, log) = root_run(&with(|c| c.data_volume_gb = Some(100)), |_| {});
    assert_eq!(rc, 0, "{out}");
    assert!(
        !log.contains("describe-images") && !log.contains("get-parameter"),
        "{log}"
    );
}

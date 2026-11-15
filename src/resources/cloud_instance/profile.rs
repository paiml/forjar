//! `cloud_instance_profile`: optional IAM instance profile NAME for `cloud_instance`.
//! Only 1..=128 characters of `[A-Za-z0-9+.@_-]` are accepted, so no shell text can pass.
//! IAM also allows `,` and `=`, but forjar refuses them: they are separators in the aws
//! shorthand `Name=<value>`, so `a,Arn=..` would change the meaning of the argument.
//! It is part of the instance key (a hash tail, only when declared), so changing it on a
//! live instance is refused by the owner/key check in `apply_script`.

use crate::core::shell_escape::sh_squote;
use crate::core::types::{Resource, ResourceType};

/// Validate the declared name: absent stays absent, empty or malformed is refused.
pub(super) fn checked(v: &Option<String>) -> Result<Option<String>, String> {
    let Some(s) = v else { return Ok(None) };
    let ok = (1..=128).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "+.@_-".contains(c));
    if ok {
        Ok(Some(s.clone()))
    } else {
        Err(
            "cloud_instance_profile must be an IAM instance profile name: 1 to 128 characters of \
             [A-Za-z0-9+.@_-] (a name, not an ARN)"
                .into(),
        )
    }
}

/// Parse-time: the field is only meaningful on a `cloud_instance`.
#[must_use]
pub fn misplaced(r: &Resource) -> Option<String> {
    let wrong = r.resource_type != ResourceType::CloudInstance;
    (wrong && r.cloud.instance_profile.is_some())
        .then(|| "cloud_instance_profile is only valid on a cloud_instance resource".into())
}

/// `--iam-instance-profile` flag followed by a space; empty when unset.
pub(super) fn args(p: &Option<String>) -> String {
    p.as_ref().map_or_else(String::new, |n| {
        format!(
            "--iam-instance-profile {} ",
            sh_squote(&format!("Name={n}"))
        )
    })
}

/// Suffix of the instance key: empty when unset (key unchanged byte for byte).
pub(super) fn key_tail(p: &Option<String>) -> String {
    p.as_ref().map_or_else(String::new, |n| {
        let h = crate::tripwire::hasher::hash_string(&format!("instance-profile={n}"));
        format!("@i{}", &h["blake3:".len()..][..12])
    })
}

#[cfg(test)]
mod tests {
    use super::super::tests::{gated, res, Sandbox};
    use super::super::tests_b1::count;
    use super::super::*;
    use super::{checked, misplaced};
    use crate::core::types::ResourceType;

    fn with(dir: &std::path::Path, p: &str) -> Resource {
        let mut r = gated(dir);
        r.cloud.instance_profile = Some(p.into());
        r
    }

    #[test]
    fn only_a_valid_profile_name_validates() {
        let long_ok = "a".repeat(128);
        for ok in ["ci-box", "A", "a+b.c@d_e-f", long_ok.as_str()] {
            assert_eq!(checked(&Some(ok.into())), Ok(Some(ok.into())), "{ok}");
        }
        assert_eq!(checked(&None), Ok(None));
        let too_long = "a".repeat(129);
        for bad in [
            "",
            " ",
            "a b",
            "a,Arn=x",
            "a=b",
            "a,b",
            " ci",
            "ci ",
            "ci\n",
            "a'b",
            "a\"b",
            "a$b",
            "$(id)",
            "a`id`",
            "a;b",
            "a|b",
            "a&b",
            "a/b",
            "/ci",
            "a\\b",
            "a{b}",
            "arn:aws:iam::123456789012:instance-profile/ci",
            "caf\u{e9}",
            "a\u{7}",
            too_long.as_str(),
        ] {
            assert!(checked(&Some(bad.into())).is_err(), "{bad:?}");
            let mut r = res();
            r.cloud.instance_profile = Some(bad.into());
            assert!(spec_of(&r).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn profile_on_another_resource_type_is_refused() {
        let mut r = Resource {
            resource_type: ResourceType::File,
            ..Default::default()
        };
        assert_eq!(misplaced(&r), None);
        r.cloud.instance_profile = Some("ci-box".into());
        assert!(misplaced(&r).is_some());
        r.resource_type = ResourceType::CloudInstance;
        assert_eq!(misplaced(&r), None);
    }

    #[test]
    fn parse_refuses_a_profile_on_a_file_and_a_bad_name_on_an_instance() {
        let cfg = |ty: &str, prof: &str| {
            format!(
                "version: '1.0'\nname: t\nmachines:\n  m: {{hostname: m, addr: 1.2.3.4}}\nresources:\n  r:\n    type: {ty}\n    machine: m\n    path: /tmp/x\n    name: box\n    provider: aws\n    image: img@sha256:{}\n    cloud_region: eu-south-2\n    cloud_instance_type: c7a.xlarge\n    cloud_max_price: \"0.2\"\n    cloud_max_lifetime_min: 60\n    cloud_instance_profile: \"{prof}\"\n",
                "0123456789abcdef".repeat(4)
            )
        };
        let errs = |y: String| {
            let c = crate::core::parser::parse_config(&y).unwrap();
            crate::core::parser::validate_config(&c)
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join("|")
        };
        assert!(errs(cfg("file", "ci-box")).contains("cloud_instance_profile"));
        assert!(!errs(cfg("file", "ci-box")).contains("(cloud_instance)"));
        assert!(errs(cfg("cloud_instance", "a;b")).contains("cloud_instance_profile"));
    }

    #[test]
    fn apply_passes_the_profile_once_quoted() {
        let sb = Sandbox::new();
        sb.open_gate();
        let r = with(sb.path(), "ci-box");
        let (rc, out) = sb.run(&apply_script(&r), "");
        assert_eq!(rc, 0, "{out}");
        let log = sb.log();
        assert_eq!(count(&log, "run-instances"), 1, "{log}");
        assert_eq!(
            count(&log, "--iam-instance-profile Name=ci-box"),
            1,
            "{log}"
        );
        assert!(apply_script(&r).contains("--iam-instance-profile 'Name=ci-box' "));
    }

    #[test]
    fn without_a_profile_the_script_is_unchanged() {
        let sb = Sandbox::new();
        let s = apply_script(&gated(sb.path()));
        assert!(!s.contains("iam-instance-profile"), "{s}");
        assert!(s.contains("Value=ci-box-a@eu-south-2@0123456789ab}"), "{s}");
        // with a profile, only the flag and the key tail differ from the unset script
        let flag = "--iam-instance-profile 'Name=ci-box' ";
        let tail = super::key_tail(&Some("ci-box".into()));
        let w = apply_script(&with(sb.path(), "ci-box"));
        assert_eq!(w.replace(flag, "").replace(&tail, ""), s);
    }

    #[test]
    fn hostile_value_never_reaches_a_script() {
        let sb = Sandbox::new();
        for bad in [
            "x;touch pwned",
            "$(touch pwned)",
            "x`touch pwned`",
            "x' ;touch pwned #",
            "a,Arn=pwned",
        ] {
            let s = apply_script(&with(sb.path(), bad));
            assert!(!s.contains("pwned"), "{bad}: {s}");
            assert!(!s.contains("run-instances"), "{bad}: {s}");
            let (rc, _) = sb.run(&s, "");
            assert_ne!(rc, 0, "{bad}");
            assert_eq!(sb.log(), "", "{bad}");
        }
    }

    #[test]
    fn key_unchanged_without_a_profile_and_differs_with_one() {
        let base = key_of(&spec_of(&res()).unwrap());
        assert!(!base.contains("@i"), "{base}");
        let mut a = res();
        a.cloud.instance_profile = Some("ci-a".into());
        let mut b = res();
        b.cloud.instance_profile = Some("ci-b".into());
        let (ka, kb) = (key_of(&spec_of(&a).unwrap()), key_of(&spec_of(&b).unwrap()));
        assert!(ka.starts_with(&base) && ka != base, "{ka}");
        assert_ne!(ka, kb);
        assert_eq!(ka, key_of(&spec_of(&a).unwrap()));
        assert_eq!(
            owner_of(&spec_of(&a).unwrap()),
            owner_of(&spec_of(&res()).unwrap())
        );
    }

    #[test]
    fn changing_the_profile_on_a_live_instance_refuses_and_never_launches() {
        let sb = Sandbox::new();
        sb.open_gate();
        let old = key_of(&spec_of(&with(sb.path(), "ci-a")).unwrap());
        let fx = format!("i-00a1b2c3 {old} ci-box-a@eu-south-2 sir-0own\n");
        let new = apply_script(&with(sb.path(), "ci-b"));
        let (rc, out) = sb.run_with(&new, "", Some(&fx));
        assert_ne!(rc, 0, "{out}");
        assert!(out.contains("REFUSED"), "{out}");
        assert_eq!(count(&sb.log(), "run-instances"), 0, "{}", sb.log());
        let same = apply_script(&with(sb.path(), "ci-a"));
        let (rc, out) = sb.run_with(&same, "", Some(&fx));
        assert_eq!(rc, 0, "{out}");
        assert!(out.contains("present"), "{out}");
        assert_eq!(count(&sb.log(), "run-instances"), 0, "{}", sb.log());
    }
}

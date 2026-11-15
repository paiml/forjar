//! `cloud_ami`: optional machine-image pin for `cloud_instance`.
//! Only a whole AMI id is accepted (`ami-` + 8 or 17 lowercase hex); it is part
//! of the instance key, so changing it on a live instance is refused by the
//! owner/key check in `apply_script` and never launches.

const DEFAULT_IMAGE: &str =
    "resolve:ssm:/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-x86_64";

/// Validate the declared pin: absent stays absent, empty or malformed is refused.
pub(super) fn checked(v: &Option<String>) -> Result<Option<String>, String> {
    let Some(s) = v else { return Ok(None) };
    let hex = s.strip_prefix("ami-").unwrap_or("");
    let ok = matches!(hex.len(), 8 | 17) && hex.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'));
    if ok {
        Ok(Some(s.clone()))
    } else {
        Err(
            "cloud_ami must be one whole AMI id: ami- followed by 8 or 17 lowercase hex digits"
                .into(),
        )
    }
}

/// The `--image-id` value: the pin, or the default image when none is declared.
pub(super) fn image_id(ami: &Option<String>) -> String {
    ami.clone().unwrap_or_else(|| DEFAULT_IMAGE.to_string())
}

/// Suffix of the instance key: empty without a pin (key unchanged byte for byte).
pub(super) fn key_tail(ami: &Option<String>) -> String {
    ami.as_ref().map_or_else(String::new, |a| format!("@{a}"))
}

#[cfg(test)]
mod tests {
    use super::super::tests::{gated, Sandbox};
    use super::super::tests_b1::count;
    use super::super::*;
    use super::{checked, image_id, key_tail};

    fn pinned(dir: &std::path::Path, ami: &str) -> Resource {
        let mut r = gated(dir);
        r.cloud.ami = Some(ami.into());
        r
    }

    #[test]
    fn only_a_whole_ami_id_validates() {
        for ok in ["ami-0123abcd", "ami-0123456789abcdef0"] {
            assert_eq!(checked(&Some(ok.into())), Ok(Some(ok.into())), "{ok}");
        }
        assert_eq!(checked(&None), Ok(None));
        for bad in [
            "",
            "ami-",
            "ami-0123abc",
            "ami-0123abcde",
            "ami-0123456789abcdef",
            "AMI-0123abcd",
            "ami-0123ABCD",
            "ami-0123abcd ",
            " ami-0123abcd",
            "ami-0123abcd\n",
            "ami-0123abcd'",
            "\"ami-0123abcd\"",
            "ami-0123abcd;x",
            "ami-0123abcg",
            "xami-0123abcd",
            "resolve:ssm:/aws/service/ami-amazon-linux-latest/al2023",
            "ami-0123abcd\u{7}",
        ] {
            assert!(checked(&Some(bad.into())).is_err(), "{bad:?}");
            let mut r = super::super::tests::res();
            r.cloud.ami = Some(bad.into());
            assert!(spec_of(&r).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn apply_launches_the_pinned_ami_once_quoted() {
        let sb = Sandbox::new();
        sb.open_gate();
        let (rc, out) = sb.run(&apply_script(&pinned(sb.path(), "ami-0123abcd")), "");
        assert_eq!(rc, 0, "{out}");
        let log = sb.log();
        assert_eq!(count(&log, "run-instances"), 1, "{log}");
        assert_eq!(count(&log, "--image-id ami-0123abcd"), 1, "{log}");
        assert!(!log.contains("resolve:ssm"), "{log}");
        assert!(
            apply_script(&pinned(sb.path(), "ami-0123abcd")).contains("--image-id 'ami-0123abcd' ")
        );
    }

    #[test]
    fn without_a_pin_the_default_image_and_key_are_unchanged() {
        let sb = Sandbox::new();
        let s = apply_script(&gated(sb.path()));
        assert!(
            s.contains(&format!("--image-id '{}' ", super::DEFAULT_IMAGE)),
            "{s}"
        );
        assert!(s.contains("Value=ci-box-a@eu-south-2@0123456789ab}"), "{s}");
        assert!(!s.contains("ami-0"), "{s}");
    }

    #[test]
    fn changing_the_ami_on_a_live_instance_refuses_and_never_launches() {
        let sb = Sandbox::new();
        sb.open_gate();
        let old = key_of(&spec_of(&pinned(sb.path(), "ami-0123abcd")).unwrap());
        let fx = format!("i-00a1b2c3 {old} ci-box-a@eu-south-2 sir-0own\n");
        let new = apply_script(&pinned(sb.path(), "ami-0fedcba9"));
        let (rc, out) = sb.run_with(&new, "", Some(&fx));
        assert_ne!(rc, 0, "{out}");
        assert!(out.contains("REFUSED"), "{out}");
        assert_eq!(count(&sb.log(), "run-instances"), 0, "{}", sb.log());
        // same AMI: present, zero launches
        let same = apply_script(&pinned(sb.path(), "ami-0123abcd"));
        let (rc, out) = sb.run_with(&same, "", Some(&fx));
        assert_eq!(rc, 0, "{out}");
        assert!(out.contains("present"), "{out}");
        assert_eq!(count(&sb.log(), "run-instances"), 0, "{}", sb.log());
    }
}

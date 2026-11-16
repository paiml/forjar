//! `cloud_user_data_file`: optional local FILE passed as the instance's user-data (#1131-#1133).
//!
//! Only the PATH is ever used. forjar never reads the content, so it cannot reach the plan,
//! the instance key, a generated script, a log or state: the fleet's user-data carries presigned
//! URLs, which are secrets. The path is NOT part of the instance key: a per-launch temp path must
//! not turn a live box into a "changed" refusal, and keys without the field stay byte for byte.
//! The path is checked against a strict allow-list at parse time and again at codegen (both
//! through `spec_of`), then single-quoted at use. The size and presence guard runs in the apply
//! script before any `aws` call; a file that is gone after launch therefore blocks a re-apply
//! of an already present box too (the wrapper keeps the file or drops the field).
//!
//! A declared file REPLACES the built-in script, which carries the instance's only lifetime
//! bound (`shutdown -h +N`) and delivers `FORJAR_IMAGE`. Fail closed: the apply guard also
//! refuses unless line 1 of the file starts with `#!` and line 2 is exactly
//! `shutdown -h +<max_lifetime_min>` (read with `sed`, compared whole, never printed), so the
//! shutdown is scheduled as the file's first command, before any workload runs. The tool CHECKS
//! that; it cannot guarantee the file's later commands do not cancel it (`shutdown -c`).
//! `image:` stays required, so it cannot
//! be refused alongside the file: it remains in the key and the plan, but is NOT delivered to
//! the box; the file must set up its own workload. A changed file CONTENT or PATH is not
//! detected: no drift, no plan line, and a present instance is never relaunched for it.
//! `[ -f ]` follows symlinks (a link to a regular file passes; the declarant picks the path),
//! and AWS CLI v2 reads `file://` as text, so gzip or other non-UTF-8 user-data is not supported.

/// The line the declared file must contain: the same bound the built-in script sets.
pub(super) fn required_line(life_min: u32) -> String {
    format!("shutdown -h +{life_min}")
}

use crate::core::shell_escape::sh_squote;

/// EC2 takes 16 KB raw; the fleet's own limit is 16000.
const MAX_BYTES: usize = 16000;
const MAX_PATH: usize = 255;

/// Validate the declared path: absent stays absent; anything outside the allow-list is refused.
pub(super) fn checked(v: &Option<String>) -> Result<Option<String>, String> {
    let Some(p) = v else { return Ok(None) };
    let ok = p.len() <= MAX_PATH
        && p.starts_with('/')
        && p.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-'))
        && p[1..]
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..");
    if ok {
        Ok(Some(p.clone()))
    } else {
        Err(
            "cloud_user_data_file must be an absolute path of at most 255 characters of \
             [A-Za-z0-9._/-] with no empty, . or .. component"
                .into(),
        )
    }
}

/// The `--user-data` value for a declared file: the path by reference, read by `aws` at apply time.
pub(super) fn file_arg(path: &str) -> String {
    sh_squote(&format!("file://{path}"))
}

/// Apply-script lines refusing (before any provider call) a file that is missing, not a regular
/// file, unreadable, empty or over the limit. Empty when no file is declared.
pub(super) fn guard(path: Option<&str>, life_min: u32) -> String {
    let Some(p) = path else { return String::new() };
    let refuse = |why: &str| format!("echo 'REFUSED: cloud_user_data_file {why}' >&2; exit 1");
    format!(
        "ud_file={p}\n\
         if [ ! -f \"$ud_file\" ]; then {missing}; fi\n\
         ud_n=\"$(wc -c < \"$ud_file\")\" || {{ {unread}; }}\n\
         ud_n=\"${{ud_n##* }}\"\n\
         case \"$ud_n\" in ''|*[!0-9]*) {unread} ;; esac\n\
         if [ \"$ud_n\" -eq 0 ]; then {empty}; fi\n\
         if [ \"$ud_n\" -gt {MAX_BYTES} ]; then {big}; fi\n\
         ud_l1=\"$(sed -n 1p \"$ud_file\" 2>/dev/null)\" || {{ {unread}; }}\n\
         ud_l2=\"$(sed -n 2p \"$ud_file\" 2>/dev/null)\" || {{ {unread}; }}\n\
         case \"$ud_l1\" in '#!'*) ;; *) {noshebang} ;; esac\n\
         if [ \"$ud_l2\" != {line} ]; then {noline}; fi\n",
        p = sh_squote(p),
        missing = refuse("is missing or not a regular file"),
        unread = refuse("could not be read"),
        empty = refuse("is empty"),
        big = refuse(&format!("is larger than {MAX_BYTES} bytes")),
        line = sh_squote(&required_line(life_min)),
        noshebang = refuse("must start with #! on line 1"),
        noline = refuse(&format!(
            "must have exactly [{}] as line 2 (the lifetime bound, first command)",
            required_line(life_min)
        )),
    )
}

#[cfg(test)]
mod tests {
    use super::super::tests::{gated, res, Sandbox, DIGEST};
    use super::super::tests_b1::count;
    use super::super::*;
    use super::checked;

    const MARKER: &str = "PRESIGNED-SECRET-MARKER-7f3a";
    const REQ: &str = "shutdown -h +60";

    fn with_file(dir: &std::path::Path, body: &[u8]) -> (Resource, String) {
        let p = dir.join("ud.sh");
        let full = [format!("#!/bin/sh\n{REQ}\n").as_bytes(), body].concat();
        std::fs::write(&p, full).unwrap();
        let path = p.to_str().unwrap().to_string();
        let mut r = gated(dir);
        r.cloud.user_data_file = Some(path.clone());
        (r, path)
    }

    #[test]
    fn only_a_strict_absolute_path_validates() {
        for ok in ["/a", "/tmp/x/ud.sh", "/mnt/nvme-raid0/tmp/ud-1_2.txt"] {
            assert_eq!(checked(&Some(ok.into())), Ok(Some(ok.into())), "{ok}");
        }
        assert_eq!(checked(&None), Ok(None));
        let long = format!("/{}", "a".repeat(255));
        for bad in [
            "",
            "/",
            "ud.sh",
            "./ud.sh",
            "~/ud.sh",
            "/a/../b",
            "/..",
            "/a/./b",
            "/a//b",
            "/a/",
            "/a b",
            "/a'b",
            "/a\"b",
            "/a;b",
            "/a$b",
            "/a`b",
            "/a\nb",
            "/a\u{e9}",
            "/{{params.f}}",
            "file:///a",
            long.as_str(),
        ] {
            assert!(checked(&Some(bad.into())).is_err(), "{bad:?}");
            let mut r = res();
            r.cloud.user_data_file = Some(bad.into());
            assert!(spec_of(&r).is_err(), "{bad:?}");
            let s = apply_script(&r);
            assert!(s.contains("REFUSED") && !s.contains("aws ec2"), "{bad:?}");
        }
        assert!(checked(&Some(format!("/{}", "a".repeat(254)))).is_ok());
    }

    #[test]
    fn user_data_file_passed_by_path_not_inlined() {
        let sb = Sandbox::new();
        sb.open_gate();
        let (r, path) = with_file(sb.path(), MARKER.as_bytes());
        let script = apply_script(&r);
        assert!(
            script.contains(&format!("--user-data 'file://{path}' ")),
            "{script}"
        );
        assert!(!script.contains("#!/bin/sh\nshutdown"), "{script}");
        let (rc, out) = sb.run(&script, "");
        assert_eq!(rc, 0, "{out}");
        let log = sb.log();
        assert_eq!(count(&log, "run-instances"), 1, "{log}");
        assert_eq!(
            count(&log, &format!("--user-data file://{path} ")),
            1,
            "{log}"
        );
    }

    #[test]
    fn unset_script_is_byte_identical_to_today() {
        let sb = Sandbox::new();
        let got = apply_script(&gated(sb.path()));
        let tok = super::super::launch_io::token(&spec_of(&gated(sb.path())).unwrap());
        let want = include_str!("userdata_today.txt")
            .replace("@DIR@", sb.path().to_str().unwrap())
            .replace("@TOKEN@", &format!("'{tok}'"));
        assert_eq!(got, want);
    }

    #[test]
    fn user_data_file_size_and_presence_refusal() {
        let sb = Sandbox::new();
        sb.open_gate();
        let (r, _) = with_file(sb.path(), b"x");
        let script = apply_script(&r);
        let ud = sb.path().join("ud.sh");
        let cases = ["missing", "directory", "empty", "16001 bytes"];
        for what in cases {
            let _ = std::fs::remove_file(&ud);
            let _ = std::fs::remove_dir(&ud);
            match what {
                "directory" => std::fs::create_dir(&ud).unwrap(),
                "empty" => std::fs::write(&ud, b"").unwrap(),
                "16001 bytes" => std::fs::write(&ud, vec![b'a'; 16001]).unwrap(),
                _ => {}
            }
            let (rc, out) = sb.run(&script, "");
            assert_eq!(rc, 1, "{what}: {out}");
            let why = match what {
                "empty" => "is empty",
                "16001 bytes" => "is larger than 16000 bytes",
                _ => "is missing or not a regular file",
            };
            assert!(
                out.contains(&format!("REFUSED: cloud_user_data_file {why}")),
                "{what}: {out}"
            );
            assert_eq!(sb.log(), "", "{what}: no aws call before the refusal");
        }
        let mut exact = format!("#!/bin/sh\n{REQ}\n").into_bytes();
        exact.resize(16000, b'a');
        std::fs::write(&ud, exact).unwrap();
        let (rc, out) = sb.run(&script, "");
        assert_eq!(rc, 0, "16000 bytes is allowed: {out}");
        assert_eq!(count(&sb.log(), "run-instances"), 1, "{}", sb.log());
    }

    #[test]
    fn the_guard_sits_before_the_first_aws_call() {
        let sb = Sandbox::new();
        let (r, _) = with_file(sb.path(), b"");
        let script = apply_script(&r);
        let g = script.find("ud_l2=").expect("line-2 guard present");
        assert!(g < script.find("aws ").unwrap(), "{script}");
        assert!(script.contains("!= 'shutdown -h +60'"), "{script}");
    }

    #[test]
    fn the_shutdown_must_be_line_two_after_a_shebang() {
        let sb = Sandbox::new();
        sb.open_gate();
        let (r, _) = with_file(sb.path(), b"");
        let script = apply_script(&r);
        let ud = sb.path().join("ud.sh");
        let bad: [&str; 9] = [
            "#!/bin/sh\n/opt/train.sh\nshutdown -h +60\n",
            "#!/bin/sh\nset -e\nshutdown -h +60\n",
            "shutdown -h +60\necho hi\n",
            "echo hi\nshutdown -h +60\n",
            "#!/bin/sh\necho hi\n",
            "#!/bin/sh\nshutdown -h +61\n",
            "#!/bin/sh\nshutdown -h +600\n",
            "#!/bin/sh\n# shutdown -h +60\n",
            "#!/bin/sh\r\nshutdown -h +60\r\n",
        ];
        for body in bad {
            std::fs::write(&ud, format!("{body}{MARKER}\n")).unwrap();
            let (rc, out) = sb.run(&script, "");
            assert_eq!(rc, 1, "{body:?}: {out}");
            assert!(
                out.contains("REFUSED: cloud_user_data_file must "),
                "{body:?}: {out}"
            );
            assert!(!out.contains(MARKER), "{out}");
            assert_eq!(sb.log(), "", "{body:?}: no aws call");
        }
        std::fs::write(&ud, "no shebang\nshutdown -h +60\n").unwrap();
        let (rc, out) = sb.run(&script, "");
        assert_eq!(rc, 1, "{out}");
        assert!(out.contains("must start with #! on line 1"), "{out}");
        std::fs::write(&ud, "#!/bin/sh\nshutdown -h +60\n/opt/train.sh\n").unwrap();
        let (rc, out) = sb.run(&script, "");
        assert_eq!(rc, 0, "{out}");
        assert_eq!(count(&sb.log(), "run-instances"), 1, "{}", sb.log());
    }

    #[test]
    fn the_required_n_comes_from_the_resource() {
        let sb = Sandbox::new();
        sb.open_gate();
        let (mut r, _) = with_file(sb.path(), b"");
        r.cloud.max_lifetime_min = Some(45);
        let script = apply_script(&r);
        assert!(script.contains("!= 'shutdown -h +45'"), "{script}");
        let ud = sb.path().join("ud.sh");
        std::fs::write(&ud, "#!/bin/sh\nshutdown -h +60\n").unwrap();
        let (rc, out) = sb.run(&script, "");
        assert_eq!(rc, 1, "{out}");
        assert!(out.contains("[shutdown -h +45]"), "{out}");
        assert_eq!(sb.log(), "");
        std::fs::write(&ud, "#!/bin/sh\nshutdown -h +45\n").unwrap();
        let (rc, out) = sb.run(&script, "");
        assert_eq!(rc, 0, "{out}");
    }

    #[test]
    fn the_plan_shows_the_declared_path_and_never_the_content() {
        let sb = Sandbox::new();
        let (r, path) = with_file(sb.path(), MARKER.as_bytes());
        let plan = plan_diff(&r);
        assert!(plan.contains(&format!("user_data_file: {path} ")), "{plan}");
        assert!(
            plan.contains("shutdown -h +60") && !plan.contains(MARKER),
            "{plan}"
        );
        assert!(!plan_diff(&gated(sb.path())).contains("user_data_file"));
    }

    #[test]
    fn user_data_file_content_never_appears_in_any_output() {
        let sb = Sandbox::new();
        sb.open_gate();
        let body = format!("#!/bin/sh\ncurl 'https://b.s3.amazonaws.com/x?sig={MARKER}'\n");
        let (r, _) = with_file(sb.path(), body.as_bytes());
        let mut gone = r.clone();
        gone.state = Some("absent".into());
        let spec = spec_of(&r).unwrap();
        let (_, out) = sb.run(&apply_script(&r), "");
        let mut seen = vec![out, sb.log(), plan_diff(&r), check_script(&r)];
        seen.extend([
            state_query_script(&r),
            apply_script(&r),
            apply_script(&gone),
        ]);
        seen.extend([key_of(&spec), owner_of(&spec), format!("{spec:?}")]);
        for (i, s) in seen.iter().enumerate() {
            assert!(
                !s.contains(MARKER) && !s.contains("b.s3."),
                "output {i}: {s}"
            );
        }
    }

    #[test]
    fn the_path_is_not_part_of_the_instance_key() {
        let sb = Sandbox::new();
        let plain = key_of(&spec_of(&gated(sb.path())).unwrap());
        let (r, _) = with_file(sb.path(), b"x");
        assert_eq!(key_of(&spec_of(&r).unwrap()), plain);
        let (mut r2, _) = with_file(sb.path(), b"y");
        r2.cloud.user_data_file = Some("/tmp/other-ud.sh".into());
        assert_eq!(key_of(&spec_of(&r2).unwrap()), plain);
    }

    #[test]
    fn the_path_is_validated_at_parse_time_and_templates_are_refused() {
        let yaml = |f: &str| {
            format!(
                "version: \"1.0\"\nname: t\nmachines:\n  m: {{hostname: m, addr: 127.0.0.1}}\nresources:\n  a:\n    type: cloud_instance\n    machine: m\n    provider: aws\n    name: boxa\n    \
                 image: img@{DIGEST}\n    cloud_region: eu-south-2\n    cloud_instance_type: c7a.xlarge\n    \
                 cloud_max_price: \"0.2\"\n    cloud_max_lifetime_min: 60\n    cloud_user_data_file: \"{f}\"\n"
            )
        };
        let errs = |f: &str| {
            let c = crate::core::parser::parse_config(&yaml(f)).unwrap();
            crate::core::parser::validate_config(&c)
        };
        assert!(errs("/tmp/ud.sh").is_empty());
        for bad in ["rel/ud.sh", "/a/../b", "/a b", "/{{params.f}}"] {
            assert!(!errs(bad).is_empty(), "{bad}");
        }
    }
}

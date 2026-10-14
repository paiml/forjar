//! I8 for install seeds: every shell fragment a seed carries goes through the
//! same bashrs gate as every script forjar ships.
//!
//! A seed is shell in disguise. `early-commands` run in the installer before
//! the disk is touched, `late-commands` run as root against the new system, and
//! a `write_files` entry with a shebang is a script the first boot executes. A
//! hand-written seed reached real hardware with none of it ever linted.
//!
//! Extracted, in `user-data`:
//!   - `autoinstall.{early,late,error}-commands`
//!   - `runcmd` / `bootcmd`, at the top level and under `autoinstall.user-data`
//!   - `write_files[].content` that starts with `#!`, in the same two places
//!
//! and every OTHER seed file whose first line is a shebang (`install.sh`).
//! A command in argv form is linted only when it is `sh|bash -c <script>`;
//! any other argv list is not shell and is not parsed as shell.

use serde_yaml_ng::Value;

/// One extracted fragment: where it came from, and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fragment {
    /// e.g. `user-data: autoinstall.early-commands[2]`.
    pub origin: String,
    /// The shell text.
    pub script: String,
}

fn key<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    v.as_mapping()
        .and_then(|m| m.get(Value::String(k.to_string())))
}

fn command_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Sequence(argv) => {
            let words: Vec<&str> = argv.iter().filter_map(Value::as_str).collect();
            match words.as_slice() {
                [sh, "-c", script, ..]
                    if matches!(*sh, "sh" | "bash" | "/bin/sh" | "/bin/bash") =>
                {
                    Some((*script).to_string())
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn commands(out: &mut Vec<Fragment>, parent: &Value, path: &str, list: &str) {
    let Some(Value::Sequence(items)) = key(parent, list) else {
        return;
    };
    for (i, item) in items.iter().enumerate() {
        if let Some(script) = command_text(item) {
            out.push(Fragment {
                origin: format!("user-data: {path}{list}[{i}]"),
                script,
            });
        }
    }
}

fn write_files(out: &mut Vec<Fragment>, parent: &Value, path: &str) {
    let Some(Value::Sequence(items)) = key(parent, "write_files") else {
        return;
    };
    for (i, item) in items.iter().enumerate() {
        let encoded = key(item, "encoding").and_then(Value::as_str).is_some();
        if let Some(content) = key(item, "content").and_then(Value::as_str) {
            if !encoded && content.starts_with("#!") {
                out.push(Fragment {
                    origin: format!("user-data: {path}write_files[{i}]"),
                    script: content.to_string(),
                });
            }
        }
    }
}

fn cloud_config(out: &mut Vec<Fragment>, v: &Value, path: &str) {
    commands(out, v, path, "bootcmd");
    commands(out, v, path, "runcmd");
    write_files(out, v, path);
}

/// Every shell fragment in a seed.
///
/// # Errors
///
/// Returns `Err` when `user-data` is not YAML: a seed the installer cannot
/// parse is refused, not skipped.
pub fn fragments(seed: &[(String, String)]) -> Result<Vec<Fragment>, String> {
    let mut out = Vec::new();
    for (name, content) in seed {
        if name == "user-data" {
            let v: Value = serde_yaml_ng::from_str(content)
                .map_err(|e| format!("media_seed user-data is not valid YAML: {e}"))?;
            cloud_config(&mut out, &v, "");
            if let Some(ai) = key(&v, "autoinstall") {
                for list in ["early-commands", "late-commands", "error-commands"] {
                    commands(&mut out, ai, "autoinstall.", list);
                }
                if let Some(ud) = key(ai, "user-data") {
                    cloud_config(&mut out, ud, "autoinstall.user-data.");
                }
            }
        } else if content.starts_with("#!") {
            out.push(Fragment {
                origin: name.clone(),
                script: content.clone(),
            });
        }
    }
    Ok(out)
}

/// Lint every fragment with the production gate.
///
/// # Errors
///
/// Returns every failing fragment, each named by its origin.
pub fn lint(seed: &[(String, String)]) -> Result<usize, String> {
    let frags = fragments(seed)?;
    let mut errs = Vec::new();
    for f in &frags {
        let script = if f.script.starts_with("#!") {
            f.script.clone()
        } else {
            format!("#!/bin/sh\n{}\n", f.script)
        };
        if let Err(e) = crate::core::purifier::validate_script(&script) {
            errs.push(format!("{}: {e}", f.origin));
        }
    }
    if errs.is_empty() {
        Ok(frags.len())
    } else {
        Err(format!(
            "media_seed shell refused by bashrs (I8):\n{}",
            errs.join("\n")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(user_data: &str) -> Vec<(String, String)> {
        vec![
            ("user-data".to_string(), user_data.to_string()),
            ("meta-data".to_string(), "instance-id: t\n".to_string()),
        ]
    }

    #[test]
    fn every_documented_location_is_extracted() {
        let ud = "#cloud-config\nautoinstall:\n  version: 1\n  early-commands:\n    - echo early\n    - [sh, -c, 'echo argv']\n    - [ip, link]\n  late-commands:\n    - echo late\n  error-commands:\n    - echo err\n  user-data:\n    runcmd:\n      - echo run\n    write_files:\n      - path: /x.sh\n        content: |\n          #!/bin/sh\n          echo wf\n      - path: /y\n        content: plain\n";
        let f = fragments(&seed(ud)).unwrap();
        let origins: Vec<&str> = f.iter().map(|x| x.origin.as_str()).collect();
        assert_eq!(
            origins,
            [
                "user-data: autoinstall.early-commands[0]",
                "user-data: autoinstall.early-commands[1]",
                "user-data: autoinstall.late-commands[0]",
                "user-data: autoinstall.error-commands[0]",
                "user-data: autoinstall.user-data.runcmd[0]",
                "user-data: autoinstall.user-data.write_files[0]",
            ]
        );
    }

    #[test]
    fn a_shebang_extra_file_is_linted_and_a_data_file_is_not() {
        let mut s = seed("#cloud-config\n");
        s.push(("install.sh".to_string(), "#!/bin/sh\necho ok\n".to_string()));
        s.push(("notes.txt".to_string(), "curl x | sh\n".to_string()));
        let f = fragments(&s).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].origin, "install.sh");
    }

    #[test]
    fn user_data_that_is_not_yaml_is_refused() {
        assert!(fragments(&seed("a: [unclosed\n")).is_err());
    }
}

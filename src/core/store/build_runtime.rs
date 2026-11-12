//! PMAT-989: the container-runtime operations an executed image layer needs.
//!
//! `BuildRuntime` is the whole surface `exec_layer` uses, so the capture logic is
//! unit-tested with a fake. `ContainerCli` is the one real implementation: it
//! shells out to `docker` or `podman` (see `detect_container_runtime`).

use std::path::Path;
use std::process::{Command, Stdio};

/// One line of `<runtime> diff`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffKind {
    /// Path added in the container.
    Added,
    /// Path changed in the container.
    Changed,
    /// Path deleted from the base.
    Deleted,
}

/// A changed path as the runtime reports it, with a leading `/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEntry {
    /// A, C or D.
    pub kind: DiffKind,
    /// Absolute path inside the container.
    pub path: String,
}

/// Outcome of one step run inside the container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecOutcome {
    /// Exit status of the step.
    pub code: i32,
    /// Last lines of what the step wrote to stderr (stdout when stderr was empty).
    pub tail: String,
}

/// What an executed layer asks of a container runtime.
pub trait BuildRuntime {
    /// Name used in messages (`docker`, `podman`).
    fn name(&self) -> String;
    /// `diff_ids` of the runtime's local copy of `image`, in order; `Err` when absent.
    fn image_diff_ids(&self, image: &str) -> Result<Vec<String>, String>;
    /// Start a container from `image` that just sleeps; never pulls. Returns its id.
    fn start(&self, image: &str) -> Result<String, String>;
    /// Run `script` with `<shell> -c` in the container.
    fn exec(&self, id: &str, shell: &str, script: &str) -> Result<ExecOutcome, String>;
    /// Filesystem changes against the image.
    fn diff(&self, id: &str) -> Result<Vec<DiffEntry>, String>;
    /// Write the container's full root filesystem as a tar to `dest`.
    fn export(&self, id: &str, dest: &Path) -> Result<(), String>;
    /// Force-remove the container. Best effort: called from a drop guard.
    fn remove(&self, id: &str);
}

/// `docker` / `podman` driven through their command line.
pub struct ContainerCli {
    runtime: String,
}

impl ContainerCli {
    /// Use whichever of docker / podman is installed.
    pub fn detect() -> Result<Self, String> {
        super::convergence_container::detect_container_runtime()
            .map(|runtime| Self { runtime })
            .ok_or_else(|| {
                "no container runtime (docker or podman) is available; a build with \
                 `packages:` or `command:` executes them in a container"
                    .to_string()
            })
    }

    fn run(&self, args: &[&str]) -> Result<std::process::Output, String> {
        Command::new(&self.runtime)
            .args(args)
            .output()
            .map_err(|e| format!("{} {}: {e}", self.runtime, args.first().unwrap_or(&"")))
    }
}

/// The last `n` lines of `text`, trimmed.
pub(super) fn last_lines(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.trim_end().lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// Parse `<runtime> diff` output (`A /path`, `C /path`, `D /path`).
pub(super) fn parse_diff(text: &str) -> Result<Vec<DiffEntry>, String> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let (kind, path) = line
                .split_once(' ')
                .ok_or_else(|| format!("unparseable diff line: {line:?}"))?;
            let kind = match kind {
                "A" => DiffKind::Added,
                "C" => DiffKind::Changed,
                "D" => DiffKind::Deleted,
                other => return Err(format!("unknown diff kind {other:?} in line {line:?}")),
            };
            Ok(DiffEntry {
                kind,
                path: path.to_string(),
            })
        })
        .collect()
}

/// Arguments of the `run` that starts the build container: never pulls.
pub(crate) fn start_args(image: &str) -> [&str; 7] {
    [
        "run",
        "-d",
        "--pull=never",
        "--entrypoint",
        "sleep",
        image,
        "7200",
    ]
}

impl BuildRuntime for ContainerCli {
    fn name(&self) -> String {
        self.runtime.clone()
    }

    fn image_diff_ids(&self, image: &str) -> Result<Vec<String>, String> {
        let out = self.run(&[
            "image",
            "inspect",
            "--format",
            "{{json .RootFS.Layers}}",
            image,
        ])?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
        serde_json::from_slice(&out.stdout).map_err(|e| {
            format!(
                "{} image inspect {image}: bad RootFS.Layers: {e}",
                self.runtime
            )
        })
    }

    fn start(&self, image: &str) -> Result<String, String> {
        let out = self.run(&start_args(image))?;
        if !out.status.success() {
            return Err(format!(
                "{} run {image}: {}",
                self.runtime,
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn exec(&self, id: &str, shell: &str, script: &str) -> Result<ExecOutcome, String> {
        let out = Command::new(&self.runtime)
            .args(["exec", id, shell, "-c", script])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("{} exec: {e}", self.runtime))?;
        let stderr = String::from_utf8_lossy(&out.stderr);
        let text = if stderr.trim().is_empty() {
            String::from_utf8_lossy(&out.stdout).into_owned()
        } else {
            stderr.into_owned()
        };
        Ok(ExecOutcome {
            code: out.status.code().unwrap_or(-1),
            tail: last_lines(&text, 20),
        })
    }

    fn diff(&self, id: &str) -> Result<Vec<DiffEntry>, String> {
        let out = self.run(&["diff", id])?;
        if !out.status.success() {
            return Err(format!(
                "{} diff failed: {}",
                self.runtime,
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        parse_diff(&String::from_utf8_lossy(&out.stdout))
    }

    fn export(&self, id: &str, dest: &Path) -> Result<(), String> {
        let dest = dest.display().to_string();
        let out = self.run(&["export", "-o", &dest, id])?;
        if out.status.success() {
            return Ok(());
        }
        Err(format!(
            "{} export failed: {}",
            self.runtime,
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }

    fn remove(&self, id: &str) {
        let _ = Command::new(&self.runtime)
            .args(["rm", "-f", id])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_never_pulls_and_runs_the_given_image() {
        let a = start_args("img:1");
        assert!(a.contains(&"--pull=never"));
        assert_eq!(a[5], "img:1");
        assert_eq!(a[0], "run");
    }

    #[test]
    fn parse_diff_reads_all_three_kinds() {
        let got = parse_diff("A /usr/bin/jq\nC /etc\nD /var/old file\n\n").unwrap();
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].kind, DiffKind::Added);
        assert_eq!(got[1].kind, DiffKind::Changed);
        assert_eq!(got[2].kind, DiffKind::Deleted);
        assert_eq!(got[2].path, "/var/old file");
    }

    #[test]
    fn parse_diff_refuses_what_it_cannot_read() {
        assert!(parse_diff("X /a").is_err());
        assert!(parse_diff("garbage").is_err());
    }

    #[test]
    fn last_lines_keeps_the_tail() {
        assert_eq!(last_lines("a\nb\nc\nd\n", 2), "c\nd");
        assert_eq!(last_lines("", 3), "");
    }
}

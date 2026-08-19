//! Refs #211: where `forjar dist` writes, resolved once for every generator.
//!
//! Split out of `dist.rs` to keep that file under the 500-line gate.

use std::path::Path;

/// Where `forjar dist` writes, resolved once for every generator.
///
/// `--help` says `-o` is an "Output file (for single artifact) or directory
/// (with --all)". Refs #211 makes that true of both halves:
///
/// * exactly one artifact selected → `-o` is that artifact's path;
/// * more than one (typically `--all`) → `-o` is the directory, i.e. an alias
///   for `--output-dir`, instead of being used as the installer's file name
///   while everything else silently fell back to `./dist`.
#[derive(Debug)]
pub(crate) enum DistOutput {
    /// One artifact, written to exactly this path.
    Single(std::path::PathBuf),
    /// Several artifacts, written under this directory.
    Dir(std::path::PathBuf),
}

impl DistOutput {
    /// Base directory for artifacts that were not given an explicit path.
    pub(crate) fn dir(&self) -> &Path {
        match self {
            Self::Single(_) => Path::new("dist"),
            Self::Dir(d) => d,
        }
    }

    /// The explicit single-artifact path, if one was requested.
    pub(crate) fn single_file(&self) -> Option<&Path> {
        match self {
            Self::Single(p) => Some(p),
            Self::Dir(_) => None,
        }
    }
}

/// Where one artifact goes: the explicit `-o` path, else `<out_dir>/<name>`.
pub(crate) fn artifact_path(
    single: Option<&Path>,
    out_dir: &Path,
    default_name: &str,
) -> std::path::PathBuf {
    single
        .map(Path::to_path_buf)
        .unwrap_or_else(|| out_dir.join(default_name))
}

/// Resolve `-o` / `--output-dir` against the set of selected artifacts.
pub(crate) fn resolve_dist_output(
    output: Option<&Path>,
    output_dir: Option<&Path>,
    selected: &[bool],
) -> Result<DistOutput, String> {
    let count = selected.iter().filter(|s| **s).count();
    match (output, output_dir) {
        (Some(o), Some(d)) => Err(format!(
            "--output {} and --output-dir {} both given — pass one",
            o.display(),
            d.display()
        )),
        (None, Some(d)) => Ok(DistOutput::Dir(d.to_path_buf())),
        (Some(o), None) if count == 1 => Ok(DistOutput::Single(o.to_path_buf())),
        // More than one artifact and a single -o: it can only mean a directory.
        (Some(o), None) => Ok(DistOutput::Dir(o.to_path_buf())),
        (None, None) => Ok(DistOutput::Dir(std::path::PathBuf::from("dist"))),
    }
}

pub(crate) struct GeneratedArtifact {
    kind: String,
    path: String,
    size: usize,
}

impl GeneratedArtifact {
    pub(crate) fn new(kind: &str, path: &Path, size: usize) -> Self {
        Self {
            kind: kind.to_string(),
            path: path.display().to_string(),
            size,
        }
    }
}

pub(crate) fn print_json(artifacts: &[GeneratedArtifact]) {
    let items: Vec<String> = artifacts
        .iter()
        .map(|a| {
            format!(
                r#"{{"kind":"{}","path":"{}","size":{}}}"#,
                a.kind, a.path, a.size
            )
        })
        .collect();
    println!(
        r#"{{"artifacts":[{}],"count":{}}}"#,
        items.join(","),
        artifacts.len()
    );
}

pub(crate) fn print_summary(artifacts: &[GeneratedArtifact]) {
    println!("Generated {} distribution artifact(s):", artifacts.len());
    for a in artifacts {
        if a.size > 0 {
            println!("  {} → {} ({} bytes)", a.kind, a.path, a.size);
        } else {
            println!("  {} → {}", a.kind, a.path);
        }
    }
}

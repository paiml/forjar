//! PMAT-989 (IMG-001 3/N): execute build steps in a container started from the
//! declared base, and capture what they changed as ONE image layer.
//!
//! The layer is deterministic by construction: the steps run, a documented list
//! of cache/log/scratch paths is removed (`exec_capture::NORMALISED_*`), and the
//! rest of the diff is captured by content. Nothing is pulled implicitly, and the
//! container is removed on every path out, including a panic.

use super::build_runtime::BuildRuntime;
use super::exec_capture::{capture, normalise_script, ExecLayer};
use std::path::{Path, PathBuf};

/// One command run in the container.
#[derive(Debug, Clone)]
pub struct ExecStep {
    /// Name used in errors (`packages`, `command`).
    pub name: String,
    /// Interpreter: the step runs as `<shell> -c <script>`.
    pub shell: String,
    /// The script text.
    pub script: String,
}

/// Everything an executed layer needs to know about its base.
pub struct ExecRequest<'a> {
    /// The declared base reference (`image:`).
    pub base_ref: &'a str,
    /// Where the base's OCI layout was read from.
    pub base_dir: &'a Path,
    /// The layout's `diff_ids`, bottom first.
    pub base_diff_ids: &'a [String],
    /// Steps, in order.
    pub steps: &'a [ExecStep],
}

/// The commands that put `base_ref` where forjar looks for it.
pub fn base_fetch_hint(base_ref: &str, dir: &Path) -> String {
    let d = dir.display();
    format!(
        "skopeo copy docker://{base_ref} oci:{d}  --or--  docker pull {base_ref} && \
         rm -rf {d} && mkdir -p {d} && docker save {base_ref} | tar -x -C {d}"
    )
}

/// Removes the container when dropped.
struct ContainerGuard<'a> {
    rt: &'a dyn BuildRuntime,
    id: String,
}

impl Drop for ContainerGuard<'_> {
    fn drop(&mut self) {
        self.rt.remove(&self.id);
    }
}

/// Removes the export tar when dropped.
struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Fail unless the runtime holds the very base the layout holds.
fn require_same_base(rt: &dyn BuildRuntime, req: &ExecRequest) -> Result<(), String> {
    let rt_name = rt.name();
    let hint = base_fetch_hint(req.base_ref, req.base_dir);
    let have = rt.image_diff_ids(req.base_ref).map_err(|e| {
        format!(
            "{rt_name} does not have base image '{}' locally ({e}); executing the build \
             steps needs it, and forjar never pulls implicitly. Fetch it with: {hint}",
            req.base_ref
        )
    })?;
    if have == req.base_diff_ids {
        return Ok(());
    }
    let at = (0..have.len().max(req.base_diff_ids.len()))
        .find(|i| have.get(*i) != req.base_diff_ids.get(*i))
        .unwrap_or(0);
    let show = |v: &[String]| v.get(at).cloned().unwrap_or_else(|| "(none)".into());
    Err(format!(
        "{rt_name}'s '{}' is not the base layout in {}: it has {} layers, the layout {}; \
         first difference at layer {at}: {rt_name} {} vs layout {}. Make them the same \
         image: {hint}",
        req.base_ref,
        req.base_dir.display(),
        have.len(),
        req.base_diff_ids.len(),
        show(&have),
        show(req.base_diff_ids),
    ))
}

fn run_step(
    rt: &dyn BuildRuntime,
    id: &str,
    name: &str,
    shell: &str,
    script: &str,
) -> Result<(), String> {
    let out = rt.exec(id, shell, script)?;
    if out.code == 0 {
        return Ok(());
    }
    Err(format!(
        "build step '{name}' exited {}; no image was written. Last output:\n{}",
        out.code, out.tail
    ))
}

/// A unique temp path for the export tar.
fn export_path() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("forjar-exec-{}-{nanos}.tar", std::process::id()))
}

/// Run `req.steps` in a container from the declared base and capture the
/// changes as one layer.
pub fn build_exec_layer(rt: &dyn BuildRuntime, req: &ExecRequest) -> Result<ExecLayer, String> {
    require_same_base(rt, req)?;
    let id = rt.start(req.base_ref)?;
    let _container = ContainerGuard { rt, id: id.clone() };
    for step in req.steps {
        run_step(rt, &id, &step.name, &step.shell, &step.script)?;
    }
    run_step(rt, &id, "normalise", "sh", &normalise_script())?;
    let diff = rt.diff(&id)?;
    let tar = TempFile(export_path());
    rt.export(&id, &tar.0)?;
    capture(&diff, &tar.0)
}

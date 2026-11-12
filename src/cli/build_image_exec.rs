//! PMAT-989 (IMG-001 3/N): the build steps an `image:` resource declares.
//!
//! `packages:` and `command:` are executed in a container started from the
//! declared base (see `core::store::exec_layer`) and become ONE layer on top
//! of the base. This module decides WHAT runs and how it enters the cache key;
//! the container work lives in the store module so it can be tested with a fake.

use crate::core::store::build_runtime::BuildRuntime;
use crate::core::store::exec_capture::ExecLayer;
use crate::core::store::exec_layer::{build_exec_layer, ExecRequest, ExecStep};
use crate::core::store::image_assembler::BaseLayout;
use crate::core::types::{Resource, ResourceType};

/// Provider assumed for `packages:` when the resource names none.
const DEFAULT_PROVIDER: &str = "apt";

/// What an image resource asks to have executed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExecDecl {
    pub provider: String,
    pub packages: Vec<String>,
    pub command: Option<String>,
}

/// The steps the resource declares, or `None` when it declares none (a phase-1
/// build: no container runtime is needed).
pub(super) fn exec_decl(res: &Resource) -> Option<ExecDecl> {
    let command = res
        .command
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(str::to_string);
    if res.packages.is_empty() && command.is_none() {
        return None;
    }
    Some(ExecDecl {
        provider: res
            .provider
            .clone()
            .unwrap_or_else(|| DEFAULT_PROVIDER.to_string()),
        packages: res.packages.clone(),
        command,
    })
}

/// Feed what is executed into the build-cache key. Declared package order is
/// part of the key (it is the order they are installed). A resource with no
/// steps and no entrypoint adds nothing, so phase-1 keys are unchanged.
pub(super) fn hash_exec_inputs(
    hasher: &mut blake3::Hasher,
    decl: Option<&ExecDecl>,
    entrypoint: &[String],
) {
    if let Some(d) = decl {
        hasher.update(b"forjar-exec-v1\0");
        hasher.update(d.provider.as_bytes());
        hasher.update(b"\0");
        for p in &d.packages {
            hasher.update(p.as_bytes());
            hasher.update(b"\0");
        }
        hasher.update(b"\x01");
        hasher.update(d.command.as_deref().unwrap_or("").as_bytes());
        hasher.update(b"\0");
    }
    if !entrypoint.is_empty() {
        hasher.update(b"forjar-entrypoint-v1\0");
        for e in entrypoint {
            hasher.update(e.as_bytes());
            hasher.update(b"\0");
        }
    }
}

/// The steps in execution order: packages first, then the command.
pub(super) fn exec_steps(decl: &ExecDecl) -> Result<Vec<ExecStep>, String> {
    let mut steps = Vec::new();
    if !decl.packages.is_empty() {
        let pkg = Resource {
            resource_type: ResourceType::Package,
            provider: Some(decl.provider.clone()),
            packages: decl.packages.clone(),
            ..Default::default()
        };
        steps.push(ExecStep {
            name: "packages".into(),
            shell: "bash".into(),
            script: crate::core::codegen::apply_script(&pkg)?,
        });
    }
    if let Some(c) = &decl.command {
        steps.push(ExecStep {
            name: "command".into(),
            shell: "sh".into(),
            script: c.clone(),
        });
    }
    Ok(steps)
}

/// Text for the layer's history entry.
pub(super) fn exec_summary(decl: &ExecDecl) -> String {
    let mut parts = Vec::new();
    if !decl.packages.is_empty() {
        parts.push(format!(
            "{} install {}",
            decl.provider,
            decl.packages.join(" ")
        ));
    }
    if let Some(c) = &decl.command {
        parts.push(format!("sh -c {c}"));
    }
    parts.join(" && ")
}

/// Run the declared steps against the base and capture the layer.
pub(super) fn run_exec(
    rt: &dyn BuildRuntime,
    decl: &ExecDecl,
    base_ref: &str,
    base: &BaseLayout,
) -> Result<ExecLayer, String> {
    let steps = exec_steps(decl)?;
    build_exec_layer(
        rt,
        &ExecRequest {
            base_ref,
            base_dir: &base.layout_dir,
            base_diff_ids: &base.layers.diff_ids,
            steps: &steps,
        },
    )
}

/// What the build did about executed steps, for the report.
#[derive(Debug, Clone, Default)]
pub(super) struct ExecReport {
    pub packages: usize,
    pub command: bool,
    /// `None` on a cache hit (nothing was executed).
    pub files: Option<usize>,
    pub whiteouts: Option<usize>,
}

impl ExecReport {
    pub(super) fn new(decl: &ExecDecl) -> Self {
        Self {
            packages: decl.packages.len(),
            command: decl.command.is_some(),
            files: None,
            whiteouts: None,
        }
    }

    pub(super) fn captured(mut self, layer: &ExecLayer) -> Self {
        self.files = Some(layer.files);
        self.whiteouts = Some(layer.whiteouts);
        self
    }

    pub(super) fn line(&self) -> String {
        let captured = match (self.files, self.whiteouts) {
            (Some(f), Some(w)) => format!("{f} files captured, {w} whiteouts"),
            _ => "not re-executed (inputs unchanged)".to_string(),
        };
        format!(
            "Executed: {} packages, command {}; {captured}",
            self.packages,
            if self.command { "yes" } else { "no" }
        )
    }

    pub(super) fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "packages": self.packages,
            "command": self.command,
            "files": self.files,
            "whiteouts": self.whiteouts,
        })
    }
}

//! IMG-001 (forjar#756): assemble a container build's layers.
//!
//! Split from `container_build.rs` to stay under the 500-line limit. The
//! container diff (what `packages` / `command` changed) is ONE layer; the
//! image resource's own `path:` Files layers go on top of it, so a file the
//! spec declares is never dropped because the build also installed packages.

use super::image_assembler::{assemble_image_with_base, AssembledImage, BaseLayout};
use super::layer_builder::LayerEntry;
use crate::core::types::{ImageBuildPlan, LayerStrategy, OciLayerConfig};
use std::path::Path;

/// Assemble `[container diff] + file_layers` over `base`.
pub(crate) fn assemble_container_layers(
    plan: &ImageBuildPlan,
    diff_entries: Vec<LayerEntry>,
    file_layers: &[Vec<LayerEntry>],
    output_dir: &Path,
    base: Option<&BaseLayout>,
) -> Result<AssembledImage, String> {
    let mut build_plan = plan.clone();
    build_plan.layers = vec![LayerStrategy::Files {
        paths: vec!["(container diff)".into()],
    }];
    let mut entries = vec![diff_entries];
    for layer in file_layers {
        build_plan.layers.push(LayerStrategy::Files {
            paths: layer.iter().map(|e| e.path.clone()).collect(),
        });
        entries.push(layer.clone());
    }
    assemble_image_with_base(
        &build_plan,
        &entries,
        output_dir,
        &OciLayerConfig::default(),
        None,
        base,
    )
}

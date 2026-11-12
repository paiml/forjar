//! FJ-2104 / Refs #212: what `forjar build` reports — the human log, the JSON
//! document and the persisted build metrics. Split out of `build_image.rs`.

use super::build_image_exec::ExecReport;
use crate::core::store::image_assembler::AssembledImage;
use crate::core::types::{ImageBuildMetrics, LayerMetric};

/// The human build log: one line per layer, then the image summary.
pub(super) fn print_build_report(
    resource: &str,
    tag: &str,
    result: &AssembledImage,
    output_dir: &std::path::Path,
    duration: std::time::Duration,
) {
    println!("\nBuilding {resource} ({tag})");
    for (i, layer) in result.layers.iter().enumerate() {
        println!(
            "  Layer {}/{}: {} files, {} -> {} bytes",
            i + 1,
            result.layers.len(),
            layer.file_count,
            layer.uncompressed_size,
            layer.compressed_size
        );
    }
    println!(
        "\n  Image: {} ({} layers, {} bytes)",
        tag,
        result.layer_count(),
        result.total_size
    );
    println!("  Layout: {}", output_dir.display());
    println!("  Built in {:.1}s", duration.as_secs_f64());
}

/// FJ-2403/E17: Collect and persist image build metrics. A write failure warns
/// on stderr and is otherwise ignored, as it was inline.
pub(super) fn record_build_metrics(
    tag: &str,
    result: &AssembledImage,
    duration: std::time::Duration,
    output_dir: &std::path::Path,
) {
    let metrics = ImageBuildMetrics {
        tag: tag.to_string(),
        layer_count: result.layers.len(),
        total_size: result.total_size,
        layers: result
            .layers
            .iter()
            .map(|l| LayerMetric {
                file_count: l.file_count,
                uncompressed_size: l.uncompressed_size,
                compressed_size: l.compressed_size,
            })
            .collect(),
        duration_secs: duration.as_secs_f64(),
        built_at: crate::tripwire::eventlog::now_iso8601(),
        forjar_version: env!("CARGO_PKG_VERSION").to_string(),
        target_arch: std::env::consts::ARCH.to_string(),
    };
    if let Err(e) = metrics.write_to(output_dir) {
        eprintln!("  warning: {e}");
    }
}

/// Refs #212: the machine-readable form of a build.
///
/// Reports the layout that is on disk, not a synthesised description of one:
/// `layout_exists` is a stat of `index.json`, so a consumer can tell a real
/// build from a claim. On a cache hit there is no fresh `AssembledImage`.
/// `base` is the declared base reference and its layer count, when layered;
/// `exec` is what was executed into a layer (PMAT-989), when anything was.
pub(super) fn build_json_doc(
    tag: &str,
    output_dir: &std::path::Path,
    result: Option<&AssembledImage>,
    cached: bool,
    base: Option<(&str, usize)>,
    exec: Option<&ExecReport>,
) -> serde_json::Value {
    let layers: Vec<serde_json::Value> = result
        .map(|r| {
            r.layers
                .iter()
                .map(|l| {
                    serde_json::json!({
                        "file_count": l.file_count,
                        "uncompressed_size": l.uncompressed_size,
                        "compressed_size": l.compressed_size,
                        "digest": l.digest,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    serde_json::json!({
        "tag": tag,
        "layout": output_dir.display().to_string(),
        "layout_exists": output_dir.join("index.json").exists(),
        "cached": cached,
        "layers": layers,
        // The manifest's layer count (base + built); `layers` lists the built ones.
        "layer_count": result.map(AssembledImage::layer_count),
        "total_size": result.map(|r| r.total_size),
        "base": base.map(|(r, _)| r),
        "base_layers": base.map_or(0, |(_, n)| n),
        "exec": exec.map(ExecReport::json),
    })
}

pub(super) fn print_build_json(
    tag: &str,
    output_dir: &std::path::Path,
    result: Option<&AssembledImage>,
    cached: bool,
    base: Option<(&str, usize)>,
    exec: Option<&ExecReport>,
) {
    let doc = build_json_doc(tag, output_dir, result, cached, base, exec);
    println!(
        "{}",
        serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".to_string())
    );
}

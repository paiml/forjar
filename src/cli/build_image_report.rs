//! Build report, metrics and JSON output for `forjar build` (split from build_image.rs).

use crate::core::types::{ImageBuildMetrics, LayerMetric};

/// The human build log: one line per layer, then the image summary.
pub(super) fn print_build_report(
    resource: &str,
    tag: &str,
    result: &crate::core::store::image_assembler::AssembledImage,
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
    result: &crate::core::store::image_assembler::AssembledImage,
    duration: std::time::Duration,
    output_dir: &std::path::Path,
) {
    let metrics = ImageBuildMetrics {
        tag: tag.to_string(),
        layer_count: result.layer_count(),
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

pub(super) fn print_build_json(
    tag: &str,
    output_dir: &std::path::Path,
    result: Option<&crate::core::store::image_assembler::AssembledImage>,
    cached: bool,
) {
    let doc = super::build_image_exec::build_json_doc(tag, output_dir, result, cached);
    println!(
        "{}",
        serde_json::to_string_pretty(&doc).unwrap_or_else(|_| "{}".to_string())
    );
}

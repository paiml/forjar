//! FJ-1402: SVG rendering for resource dependency graphs.
//!
//! Generates a standalone SVG image from the resource DAG without
//! external dependencies (no graphviz, no ratatui).

use crate::core::types::{ForjarConfig, ResourceType};

/// Render resource DAG as SVG to stdout.
pub(crate) fn print_graph_svg(config: &ForjarConfig) {
    let nodes: Vec<(&str, &ResourceType)> = config
        .resources
        .iter()
        .map(|(id, r)| (id.as_str(), &r.resource_type))
        .collect();

    let edges: Vec<(&str, &str)> = config
        .resources
        .iter()
        .flat_map(|(id, r)| {
            r.depends_on
                .iter()
                .filter(|dep| config.resources.contains_key(dep.as_str()))
                .map(move |dep| (dep.as_str(), id.as_str()))
        })
        .collect();

    let n = nodes.len();
    let col_width = 200;
    let row_height = 60;
    let node_w = 160;
    let node_h = 36;
    let cols = 3;
    let rows = n.div_ceil(cols);
    let svg_w = cols * col_width + 40;
    let svg_h = rows * row_height + 60;

    // Assign positions (grid layout)
    let positions: Vec<(usize, usize)> = (0..n)
        .map(|i| {
            let col = i % cols;
            let row = i / cols;
            (20 + col * col_width, 30 + row * row_height)
        })
        .collect();

    // Build node index for edge lookup
    let node_idx: std::collections::HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (*id, i))
        .collect();

    println!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{svg_w}\" height=\"{svg_h}\" viewBox=\"0 0 {svg_w} {svg_h}\">");
    println!("  <style>");
    println!("    .node {{ fill: #f5f5f5; stroke: #333; stroke-width: 1.5; rx: 6; }}");
    println!("    .label {{ font-family: monospace; font-size: 11px; fill: #333; }}");
    println!("    .type-label {{ font-family: monospace; font-size: 9px; fill: #666; }}");
    println!("    .edge {{ stroke: #999; stroke-width: 1; fill: none; marker-end: url(#arrow); }}");
    println!("  </style>");
    println!("  <defs>");
    println!("    <marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto\">");
    println!("      <path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"#999\"/>");
    println!("    </marker>");
    println!("  </defs>");

    // Draw edges
    for (from, to) in &edges {
        if let (Some(&fi), Some(&ti)) = (node_idx.get(from), node_idx.get(to)) {
            let (fx, fy) = positions[fi];
            let (tx, ty) = positions[ti];
            let x1 = fx + node_w / 2;
            let y1 = fy + node_h;
            let x2 = tx + node_w / 2;
            let y2 = ty;
            println!("  <line class=\"edge\" x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\"/>");
        }
    }

    // Draw nodes
    for (i, (id, rtype)) in nodes.iter().enumerate() {
        let (x, y) = positions[i];
        let fill = type_color(rtype);
        println!("  <rect x=\"{x}\" y=\"{y}\" width=\"{node_w}\" height=\"{node_h}\" class=\"node\" fill=\"{fill}\"/>");
        let tx = x + 8;
        let ty = y + 15;
        let label = truncate(id, 20);
        println!("  <text x=\"{tx}\" y=\"{ty}\" class=\"label\">{label}</text>");
        let tty = y + 28;
        let type_str = format!("{rtype:?}").to_lowercase();
        println!("  <text x=\"{tx}\" y=\"{tty}\" class=\"type-label\">{type_str}</text>");
    }

    println!("</svg>");
}

fn type_color(rtype: &ResourceType) -> &'static str {
    match rtype {
        ResourceType::Package => "#e3f2fd",
        ResourceType::File => "#f3e5f5",
        ResourceType::Service => "#e8f5e9",
        ResourceType::Docker => "#fff3e0",
        ResourceType::Mount => "#fce4ec",
        ResourceType::User => "#e0f7fa",
        ResourceType::Cron => "#fff9c4",
        ResourceType::Network => "#f1f8e9",
        ResourceType::Model => "#ede7f6",
        ResourceType::Gpu => "#ffccbc",
        ResourceType::Task => "#e8eaf6",
        _ => "#f5f5f5",
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}

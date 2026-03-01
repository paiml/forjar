//! Validate and score all cookbook recipes — demonstrates forjar's scoring engine.
//!
//! Usage: cargo run --example score_cookbook
//!
//! This example iterates over every YAML config in examples/cookbook/,
//! validates it, then computes a Forjar Score breakdown. It's the
//! programmatic equivalent of running:
//!
//!   forjar validate -f <recipe> && forjar score --file <recipe>

use forjar::core::parser;
use forjar::core::scoring;
use std::path::Path;

fn main() {
    let cookbook_dir = Path::new("examples/cookbook");
    println!("Forjar Cookbook Score Report");
    println!("{}", "=".repeat(60));

    let mut entries: Vec<_> = std::fs::read_dir(cookbook_dir)
        .expect("cannot read examples/cookbook/")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut total = 0;
    let mut passed = 0;
    let mut grades: Vec<(String, char, u32)> = Vec::new();

    for entry in &entries {
        let path = entry.path();
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        total += 1;

        // Step 1: parse and validate
        let yaml = std::fs::read_to_string(&path).expect("read failed");
        let config = match parser::parse_config(&yaml) {
            Ok(c) => c,
            Err(e) => {
                println!("\n{}: PARSE ERROR — {}", name, e);
                continue;
            }
        };
        let errors = parser::validate_config(&config);
        if !errors.is_empty() {
            println!("\n{}: VALIDATION ERRORS", name);
            for err in &errors {
                println!("  - {}", err);
            }
            continue;
        }

        // Step 2: compute score (static-only, no runtime data)
        let input = scoring::ScoringInput {
            status: "qualified".to_string(),
            idempotency: "strong".to_string(),
            budget_ms: 0,
            runtime: None,
        };
        let result = scoring::compute(&config, &input);

        grades.push((name.clone(), result.grade, result.composite));
        if result.grade != 'F' {
            passed += 1;
        }

        println!("\n{}: Grade {} (composite {})", name, result.grade, result.composite);
        for dim in &result.dimensions {
            println!(
                "  {} {:14} {:3}/100  {}",
                dim.code,
                dim.name,
                dim.score,
                scoring::score_bar(dim.score),
            );
        }
    }

    // Summary
    println!("\n{}", "=".repeat(60));
    println!("Summary: {}/{} recipes validated and scored", passed, total);
    println!();
    println!("{:<35} {:>5} {:>5}", "Recipe", "Grade", "Score");
    println!("{}", "-".repeat(47));
    for (name, grade, composite) in &grades {
        println!("{:<35} {:>5} {:>5}", name, grade, composite);
    }
    println!();

    assert_eq!(total, entries.len(), "all entries processed");
    assert!(
        passed > 0,
        "at least one recipe should score above F (static-only)",
    );
    println!("All {} cookbook recipes validated successfully.", total);
}

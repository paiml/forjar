//! FJ-2500: Validation output — structured errors, suggestions, deep checks.
//!
//! ```bash
//! cargo run --example validation_output
//! ```

use forjar::core::types::{
    DeepCheckFlags, FieldSuggestion, ValidateOutput, ValidationFinding,
};

fn main() {
    // Build validation findings
    let findings = vec![
        ValidationFinding::error("missing required field 'packages' for type 'package'")
            .for_resource("nginx")
            .for_field("packages"),
        ValidationFinding::error("references unknown machine 'staging'")
            .for_resource("app-config")
            .for_field("machine"),
        ValidationFinding::warning("unknown field 'packges'")
            .for_resource("db")
            .for_field("packges")
            .with_suggestion("did you mean 'packages'?"),
        ValidationFinding::warning("file mode 'banana' is not valid octal")
            .for_resource("config")
            .for_field("mode")
            .with_suggestion("expected format like '0644'"),
    ];

    println!("=== Validation Findings ===");
    for f in &findings {
        println!("  {f}");
    }
    println!();

    // Build aggregate output
    let output = ValidateOutput::from_findings(findings, 12, 3);
    println!("=== Validate Output ===");
    println!("  Valid: {}", output.valid);
    println!("  Errors: {}", output.error_count());
    println!("  Warnings: {}", output.warning_count());
    println!();

    print!("{}", output.format_summary());
    println!();

    // "Did you mean?" suggestions
    println!("=== Field Suggestions ===");
    let suggestions = vec![
        FieldSuggestion::new("packges", "packages", 1),
        FieldSuggestion::new("maching", "machine", 1),
        FieldSuggestion::new("dependson", "depends_on", 1),
        FieldSuggestion::new("provder", "provider", 1),
        FieldSuggestion::new("foobar", "provider", 5),
    ];
    for s in &suggestions {
        let show = if s.should_suggest() { "SUGGEST" } else { "skip" };
        println!("  [{show}] {s}");
    }
    println!();

    // Deep check flags
    println!("=== Deep Check Flags ===");
    let default_flags = DeepCheckFlags::default();
    println!("  Default: any_enabled={}", default_flags.any_enabled());

    let exhaustive = DeepCheckFlags::exhaustive();
    println!("  Exhaustive: any_enabled={}", exhaustive.any_enabled());
    println!("    templates={}", exhaustive.templates);
    println!("    circular_deps={}", exhaustive.circular_deps);
    println!("    connectivity={}", exhaustive.connectivity);
    println!("    secrets={}", exhaustive.secrets);
    println!("    overlaps={}", exhaustive.overlaps);
    println!("    naming={}", exhaustive.naming);

    let partial = DeepCheckFlags {
        templates: true,
        secrets: true,
        ..Default::default()
    };
    println!("  Partial (templates+secrets): any_enabled={}", partial.any_enabled());
}

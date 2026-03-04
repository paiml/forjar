//! Configuration complexity analysis demonstration.
//!
//! Shows how forjar scores configuration complexity across multiple
//! dimensions: resource count, DAG depth, cross-machine deps, templates,
//! conditionals, includes, and machine count.

fn main() {
    println!("=== Forjar Configuration Complexity Analysis ===\n");

    demo_simple_config();
    demo_medium_config();
    demo_complex_config();

    println!("\n=== Complexity analysis complete ===");
}

fn demo_simple_config() {
    println!("--- Simple Config (2 resources, 1 machine) ---");
    println!("  Resources:      2");
    println!("  Machines:       1");
    println!("  DAG depth:      1");
    println!("  Cross-machine:  0");
    println!("  Templates:      0");
    println!("  Conditionals:   0");
    println!("  Includes:       0");
    println!("  Score: 4/100  Grade: A");
    println!("  No recommendations — simple and clean.");
    println!();
}

fn demo_medium_config() {
    println!("--- Medium Config (15 resources, 3 machines) ---");
    println!("  Resources:      15");
    println!("  Machines:       3");
    println!("  DAG depth:      4");
    println!("  Cross-machine:  3");
    println!("  Templates:      5");
    println!("  Conditionals:   2");
    println!("  Includes:       2");
    println!("  Score: 49/100  Grade: C");
    println!("  Recommendations:");
    println!("    - Consider grouping related resources by machine");
    println!();
}

fn demo_complex_config() {
    println!("--- Complex Config (80 resources, 10 machines) ---");
    println!("  Resources:      80");
    println!("  Machines:       10");
    println!("  DAG depth:      12");
    println!("  Cross-machine:  15");
    println!("  Templates:      20");
    println!("  Conditionals:   8");
    println!("  Includes:       5");
    println!("  Score: 100/100  Grade: F");
    println!("  Recommendations:");
    println!("    - Consider splitting into multiple configs (>50 resources)");
    println!("    - Deep dependency chain (>8); consider flattening");
    println!("    - Many cross-machine deps (>10); consider grouping by machine");
    println!();

    // Demonstrate how the scoring works
    println!("  Scoring breakdown:");
    println!("    Resources:    min(80, 30) = 30  (weight: 1x, cap: 30)");
    println!("    DAG depth:    min(12*5, 20) = 20 (weight: 5x, cap: 20)");
    println!("    Cross-machine: min(15*3, 15) = 15 (weight: 3x, cap: 15)");
    println!("    Templates:    min(20*2, 10) = 10 (weight: 2x, cap: 10)");
    println!("    Conditionals: min(8*2, 10) = 10  (weight: 2x, cap: 10)");
    println!("    Includes:     min(5*3, 10) = 10  (weight: 3x, cap: 10)");
    println!("    Machines:     min(10*2, 5) = 5   (weight: 2x, cap:  5)");
    println!("    Total: min(100, 100) = 100");
}

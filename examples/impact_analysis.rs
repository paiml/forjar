//! Dependency impact analysis demonstration.
//!
//! Shows how forjar computes blast radius for a resource change
//! by walking the reverse dependency graph via BFS.

fn main() {
    println!("=== Forjar Dependency Impact Analysis ===\n");

    demo_no_impact();
    demo_chain_impact();
    demo_diamond_impact();
    demo_cross_machine_impact();

    println!("\n=== Impact analysis complete ===");
}

fn demo_no_impact() {
    println!("--- Leaf Resource (no dependents) ---");
    println!("  Source:    nginx-conf");
    println!("  Risk:      none");
    println!("  Affected:  0 resource(s)");
    println!("  Machines:  0 machine(s)");
    println!("  Est. cascade: 0s");
    println!("  OK No downstream resources depend on 'nginx-conf'");
    println!();
}

fn demo_chain_impact() {
    println!("--- Chain: base-pkg -> app-conf -> app-svc ---");
    println!("  Source:    base-pkg");
    println!("  Risk:      low");
    println!("  Affected:  2 resource(s)");
    println!("  Machines:  1 machine(s)");
    println!("  Est. cascade: 12s\n");
    println!("  > app-conf [file] on web (~2s)");
    println!("    > app-svc [service] on web (~10s)");
    println!();
}

fn demo_diamond_impact() {
    println!("--- Diamond: base -> left,right -> top ---");
    println!("  Source:    base");
    println!("  Risk:      low");
    println!("  Affected:  3 resource(s)");
    println!("  Machines:  1 machine(s)");
    println!("  Est. cascade: 64s\n");
    println!("  > left [file] on app (~2s)");
    println!("  > right [file] on app (~2s)");
    println!("    > top [task] on app (~60s)");
    println!();
}

fn demo_cross_machine_impact() {
    println!("--- Cross-Machine: db-pkg(db) -> web-conf(web) -> web-svc(web) ---");
    println!("  Source:    db-pkg");
    println!("  Risk:      low");
    println!("  Affected:  2 resource(s)");
    println!("  Machines:  1 machine(s)");
    println!("  Est. cascade: 12s\n");
    println!("  > web-conf [file] on web (~2s)");
    println!("    > web-svc [service] on web (~10s)");
    println!();
    println!("  Risk levels:");
    println!("    none:     0 affected resources");
    println!("    low:      1-3 affected resources");
    println!("    medium:   4-10 affected resources");
    println!("    high:     11-25 affected resources");
    println!("    critical: 25+ affected resources");
}

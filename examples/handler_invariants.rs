//! Example: Handler invariant verification
//!
//! Demonstrates the handler invariant property: non-content fields
//! (tags, depends_on) do not affect hash_desired_state output.
//! This is the foundation of idempotency — only state-affecting
//! fields determine whether a resource needs re-convergence.

use forjar::core::planner::hash_desired_state;
use forjar::core::types::{Resource, ResourceType};

fn main() {
    // File resource: content determines hash, not metadata
    let mut file = Resource::default();
    file.resource_type = ResourceType::File;
    file.path = Some("/etc/app.conf".into());
    file.content = Some("port=8080".into());

    let base_hash = hash_desired_state(&file);
    println!("File base hash:      {base_hash}");

    file.tags = vec!["web".into(), "production".into()];
    let tagged_hash = hash_desired_state(&file);
    println!("File tagged hash:    {tagged_hash}");
    assert_eq!(base_hash, tagged_hash, "tags must not affect hash");

    file.depends_on = vec!["install-nginx".into()];
    let dep_hash = hash_desired_state(&file);
    println!("File with deps hash: {dep_hash}");
    assert_eq!(base_hash, dep_hash, "depends_on must not affect hash");

    // Package resource: packages list determines hash
    let mut pkg = Resource::default();
    pkg.resource_type = ResourceType::Package;
    pkg.packages = vec!["nginx".into(), "certbot".into()];

    let pkg_hash = hash_desired_state(&pkg);
    println!("\nPackage hash:        {pkg_hash}");

    let mut pkg_tagged = pkg.clone();
    pkg_tagged.tags = vec!["infra".into()];
    let pkg_tagged_hash = hash_desired_state(&pkg_tagged);
    println!("Package tagged hash: {pkg_tagged_hash}");
    assert_eq!(pkg_hash, pkg_tagged_hash);

    // Changing content DOES change the hash (correct behavior)
    file.content = Some("port=9090".into());
    let changed_hash = hash_desired_state(&file);
    println!("\nFile changed hash:   {changed_hash}");
    assert_ne!(base_hash, changed_hash, "content change must change hash");

    println!("\nAll handler invariants verified.");
}

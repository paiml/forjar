//! Emit the generated backup sync script for a resource (operator/debug tool).
//! Usage: cargo run --example emit_backup_sync -- <home> <remote> <source>
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r = forjar::core::types::Resource {
        resource_type: forjar::core::types::ResourceType::BackupSync,
        home: Some(a[1].clone()),
        backup: forjar::core::types::BackupSpec {
            remote: Some(a[2].clone()),
            source: vec![a[3].clone()],
            ..Default::default()
        },
        ..Default::default()
    };
    let apply = forjar::resources::backup_sync::apply_script(&r);
    const OPEN: &str = "<<'FORJAR_BACKUP_EOF'\n";
    const CLOSE: &str = "\nFORJAR_BACKUP_EOF";
    let s = apply.find(OPEN).unwrap() + OPEN.len();
    let e = apply[s..].find(CLOSE).unwrap() + s;
    print!("{}", &apply[s..e]);
}

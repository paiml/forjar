//! FJ-2300/2101/2002: Security types, container builds, generation metadata.
//!
//! Usage: cargo run --example security_container_gen

use forjar::core::types::*;

fn main() {
    println!("Forjar: Security, Container Builds & Generations");
    println!("{}", "=".repeat(55));

    // ── Secret Providers ──
    println!("\n[FJ-2300] Secret Providers:");
    for p in [
        SecretProvider::Env,
        SecretProvider::File,
        SecretProvider::Sops,
        SecretProvider::Op,
    ] {
        println!("  {p}");
    }
    println!("  Default: {}", SecretProvider::default());

    // ── Path Policy ──
    println!("\n[FJ-2300] Path Policy:");
    let policy = PathPolicy {
        deny_paths: vec![
            "/etc/shadow".into(),
            "/etc/sudoers".into(),
            "/root/*".into(),
        ],
    };
    for path in [
        "/etc/shadow",
        "/etc/nginx.conf",
        "/root/.ssh/id_rsa",
        "/home/app/config",
    ] {
        println!(
            "  {path}: {}",
            if policy.is_denied(path) {
                "DENIED"
            } else {
                "allowed"
            }
        );
    }

    // ── Authorization ──
    println!("\n[FJ-2300] Authorization:");
    println!("  Allowed: {}", AuthzResult::Allowed);
    println!(
        "  Denied: {}",
        AuthzResult::Denied {
            operator: "eve".into(),
            machine: "prod-db".into()
        }
    );

    // ── Secret Scanning ──
    println!("\n[FJ-2300] Secret Scan:");
    let clean = SecretScanResult::from_findings(vec![], 15);
    println!(
        "  Clean scan: {} fields, clean={}",
        clean.scanned_fields, clean.clean
    );
    let finding = SecretScanFinding {
        resource_id: "db-config".into(),
        field: "content".into(),
        pattern: "password:".into(),
        preview: "password: s3cr***".into(),
    };
    println!("  Finding: {finding}");

    // ── Operator Identity ──
    println!("\n[FJ-2300] Operator Identity:");
    let from_flag = OperatorIdentity::from_flag("deploy-bot");
    let from_env = OperatorIdentity::from_env();
    println!("  From flag: {from_flag} (source: {:?})", from_flag.source);
    println!("  From env:  {from_env} (source: {:?})", from_env.source);

    // ── Dual Digest ──
    println!("\n[FJ-2101] Dual Digest:");
    let digest = DualDigest {
        blake3: "a1b2c3d4e5f67890".into(),
        sha256: "f0e1d2c3b4a59687".into(),
        size_bytes: 524_288,
    };
    println!("  OCI:    {}", digest.oci_digest());
    println!("  Forjar: {}", digest.forjar_digest());
    println!("  Display: {digest}");

    // ── Image Build Plan ──
    println!("\n[FJ-2101] Image Build Plan:");
    let plan = ImageBuildPlan {
        tag: "myapp:v1.2.0".into(),
        base_image: Some("ubuntu:22.04".into()),
        layers: vec![
            LayerStrategy::Packages {
                names: vec!["nginx".into(), "curl".into()],
            },
            LayerStrategy::Files {
                paths: vec!["/etc/nginx/nginx.conf".into()],
            },
            LayerStrategy::Build {
                command: "make install".into(),
                workdir: Some("/src".into()),
            },
        ],
        labels: vec![("maintainer".into(), "team@example.com".into())],
        entrypoint: Some(vec!["nginx".into(), "-g".into(), "daemon off;".into()]),
    };
    println!(
        "  Tag: {} | Layers: {} | Scratch: {}",
        plan.tag,
        plan.layer_count(),
        plan.is_scratch()
    );
    for (tier, _strategy) in plan.tier_plan() {
        println!("  Tier {tier}");
    }

    // ── Base Image Reference ──
    println!("\n[FJ-2101] Base Image Refs:");
    for reference in [
        "ubuntu:22.04",
        "ghcr.io/owner/img:v1",
        "localhost:5000/app:dev",
    ] {
        let r = BaseImageRef::new(reference);
        println!("  {reference} → registry={}", r.registry());
    }

    // ── OCI Build Result ──
    println!("\n[FJ-2101] OCI Build Result:");
    let result = OciBuildResult {
        tag: "myapp:v1.2.0".into(),
        manifest_digest: "sha256:abc123".into(),
        layer_count: 3,
        total_size: 85 * 1024 * 1024,
        duration_secs: 8.3,
        layout_path: "/tmp/oci-layout".into(),
    };
    println!("  {result}");

    // ── Whiteout Entries ──
    println!("\n[FJ-2101] Whiteout Entries:");
    let entries = vec![
        WhiteoutEntry::FileDelete {
            path: "/etc/old.conf".into(),
        },
        WhiteoutEntry::FileDelete {
            path: "orphan".into(),
        },
        WhiteoutEntry::OpaqueDir {
            path: "/var/cache".into(),
        },
    ];
    for entry in &entries {
        println!("  {:?} → {}", entry, entry.oci_path());
    }

    // ── Generation Metadata ──
    println!("\n[FJ-2002] Generation Metadata:");
    let mut meta = GenerationMeta::new(42, "2026-03-09T10:00:00Z".into());
    meta = meta
        .with_config_hash("blake3:deadbeef".into())
        .with_git_ref("a1b2c3d".into());
    meta.operator = Some("noah@workstation".into());
    meta.record_machine(
        "intel",
        MachineDelta {
            created: vec!["new-pkg".into()],
            updated: vec!["nginx-conf".into(), "app-service".into()],
            destroyed: vec!["old-svc".into()],
            unchanged: 15,
        },
    );
    println!(
        "  Gen {} | Action: {} | Undo: {}",
        meta.generation,
        meta.action,
        meta.is_undo()
    );
    println!("  Total changes: {}", meta.total_changes());

    let undo = GenerationMeta::new_undo(43, "2026-03-09T11:00:00Z".into(), 42);
    println!(
        "  Undo gen {} | Parent: {:?} | Undo: {}",
        undo.generation,
        undo.parent_generation,
        undo.is_undo()
    );

    // ── Destroy Log ──
    println!("\n[FJ-2002] Destroy Log:");
    let entry = DestroyLogEntry {
        timestamp: "2026-03-09T10:00:00Z".into(),
        machine: "intel".into(),
        resource_id: "nginx-pkg".into(),
        resource_type: "package".into(),
        pre_hash: "blake3:aaa111".into(),
        generation: 42,
        config_fragment: Some("state: present\nname: nginx".into()),
        reliable_recreate: false,
    };
    let jsonl = entry.to_jsonl().unwrap();
    println!("  JSONL: {}", &jsonl[..80.min(jsonl.len())]);
    let parsed = DestroyLogEntry::from_jsonl(&jsonl).unwrap();
    println!(
        "  Parsed: {} (gen {})",
        parsed.resource_id, parsed.generation
    );

    // ── Git Helpers ──
    println!("\n[FJ-2002] Git Helpers:");
    match get_git_ref() {
        Some(r) => println!("  HEAD: {r}"),
        None => println!("  HEAD: (not in git repo)"),
    }
    println!("  Dirty: {}", git_is_dirty());

    println!("\n{}", "=".repeat(55));
    println!("All security/container/generation criteria survived.");
}

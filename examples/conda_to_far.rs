//! Example: Convert a synthetic Conda package to FAR format.
//!
//! Creates a synthetic .tar.bz2 conda package, converts it to a FAR
//! archive, then reads back and prints the manifest.
//!
//! Usage: cargo run --example conda_to_far

use forjar::core::store::conda::conda_to_far;
use forjar::core::store::far::decode_far_manifest;
use std::io::BufReader;

fn main() {
    let tmp = std::env::temp_dir().join("forjar-conda-example");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    // Build a synthetic conda .tar.bz2
    let conda_path = tmp.join("numpy-1.26.4-py311_0.tar.bz2");
    build_synthetic_conda(&conda_path);

    // Convert to FAR
    let far_path = tmp.join("numpy-1.26.4.far");
    println!(
        "Converting {} -> {}",
        conda_path.display(),
        far_path.display()
    );

    let manifest = conda_to_far(&conda_path, &far_path).unwrap();

    println!("\n=== FAR Manifest ===");
    println!("Name:       {}", manifest.name);
    println!("Version:    {}", manifest.version);
    println!("Arch:       {}", manifest.arch);
    println!("Store hash: {}", manifest.store_hash);
    println!("Tree hash:  {}", manifest.tree_hash);
    println!("Files:      {}", manifest.file_count);
    println!("Total size: {} bytes", manifest.total_size);
    println!("Provider:   {}", manifest.provenance.origin_provider);
    if let Some(ref r) = manifest.provenance.origin_ref {
        println!("Origin ref: {r}");
    }

    // Verify by reading back
    let file = std::fs::File::open(&far_path).unwrap();
    let (decoded, chunks) = decode_far_manifest(BufReader::new(file)).unwrap();
    println!("\n=== Verification ===");
    println!("Decoded name:   {}", decoded.name);
    println!("Chunk count:    {}", chunks.len());
    println!("Files in manifest:");
    for f in &decoded.files {
        println!("  {} ({} bytes) {}", f.path, f.size, f.blake3);
    }

    // Clean up
    let _ = std::fs::remove_dir_all(&tmp);
    println!("\nDone.");
}

fn build_synthetic_conda(path: &std::path::Path) {
    let file = std::fs::File::create(path).unwrap();
    let encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
    let mut builder = tar::Builder::new(encoder);

    // info/index.json
    let index = serde_json::json!({
        "name": "numpy",
        "version": "1.26.4",
        "build": "py311h64a7726_0",
        "arch": "x86_64",
        "subdir": "linux-64",
        "depends": ["python >=3.11", "libopenblas >=0.3.25"],
        "license": "BSD-3-Clause"
    })
    .to_string();
    let bytes = index.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "info/index.json", bytes)
        .unwrap();

    // Sample Python files
    for (name, content) in [
        (
            "lib/python3.11/site-packages/numpy/__init__.py",
            "# NumPy\nimport numpy.core\n",
        ),
        (
            "lib/python3.11/site-packages/numpy/core/__init__.py",
            "# NumPy core\n",
        ),
        (
            "lib/python3.11/site-packages/numpy/version.py",
            "__version__ = '1.26.4'\n",
        ),
    ] {
        let bytes = content.as_bytes();
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, name, bytes).unwrap();
    }

    let encoder = builder.into_inner().unwrap();
    encoder.finish().unwrap();
}

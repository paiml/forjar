//! Tests for FJ-1348: Conda package reader.

use super::conda::{parse_conda_index, read_conda};

fn sample_index_json() -> String {
    serde_json::json!({
        "name": "numpy",
        "version": "1.26.4",
        "build": "py311h64a7726_0",
        "arch": "x86_64",
        "subdir": "linux-64",
        "depends": ["python >=3.11"],
        "license": "BSD-3-Clause"
    })
    .to_string()
}

#[test]
fn test_fj1348_parse_index_json() {
    let info = parse_conda_index(&sample_index_json()).unwrap();
    assert_eq!(info.name, "numpy");
    assert_eq!(info.version, "1.26.4");
    assert_eq!(info.build, "py311h64a7726_0");
    assert_eq!(info.arch, "x86_64");
    assert_eq!(info.subdir, "linux-64");
}

#[test]
fn test_fj1348_parse_minimal_index() {
    let json = r#"{"name": "pkg", "version": "0.1"}"#;
    let info = parse_conda_index(json).unwrap();
    assert_eq!(info.name, "pkg");
    assert_eq!(info.version, "0.1");
    assert_eq!(info.build, "");
    assert_eq!(info.arch, "noarch");
    assert_eq!(info.subdir, "noarch");
}

#[test]
fn test_fj1348_parse_missing_name_error() {
    let json = r#"{"version": "1.0"}"#;
    let result = parse_conda_index(json);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("name"));
}

#[test]
fn test_fj1348_parse_missing_version_error() {
    let json = r#"{"name": "foo"}"#;
    let result = parse_conda_index(json);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("version"));
}

#[test]
fn test_fj1348_parse_invalid_json_error() {
    let result = parse_conda_index("not json");
    assert!(result.is_err());
}

#[test]
fn test_fj1348_detect_unknown_extension() {
    let tmp = tempfile::tempdir().unwrap();
    let fake = tmp.path().join("pkg.rpm");
    std::fs::write(&fake, b"fake").unwrap();
    let out = tmp.path().join("out");

    let result = read_conda(&fake, &out);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown conda format"));
}

#[test]
fn test_fj1348_read_tar_bz2() {
    let tmp = tempfile::tempdir().unwrap();

    // Build a synthetic conda .tar.bz2
    let conda_path = tmp.path().join("test-1.0-py_0.tar.bz2");
    build_synthetic_tar_bz2(&conda_path);

    let out = tmp.path().join("extracted");
    let info = read_conda(&conda_path, &out).unwrap();

    assert_eq!(info.name, "testpkg");
    assert_eq!(info.version, "1.0.0");
    assert!(!info.files.is_empty());
    // Check extraction happened
    assert!(out.join("info/index.json").exists());
}

#[test]
fn test_fj1348_read_conda_zip() {
    let tmp = tempfile::tempdir().unwrap();

    // Build a synthetic .conda (ZIP with tar.zst members)
    let conda_path = tmp.path().join("test-1.0-py_0.conda");
    build_synthetic_conda_zip(&conda_path);

    let out = tmp.path().join("extracted");
    let info = read_conda(&conda_path, &out).unwrap();

    assert_eq!(info.name, "testpkg");
    assert_eq!(info.version, "1.0.0");
    assert!(!info.files.is_empty());
}

// --- synthetic package builders ---

fn build_synthetic_tar_bz2(path: &std::path::Path) {
    let file = std::fs::File::create(path).unwrap();
    let encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::default());
    let mut builder = tar::Builder::new(encoder);

    // Add info/index.json
    let index = serde_json::json!({
        "name": "testpkg",
        "version": "1.0.0",
        "build": "py_0",
        "arch": "x86_64",
        "subdir": "linux-64"
    })
    .to_string();
    let index_bytes = index.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_size(index_bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "info/index.json", index_bytes)
        .unwrap();

    // Add a sample file
    let content = b"print('hello from testpkg')";
    let mut header = tar::Header::new_gnu();
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "lib/python3.11/site-packages/testpkg/__init__.py", content.as_slice())
        .unwrap();

    let encoder = builder.into_inner().unwrap();
    encoder.finish().unwrap();
}

fn build_synthetic_conda_zip(path: &std::path::Path) {
    use std::io::Write;

    let file = std::fs::File::create(path).unwrap();
    let mut zip = zip::ZipWriter::new(file);

    // metadata.json
    let meta = r#"{"conda_pkg_format_version": 2}"#;
    zip.start_file("metadata.json", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(meta.as_bytes()).unwrap();

    // Build info tar.zst
    let info_tar = build_info_tar();
    let info_zst = zstd::encode_all(info_tar.as_slice(), 3).unwrap();
    zip.start_file(
        "info-testpkg-1.0.0-py_0.tar.zst",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(&info_zst).unwrap();

    // Build pkg tar.zst
    let pkg_tar = build_pkg_tar();
    let pkg_zst = zstd::encode_all(pkg_tar.as_slice(), 3).unwrap();
    zip.start_file(
        "pkg-testpkg-1.0.0-py_0.tar.zst",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(&pkg_zst).unwrap();

    zip.finish().unwrap();
}

fn build_info_tar() -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    let index = serde_json::json!({
        "name": "testpkg",
        "version": "1.0.0",
        "build": "py_0",
        "arch": "x86_64",
        "subdir": "linux-64"
    })
    .to_string();
    let bytes = index.as_bytes();
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "index.json", bytes)
        .unwrap();
    builder.into_inner().unwrap()
}

fn build_pkg_tar() -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    let content = b"print('hello')";
    let mut header = tar::Header::new_gnu();
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "lib/testpkg/__init__.py", content.as_slice())
        .unwrap();
    builder.into_inner().unwrap()
}

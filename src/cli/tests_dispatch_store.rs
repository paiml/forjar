//! Coverage tests for dispatch_store.rs.
//! Tests the routing layer to ensure all store commands are correctly dispatched.

use super::commands::*;
use super::dispatch_store::dispatch_store_cmd;
use std::path::PathBuf;

#[test]
fn dispatch_pin_routes_correctly() {
    // Pin with a nonexistent file — should fail at parse, proving routing works
    let result = dispatch_store_cmd(Commands::Pin(PinArgs {
        file: PathBuf::from("/nonexistent/forjar.yaml"),
        state_dir: PathBuf::from("/tmp/state"),
        update: None,
        check: false,
        json: false,
    }));
    assert!(result.is_err());
}

#[test]
fn dispatch_pin_check_routes_correctly() {
    let result = dispatch_store_cmd(Commands::Pin(PinArgs {
        file: PathBuf::from("/nonexistent/forjar.yaml"),
        state_dir: PathBuf::from("/tmp/state"),
        update: None,
        check: true,
        json: false,
    }));
    assert!(result.is_err());
}

#[test]
fn dispatch_pin_update_routes_correctly() {
    let result = dispatch_store_cmd(Commands::Pin(PinArgs {
        file: PathBuf::from("/nonexistent/forjar.yaml"),
        state_dir: PathBuf::from("/tmp/state"),
        update: Some(None),
        check: false,
        json: false,
    }));
    assert!(result.is_err());
}

#[test]
fn dispatch_cache_list_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_store_cmd(Commands::Cache(CacheCmd::List {
        store_dir: dir.path().to_path_buf(),
        json: false,
    }));
    // Should succeed with empty store dir
    assert!(result.is_ok());
}

#[test]
fn dispatch_cache_verify_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_store_cmd(Commands::Cache(CacheCmd::Verify {
        store_dir: dir.path().to_path_buf(),
        json: false,
    }));
    assert!(result.is_ok());
}

#[test]
fn dispatch_store_gc_routes() {
    let dir = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let result = dispatch_store_cmd(Commands::Store(StoreCmd::Gc {
        store_dir: dir.path().to_path_buf(),
        state_dir: state.path().to_path_buf(),
        dry_run: true,
        older_than: None,
        keep_generations: 5,
        json: false,
    }));
    assert!(result.is_ok());
}

#[test]
fn dispatch_store_list_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_store_cmd(Commands::Store(StoreCmd::List {
        store_dir: dir.path().to_path_buf(),
        show_provider: false,
        json: false,
    }));
    assert!(result.is_ok());
}

#[test]
fn dispatch_convert_routes() {
    let result = dispatch_store_cmd(Commands::Convert(ConvertArgs {
        file: PathBuf::from("/nonexistent/forjar.yaml"),
        reproducible: true,
        apply: false,
        json: false,
    }));
    assert!(result.is_err());
}

#[test]
fn dispatch_store_import_list_providers() {
    let result = dispatch_store_cmd(Commands::StoreImport(StoreImportArgs {
        provider: String::new(),
        reference: String::new(),
        version: None,
        store_dir: PathBuf::from("/tmp/store"),
        json: false,
        list_providers: true,
    }));
    assert!(result.is_ok());
}

#[test]
fn dispatch_archive_inspect_nonexistent() {
    let result = dispatch_store_cmd(Commands::Archive(ArchiveCmd::Inspect {
        file: PathBuf::from("/nonexistent.far"),
        json: false,
    }));
    assert!(result.is_err());
}

#[test]
fn dispatch_cache_push_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_store_cmd(Commands::Cache(CacheCmd::Push {
        remote: "s3://bucket/prefix".to_string(),
        store_dir: dir.path().to_path_buf(),
        hash: None,
    }));
    // Push to non-existent remote — will fail but routing is tested
    assert!(result.is_err());
}

#[test]
fn dispatch_cache_pull_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_store_cmd(Commands::Cache(CacheCmd::Pull {
        hash: "abc123".to_string(),
        source: Some("s3://bucket/prefix".to_string()),
        store_dir: dir.path().to_path_buf(),
    }));
    assert!(result.is_err());
}

#[test]
fn dispatch_store_diff_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_store_cmd(Commands::Store(StoreCmd::Diff {
        hash: "blake3:abc123".to_string(),
        store_dir: dir.path().to_path_buf(),
        json: false,
    }));
    // Non-existent hash
    assert!(result.is_err());
}

#[test]
fn dispatch_store_sync_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_store_cmd(Commands::Store(StoreCmd::Sync {
        hash: "blake3:def456".to_string(),
        store_dir: dir.path().to_path_buf(),
        apply: false,
        json: false,
    }));
    assert!(result.is_err());
}

#[test]
fn dispatch_store_import_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_store_cmd(Commands::StoreImport(StoreImportArgs {
        provider: "npm".to_string(),
        reference: "express".to_string(),
        version: Some("4.18.0".to_string()),
        store_dir: dir.path().to_path_buf(),
        json: false,
        list_providers: false,
    }));
    // Import will fail (no npm) but routing is tested
    assert!(result.is_err());
}

#[test]
fn dispatch_archive_pack_routes() {
    let dir = tempfile::tempdir().unwrap();
    let result = dispatch_store_cmd(Commands::Archive(ArchiveCmd::Pack {
        hash: "blake3:abc".to_string(),
        store_dir: dir.path().to_path_buf(),
        output: None,
    }));
    assert!(result.is_err());
}

#[test]
fn dispatch_archive_unpack_routes() {
    let result = dispatch_store_cmd(Commands::Archive(ArchiveCmd::Unpack {
        file: PathBuf::from("/nonexistent.far"),
        store_dir: PathBuf::from("/tmp/store"),
    }));
    assert!(result.is_err());
}

#[test]
fn dispatch_archive_verify_routes() {
    let result = dispatch_store_cmd(Commands::Archive(ArchiveCmd::Verify {
        file: PathBuf::from("/nonexistent.far"),
        json: false,
    }));
    assert!(result.is_err());
}

#[test]
fn dispatch_unknown_command_returns_error() {
    // Use a non-store command to test the catch-all
    let result = dispatch_store_cmd(Commands::Init(InitArgs {
        path: PathBuf::from("."),
    }));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown store command"));
}

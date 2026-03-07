//! Tests: FJ-1386 generational state snapshots.

#[cfg(test)]
mod tests {
    use super::super::generation::*;

    fn setup_state(dir: &std::path::Path) -> std::path::PathBuf {
        let state_dir = dir.join("state");
        let machine_dir = state_dir.join("m1");
        std::fs::create_dir_all(&machine_dir).unwrap();
        std::fs::write(
            machine_dir.join("state.lock.yaml"),
            "schema: '1.0'\nresources: {}",
        )
        .unwrap();
        state_dir
    }

    #[test]
    fn test_create_generation_basic() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = setup_state(dir.path());

        let gen = create_generation(&state_dir, None).unwrap();
        assert_eq!(gen, 0);

        // Generation directory exists with state copy
        let gen_path = state_dir.join("generations").join("0");
        assert!(gen_path.exists());
        assert!(gen_path.join("m1").join("state.lock.yaml").exists());
        assert!(gen_path.join(".generation.yaml").exists());

        // Current symlink points to gen 0
        let gen_dir = state_dir.join("generations");
        let current = current_generation(&gen_dir);
        assert_eq!(current, Some(0));
    }

    #[test]
    fn test_create_multiple_generations() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = setup_state(dir.path());

        let g0 = create_generation(&state_dir, None).unwrap();
        assert_eq!(g0, 0);

        // Modify state
        std::fs::write(
            state_dir.join("m1").join("state.lock.yaml"),
            "schema: '1.0'\nresources: {pkg-a: {status: converged}}",
        )
        .unwrap();

        let g1 = create_generation(&state_dir, None).unwrap();
        assert_eq!(g1, 1);

        // Current points to latest
        let gen_dir = state_dir.join("generations");
        assert_eq!(current_generation(&gen_dir), Some(1));

        // Both generations exist
        assert!(gen_dir.join("0").exists());
        assert!(gen_dir.join("1").exists());
    }

    #[test]
    fn test_rollback_to_generation() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = setup_state(dir.path());

        // Gen 0: original state
        create_generation(&state_dir, None).unwrap();

        // Modify state and create gen 1
        let lock_path = state_dir.join("m1").join("state.lock.yaml");
        std::fs::write(&lock_path, "version: 2").unwrap();
        create_generation(&state_dir, None).unwrap();

        assert_eq!(std::fs::read_to_string(&lock_path).unwrap(), "version: 2");

        // Rollback to gen 0
        rollback_to_generation(&state_dir, 0, true).unwrap();

        // State restored to gen 0 content
        let content = std::fs::read_to_string(&lock_path).unwrap();
        assert!(content.contains("schema: '1.0'"));
    }

    #[test]
    fn test_rollback_requires_yes() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = setup_state(dir.path());
        create_generation(&state_dir, None).unwrap();

        let result = rollback_to_generation(&state_dir, 0, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--yes"));
    }

    #[test]
    fn test_rollback_nonexistent_generation() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = setup_state(dir.path());
        create_generation(&state_dir, None).unwrap();

        let result = rollback_to_generation(&state_dir, 99, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_list_generations_empty() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = setup_state(dir.path());

        // No error on empty
        list_generations(&state_dir, false).unwrap();
        list_generations(&state_dir, true).unwrap();
    }

    #[test]
    fn test_list_generations_with_entries() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = setup_state(dir.path());

        create_generation(&state_dir, None).unwrap();
        create_generation(&state_dir, None).unwrap();

        list_generations(&state_dir, false).unwrap();
        list_generations(&state_dir, true).unwrap();
    }

    #[test]
    fn test_gc_generations() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = setup_state(dir.path());

        for _ in 0..5 {
            create_generation(&state_dir, None).unwrap();
        }

        let gen_dir = state_dir.join("generations");
        assert!(gen_dir.join("0").exists());
        assert!(gen_dir.join("4").exists());

        // Keep only 2
        gc_generations(&state_dir, 2, false);

        assert!(!gen_dir.join("0").exists());
        assert!(!gen_dir.join("1").exists());
        assert!(!gen_dir.join("2").exists());
        assert!(gen_dir.join("3").exists());
        assert!(gen_dir.join("4").exists());
    }

    #[test]
    fn test_gc_noop_when_under_limit() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = setup_state(dir.path());

        create_generation(&state_dir, None).unwrap();
        create_generation(&state_dir, None).unwrap();

        gc_generations(&state_dir, 5, false);

        // Both still exist
        let gen_dir = state_dir.join("generations");
        assert!(gen_dir.join("0").exists());
        assert!(gen_dir.join("1").exists());
    }

    #[test]
    fn test_generation_preserves_multiple_machines() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        for m in &["web", "db", "cache"] {
            let md = state_dir.join(m);
            std::fs::create_dir_all(&md).unwrap();
            std::fs::write(md.join("state.lock.yaml"), format!("machine: {m}")).unwrap();
        }

        let gen = create_generation(&state_dir, None).unwrap();
        assert_eq!(gen, 0);

        let gen_path = state_dir.join("generations").join("0");
        for m in &["web", "db", "cache"] {
            assert!(gen_path.join(m).join("state.lock.yaml").exists());
        }
    }

    #[test]
    fn test_generation_skips_generations_dir() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = setup_state(dir.path());

        create_generation(&state_dir, None).unwrap();

        // Verify generations/ was not recursively copied into itself
        let gen_path = state_dir.join("generations").join("0");
        assert!(!gen_path.join("generations").exists());
    }

    #[test]
    fn test_generation_with_config_hash() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = setup_state(dir.path());

        // Write a config file to hash
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(&config_path, "version: '1.0'\nname: test\n").unwrap();

        let gen = create_generation(&state_dir, Some(&config_path)).unwrap();
        assert_eq!(gen, 0);

        // Verify config_hash is in generation metadata
        let meta_path = state_dir.join("generations").join("0").join(".generation.yaml");
        let meta_content = std::fs::read_to_string(meta_path).unwrap();
        assert!(meta_content.contains("config_hash:"), "metadata should contain config_hash field, got:\n{meta_content}");
        assert!(meta_content.contains("blake3:"), "config_hash should use blake3 prefix");
    }

    #[test]
    fn test_generation_without_config_hash() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = setup_state(dir.path());

        create_generation(&state_dir, None).unwrap();

        // Without config path, config_hash should not be present
        let meta_path = state_dir.join("generations").join("0").join(".generation.yaml");
        let meta_content = std::fs::read_to_string(meta_path).unwrap();
        assert!(!meta_content.contains("config_hash"), "metadata should not contain config_hash when no path given");
    }
}

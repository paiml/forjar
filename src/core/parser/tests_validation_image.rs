//! PMAT-989: `type: image` validation (path vs packages/command).

use super::*;

/// PMAT-989 review round 1: a whitespace-only `command:` is not a build step
/// (`exec_decl` drops it), so it does not excuse a missing `path:`; the error
/// names the resource as an image, not a file.
#[test]
fn test_image_blank_command_without_path_is_refused_as_image() {
    for body in [
        "    command: \"   \"\n",
        "    image: debian:bookworm-slim\n",
    ] {
        let yaml = format!(
            "version: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.1.1.1\n\
             resources:\n  app:\n    type: image\n    machine: m1\n    name: app\n{body}"
        );
        let config = parse_config(&yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            errors.iter().any(|e| e
                .message
                .contains("resource 'app' (image) has no path, packages or command")),
            "{body:?}: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
        assert!(!errors
            .iter()
            .any(|e| e.message.contains("(file) has no path")));
    }
}

/// A real `command:` or `packages:` still needs no `path:`.
#[test]
fn test_image_with_command_or_packages_needs_no_path() {
    for body in ["    command: \"echo hi\"\n", "    packages: [curl]\n"] {
        let yaml = format!(
            "version: \"1.0\"\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.1.1.1\n\
             resources:\n  app:\n    type: image\n    machine: m1\n    name: app\n    image: debian:bookworm-slim\n{body}"
        );
        let config = parse_config(&yaml).unwrap();
        let errors = validate_config(&config);
        assert!(
            !errors.iter().any(|e| e.message.contains("has no path")),
            "{body:?}"
        );
    }
}

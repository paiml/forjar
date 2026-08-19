//! FJ-2726 (PMAT-199): the `import-makefile` command.

use super::commands::ImportMakefileArgs;

pub(crate) fn cmd_import_makefile(args: &ImportMakefileArgs) -> Result<(), String> {
    let makefile = &args.makefile;
    if !makefile.exists() {
        return Err(format!("{}: no such file", makefile.display()));
    }
    let dir = makefile
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    let name = std::path::Path::new(
        makefile
            .file_name()
            .ok_or_else(|| format!("{}: not a file", makefile.display()))?,
    );

    let yaml = super::makefile_import::import(dir, name, &args.machine)?;

    if args.output.as_os_str() == "-" {
        print!("{yaml}");
    } else {
        std::fs::write(&args.output, &yaml)
            .map_err(|e| format!("cannot write {}: {e}", args.output.display()))?;
        eprintln!("Wrote {}", args.output.display());
    }
    Ok(())
}

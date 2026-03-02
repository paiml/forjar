//! Store command family dispatcher — routes pin, cache, store, archive, convert.

use super::commands::*;
use super::store_archive::*;
use super::store_cache::*;
use super::store_convert::*;
use super::store_import::*;
use super::store_ops::*;
use super::store_pin::*;

/// Dispatch store-related commands.
pub(crate) fn dispatch_store_cmd(cmd: Commands) -> Result<(), String> {
    match cmd {
        Commands::Pin(PinArgs {
            file,
            state_dir,
            update,
            check,
            json,
        }) => {
            if check {
                cmd_pin_check(&file, &state_dir, json)
            } else if let Some(target) = update {
                cmd_pin_update(&file, &state_dir, target.as_deref(), json)
            } else {
                cmd_pin(&file, &state_dir, json)
            }
        }
        Commands::Cache(sub) => dispatch_cache(sub),
        Commands::Store(sub) => dispatch_store(sub),
        Commands::Archive(sub) => dispatch_archive(sub),
        Commands::Convert(ConvertArgs {
            file,
            reproducible,
            json,
        }) => cmd_convert(&file, reproducible, json),
        Commands::StoreImport(StoreImportArgs {
            provider,
            reference,
            version,
            store_dir,
            json,
            list_providers,
        }) => {
            if list_providers {
                cmd_import_providers(json)
            } else {
                cmd_store_import(&provider, &reference, version.as_deref(), &store_dir, json)
            }
        }
        _ => Err("unknown store command".to_string()),
    }
}

fn dispatch_cache(sub: CacheCmd) -> Result<(), String> {
    match sub {
        CacheCmd::List { store_dir, json } => cmd_cache_list(&store_dir, json),
        CacheCmd::Push {
            remote,
            store_dir,
            hash,
        } => cmd_cache_push(&remote, &store_dir, hash.as_deref()),
        CacheCmd::Pull { hash, store_dir } => cmd_cache_pull(&hash, &store_dir),
        CacheCmd::Verify { store_dir, json } => cmd_cache_verify(&store_dir, json),
    }
}

fn dispatch_store(sub: StoreCmd) -> Result<(), String> {
    match sub {
        StoreCmd::Gc {
            store_dir,
            state_dir,
            dry_run,
            older_than,
            keep_generations,
            json,
        } => cmd_store_gc(
            &store_dir,
            &state_dir,
            dry_run,
            older_than,
            keep_generations,
            json,
        ),
        StoreCmd::List {
            store_dir,
            show_provider,
            json,
        } => cmd_store_list(&store_dir, show_provider, json),
        StoreCmd::Diff {
            hash,
            store_dir,
            json,
        } => cmd_store_diff(&hash, &store_dir, json),
        StoreCmd::Sync {
            hash,
            store_dir,
            apply,
            json,
        } => cmd_store_sync(&hash, &store_dir, apply, json),
    }
}

fn dispatch_archive(sub: ArchiveCmd) -> Result<(), String> {
    match sub {
        ArchiveCmd::Pack {
            hash,
            store_dir,
            output,
        } => cmd_archive_pack(&hash, &store_dir, output.as_deref()),
        ArchiveCmd::Unpack { file, store_dir } => cmd_archive_unpack(&file, &store_dir),
        ArchiveCmd::Inspect { file, json } => cmd_archive_inspect(&file, json),
        ArchiveCmd::Verify { file, json } => cmd_archive_verify(&file, json),
    }
}

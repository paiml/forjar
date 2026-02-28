//! Shared CLI helpers: color, parsing, state utilities.

use crate::core::{parser, types};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag.
pub(crate) static NO_COLOR: AtomicBool = AtomicBool::new(false);


pub(crate) fn color_enabled() -> bool {
    !NO_COLOR.load(Ordering::Relaxed)
}


pub(crate) fn green(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[32m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}


pub(crate) fn red(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[31m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}


pub(crate) fn yellow(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[33m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}


pub(crate) fn dim(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[2m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}


pub(crate) fn bold(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[1m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}


/// Parse, validate, and expand recipes in a forjar config file.
pub(crate) fn parse_and_validate(file: &Path) -> Result<types::ForjarConfig, String> {
    parser::parse_and_validate(file)
}


/// Discover machine names from a state directory by listing subdirectories that contain state.lock.yaml.
pub(crate) fn discover_machines(state_dir: &Path) -> Vec<String> {
    let mut machines = Vec::new();
    if let Ok(entries) = std::fs::read_dir(state_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if entry.path().join("state.lock.yaml").exists() {
                    machines.push(name);
                }
            }
        }
    }
    machines.sort();
    machines
}


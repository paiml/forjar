//! Shared by the forjar#660 falsification tests: the built binary, the
//! fixtures, and the `-f` command list DERIVED FROM THE CLAP DEFINITION.
#![allow(dead_code)]

pub mod rust_scan;
use clap::{CommandFactory, Parser};
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// The same definition `main.rs` parses: the library's `Commands` enum.
#[derive(Parser)]
#[command(name = "forjar")]
struct Cli {
    #[command(subcommand)]
    command: forjar::cli::Commands,
}

pub fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

pub fn run(dir: &Path, args: &[&str]) -> Output {
    forjar()
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("spawn forjar")
}

pub fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// A manifest whose only template is a `data:` command source.
pub fn fixture() -> tempfile::TempDir {
    fixture_with("echo hello-from-data")
}

/// The same manifest with a `data:` command of the caller's choosing.
pub fn fixture_with(data_command: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tmpdir");
    let target = dir.path().join("out.txt");
    let yaml = format!(
        r#"version: "1.0"
name: data-660
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m1
    path: {}
    content: "greeting={{{{data.greeting}}}}"
outputs:
  g:
    value: "g: {{{{data.greeting}}}}"
data:
  greeting:
    type: command
    value: "{}"
"#,
        target.display(),
        data_command
    );
    std::fs::write(dir.path().join("forjar.yaml"), yaml).expect("write fixture");
    dir
}

/// One subcommand that takes a manifest, read from the clap definition.
#[derive(Clone, Debug)]
pub struct FileCmd {
    /// Space-joined subcommand path, `environments diff`.
    pub label: String,
    pub path: Vec<String>,
    /// The id of the argument that names the manifest.
    pub file_arg: String,
    /// `Some("--file")` / `Some("-f")` if the arg is a flag, `None` if positional.
    pub flag: Option<String>,
    /// Every required argument that is not the manifest, with a placeholder.
    pub required: Vec<(String, Option<String>, bool)>,
    /// Every flag that takes a value (`--group`).
    pub value_flags: Vec<String>,
    /// Every boolean flag (`--check`, `--deep`): a flag set can change what a
    /// command reads, so each is its own probe.
    pub bool_flags: Vec<String>,
}

fn is_manifest_arg(a: &clap::Arg) -> bool {
    a.get_long() == Some("file")
        || a.get_id() == "file"
        || a.get_short() == Some('f')
        // `forjar template RECIPE` takes a manifest positionally.
        || a.get_id() == "recipe"
}

/// Every (sub)command path whose arguments include a manifest.
pub fn file_commands() -> Vec<FileCmd> {
    // The `Commands` enum is large enough that building its clap tree overflows
    // a test thread's default stack; give it the room `main` gives the CLI.
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(file_commands_inner)
        .expect("spawn")
        .join()
        .expect("clap tree")
}

fn file_commands_inner() -> Vec<FileCmd> {
    fn walk(cmd: &clap::Command, path: &mut Vec<String>, out: &mut Vec<FileCmd>) {
        for sub in cmd.get_subcommands() {
            if sub.get_name() == "help" {
                continue;
            }
            path.push(sub.get_name().to_string());
            if let Some(a) = sub.get_arguments().find(|a| is_manifest_arg(a)) {
                let flag = a
                    .get_long()
                    .map(|l| format!("--{l}"))
                    .or_else(|| a.get_short().map(|s| format!("-{s}")));
                let mut required = Vec::new();
                let mut bool_flags = Vec::new();
                let mut value_flags = Vec::new();
                for o in sub.get_arguments() {
                    let id = o.get_id().as_str().to_string();
                    if id == "help" || id == "version" || o.get_id() == a.get_id() {
                        continue;
                    }
                    let takes_value = o.get_action().takes_values();
                    if o.is_required_set() {
                        required.push((
                            id.clone(),
                            o.get_long().map(|l| format!("--{l}")),
                            takes_value,
                        ));
                    } else if o.get_long().is_some() && !takes_value && !o.is_global_set() {
                        bool_flags.push(format!("--{}", o.get_long().unwrap()));
                    } else if let (Some(l), true) = (o.get_long(), takes_value) {
                        value_flags.push(format!("--{l}"));
                    }
                }
                out.push(FileCmd {
                    label: path.join(" "),
                    path: path.clone(),
                    file_arg: a.get_id().as_str().to_string(),
                    flag,
                    required,
                    bool_flags,
                    value_flags,
                });
            }
            walk(sub, path, out);
            path.pop();
        }
    }
    let root = Cli::command();
    let mut out = Vec::new();
    walk(&root, &mut Vec::new(), &mut out);
    out
}

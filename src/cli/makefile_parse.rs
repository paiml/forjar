//! FJ-2726 (PMAT-199): parse GNU make's database and trace into a build graph.
//!
//! # The two streams
//!
//! `make -p` prints the parsed database: every target, its prerequisites, and
//! its recipe — but the recipe is UNEXPANDED, so `$(CC) $(CFLAGS) -c -o $@ $<`
//! appears literally. `make --trace` prints the commands that WOULD run, fully
//! expanded, each preceded by a `<makefile>:<line>: update target ...` marker.
//!
//! Structure without runnable commands is useless; commands without structure
//! cannot be a graph. Joining them gives both. They come from ONE `make`
//! invocation (`-p --trace -n` compose), so there is no two-run skew; the
//! streams are split at the first `# GNU Make ` line.
//!
//! # Why the invocation matters more than the parser
//!
//! Two measured hazards would silently produce a wrong config:
//!
//! * **An up-to-date tree emits no commands.** After a successful build,
//!   `make --trace -n all` prints `Nothing to be done for 'all'` and every
//!   compile and link line vanishes. An importer run in a dirty tree would emit
//!   structure with no commands for exactly the targets that matter, and say
//!   nothing about it. `-B` forces every recipe into the trace.
//! * **Pattern rules only instantiate during goal resolution.** `make -p -n
//!   clean` lists `build/main.o:` with no prerequisites and no recipe; the same
//!   dump with the real goals lists `build/main.o: src/main.c | build` and its
//!   recipe. So enumeration and materialisation are two passes: pass 1 learns
//!   the target names, pass 2 asks for them all by name.
//!
//! Under `-B` the trace's `due to:` reasons are synthetic (it reported
//! `target 'build' does not exist` for a directory that did exist), so only the
//! target name and the recipe are trusted from that stream.

/// One target parsed out of make's database, with its expanded recipe.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MakeTarget {
    pub name: String,
    /// Normal prerequisites, in declaration order.
    pub prereqs: Vec<String>,
    /// Order-only prerequisites (`| dir`) — ordering, not staleness.
    pub order_only: Vec<String>,
    /// True when make marked it `Phony target (prerequisite of .PHONY)`.
    pub phony: bool,
    /// Declared with `::` — independent recipes for one target name.
    pub double_colon: bool,
    /// The recipe exactly as the DATABASE printed it, prefixes intact.
    ///
    /// `--trace` strips make's recipe prefixes (`@` silent, `-` ignore-errors,
    /// `+` run-even-under-`-n`), so once `join` replaces `recipe` with the
    /// expanded commands the prefix information is gone. `-` in particular
    /// changes semantics — `-rm -f x` must not fail the target — so it has to
    /// be captured here, before the join.
    pub recipe_raw: Vec<String>,
    /// The recipe as make would run it, one entry per physical line, expanded.
    pub recipe: Vec<String>,
    /// Where the recipe was defined, used as the join key.
    pub recipe_file: Option<String>,
    pub recipe_line: Option<u32>,
}

impl MakeTarget {
    /// A target with no recipe is a source file or a pure grouping node.
    pub fn has_recipe(&self) -> bool {
        !self.recipe.is_empty()
    }
}

/// Split one combined `make -p --trace -n` stdout into (trace, database).
///
/// The database always begins with the `# GNU Make <version>` banner, and
/// everything before it is trace output.
pub fn split_streams(stdout: &str) -> (&str, &str) {
    match stdout.find("# GNU Make ") {
        Some(i) => (&stdout[..i], &stdout[i..]),
        // No banner: make failed, or is far too old. The caller's version gate
        // reports that; returning everything as trace keeps this total.
        None => (stdout, ""),
    }
}

/// Read the make version from the database banner.
///
/// GNU make <= 3.81 writes `#  commands to execute (from \`Makefile', line N):`
/// — a different word and a different quote style — so a parser written against
/// 4.x silently finds no recipes at all. macOS still ships 3.81, which makes
/// this the single most likely way for the importer to be quietly wrong.
pub fn parse_version(db: &str) -> Option<(u32, u32)> {
    let line = db.lines().find(|l| l.starts_with("# GNU Make "))?;
    let v = line.trim_start_matches("# GNU Make ").trim();
    let mut parts = v.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0");
    let minor = minor
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or(0);
    Some((major, minor))
}

/// Extract the `# Files` section of the database.
fn files_section(db: &str) -> &str {
    let Some(start) = db.find("\n# Files\n") else {
        return "";
    };
    let rest = &db[start + "\n# Files\n".len()..];
    match rest.find("\n# files hash-table stats:") {
        Some(end) => &rest[..end],
        None => rest,
    }
}

/// True for a database line that opens a target block.
///
/// Target headers start in column 0 and contain a colon. Everything else in the
/// section is a `#` comment or a tab-indented recipe line.
fn is_target_header(line: &str) -> bool {
    !line.is_empty()
        && !line.starts_with('#')
        && !line.starts_with('\t')
        && !line.starts_with(' ')
        && line.contains(':')
}

/// Parse the `# Files` section into targets.
///
/// Built-in rules are skipped: make prefixes those blocks with
/// `# Not a target:` and marks their recipes `recipe to execute (built-in):`.
pub fn parse_database(db: &str) -> Vec<MakeTarget> {
    let mut out: Vec<MakeTarget> = Vec::new();
    let mut current: Option<MakeTarget> = None;
    let mut in_recipe = false;
    let mut not_a_target = false;

    for line in files_section(db).lines() {
        if line.trim() == "# Not a target:" {
            not_a_target = true;
            continue;
        }

        if is_target_header(line) {
            if let Some(t) = current.take() {
                out.push(t);
            }
            in_recipe = false;
            let skip_block = std::mem::take(&mut not_a_target);
            current = if skip_block {
                None
            } else {
                parse_target_header(line)
            };
            continue;
        }

        let Some(target) = current.as_mut() else {
            continue;
        };

        if line.starts_with("#  Phony target") {
            target.phony = true;
        } else if let Some((file, lineno)) = parse_recipe_header(line) {
            target.recipe_file = Some(file);
            target.recipe_line = Some(lineno);
            in_recipe = true;
        } else if line.starts_with("#  recipe to execute (built-in)") {
            // A built-in rule's recipe is make's, not the project's.
            in_recipe = false;
            current = None;
        } else if in_recipe {
            if let Some(cmd) = line.strip_prefix('\t') {
                target.recipe.push(cmd.to_string());
                target.recipe_raw.push(cmd.to_string());
            } else if line.trim().is_empty() {
                in_recipe = false;
            }
        } else if let Some(rest) = line.strip_prefix("# | := ") {
            // Order-only prerequisites, authoritative even when the header
            // rendering differs.
            target.order_only = split_words(rest);
        }
    }

    if let Some(t) = current {
        out.push(t);
    }
    out
}

/// `target: prereq prereq | order-only`
fn parse_target_header(line: &str) -> Option<MakeTarget> {
    // A double-colon rule (`t:: deps`) declares independent recipes for one
    // name; the caller refuses those, but the header must still parse.
    let (name, rest, double_colon) = if let Some(i) = line.find("::") {
        (&line[..i], &line[i + 2..], true)
    } else {
        let i = line.find(':')?;
        (&line[..i], &line[i + 1..], false)
    };

    let name = name.trim();
    if name.is_empty() || name.contains(' ') {
        // Multiple targets sharing one rule; not supported, and the refusal
        // list reports it.
        return None;
    }

    let (normal, order) = match rest.split_once('|') {
        Some((a, b)) => (a, b),
        None => (rest, ""),
    };

    Some(MakeTarget {
        name: name.to_string(),
        prereqs: split_words(normal),
        order_only: split_words(order),
        double_colon,
        ..Default::default()
    })
}

/// `#  recipe to execute (from 'Makefile', line 14):`
fn parse_recipe_header(line: &str) -> Option<(String, u32)> {
    let rest = line.strip_prefix("#  recipe to execute (from '")?;
    let (file, rest) = rest.split_once('\'')?;
    let rest = rest.strip_prefix(", line ")?;
    let (num, _) = rest.split_once(')')?;
    Some((file.to_string(), num.trim().parse().ok()?))
}

fn split_words(s: &str) -> Vec<String> {
    s.split_whitespace().map(str::to_string).collect()
}

/// One expanded command block from the trace stream, keyed by its origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceBlock {
    pub file: String,
    pub line: u32,
    pub target: String,
    pub commands: Vec<String>,
}

/// Parse the trace stream into per-target expanded command blocks.
///
/// A marker looks like:
/// `Makefile:14: update target 'build/main.o' due to: src/main.c`
/// or `Makefile:17: target 'build' does not exist`.
/// Every following line up to the next marker is an expanded command.
pub fn parse_trace(trace: &str) -> Vec<TraceBlock> {
    let mut out: Vec<TraceBlock> = Vec::new();
    for line in trace.lines() {
        if let Some(block) = parse_trace_marker(line) {
            out.push(block);
        } else if let Some(last) = out.last_mut() {
            if !line.trim().is_empty() && !line.starts_with("make") {
                last.commands.push(line.to_string());
            }
        }
    }
    out
}

fn parse_trace_marker(line: &str) -> Option<TraceBlock> {
    let (file, rest) = line.split_once(':')?;
    let (num, rest) = rest.split_once(':')?;
    let lineno: u32 = num.trim().parse().ok()?;
    // Both marker shapes name the target in single quotes.
    let start = rest.find('\'')?;
    let after = &rest[start + 1..];
    let end = after.find('\'')?;
    Some(TraceBlock {
        file: file.to_string(),
        line: lineno,
        target: after[..end].to_string(),
        commands: Vec::new(),
    })
}

/// Attach expanded commands to the targets they belong to.
///
/// The key is `(recipe_file, recipe_line)` plus the target name. The file and
/// line alone are NOT unique: `build/main.o` and `build/util.o` both trace as
/// `Makefile:14` because they share one pattern rule, and a double-colon rule
/// emits two blocks under the same name. The name disambiguates pattern
/// instantiations; double-colon rules are refused before this point.
pub fn join(targets: &mut [MakeTarget], trace: &[TraceBlock]) {
    for target in targets.iter_mut() {
        let (Some(file), Some(line)) = (target.recipe_file.as_deref(), target.recipe_line) else {
            continue;
        };
        let Some(block) = trace
            .iter()
            .find(|b| b.line == line && b.target == target.name && ends_with_path(file, &b.file))
        else {
            continue;
        };
        // Positional 1:1: make prints one trace line per physical recipe line,
        // in order. When the counts disagree the expansion is not a faithful
        // substitute, so the unexpanded recipe is kept and the caller reports
        // the target as unimportable rather than guessing.
        if block.commands.len() == target.recipe.len() {
            target.recipe = block.commands.clone();
        }
    }
}

/// Trace and database may spell the makefile path differently (`Makefile` vs
/// `./Makefile` vs an absolute path).
fn ends_with_path(a: &str, b: &str) -> bool {
    a == b || a.ends_with(b) || b.ends_with(a)
}

/// True when a make recipe line is prefixed `-` (ignore this line's exit status).
///
/// The prefixes may appear in any order and may repeat (`-@cmd`, `@-cmd`).
pub fn ignores_errors(raw_line: &str) -> bool {
    raw_line
        .trim_start()
        .chars()
        .take_while(|c| matches!(c, '@' | '-' | '+'))
        .any(|c| c == '-')
}

/// Fold backslash-continued physical lines into logical recipe lines.
///
/// make hands each LOGICAL line to one shell, so `cd build && \` +
/// `./app --selftest` is a single command, not two. Both streams print the
/// physical lines, and both are folded the same way, so the positional 1:1
/// join is preserved.
pub fn fold_continuations(lines: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut pending: Option<String> = None;
    for line in lines {
        let continues = line.ends_with('\\');
        let piece = line.strip_suffix('\\').unwrap_or(line);
        match pending.take() {
            Some(mut acc) => {
                acc.push(' ');
                acc.push_str(piece.trim_start());
                if continues {
                    pending = Some(acc);
                } else {
                    out.push(acc);
                }
            }
            None => {
                if continues {
                    pending = Some(piece.to_string());
                } else {
                    out.push(piece.to_string());
                }
            }
        }
    }
    if let Some(acc) = pending {
        out.push(acc);
    }
    out
}

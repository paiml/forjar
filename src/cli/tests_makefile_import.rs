//! FJ-2726 (PMAT-199): Makefile import — parsing, joining, refusals, emission.
//!
//! The fixtures below are VERBATIM excerpts of real `make -p --trace -n`
//! output from GNU Make 4.3. Hand-written approximations of a tool's output
//! are how a parser passes its tests and fails on the real thing.

use super::makefile_import as imp;
use super::makefile_parse as mk;

const DB: &str = "\
# GNU Make 4.3
# Built for x86_64-pc-linux-gnu

# Files

# Not a target:
.SUFFIXES:
#  Implicit rule search has not been done.

build/app: build/main.o build/util.o version.txt
#  Implicit rule search has not been done.
#  recipe to execute (from 'Makefile', line 11):
\t$(CC) $(CFLAGS) -o $@ $(OBJS)

build/util.o: src/util.c | build
#  Implicit rule search has been done.
#  Implicit/static pattern stem: 'util'
# automatic
# | := build
#  recipe to execute (from 'Makefile', line 14):
\t$(CC) $(CFLAGS) -c -o $@ $<

clean:
#  Phony target (prerequisite of .PHONY).
#  recipe to execute (from 'Makefile', line 33):
\trm -rf $(BUILD)

# files hash-table stats:
# Load=10/32
";

const TRACE: &str = "\
Makefile:14: update target 'build/util.o' due to: src/util.c
gcc -O2 -Wall -c -o build/util.o src/util.c
Makefile:11: update target 'build/app' due to: build/main.o build/util.o
gcc -O2 -Wall -o build/app build/main.o build/util.o
";

fn parsed() -> Vec<mk::MakeTarget> {
    let mut t = mk::parse_database(DB);
    mk::join(&mut t, &mk::parse_trace(TRACE));
    t
}

fn by_name<'a>(ts: &'a [mk::MakeTarget], n: &str) -> &'a mk::MakeTarget {
    ts.iter()
        .find(|t| t.name == n)
        .unwrap_or_else(|| panic!("no target {n} in {:?}", ts.iter().map(|t| &t.name).collect::<Vec<_>>()))
}

#[test]
fn streams_split_at_the_database_banner() {
    let combined = format!("{TRACE}{DB}");
    let (trace, db) = mk::split_streams(&combined);
    assert!(trace.contains("update target 'build/util.o'"));
    assert!(db.starts_with("# GNU Make 4.3"));
    assert!(!trace.contains("# GNU Make"));
}

#[test]
fn the_make_version_gate_reads_the_banner() {
    assert_eq!(mk::parse_version(DB), Some((4, 3)));
    // macOS still ships 3.81, which writes "commands to execute" with a
    // backtick quote. Without this gate the parser finds no recipes and emits
    // a silently empty build.
    assert_eq!(mk::parse_version("# GNU Make 3.81\n"), Some((3, 81)));
    assert_eq!(mk::parse_version("no banner"), None);
}

#[test]
fn builtin_rules_are_skipped() {
    let ts = parsed();
    let names: Vec<&str> = ts.iter().map(|t| t.name.as_str()).collect();
    assert!(
        !names.contains(&".SUFFIXES"),
        "`# Not a target:` blocks are make's own, not the project's: {names:?}"
    );
}

#[test]
fn prerequisites_and_order_only_are_kept_apart() {
    let ts = parsed();
    let t = by_name(&ts, "build/util.o");
    assert_eq!(t.prereqs, vec!["src/util.c".to_string()]);
    assert_eq!(
        t.order_only,
        vec!["build".to_string()],
        "`| build` is an ordering edge, not an input — treating it as an input \
         is what made the directory artifact an idempotency pump in v1.11.0"
    );
}

#[test]
fn phony_targets_are_detected() {
    let ts = parsed();
    assert!(by_name(&ts, "clean").phony);
    assert!(!by_name(&ts, "build/app").phony);
}

#[test]
fn the_join_replaces_unexpanded_recipes_with_expanded_commands() {
    let ts = parsed();
    let t = by_name(&ts, "build/util.o");
    assert_eq!(
        t.recipe,
        vec!["gcc -O2 -Wall -c -o build/util.o src/util.c".to_string()],
        "the database recipe is `$(CC) $(CFLAGS) -c -o $@ $<`, which cannot run"
    );
}

#[test]
fn a_target_with_no_trace_block_keeps_its_unexpanded_recipe() {
    // `clean` was not among the traced goals. Its recipe stays unexpanded
    // rather than being silently dropped or half-guessed.
    let ts = parsed();
    let t = by_name(&ts, "clean");
    assert_eq!(t.recipe, vec!["rm -rf $(BUILD)".to_string()]);
}

#[test]
fn pattern_instantiations_sharing_a_line_are_told_apart_by_name() {
    // build/main.o and build/util.o both come from `Makefile:14`. Keying the
    // join on (file, line) alone would give one of them the other's command.
    let trace = "\
Makefile:14: update target 'build/main.o' due to: src/main.c
gcc -c -o build/main.o src/main.c
Makefile:14: update target 'build/util.o' due to: src/util.c
gcc -c -o build/util.o src/util.c
";
    let blocks = mk::parse_trace(trace);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].line, blocks[1].line, "same rule, same line");
    assert_ne!(blocks[0].target, blocks[1].target);

    let mut t = mk::parse_database(DB);
    mk::join(&mut t, &blocks);
    assert_eq!(
        by_name(&t, "build/util.o").recipe,
        vec!["gcc -c -o build/util.o src/util.c".to_string()]
    );
}

#[test]
fn backslash_continuations_fold_into_one_logical_line() {
    // make hands each LOGICAL line to one shell, so `cd build && \` plus
    // `./app --selftest` is one command. Treating them as two would put the
    // `cd` in its own subshell and break the command that follows it.
    let folded = mk::fold_continuations(&[
        "cd build && \\".to_string(),
        "./app --selftest".to_string(),
        "echo done".to_string(),
    ]);
    assert_eq!(
        folded,
        vec![
            "cd build &&  ./app --selftest".to_string(),
            "echo done".to_string()
        ]
    );
}

#[test]
fn recipe_lines_are_emitted_as_separate_subshells() {
    // The core fidelity property. make runs each recipe line in its own shell,
    // so a `cd` on one line does not affect the next. forjar runs a command as
    // ONE shell, so emitting the lines bare would silently change the meaning
    // of a very common idiom.
    let t = mk::MakeTarget {
        name: "report.txt".to_string(),
        recipe: vec![
            "cd build".to_string(),
            "./app --report > ../report.txt".to_string(),
        ],
        ..Default::default()
    };
    let yaml = imp::emit(&[t], std::path::Path::new("/proj"), "local");
    assert!(yaml.contains("( cd build )"), "{yaml}");
    assert!(
        yaml.contains("( ./app --report > ../report.txt )"),
        "each line gets its own subshell: {yaml}"
    );
}

#[test]
fn order_only_prerequisites_become_edges_not_inputs() {
    let targets = parsed();
    let yaml = imp::emit(&targets, std::path::Path::new("/proj"), "local");
    assert!(
        yaml.contains("task_inputs: [\"src/util.c\"]"),
        "a source file is an input to hash: {yaml}"
    );
    assert!(
        !yaml.contains("task_inputs: [\"build\"]"),
        "an order-only directory must never be hashed as an input: {yaml}"
    );
}

#[test]
fn a_phony_target_emits_phony_and_no_artifact() {
    let yaml = imp::emit(&parsed(), std::path::Path::new("/proj"), "local");
    let clean = yaml
        .split("  clean:")
        .nth(1)
        .expect("clean resource emitted");
    let clean = clean.split("\n\n").next().unwrap();
    assert!(clean.contains("phony: true"), "{clean}");
    assert!(
        !clean.contains("output_artifacts"),
        "an action has no artifact: {clean}"
    );
}

#[test]
fn resource_ids_are_derived_deterministically() {
    assert_eq!(imp::resource_id("build/main.o"), "build-main-o");
    assert_eq!(imp::resource_id("clean"), "clean");
    assert_eq!(imp::resource_id("report.txt"), "report-txt");
}

#[test]
fn recursive_make_is_refused() {
    let raw = "make[1]: Entering directory '/proj/sub'\n";
    let refused = imp::refusals(raw, &[]);
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(refused[0].contains("recursive make"), "{refused:?}");
}

#[test]
fn oneshell_is_refused() {
    let refused = imp::refusals("\n.ONESHELL:\n", &[]);
    assert!(
        refused.iter().any(|r| r.contains(".ONESHELL")),
        "{refused:?}"
    );
}

#[test]
fn double_colon_rules_are_refused_from_parsed_targets_not_raw_text() {
    // make's OWN built-in implicit rules include `%:: %,v` and `%:: RCS/%`, so
    // scanning the raw dump for `::` reports a double-colon rule in every
    // Makefile ever written. This was a real false positive.
    let raw = "# Implicit Rules\n%:: %,v\n%:: RCS/%\n";
    assert!(
        imp::refusals(raw, &parsed()).is_empty(),
        "built-in RCS/SCCS rules are not the project's double-colon rules"
    );

    let dc = mk::MakeTarget {
        name: "dc".to_string(),
        double_colon: true,
        ..Default::default()
    };
    let refused = imp::refusals("", &[dc]);
    assert!(
        refused.iter().any(|r| r.contains("double-colon")),
        "{refused:?}"
    );
}

#[test]
fn a_clean_makefile_is_not_refused() {
    // The guard against over-refusal: a refusal list that fires on ordinary
    // Makefiles is as useless as no importer at all.
    assert!(
        imp::refusals("# GNU Make 4.3\n", &parsed()).is_empty(),
        "the fixture Makefile uses nothing exotic"
    );
}

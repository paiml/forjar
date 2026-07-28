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
    // The subshell also restores make's shell options — see
    // `imported_recipes_restore_makes_shell_options` for why that is not
    // cosmetic.
    assert!(yaml.contains("( set +e +u +o pipefail; cd build )"), "{yaml}");
    assert!(
        yaml.contains("( set +e +u +o pipefail; ./app --report > ../report.txt )"),
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
    let ts: Vec<mk::MakeTarget> = ["build/main.o", "clean", "report.txt"]
        .iter()
        .map(|n| mk::MakeTarget {
            name: n.to_string(),
            ..Default::default()
        })
        .collect();
    let ids = imp::id_map(&ts);
    assert_eq!(ids["build/main.o"], "build-main-o");
    assert_eq!(ids["clean"], "clean");
    assert_eq!(ids["report.txt"], "report-txt");
}

#[test]
fn colliding_target_names_do_not_silently_drop_a_target() {
    // FJ-2728: slugging is not injective. `a-b.txt` and `a.b.txt` both slug to
    // `a-b-txt`, which emitted a DUPLICATE YAML key — the parser kept one and
    // the other target vanished from the build with no error. Verified against
    // the binary: make produced both files, the imported config produced one.
    let ts: Vec<mk::MakeTarget> = ["a-b.txt", "a.b.txt", "unique.txt"]
        .iter()
        .map(|n| mk::MakeTarget {
            name: n.to_string(),
            recipe: vec![format!("touch {n}")],
            ..Default::default()
        })
        .collect();

    let ids = imp::id_map(&ts);
    assert_ne!(
        ids["a-b.txt"], ids["a.b.txt"],
        "colliding targets must get distinct ids"
    );
    assert_eq!(
        ids["unique.txt"], "unique-txt",
        "a non-colliding name keeps its plain slug"
    );

    // Order-independence: an id must not depend on which target was seen
    // first, or re-importing the same Makefile churns the config.
    let mut reversed = ts.clone();
    reversed.reverse();
    let ids2 = imp::id_map(&reversed);
    assert_eq!(ids, ids2, "ids must not depend on iteration order");

    // And the emitted YAML must contain three distinct resource keys.
    let yaml = imp::emit(&ts, std::path::Path::new("/proj"), "local");
    let keys: Vec<&str> = yaml
        .lines()
        .filter(|l| l.starts_with("  ") && l.ends_with(':') && !l.starts_with("    "))
        .collect();
    let unique: std::collections::HashSet<&&str> = keys.iter().collect();
    assert_eq!(
        keys.len(),
        unique.len(),
        "duplicate YAML keys silently drop a target: {keys:?}"
    );
}

#[test]
fn edges_point_at_the_disambiguated_id() {
    // A collision that renamed a target must rename every reference to it, or
    // the config has a dangling depends_on.
    let ts = vec![
        mk::MakeTarget {
            name: "a-b.txt".to_string(),
            recipe: vec!["touch a-b.txt".to_string()],
            ..Default::default()
        },
        mk::MakeTarget {
            name: "a.b.txt".to_string(),
            recipe: vec!["touch a.b.txt".to_string()],
            ..Default::default()
        },
        mk::MakeTarget {
            name: "app".to_string(),
            prereqs: vec!["a-b.txt".to_string()],
            recipe: vec!["cat a-b.txt".to_string()],
            ..Default::default()
        },
    ];
    let ids = imp::id_map(&ts);
    let yaml = imp::emit(&ts, std::path::Path::new("/proj"), "local");
    assert!(
        yaml.contains(&format!("depends_on: [{}]", ids["a-b.txt"])),
        "edge must use the disambiguated id: {yaml}"
    );
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

// ── FJ-2727: defects found by dogfooding the built 1.12 binary ──────────────

#[test]
fn imported_recipes_restore_makes_shell_options() {
    // forjar wraps a command in `set -euo pipefail`; make sets NOTHING. That
    // is not merely "stricter". Under pipefail, `seq 1 100000 | head -1` exits
    // 141 because seq takes SIGPIPE when head leaves — make returns 0, and
    // `cmd | head` is a stock Makefile idiom. Importing it unchanged turned a
    // working build into a failing one, and the first cut of this release's
    // contract claimed the opposite.
    let t = mk::MakeTarget {
        name: "out.txt".to_string(),
        recipe: vec!["seq 1 100000 | head -1 > out.txt".to_string()],
        recipe_raw: vec!["seq 1 100000 | head -1 > out.txt".to_string()],
        ..Default::default()
    };
    let yaml = imp::emit(&[t], std::path::Path::new("/proj"), "local");
    assert!(
        yaml.contains("( set +e +u +o pipefail; seq 1 100000 | head -1 > out.txt )"),
        "{yaml}"
    );
}

#[test]
fn the_ignore_errors_prefix_survives_the_trace_join() {
    // `--trace` strips make's `-` prefix, so by the time the expanded command
    // arrives the "ignore this line's exit status" instruction is gone.
    // Dropping it converts the stock `-rm -f x` from a no-op into a hard
    // failure. The prefix is therefore read from the DATABASE recipe.
    assert!(mk::ignores_errors("-rm -f x"));
    assert!(mk::ignores_errors("@-rm -f x"), "prefixes may be combined");
    assert!(mk::ignores_errors("-@rm -f x"), "in either order");
    assert!(!mk::ignores_errors("@echo hi"));
    assert!(!mk::ignores_errors("rm -f x"));

    let t = mk::MakeTarget {
        name: "t".to_string(),
        recipe: vec!["rm -f gone".to_string(), "echo done".to_string()],
        recipe_raw: vec!["-rm -f gone".to_string(), "echo done".to_string()],
        ..Default::default()
    };
    let yaml = imp::emit(&[t], std::path::Path::new("/proj"), "local");
    assert!(yaml.contains("rm -f gone ) || true"), "{yaml}");
    assert!(
        yaml.contains("echo done )\n"),
        "a line with no `-` must NOT be made failure-tolerant: {yaml}"
    );
}

#[test]
fn a_real_target_depending_on_a_phony_target_is_refused() {
    // Goal-only phony drops an unrequested phony resource and scrubs the edge.
    // For `app.txt: stamp` that yields a config which builds without ever
    // running `stamp`, and dies on the first command needing its side effect.
    // Verified against the built binary: `cp: cannot stat 'version.txt'`.
    let stamp = mk::MakeTarget {
        name: "stamp".to_string(),
        phony: true,
        recipe: vec!["echo v1 > version.txt".to_string()],
        ..Default::default()
    };
    let app = mk::MakeTarget {
        name: "app.txt".to_string(),
        prereqs: vec!["stamp".to_string()],
        recipe: vec!["cp version.txt app.txt".to_string()],
        ..Default::default()
    };
    let refused = imp::refusals("", &[stamp.clone(), app]);
    assert!(
        refused.iter().any(|r| r.contains("depends on the .PHONY target 'stamp'")),
        "{refused:?}"
    );

    // A PHONY target depending on another phony target is fine — `all: test`
    // is ordinary, and requesting `all` is not requesting `test`.
    let all = mk::MakeTarget {
        name: "all".to_string(),
        phony: true,
        prereqs: vec!["stamp".to_string()],
        ..Default::default()
    };
    assert!(
        imp::refusals("", &[stamp, all]).is_empty(),
        "phony-to-phony edges must not be refused"
    );
}

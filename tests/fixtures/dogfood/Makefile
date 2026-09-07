# A build graph for the docs gate, small enough to read in one screen.
#
# README documents `forjar import-makefile Makefile -o forjar.yaml` followed by
# `forjar make link` and `forjar make clean`. Running those needs A Makefile,
# and it must be one whose recipes cannot touch anything outside the sandbox:
# every rule below writes only into the current directory, which the gate sets
# to a temp dir. `clean` is the `phony: true` action target the README names.
#
# EACH RULE WRITES THE FILE ITS TARGET IS NAMED AFTER, and that is not a style
# choice. `import-makefile` derives `output_artifacts: ["<target>"]` from the
# target name, and forjar then asks the host whether that artifact appeared —
# a command that exits 0 without producing it is reported NOT CONVERGED. A
# fixture whose `link:` rule wrote `app` would therefore fail the gate for a
# reason that is about the fixture and not about the documented command.
all: link

link: main.o
	printf 'linked\n' > link

main.o: main.c
	printf 'object\n' > main.o

main.c:
	printf 'int main(void){return 0;}\n' > main.c

clean:
	rm -f link main.o main.c

.PHONY: all clean

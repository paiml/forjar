# PMAT-240 — the lanes

One round, three sandboxed agy quorum lanes, width 3, `writes=false`, on the
diff with `--not-before` pinned to the dispatch instant.

| lane | conversation | exit | duration | verdict |
|---|---|---|---|---|
| 1 | conv-d9a35c6c | 0 | 311s | FAIL |
| 2 | conv-dcce07d0 | 0 | 433s | FAIL |
| 3 | conv-595732d6 | 0 | 441s | FAIL |

## The round that most earned its cost

The change was fifteen mechanical shell edits and one rule. The edits were
fine. **The rule had four holes, each openable with one line of shell**, and
all three lanes found at least two:

```
cat f | grep " # " | head -1     # the rule saw no pipeline at all
echo " | head "                   # the rule saw a fatal one
cat f \                           # the rule saw two lines
  | head -1
cmd || head -1                    # the rule saw a pipe
```

A rule against a hazard class is only worth the holes it does not have, and
four of them survived a careful author, a census, and a first round of testing.
They did not survive three readers each handed one instruction.

## The pattern, third time in this release

The standing instruction — quote any sentence a reader could check and find
false — caught a receipt describing a log file that did not contain what it
claimed. That is now three for three across this release: a truncated Gate R
quote in the 1.29.0 record, a 9-line proof log in PMAT-537, and "six gates pass
either way" here.

Each time the lane opened the file. That is the whole technique, and it has a
100% hit rate against receipts written by someone who ran the commands and
remembered the results rather than reading them back.

## The no-write rule

The brief opened with it and named the five earlier rounds in this session
whose lanes wrote into the repository root. The tree was clean afterwards.

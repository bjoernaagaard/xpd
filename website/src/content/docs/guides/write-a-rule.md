---
title: Write a rule
description: Add a rule for your own project with xpd new and xpd test, or add a rule to one of xpd's packs.
---

Run `xpd new rule` in your project to make a rule from a code pattern, and `xpd check` reports the
pattern from then on, in the terminal, in the editor and in the agent hook. A rule is an
[ast-grep rule](https://ast-grep.github.io/guide/rule-config.html) in a YAML file, with a Markdown
file beside it that explains the rule. The second half of this page shows how to add a rule to one
of the packs that ship with xpd.

## Add a rule to your project

The example adds a rule that reports `print` calls, in an empty folder. First, create the rule
project.

```sh
xpd new project --yes
```

xpd writes `sgconfig.yml` and `xpd.yml`, and the folders `rules/`, `rule-tests/` and `utils/`.
Second, create the rule from a pattern, where `$X` matches any one argument.

```sh
xpd new rule no-print --language python --pattern 'print($X)' --message 'Use the logger, so the output has a level and a time.' --with-test --yes
```

xpd writes the rule, its Markdown and an empty test file.

```text
.
├── sgconfig.yml                  ruleDirs: rules, utilDirs: utils, testConfigs: rule-tests
├── xpd.yml                       the project's packs, none yet
├── rules/no-print.yml            the rule
├── rules/no-print.md             what the rule reports and what to write instead
└── rule-tests/no-print-test.yml  id: no-print, valid: [], invalid: []
```

```yaml title="rules/no-print.yml"
id: no-print
language: python
severity: warning
message: Use the logger, so the output has a level and a time.
rule:
  pattern: print($X)
```

Third, check a file. Save the following as `app.py` and run `xpd check app.py`.

```python title="app.py"
import logging

print("loaded")
logging.info("loaded")
```

```console
$ xpd check app.py --format text
app.py:3:1: warning [local/no-print] Use the logger, so the output has a level and a time.
xpd: 1 issue
```

xpd names a project rule `local/<id>`, and it exits with 0, because the issue is a warning. Edit
`rules/no-print.md` to explain the rule, because the editor shows that text on hover.

## Test the rule

Give the rule code that it must leave alone and code that it must report, in its test file.

```yaml title="rule-tests/no-print-test.yml"
id: no-print
valid:
  - logging.info("loaded")
invalid:
  - print("loaded")
```

The first run of `xpd test --update-all` writes a snapshot of each report, and later runs of
`xpd test` compare the reports with the snapshot.

```console
$ xpd test --update-all
no-print: passed
no-print: updated
2 cases, 0 failed
$ xpd test
no-print: passed
no-print: passed
2 cases, 0 failed
```

`xpd test` exits with 1 when a case fails, for example when the rule misses an invalid case or
its report differs from the snapshot. Commit the `rule-tests/__snapshots__/` folder with the rule.

## Rules for one library

A rule about a library should run only on the files that import it. Put such rules in a pack,
named after the module that the rules are about.

```sh
xpd new rule old-api --pack mylib --pattern 'mylib.old($X)' --fix 'mylib.new($X)' --fix-safety safe --message 'old() was removed in 2.0.' --yes
```

`--pack` adds the pack to `xpd.yml` and writes the rule to `rules/mylib/`. The rule then runs on
files that import `mylib`, and `--fix` gives `xpd check --fix` the rewrite. `--since` and
`--until` limit the rule to library versions, which xpd reads from the project's virtual
environment. [`xpd new rule`](../../commands/new/rule/) lists every option, and
[`xpd check`](../../commands/check/#project-rules) says how xpd finds the project's rules.

## Add a rule to xpd

A rule that helps every user of a library belongs in one of xpd's packs. Each pack is a folder
under `rules/` in the repository, named after the module you import, and Cargo compiles the rules
and their Markdown into the binary.

```text
rules/pandas/
├── pack.yml        import aliases and package checks. A pack may leave it out
├── pack.md         the pack's introduction, with each rule id listed once under a heading
├── row-loop.yml    one file per rule, named after the rule id
├── row-loop.md     what the rule reports and how to fix it
└── ...
```

The build fails when a rule has no Markdown file, when a Markdown file names no rule, and when
`pack.md` doesn't list each rule once. `xpd rules`, `xpd find`, the editor hover, the MCP server
and the rule's page on this site all show the Markdown, so write it for a reader who has just
seen the issue.

```text
rules/<pack>/<id>.md
  first paragraph     what goes wrong, and what to write instead
  a diff              the reported code and the fixed code
  ## What the rule reports    the forms the rule finds, and the forms it leaves alone
  ## How to fix it            the fix, and what an unsafe fix can change
```

Build xpd and run it on the samples. Each pack has a folder of samples under `samples/<module>/`,
and a sample marks each line that a rule must report with a comment that names the rule.

```sh
cargo build
target/debug/xpd check samples/bad.py --format text
```

```python
old.setup(problem, ("n_gen", 7), seed=1)  # xpd: setup-positional-termination
```

A new folder under `rules/` is a new pack, and it needs no Rust change. Add it to
`website/src/packs.mjs` as well, so the site lists it.

### Settings of a rule and a pack

| Setting | What it does |
| --- | --- |
| `fix:` | One ast-grep rewrite for the rule. The rule must then say whether the fix keeps what the code does, with `metadata: { fix: safe }` or `metadata: { fix: unsafe }` |
| `aliases` in `pack.yml` | A fix may write an alias that the pack lists, such as `pd.`, and xpd applies the fix only in a file that imports the module under that alias |
| `members` in `pack.yml` | A fix may write a bare name that the pack lists with its module, such as `assert_allclose: numpy.testing`, and xpd applies the fix only where the file imports it |
| `metadata: { since: "3.1" }` | xpd skips the rule when the project's virtual environment holds an older version of the library |
| `metadata: { until: "0.6.1.6" }` | The rule's advice holds only before a release, and a second document with `since: "0.6.1.6"` gives the newer advice |
| [`files` and `ignores`](https://ast-grep.github.io/reference/yaml.html#files) | Globs that limit the files a rule runs on. Start a glob with `**/` to match at any depth |
| `kind: module` | A rule about the file itself, reported on line 1 |
| `everyFile: true` in `pack.yml` | The pack's rules run on every file, as for `rules/python/` |
| `packages` in `pack.yml` | Package checks, which warn when the virtual environment has the library and lacks a package, such as pyarrow for pandas |

### When a rule gets a fix

Give a rule a fix when its message names one rewrite of the code it matches, and mark the fix safe
only when the result can't differ.

```text
on(a rule document)
  if the message names one rewrite, and the matched code holds everything the rewrite needs
    if the fixed code gives the same object, or an equal value of the same type
      fix, with metadata: { fix: safe }
    else
      fix, with metadata: { fix: unsafe }     # a comment above the document says what can differ
  else
    no fix                                    # a comment above the document names what is missing
```

A rewrite that needs a value the file doesn't hold, such as a seed, a unit or a column name, gets
no fix, and so does advice to use another design. xpd parses each fixed file again with
[LibCST](https://github.com/Instagram/LibCST), and it drops a fix, and keeps the report, when the
fixed file no longer parses.

### Index the library of a new pack

Give a pack backed by a library a source index, so `xpd find` and `xpd api` know the library's
names. Install the library without its dependencies in an empty folder outside the repository, and
index it with Griffe 2.4.1. Use the Python version folder that uv created.

```sh
uv venv /var/tmp/xpd-source/venv
uv pip install --python /var/tmp/xpd-source/venv/bin/python --no-deps scikit-lego
uv run --with griffe==2.4.1 xpd index /var/tmp/xpd-source/venv/lib/python3.15/site-packages --package sklego
```

The command writes `indexes/sklego.json`, which you commit and never edit by hand.
[`xpd index`](../../commands/xpd-index/) shows how to add recipes from the library's
documentation. After you add or change a record, embed it and check that real questions still find
their entries.

```text
on(add or change records)
  cargo build                                                 # new and changed records get no vector
  target/debug/xpd find --write-vectors indexes/vectors.bin   # embeds only those records
  cargo run --quiet -- find --check indexes/questions.json    # builds again and checks
```

Add questions for the new pack to `indexes/questions.json`. When a question fails, keep it and
improve the ranking for the general pattern of words.

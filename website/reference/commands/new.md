Use `xpd new` to start a rule project or to add a rule, a test case or a utility to one. Run it
without a command, and it asks what to create and asks for each missing value. With `--yes` it
never asks, which suits scripts and agents.

## Examples

```sh
xpd new
xpd new project --pack team --yes
xpd new rule team/no-print --pattern 'print($X)' --message 'Use the project logger.' --with-test --yes
xpd new test team/no-print --yes
xpd new util print-call --lang python --pattern 'print($X)' --yes
```

The `project`, `rule` and `util` commands above, run in an empty folder, write these files. Each
folder that `project` makes also gets an empty `.gitkeep`.

```text
.
├── sgconfig.yml                       # ruleDirs: rules, utilDirs: utils, testConfigs: rule-tests
├── xpd.yml                            # the pack team, with ruleDirs: rules/team
├── rules/team/no-print.yml
├── rules/team/no-print.md             # the Markdown that explains team/no-print
├── rule-tests/team/no-print-test.yml  # id: team/no-print, valid: [], invalid: []
└── utils/print-call.yml
```

`--format json` prints `createdFiles` and `modifiedFiles` as absolute paths.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The files were written |
| 2 | A value is missing with `--yes`, a file it would write exists, or the rule does not parse. Nothing is written |

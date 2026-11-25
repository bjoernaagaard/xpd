Use `xpd rules new` to start a rule project or to add a rule, a test case or a utility to one. Run it
without a command, and it asks what to create and asks for each missing value. With `--yes` it
never asks, which suits scripts and agents.

## Examples

```console
$ xpd rules new project --pack team --yes
Created project
  created  /tmp/project/sgconfig.yml
  created  /tmp/project/xpd.yml
  created  /tmp/project/rules/.gitkeep
  created  /tmp/project/rule-tests/.gitkeep
  created  /tmp/project/utils/.gitkeep
  created  /tmp/project/rules/team/.gitkeep
$ xpd rules new rule team/no-print --pattern 'print($X)' --message 'Use the project logger.' --with-test --yes
Created rule team/no-print
  created  /tmp/project/rules/team/no-print.yml
  created  /tmp/project/rules/team/no-print.md
  created  /tmp/project/rule-tests/team/no-print-test.yml
$ xpd rules new util print-call --lang python --pattern 'print($X)' --yes
Created util print-call
  created  /tmp/project/utils/print-call.yml
```

The commands above write these files. Each folder that `project` makes also gets an empty
`.gitkeep`.

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

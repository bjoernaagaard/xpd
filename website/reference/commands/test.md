Use `xpd test` when you write your own rules. It checks the valid and invalid examples of each
project rule, and the snapshots of their issues and fixes. A snapshot is a saved file of the
issues and fixes that a rule gives on its invalid examples, such as
`rule-tests/__snapshots__/team-no-print-snapshot.yml`.

A test file has the ast-grep fields `id`, `valid` and `invalid`. A valid example must give no
issue from its rule, and an invalid example must give one. A case can set `path`, so that the
rule's `files` and `ignores` globs and the language detection see that file name. The default
engine runs each example through the same scanner as `xpd check`, with import gates, value
proofs and version bounds. `--engine structural` runs ast-grep's own test runner, which checks
plain rules without them.

## Examples

[`xpd new`](../new/) makes the project, the rule and an empty test file.

```console
$ xpd new project --pack team --yes --format json | jq -c .item
"project"
$ xpd new rule team/no-print --pattern 'print($X)' --message 'Use the project logger.' --with-test --yes --format json | jq -c .id
"team/no-print"
```

With this test file, the first run fails because no snapshot exists yet. `xpd test -U` then writes
the snapshot.

```yaml title="rule-tests/team/no-print-test.yml"
id: team/no-print
valid:
  - "log(1)"
invalid:
  - "import team\nprint(1)"
```

```console
$ xpd test
team/no-print: passed
team/no-print: snapshot mismatch
2 cases, 1 failed
$ xpd test -U
team/no-print: passed
team/no-print: updated
2 cases, 0 failed
```

`--format json` prints each case with its `id`, `source` and `status`, and the `failed` count.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | Every example and snapshot passed, or was updated |
| 1 | An example or a snapshot failed |
| 2 | The command could not run, for example because no case matched |

Use `new` to create a rule project, a rule, an empty test case or a utility without
prompts, as [`xpd new`](../../../commands/new/) does with `--yes`. `item.entity` says what to create, and
the other fields of `item` are the answers that `xpd new` would otherwise ask for.

## Example

The first call makes a project with the pack `team` in the server's root, here `/tmp/project`.

```json
{"item":{"entity":"project","pack":"team"}}
```

```json
{"createdFiles":["/tmp/project/sgconfig.yml","/tmp/project/xpd.yml","..."],"modifiedFiles":[],"item":"project"}
```

The second call adds a rule with its Markdown and an empty test case.

```json
{"item":{"entity":"rule","name":"team/no-print","pattern":"print($X)","message":"Use the project logger.","with_test":true}}
```

```json
{"createdFiles":["/tmp/project/rules/team/no-print.yml","/tmp/project/rules/team/no-print.md","/tmp/project/rule-tests/team/no-print-test.yml"],"modifiedFiles":[],"item":"rule","id":"team/no-print"}
```

## Errors

A missing value fails as it does with `xpd new --yes`, and so does a rule that exists already.
Nothing is written then. A `directory` outside the project root fails before xpd runs.

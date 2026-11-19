Use `new` to create a rule project, a rule, an empty test case or a utility without
prompts, as [`xpd new`](../../../commands/new/) does with `--yes`. `item.entity` says what to create, and
the other fields of `item` are the answers that `xpd new` would otherwise ask for.

## Example

In a project that `new` made with `{"entity": "project", "pack": "team"}`, the call below adds
a rule with its Markdown and an empty test case. The paths are those of a temporary folder.

```json
{"item":{"entity":"rule","name":"team/no-print","pattern":"print($X)","message":"Use the project logger.","with_test":true}}
```

```json
{"createdFiles":["/tmp/xpd-ref-check/tmpdb8ik91b/rules/team/no-print.yml","/tmp/xpd-ref-check/tmpdb8ik91b/rules/team/no-print.md","/tmp/xpd-ref-check/tmpdb8ik91b/rule-tests/team/no-print-test.yml"],"modifiedFiles":[],"item":"rule","id":"team/no-print"}
```

## Errors

A missing value fails as it does with `xpd new --yes`, and so does a rule that exists already.
Nothing is written then. A `directory` outside the project root fails before xpd runs.

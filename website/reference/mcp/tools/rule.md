Use `rule` to learn why a rule fires and how to fix what it reports. It returns the
rule with its documentation, which is the text of its [rule page](../../../rules/), and its
examples.

## Example

```json
{"package":"pandas","rule":"no-inplace"}
```

The result has `package`, `rule`, `severity`, `message`, `fix`, `since`, `url` and `doc`.

```json
{"package":"pandas","rule":"no-inplace","severity":"info","message":"inplace=True gives no speedup and blocks chaining. Reassign instead.","fix":"unsafe","since":null}
```

## Errors

A package or rule that does not exist fails with a message that names it. A client can ask the
server to complete `package` and `rule`.

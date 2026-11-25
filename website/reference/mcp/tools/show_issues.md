Use `show_issues` to show a person the issues as a table, in a host that renders
[MCP Apps](https://modelcontextprotocol.io/docs/extensions/apps). It runs as
[`xpd check`](../../../commands/check/) does and returns a view built with Prefab, which the host draws with the
server's renderer resource. A host that does not render apps gets the JSON of `check` as text.

## Example

```json
{"paths":["samples/bad.py"]}
```

The text content of the result is the report of `check`. Its first issue is shown without its
message, and `"..."` stands for the other issues.

```json
{"fixed":0,"fixable":0,"issues":[{"package":"pandas","rule":"row-loop","file":"samples/bad.py","line":5,"column":15,"severity":"warning"},"..."]}
```

## Errors

The errors are those of [`xpd check`](../../../commands/check/).

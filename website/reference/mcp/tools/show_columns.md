Use `show_columns` to show a person the columns as a table, in a host that renders
[MCP Apps](https://modelcontextprotocol.io/docs/extensions/apps). It runs as
[`xpd profile`](../../../commands/profile/) does and returns a view built with Prefab, which the host draws with the
server's renderer resource. A host that does not render apps gets the JSON of `profile` as text.

## Example

```json
{"file":"samples/data.csv"}
```

The text content of the result is the report of `profile`. Its first column is shown, and
`"..."` stands for the other column.

```json
{"file":"samples/data.csv","rows":3,"columns":[{"column":"a","dtype":"float64","nulls":1,"unique":2,"memory_bytes":24},"..."]}
```

## Errors

The errors are those of [`xpd profile`](../../../commands/profile/).

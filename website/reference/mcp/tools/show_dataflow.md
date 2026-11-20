Use `show_dataflow` to show a person the dataflow as a chart and a table, in a host that renders
[MCP Apps](https://modelcontextprotocol.io/docs/extensions/apps). It runs as
[`graph`](../graph/) does and returns a view built with Prefab, which the host draws with the
server's renderer resource. A host that does not render apps gets the JSON of `graph` as text.

## Example

```json
{"paths":["samples/graph/steps.py"]}
```

The text content of the result is the report of `graph`. The first step of its one pipeline is
shown without its columns, and `"..."` stands for the other steps.

```json
{"pipelines":[{"name":"features","file":"samples/graph/steps.py","line":51,"steps":[{"name":"totals","kind":"function","line":18},"..."]}]}
```

## Errors

The errors are those of [`graph`](../graph/).

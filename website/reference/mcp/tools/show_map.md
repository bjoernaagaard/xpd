Use `show_map` to render the module coupling and health map of [`xpd untangle`](../../../commands/untangle/) as an interactive Mermaid diagram and sortable hotspot table in hosts that render [MCP Apps](https://modelcontextprotocol.io/docs/extensions/apps). Hosts that do not render apps receive the structured JSON graph as text.

## Example

```json
{"paths":["samples/bad.py"]}
```

```json
{"total_modules":1,"nodes":["..."],"edges":[],"hidden_links":[]}
```

## Errors

Paths outside the project root and failing `xpd untangle --map` invocations return MCP tool errors with the command message.

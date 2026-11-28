Use `show_ledger` to inspect either the active recorded Duroxide run or a read-only survey for any of xpd's ten registered ledger workflows (`untangle`, `optimize`, `harden`, `discover`, `data-prep`, `ts-review`, `feature-review`, `model-review`, `opt-review`, or `chart-review`) in hosts that render [MCP Apps](https://modelcontextprotocol.io/docs/extensions/apps). Hosts that do not render apps receive the structured JSON payload as text.

## Example

```json
{"workflow":"harden","paths":["samples/bad.py"]}
```

```json
{"workflow":"harden","mode":"survey","payload":{"read":{"files":1}}}
```

## Errors

An unknown workflow name, a path outside the project root, or a failing ledger command returns an MCP tool error with the underlying message.

Use `outline` to map what files import and define before reading them in full, as
[`xpd outline`](../../../commands/outline/) does. It returns each item with its kind, signature, source range
and members, and runs no code.

## Example

```json
{"paths":["samples/graph/steps.py"],"options":{"items":"structure"}}
```

The result is a list with one entry for each file. This is the third item of the file, without
its ranges and members.

```json
{"name":"totals","symbolType":"function","signature":"def totals(orders: pd.DataFrame) -> pd.DataFrame","astKind":"function_definition","isExported":true}
```

## Errors

A path outside the project root fails before xpd runs. A path that cannot be read comes back as a tool error from the binary.

Use `rules` to list the rules that match some filters, such as every pandas rule
with an unsafe fix. It lists the rules of xpd and of the project, without their documentation,
and [`rule`](../rule/) reads one in full. `package` is a module name, such as `numpy` or
`sklearn`.

## Example

```json
{"package":"pandas","query":"inplace"}
```

The result has `rules`. These are their names and fixes.

```json
[{"rule":"inplace-on-column","fix":"unsafe"},{"rule":"inplace-result-assigned","fix":"unsafe"},{"rule":"no-inplace","fix":"unsafe"}]
```

## Errors

No match is an empty list. The project's own rules that cannot be read fail with the binary's
message.

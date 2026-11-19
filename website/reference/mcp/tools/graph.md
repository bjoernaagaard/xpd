Use `graph` after the agent adds, moves or renames a step of a pandas pipeline or a
node of an Apache Hamilton module, as [`xpd graph`](../../../commands/graph/) does. It reads which step or
node reads which, and reports what would fail, without running the code.

## Example

```json
{"paths":["samples/graph/steps_wrong_order.py"]}
```

The result holds `pipelines`, `nodes`, `inputs`, `unread` and `problems`. These are its
`problems`.

```json
[{"file":"samples/graph/steps_wrong_order.py","line":11,"severity":"error","message":"margin() reads the column total_cost, which costs() adds later in report(). The read raises KeyError. Move margin() after costs()."},{"file":"samples/graph/steps_wrong_order.py","line":27,"severity":"error","message":"costs_again() joins the column total_cost, which costs() added already. The join raises \"columns overlap\"."}]
```

## Errors

Problems in the code are a result, not an error. A path outside the project root fails before xpd runs.

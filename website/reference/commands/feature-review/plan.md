Use `xpd feature-review plan` in the [recorded feature review](../../feature-review/#recorded-stages).

Pass a JSON file or use `-` to read stdin. Record mechanical fixes with finding IDs. A design fix uses `"goal": "design"` and cites question IDs in `questions`. Use `answered` entries with `item`, `verdict` and `reason`.

## Example

```console
$ xpd feature-review plan --help
Record the plan of a round: the fixes in order, and the findings that are right as they are.
...
```

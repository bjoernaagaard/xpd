Use `xpd model-review plan` in a [recorded model review](../../model-review/#recorded-stages).

Pass a plan JSON file or `-` for stdin. Use `fixes` for ordered changes, `accepted` for findings kept with a reason, and `answered` for checklist entries with `item`, `verdict` and `reason`. The verdict must be `pass`, `na` or `accepted`.

## Example

```console
$ xpd model-review plan --help
Record the plan of a round: the fixes in order, and the findings that are right as they are.
...
```

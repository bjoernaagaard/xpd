Use `xpd opt-review plan` during a [recorded optimization review](../../opt-review/#recorded-stages).

Supply a JSON file or use `-` for stdin. A fix may cite `findings` or use `"goal":"design"` and cite `questions`. Put reasoned finding acceptances in `accepted` and checklist verdicts in `answered`. Supported checklist verdicts are `pass`, `na` and `accepted`.

## Example

```console
$ xpd opt-review plan --help | head -n 1
Record the plan of a round: the fixes in order, and the findings that are right as they are.
```

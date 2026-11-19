Use `xpd rules` to see which rules a project runs, or to read a rule's text without a browser.
It lists every rule and package check of every pack, whatever the project's Python has installed,
and the project's own rules unless you pass `--builtin`. The [rules](../../rules/) pages show the
same text.

## Examples

```console
$ xpd rules --format text --rule pandas/chained-assignment --rule pandas/no-inplace
error [pandas/chained-assignment] (unsafe fix) Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to $DF.loc[$ROW, $COL] instead.
info [pandas/no-inplace] (unsafe fix) inplace=True gives no speedup and blocks chaining. Reassign instead.
```

Each text line gives the severity, the rule, its fix, the first library version it applies to
when there is one, and the message. A name that starts with `$` stands for code from the checked
file, which `xpd check` fills in.

The JSON is a list with one object per rule, with `package`, `rule`, `severity`, `message`,
`fix`, `since`, `url` and `doc`. `fix` is `safe`, `unsafe` or `null`. `since` is the first
library version the rule applies to, or `null`. `doc` is the Markdown that explains the rule.

```console
$ xpd rules --builtin --format json | jq -c '.[] | select(.rule == "no-inplace") | {package, rule, severity, fix, since}'
{"package":"pandas","rule":"no-inplace","severity":"info","fix":"unsafe","since":null}
```

A rule file may hold several YAML documents, one for each form that the rule matches. `rules`
lists them as one rule, with the highest severity of the documents, the message of the first and
the least safe fix.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The rules were listed |
| 2 | xpd could not read the rules, or `--rule` named no rule |

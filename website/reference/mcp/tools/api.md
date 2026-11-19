Use `api` to read how to call a symbol before writing the call, as
[`xpd api`](../../../commands/api/) does. It returns the parameters as the source writes them, the summary,
the members and the rules that may apply. Add a member to its class, as in
`pandas.DataFrame.merge`, to read one method or property.

## Example

```json
{"name":"arch.unitroot.KPSS.pvalue"}
```

```json
{"kind":"symbol","package":"arch","name":"arch.unitroot.KPSS.pvalue","callable":"property","parameters":[],"parameters_written":[],"summary":"The p-value for the test statistic.","members":[],"rules":[],"signature":null}
```

## Errors

A name that the index does not hold fails with the binary's message.

```text
xpd api -- no.such.name exited with 2: xpd: no entry named no.such.name
```

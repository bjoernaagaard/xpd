Use `rewrite` to apply a structural replacement across files. It previews the
matches of `query`, asks the user to confirm, and only then writes the replacement with
`xpd run --update-all`. xpd does not check a rewrite or mark it safe the way it does a rule fix.

## Example

On `samples/bad.py`, the user declined the question, so the tool wrote nothing and
returned the preview. The match below is shown without its offsets.

```json
{"query":{"pattern":"pd.merge($$$A)","rewrite":"df.merge(other, on=\"k\")","language":"python"},"paths":["samples/bad.py"]}
```

```json
[{"text":"pd.merge(df, other, on=\"k\")","file":"samples/bad.py","replacement":"df.merge(other, on=\"k\")","lines":"m = pd.merge(df, other, on=\"k\")"}]
```

## Errors

A `query` without a `rewrite` template fails with "Provide a rewrite template." The other
errors are those of [`query`](../query/).

Use `query` to find code by its syntax, as [`xpd run`](../../../commands/run/) does, and to
preview a replacement. Give exactly one of `pattern` and `kind`. It changes no file, and
[`rewrite`](../rewrite/) writes a replacement.

## Example

```json
{"query":{"pattern":"$X.iterrows()","language":"python"},"paths":["samples/bad.py"]}
```

The result is a list with one entry for each match. This is the first, without its byte offsets
and metavariable ranges. Lines and columns count from zero.

```json
{"text":"df.iterrows()","file":"samples/bad.py","language":"Python","lines":"for i, row in df.iterrows():","replacement":null,"start":{"line":4,"column":14}}
```

## Errors

A query with both or neither of `pattern` and `kind` fails with "Provide exactly one of pattern
or kind." A path outside the project root fails before xpd runs. No match is an empty list, not an error.

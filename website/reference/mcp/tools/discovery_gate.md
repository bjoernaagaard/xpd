Use `discovery_gate` after choosing a library with [`find`](../find/) and
[`api`](../api/), before any project code runs. It checks that the files call the chosen symbols
with names and keywords that xpd's index knows, and that the record of the choice is consistent.
It reads the source and runs nothing. Pass its result to [`gates`](../gates/) as
`proof.discovery`.

```text
discovery_gate(paths, discovery)
  fail if choice.name is not among the candidates
  fail if the choice is a symbol and choice.symbols leaves it out
  for each direct call to a name from the modules of choice.symbols
    fail if api does not know it as a callable symbol
    fail on a keyword that is not among its indexed parameters, and on ** arguments
  fail for each chosen symbol that no call uses
```

It checks how the source spells each call, so it cannot see instance methods, runtime aliases,
argument values or whether the library is installed.

## Example

With the server's root at `samples/pipeline`, the call below passes the record in
`discovery.json`.

```json
{"paths":["train.py","predict.py","features.py"],"discovery":"(the contents of discovery.json)"}
```

The result repeats the record and the paths, and adds the gate.

```json
{"name":"indexed-api-calls","stage":"discovery","evidence":["api","source"],"passed":true,"detail":"Checked 1 direct imported calls for indexed callable names and literal keyword names in api's parameter lists. No code was run."}
```

## Errors

A record that does not match the schema fails before the gate runs. A failed check is a result,
with `passed: false` and a `detail` that names the call.

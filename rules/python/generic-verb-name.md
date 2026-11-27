A public function named with a bare verb, such as `load` or `clean`, does not say what it acts on, so a search for the name lands on every module that loads or cleans something else. Add the object of the verb to the name.

```diff lang="python"
-def clean(text: str) -> str:
+def clean_title(text: str) -> str:
     return " ".join(text.split())
```

## What the rule reports

The rule reports a module level function whose whole name is one of these verbs: `analyze`,
`apply`, `build`, `check`, `clean`, `compute`, `convert`, `create`, `diff`, `execute`, `fetch`,
`filter`, `format`, `get`, `handle`, `load`, `make`, `merge`, `normalize`, `parse`, `prepare`,
`process`, `read`, `render`, `run`, `sanitize`, `save`, `sort`, `transform`, `update`, `validate`
and `write`.

The rule leaves alone the cases in the list:

- A method, whose class already names the object, such as `Orders.load`.
- A function with a decorator, such as a CLI command or a pytest fixture, which is often
  registered under its name.
- `main`, and a name that already has an object, such as `load_orders`.
- A file that imports `hamilton`, where the function name is the name of a column.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Add the thing the verb acts on, and stop as soon as the name is the only one of its kind in the
project. `xpd discover` reports a name that two modules define, and a rename that it gates checks
that the new name is defined once. Rename every call in the same commit.

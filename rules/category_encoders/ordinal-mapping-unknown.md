With a `mapping` and no `handle_unknown`, a category the mapping does not name is coded -1 without a warning. Pass `handle_unknown="error"` or the value you want.

```diff lang="python"
-encoder = ce.OrdinalEncoder(cols=["size"], mapping=[{"col": "size", "mapping": {"small": 1, "medium": 2}}])
+encoder = ce.OrdinalEncoder(
+    cols=["size"],
+    mapping=[{"col": "size", "mapping": {"small": 1, "medium": 2, "large": 3}}],
+    handle_unknown="error",
+)
```

## What the rule reports

The rule reports `OrdinalEncoder(mapping=...)` without `handle_unknown`. A category that the
mapping doesn't name is then coded -1, with no error and no warning.

## How to fix it

`xpd check --fix --unsafe-fixes` adds `handle_unknown="error"` to `OrdinalEncoder(mapping=...)` calls so unseen categories raise instead of silently becoming `-1`. It is unsafe because another `handle_unknown` policy may be desired.

| `{"a": 1, "b": 2}` on the column a, b, c | Result |
| --- | --- |
| without `handle_unknown` | 1.0, 2.0, -1.0 |
| with `handle_unknown="error"` | `ValueError: Unexpected categories found in column c` at fit |

A key with a typing error, or a category that appears later, reaches the model as -1, which is
below every code that the mapping gives. Whether the mapping names every category can't be read from the code, so
the rule is info. A call that states `handle_unknown`, with any value, is left alone.

A function that calls `warnings.warn(..., DeprecationWarning)` in its body warns only when it runs. Mark it with `@deprecated` from `warnings` or `typing_extensions`, so the type checker reports every call to it.

```diff lang="python"
 import warnings
 
 
+@warnings.deprecated("Use load_orders_parquet")
 def load_orders_csv(path: str) -> str:
-    warnings.warn("Use load_orders_parquet", DeprecationWarning, stacklevel=2)
     return path
```

## What the rule reports

The rule reports a call to `warnings.warn` with `DeprecationWarning` or
`PendingDeprecationWarning` when the call is a statement straight in the body of a function.

The rule leaves alone the cases in the list:

- A warning inside an `if` or another block, which usually marks one argument as deprecated.
- A warning at module level, which marks the whole module.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`warnings.deprecated` is in the standard library from Python 3.13, and `typing_extensions`
has the same decorator for older versions. It raises the same `DeprecationWarning` when the
function runs. The type checker also reports each call to the function, so a caller learns of the
change when it writes the call. The message sits on the line above the definition, where a
search for the function's name lands.

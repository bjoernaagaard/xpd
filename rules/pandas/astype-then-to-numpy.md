`astype` builds a whole Series only for `to_numpy` to take its values. Pass the dtype to `to_numpy` instead.

```diff lang="python"
-limit = rows["Economic Maximum"].astype(float).to_numpy()
+limit = rows["Economic Maximum"].to_numpy(dtype=float)
```

## What the rule reports

The rule reports `s.astype(float).to_numpy()`.

It leaves a text dtype alone, such as `s.astype(str).to_numpy()`, because the two forms differ for it. On pandas 3.0.5 `astype(str).to_numpy()` gave an object array and `to_numpy(dtype=str)` a fixed-width one, and a datetime column gave `"2025-01-01"` with the first and `"2025-01-01T00:00:00.000000"` with the second.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `s.to_numpy(dtype=float)`, because a missing value in a nullable column can convert in another way.

xpd applies the replacement with `--fix --unsafe-fixes`.

`astype` builds a second Series with the new dtype only for `to_numpy` to take its values.
`to_numpy` takes the dtype itself. On pandas 3.1.1 both forms gave an equal float64 array for an
int column.

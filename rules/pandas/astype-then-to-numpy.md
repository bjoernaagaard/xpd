`astype` builds a whole Series only for `to_numpy` to take its values. Pass the dtype to `to_numpy` instead.

```diff lang="python"
-limit = rows["Economic Maximum"].astype(float).to_numpy()
+limit = rows["Economic Maximum"].to_numpy(dtype=float)
```

## What the rule reports

The rule reports `s.astype(float).to_numpy()`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `s.to_numpy(dtype=float)`, because a missing value in a nullable column can convert in another way.

xpd applies the replacement with `--fix --unsafe-fixes`.

`astype` builds a second Series with the new dtype only for `to_numpy` to take its values.
`to_numpy` takes the dtype itself. On pandas 3.1.1 both forms gave an equal float64 array for an
int column.

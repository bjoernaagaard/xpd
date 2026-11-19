Indexing with a mask or a list of positions gives a copy, so writing into that result never reaches the array. Index the array once with the selections combined.

```diff lang="python"
-x[x > 2][0] = -1
+x[np.flatnonzero(x > 2)[0]] = -1
```

## What the rule reports

The rule reports `x[x > 2][0] = v`, `x[[0, 1]][0] = v`, `m.flatten()[0] = v` and `x.astype(int)[0] = v`. Each one writes to a copy, so the array keeps its old values.

The rule leaves `x[1:4][0] = v` and `m[0][mask] = v` alone, because indexing with a slice or a whole number returns a view.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `x[0] = 9` for `x.astype(int)[0] = 9` and `x.copy()[0] = 9`, so the write reaches `x`, cast to the dtype of `x`. A mask or a list of positions, and `flatten`, have no fix.

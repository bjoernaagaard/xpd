Casting to `category` after `concat` makes a second pass over the data. On pandas 3.1 and later, pass `union_categories=True` to `concat` instead.

```diff lang="python"
-combined = pd.concat(parts).astype("category")
+combined = pd.concat(parts, union_categories=True)
```

## What the rule reports

The rule reports `pd.concat(parts).astype("category")` on pandas 3.1 and later.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `pd.concat(parts, union_categories=True)`.

The fix is unsafe, because the order of the categories differs. Parts that are not categorical also stay as they are.

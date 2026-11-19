`len(df[mask])` builds a filtered frame only to count its rows. Sum the mask instead, as in `int(mask.sum())`.

```diff lang="python"
-count = len(df[df["load"] > 50])
+count = int((df["load"] > 50).sum())
```

## What the rule reports

The rule reports `len(df[df["a"] > 50])`, which builds a filtered frame only to count its rows. The frame must be a plain name, and the rule doesn't run on test files.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `int((df["a"] > 50).sum())`.

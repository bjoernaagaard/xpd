Under copy-on-write a selection already behaves as a copy, so `.copy()` after it only costs memory and time. Drop the `.copy()`.

```diff lang="python"
-part = df[["load", "wind"]].copy()
+part = df[["load", "wind"]]
```

## What the rule reports

The rule reports `df[["a", "b"]].copy()`, `df[mask].copy()` and `df.loc[...].copy()`, because under copy-on-write the selection already behaves as a copy. It leaves a single text label and a slice alone, because on a dict or a NumPy array the copy keeps a later write away from the original.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix drops `.copy()`. Without the copy, `.values` and `.to_numpy()` of the result give an array that is read-only or shares memory with the source.

Methods such as `rename` and `assign` return a new frame, so a `.copy()` next to them is thrown away. Drop the `.copy()`.

```diff lang="python"
-clean = df.copy().rename(columns=names)
-tidy = df.reset_index(drop=True).copy()
+clean = df.rename(columns=names)
+tidy = df.reset_index(drop=True)
```

## What the rule reports

The rule reports `df.copy().rename(...)` and `df.rename(...).copy()`, also with `assign`, `reset_index`, `set_index`, `astype`, `fillna`, `drop(columns=)` and `pd.concat`, and it reports `drop(name, axis=1).copy()`. Under copy-on-write each of the methods returns a new frame, so the copy does the work twice. Each copy cost 4 to 10 milliseconds on 6 columns, and dropping it was 3 to 118 times faster.

The rule leaves a copy next to a slow method such as `sort_values` or `merge` alone, because dropping the copy made the code less than 2 times faster. It leaves `df.copy().pipe(f)` alone, because `f` may write to its argument.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix drops `.copy()`. Without the copy, `.values` and `.to_numpy()` of the result give an array that is read-only or shares memory with the source.

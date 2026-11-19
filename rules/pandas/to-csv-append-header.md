`to_csv(mode="a")` writes the header again on every call, so header rows end up among the data. Pass `header=False`, or write the header only on the first call.

```diff lang="python"
 for part in parts:
-    part.to_csv(path, mode="a", index=False)
+    part.to_csv(path, mode="a", index=False, header=not os.path.exists(path))
```

## What the rule reports

The rule reports `df.to_csv(path, mode="a")` without `header=`, which writes the header again on every call.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The repeated header rows are read back as data, so every number in the file comes back as text.

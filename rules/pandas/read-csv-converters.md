`converters` in `read_csv` calls a Python function once for every value, which was 3.6 to 5.1 times slower. Pass `dtype` for a type, or change the column after the read.

```diff lang="python"
-df = pd.read_csv(path, converters={"price": float, "name": str.strip})
+df = pd.read_csv(path, dtype={"price": float})
+df["name"] = df["name"].str.strip()
```

## What the rule reports

The rule reports `read_csv(..., converters=...)`, which calls a Python function once per value and was 3.6 to 5.1 times slower than `dtype` or a column method after the read, measured with the reader's default threads on a 32-core machine.

## How to fix it

xpd has no automatic fix, so make the change by hand.

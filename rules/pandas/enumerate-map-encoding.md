Numbering values with a dict built from `enumerate(unique())` looks up every value in Python, which was 7 to 17 times slower. Use `pd.factorize(s)[0]`, which numbers the values in the same order.

```diff lang="python"
-codes = df["region"].map({name: code for code, name in enumerate(df["region"].unique())})
+codes = pd.factorize(df["region"])[0]
```

## What the rule reports

The rule reports `df["k"].map({v: i for i, v in enumerate(df["k"].unique())})`, a dict of codes built by hand, which was 7 to 17 times slower than `pd.factorize`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `pd.factorize(s)[0]`, and `pd.factorize(s, sort=True)[0]` for the sorted values. factorize returns a NumPy array where the lookup returned a Series, and it gives a missing value the code -1.

The table compares the result of `factorize` with the result of the dict. Pass `sort=True` to `factorize` for the order of `sorted`.

| What | The dict | `pd.factorize` |
| --- | --- | --- |
| Order of the codes | the order in which the values first appear | the same |
| Type of the result | Series | NumPy array |
| Code of a missing value | a code of its own | -1 |

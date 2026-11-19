A lambda that only reads columns, as in `assign(b=lambda d: d["a"] * 2)`, hides a simple expression. On pandas 3.1 and later write it with `pd.col`, which gave an equal frame.

```diff lang="python"
-df = df.assign(total=lambda d: d["price"] * d["units"])
+df = df.assign(total=pd.col("price") * pd.col("units"))
```

## What the rule reports

The rule reports a lambda in `df.assign(name=lambda d: d["a"] * 2)`, `df.loc[lambda d: d["a"] > 2]` and `df[lambda d: d["a"] > 2]` that reads its frame only as `d["name"]`. On pandas 3.1.1, `pd.col` gave an equal frame for each kind of lambda body that the rule reports, as the table shows.

| Lambda body | Reported |
| --- | --- |
| arithmetic, a comparison, a mask operator such as `&` or `~`, a method, a reduction, or the `str` accessor | yes |
| a method that takes another column, or an earlier keyword of the same `assign` | yes |
| a column of a frame renamed earlier in the chain | yes |
| the frame used in another way, as in `d.a` or `len(d)` | no |
| a column passed to a function, as in `f(d["a"])` | no |
| another frame read by name, as in `other["x"]` | no |

## How to fix it

`xpd check --fix` applies the fix of this rule.

The safe fix writes `pd.col("a") * 2`. The rule runs on pandas 3.0 and later.

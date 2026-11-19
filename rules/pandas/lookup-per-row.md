Looking up a key in another frame inside `apply` reads that frame once for every row. Look up the whole column at once with `map`.

```diff lang="python"
-df["price"] = df["item"].apply(lambda item: prices.loc[item, "price"])
+df["price"] = df["item"].map(prices["price"])
```

## What the rule reports

The rule reports `df["k"].apply(lambda key: table.loc[key, "v"])` and `lambda key: table[table["k"] == key]["v"].iloc[0]`, which read another frame once per row. `map` with a Series looks up the whole column at once.

| Form | Rows | `map` with a Series was faster by |
| --- | --- | --- |
| the `.loc` lambda | 100,000 | 50 to 64 times |
| the mask lambda | 20,000 | 777 to 848 times |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df["k"].map(table["v"])`, and `df["k"].map(codes)` for `codes.get(key)` per value. A lookup through a mask has no fix.

The fix is unsafe, because a key that is absent gives a missing value where the lambda raised.

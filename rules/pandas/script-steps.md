A module that builds its frames in ten or more statements at its top, with no function that takes a frame, runs only from its first line to its last. Move the statements into step functions and join them with `pipe`.

```diff lang="python"
-df = pd.read_parquet("orders.parquet")
-df = df[df["price"] > 0]
-df["total"] = df["price"] * df["qty"]
-df = df.merge(cities, on="city", validate="many_to_one")
+def keep_sold(frame: pd.DataFrame) -> pd.DataFrame:
+    return frame.loc[pd.col("price") > 0]
+
+
+def add_total(frame: pd.DataFrame) -> pd.DataFrame:
+    return frame.assign(total=pd.col("price") * pd.col("qty"))
+
+
+def add_city(frame: pd.DataFrame, cities: pd.DataFrame) -> pd.DataFrame:
+    return frame.merge(cities, on="city", validate="many_to_one")
+
+
+df = (
+    pd.read_parquet("orders.parquet")
+    .pipe(keep_sold)
+    .pipe(add_total)
+    .pipe(add_city, cities)
+)
```

## What the rule reports

The rule reports the first statement of a module that has both of the following:

- ten or more statements at its top that give a name a frame, as in `df = df.dropna()`, or write into a frame, as in `df["x"] = ...`
- no function that takes a frame

xpd counts a statement only when it can prove the frame. The value must be a call such as `pd.read_parquet(...)`, a method of a frame that returns a frame, or a selection of rows from a frame. A statement inside a function, a loop or a branch does not count. A function takes a frame when a parameter is annotated `pd.DataFrame`, when a call passes it a frame, or when a `.pipe(...)` call names it.

The script and its rewrite as step functions gave an equal frame, by [`xpd equiv`](https://bjoernaagaard.github.io/xpd/commands/equiv/). [`xpd graph`](https://bjoernaagaard.github.io/xpd/commands/graph/) reads the `pipe` form as a pipeline of steps, and it lists the columns that each step reads and adds.

## How to fix it

xpd has no automatic fix, so make the change by hand.

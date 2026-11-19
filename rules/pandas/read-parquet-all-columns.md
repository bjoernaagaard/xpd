Selecting columns after `read_parquet` reads every column of the file first. Pass `columns=` to `read_parquet`, which read 2 of 50 columns 9 times faster.

```diff lang="python"
-prices = pd.read_parquet(path)[["id", "price"]]
+prices = pd.read_parquet(path, columns=["id", "price"])
```

## What the rule reports

The rule reports `pd.read_parquet(path)[["id", "price"]]` and `pd.read_parquet(path)["price"]`, which read every column and keep the named ones. The names must be strings. The two frames were equal, with the columns in the order of the list, and a file with a named index gave the same index both ways.

| Columns read with `columns=` | Faster by |
| --- | --- |
| 2 of 50 | 9.6 and 9.2 times in two runs, measured with the reader's default threads on a 32-core machine |
| 2 of 6 | 1.7 times |

## How to fix it

`xpd check --fix` applies the fix of this rule.

The safe fix writes `columns=["id", "price"]`.

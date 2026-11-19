`MultiIndex.from_tuples` with `zip` or `product` builds one Python tuple for every row. Pass the columns to `MultiIndex.from_arrays` or use `from_product`.

```diff lang="python"
-index = pd.MultiIndex.from_tuples(list(zip(df["region"], df["year"])), names=["region", "year"])
+index = pd.MultiIndex.from_arrays([df["region"], df["year"]], names=["region", "year"])
```

## What the rule reports

The rule reports `pd.MultiIndex.from_tuples(zip(df["k"], df["a"]))`, `from_tuples(itertools.product(keys, numbers))` and `from_tuples([(k, n) for k in keys for n in numbers])`, which build one Python tuple per row.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `pd.MultiIndex.from_arrays([df["k"], df["a"]])`, `pd.MultiIndex.from_product([keys, numbers])`, also for the comprehension with two `for` clauses.

On 1,000,000 rows the methods that take whole levels ran faster, as the table shows.

| Reported code | Write instead | Faster by |
| --- | --- | --- |
| `from_tuples` over `zip` | `from_arrays` | 27 times |
| `from_tuples` over a comprehension | `from_product` | 118 times |

The fix is unsafe, because without `names=` the method `from_arrays` takes the level names from the Series it gets. `from_tuples` left the level names empty. The rule leaves `itertools.product(keys, repeat=2)` alone.

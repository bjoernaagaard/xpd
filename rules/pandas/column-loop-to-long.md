A loop that builds one frame per column and stacks them does what `melt` does in one call. Use `df.melt` with `id_vars` and `value_vars`, which gave the same rows faster.

```diff lang="python"
-parts = [df[["id", year]].rename(columns={year: "sales"}).assign(year=year) for year in years]
-long = pd.concat(parts, ignore_index=True)
+long = df.melt(id_vars="id", value_vars=years, var_name="year", value_name="sales")
```

## What the rule reports

The rule reports a loop over column names that selects one column at a time, renames it to a shared name and stacks the parts with `pd.concat`. The loop can be a list comprehension, passed to `pd.concat` directly or through a name that the same function passes to it, or a `for` statement that appends. For 100,000 rows and 20 columns, `melt` gave the same 2,000,000 rows in the same order and ran 1.4 times faster.

## How to fix it

xpd has no automatic fix, so make the change by hand.

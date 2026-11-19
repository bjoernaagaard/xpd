`stack().reset_index()` gives a long frame with columns named `level_1` and `0`. Use `melt`, which names the columns and was 2.9 times faster.

```diff lang="python"
-long = df.set_index("id").stack().reset_index()
+long = df.melt(id_vars="id", var_name="year", value_name="sales")
```

## What the rule reports

The rule reports `df.set_index(keys).stack().reset_index()`, which builds a long frame whose new columns are named `level_1` and `0`. On 100,000 rows with 20 columns, `melt` gave the same values and ran 2.9 times faster. The table shows where the two results differ.

| What differs | `stack().reset_index()` | `melt` |
| --- | --- | --- |
| Names of the new columns | `level_1` and `0` | `variable` and `value`, or the names passed to `melt` |
| Order of the values | row by row | column by column |

## How to fix it

xpd has no automatic fix, so make the change by hand.

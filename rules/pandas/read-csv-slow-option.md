`engine="python"`, `sep=None` and `low_memory=False` turn off the fast C reader of `read_csv`, and `sep=None` was 68 times slower. Drop the option unless the file needs it.

```diff lang="python"
-df = pd.read_csv(path, engine="python")
+df = pd.read_csv(path)
```

## What the rule reports

The rule reports `read_csv(..., engine="python")`, `sep=None` and `low_memory=False`, which turn off the fast C reader. On 1,000,000 rows `sep=None` was 68 times slower than a named separator, measured with the reader's default threads on a 32-core machine. The rule leaves `engine="python"` alone when the call needs it, which is for `skipfooter` or for a separator of several characters other than `\s+`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix drops `engine="python"` when a comma comes before it. An option that only the python engine takes, such as a callable `on_bad_lines`, then raises ValueError. `low_memory=False` and `sep=None` have no fix, because the dtypes and the separator are not in the file.

The replacement assumes that the call uses no option that only the Python engine has. In place of `low_memory=False`, pass the dtypes of the columns with `dtype=`.

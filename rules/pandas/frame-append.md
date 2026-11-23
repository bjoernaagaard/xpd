`DataFrame.append` and `Series.append` were removed, so the call raises AttributeError on pandas 3. Collect the parts in a list and call `pd.concat` once.

```diff lang="python"
-out = pd.DataFrame()
-for part in parts:
-    out = out.append(part)
+out = pd.concat(parts)
```

## What the rule reports

The rule reports `df = df.append(row)` and `df.append(row, ignore_index=True)`, which raise AttributeError, because pandas 2.0 removed `DataFrame.append`.

## How to fix it

`xpd check --fix --unsafe-fixes` rewrites non-loop `df.append(other)` and `df.append(other, ignore_index=...)` calls to `pd.concat([df, other], ...)`. Calls inside loops or with dict literals remain manual so you can collect parts in a list and call `pd.concat` once.

The replacement assumes that `parts` is not empty. For single rows, collect the rows in a list and build the frame once.

The rule tells a frame from a list by the way the code uses the result, because `list.append` returns None and takes no keywords. The table shows what it reports.

| Code | Reported |
| --- | --- |
| an `append` whose result is assigned, returned, passed on or read | yes |
| an `append` that passes `ignore_index`, `verify_integrity` or `sort` | yes |
| an `append` on a module named like NumPy, such as `np`, `_np` or `anp` | no |
| `df.index.append(x)` and `df.columns.append(x)` | no, because `Index.append` still exists |
| `labels = labels.append(more)` on an Index held in a plain name | yes, by mistake |

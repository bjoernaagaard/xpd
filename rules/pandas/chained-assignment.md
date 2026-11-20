Under copy-on-write, assigning through a chain such as `df["a"][i] = v` writes to a copy, so the frame keeps its old values. Assign to the frame in one step with `.loc`.

```diff lang="python"
-df["load"][row] = value
+df.loc[row, "load"] = value
```

## What the rule reports

The rule reports `df["a"][i] = v`, `df[mask]["a"] = v`, `df.loc[mask]["a"] = v`, `df.a[mask] = v`, `df["a"].iloc[0] = v` and `df["a"].update(other)`, which write to a copy, so the frame keeps its old values. It also reports `df[col][mask] = v`, where a variable names the column.

The rule leaves alone code that more often fills a list or a dict, as the table shows.

| Left alone | Reason |
| --- | --- |
| a dict, list or set assigned to `x["a"][key]` | the code fills a nested dict |
| two plain keys, as in `rows[0]["name"] = v` | the value is more often a list or a dict |
| a name that the file shows holds a dict, as in `tables["direction"].loc[row] = v` after `tables = {...}` | the first key takes a frame or an array out of the dict, and a dict has no `.loc` |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `df.loc[i, "a"] = v`, and `df.loc[df.index[0], "a"] = v` for a write through `.iloc` or `.iat` on a column, which writes every row when the index holds the label twice. Other chains and `update` on a column have no fix.

The fix is unsafe, because xpd cannot always see whether the value on the left is a frame. A name
counts as a dict when the function or the module assigns it a dict, a dict comprehension or a call
of `dict`, or annotates it as `dict` or `Mapping`. A dict that a parameter without an annotation or
another function's result holds is still reported. The table shows which forms get a fix.

| Reported form | Fix |
| --- | --- |
| `df["a"][i] = v` and `df[col][mask] = v` | `df.loc[i, "a"] = v` and `df.loc[mask, col] = v` |
| `df[mask]["a"] = v` | `df.loc[mask, "a"] = v` |
| `df.loc[mask]["a"] = v` | `df.loc[mask, "a"] = v` |
| any other chain with a text label, such as `df.iloc[0]["a"] = v` | none |
| `df["a"].iloc[0] = v` and `df["a"].iat[0] = v` | `df.loc[df.index[0], "a"] = v` |
| `df.a[mask] = v` and `df["a"].update(other)` | none |

A mask held in a name gets the fix when the enclosing function assigns only masks to the name, such
as a comparison or a call to `isin`. The rule reports `df.a[mask] = v` only when the mask reads the
same frame, so `self.items[i > 3] = v` is left alone. It reports `update` only on a name such as
`df` or `frame`, because `state["seen"].update(extra)` is how code updates a dict of dicts.

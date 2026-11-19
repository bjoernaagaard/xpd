A tuple with `slice(None)` in `.loc` is hard to read. Write the same selection with `pd.IndexSlice` and a colon.

```diff lang="python"
-rows = df.loc[("a", slice(None), 1), :]
+rows = df.loc[pd.IndexSlice["a", :, 1], :]
```

## What the rule reports

The rule reports a tuple with `slice(None)` inside `.loc[...]`, on the rows or on the columns. `pd.IndexSlice["a", :, 1]` was equal to `("a", slice(None), 1)`, and `.loc` gave equal frames with the two. A slice that has bounds, as in `slice("a", "c")`, becomes `"a":"c"`, and `pd.IndexSlice` builds the same tuple.

## How to fix it

`xpd check --fix` applies the fix of this rule.

The safe fix writes `pd.IndexSlice["a", :, 1]`, and `pd.IndexSlice["a":"c", :]` for a slice with bounds. None for a slice call with keywords or unpacked arguments.

`OneHotEncoder` returns a sparse matrix by default, and pandas output raises ValueError on it. Pass `sparse_output=False`.

```diff lang="python"
-encoder = OneHotEncoder(handle_unknown="ignore")
+encoder = OneHotEncoder(handle_unknown="ignore", sparse_output=False)
 encoder.set_output(transform="pandas")
```

## What the rule reports

The rule reports pandas output on a OneHotEncoder that keeps its default `sparse_output=True`,
set with `set_output` or with `set_config(transform_output="pandas")`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `OneHotEncoder(sparse_output=False)` in a file that sets pandas output for every transformer, because the old form raises. The `set_output` form has no fix.

xpd applies the replacement with `--fix --unsafe-fixes`.

Each form raises "ValueError: Pandas output does not support sparse data" on scikit-learn 1.9.1.
The rule reports `set_output(transform="pandas")` on a OneHotEncoder, or on a Pipeline,
ColumnTransformer or FeatureUnion that holds one, in the same expression or assigned in an
earlier statement of the block. With `set_config` or `config_context` it reports every
OneHotEncoder in the file. A OneHotEncoder with `sparse_output=False` is left alone. Recall was 29
of 31.

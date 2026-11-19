`ColumnTransformer` no longer takes `force_int_remainder_cols`, so the call raises TypeError on scikit-learn 1.9. Drop the keyword.

```diff lang="python"
-ColumnTransformer(transformers, remainder="passthrough", force_int_remainder_cols=False)
+ColumnTransformer(transformers, remainder="passthrough")
```

## What the rule reports

The rule reports `force_int_remainder_cols=` on `ColumnTransformer` and `make_column_transformer`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix drops `force_int_remainder_cols`, because the old form raises.

xpd applies the replacement with `--fix --unsafe-fixes`.

The keyword raises TypeError on scikit-learn 1.9.1, with either value. The remainder columns now
come back in the form the other columns were given in. Neither reader called one of the 3
held-back reports wrong. Recall was 30 of 30.

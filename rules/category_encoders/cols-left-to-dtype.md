A category_encoders encoder without `cols` encodes only text and category columns, so a numeric column that a `ColumnTransformer` hands it passes through unchanged. Pass `cols="all"` to encode every column it gets.

```diff lang="python"
-columns = make_column_transformer((ce.BinaryEncoder(), ["zip_code"]), remainder="passthrough")
+columns = make_column_transformer((ce.BinaryEncoder(cols="all"), ["zip_code"]), remainder="passthrough")
```

## What the rule reports

The rule reports an encoder built without `cols` inside a `ColumnTransformer` or
`make_column_transformer`. Without `cols`, an encoder encodes the columns of the object, category
and string dtypes and passes every other column through.

## How to fix it

`xpd check --fix --unsafe-fixes` adds `cols="all"` to category_encoders-only estimators inside a `ColumnTransformer`. Edit `OneHotEncoder`, `OrdinalEncoder`, and `TargetEncoder` calls by hand.

Inside a `ColumnTransformer` the columns are already chosen, so a numeric column that was chosen
on purpose comes back as it went in. On a whole-number column of 5 values, the steps gave the following output:

| Step | Output |
| --- | --- |
| `(ce.OneHotEncoder(), ["zip"])` | 1 column, equal to the input, and `Warning: No categorical columns found` printed with `print`, where a warning would be raised |
| `(ce.OneHotEncoder(cols="all"), ["zip"])` | 5 columns |
| `ce.TargetEncoder()` | the column unchanged |

`cols="all"` was added in category_encoders 2.10, so the rule carries `since: "2.10"`. The rule
can't see the dtype of the chosen columns, and on text columns the old form is right. The rule is
therefore info. For `OneHotEncoder`, `OrdinalEncoder` and `TargetEncoder` the message also names the
scikit-learn class, which encodes every column it is given.

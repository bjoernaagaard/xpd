scikit-learn ships an encoder of the same name that was 2.7 to 4.5 times faster. Use the scikit-learn class unless you need something only the category_encoders class has.

```diff lang="python"
-encoder = ce.OneHotEncoder(cols=["city", "shop"])
+encoder = make_column_transformer(
+    (OneHotEncoder(handle_unknown="ignore", sparse_output=False), ["city", "shop"]),
+    remainder="passthrough",
+)
 X_encoded = encoder.fit_transform(X)
```

## What the rule reports

The rule reports a category_encoders `OneHotEncoder`, `OrdinalEncoder` or `TargetEncoder` that
uses nothing the scikit-learn class of the same name lacks.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`fit_transform` on 4 text columns of 50 levels each took the following times, as the median of 7
runs on one thread, category_encoders against scikit-learn:

| Class | 10,000 rows | 100,000 rows |
| --- | --- | --- |
| `OneHotEncoder` | 23.1 ms against 5.1 ms, 4.51 times | 113.5 ms against 42.4 ms, 2.68 times |
| `OrdinalEncoder` | 7.5 ms against 3.9 ms, 1.94 times | 47.0 ms against 28.1 ms, 1.67 times |
| `TargetEncoder` | 18.5 ms against 6.5 ms, 2.83 times | 99.7 ms against 46.7 ms, 2.14 times |

The two libraries do not return the same thing, so the rule is info and has no fix:

- category_encoders returns a DataFrame with every column of the input. scikit-learn returns an
  array of the encoded columns, and a `ColumnTransformer` chooses the columns.
- The scikit-learn `OrdinalEncoder` starts its codes at 0 and follows sorted order.
  category_encoders starts at 1 and follows the order in which the categories first appear.
- The scikit-learn `TargetEncoder`, added in scikit-learn 1.3, cross-fits in `fit_transform`, so
  a training row is never coded from its own label.
- The scikit-learn `OneHotEncoder` raises on an unseen category unless it is given
  `handle_unknown="ignore"`.

The rule reports a call only when every keyword is one of `cols`, `verbose`, `return_df` and, on
`OneHotEncoder`, `use_cat_names`. Any other keyword, such as `mapping`, `hierarchy`,
`handle_missing` or `smoothing`, asks for something the scikit-learn class does in another way
or not at all. An encoder handed to `PolynomialWrapper` or `NestedCVWrapper` is left alone, and
an encoder without `cols` inside a `ColumnTransformer` is left to
[`cols-left-to-dtype`](cols-left-to-dtype.md).

A Pipeline with the scikit-learn `TargetEncoder` was also tested against one with the
category_encoders class, and it didn't score better on the test rows. On 2,000 rows with a target
that depends on a column of 300 levels, the category_encoders `TargetEncoder` scored 0.655 AUC and
the scikit-learn class 0.616, over 30 seeds with a random forest. The rule therefore claims the
speed and nothing about the score.

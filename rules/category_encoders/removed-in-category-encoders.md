These forms were removed from category_encoders, so the call raises on 2.11.2. Use the replacement that the message names.

```diff lang="python"
-encoder = ce.OneHotEncoder(impute_missing=True)
+encoder = ce.OneHotEncoder(handle_missing="value")
```

## What the rule reports

The rule reports forms that a release of category_encoders removed and that raise on 2.11.2.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `list(...)` around `feature_names_out_` and `get_feature_names_out()` before `index` and `count`, which gave the same values on 2.6.4 and on 2.6.0, where the names were a list. The unsafe fix writes the two renamed names in `category_encoders.utils`, because the old mixin is another class and `get_categorical_cols` also names category and string columns, `handle_missing="value"` for `impute_missing=True` in a call without `handle_missing`, which no release took beside `impute_missing`, and dropping `randomized=` from a `LeaveOneOutEncoder` that has `sigma` or says `randomized=False`, because on 1.3.0 `sigma` alone added no noise. The other forms have no fix, and `remove`, `append`, `extend`, `insert` and `pop` have none because on a copy of the names they would change nothing that the code reads afterwards.

| Old form | Removed in | On 2.11.2 | Write instead |
| --- | --- | --- | --- |
| `impute_missing=` on any encoder | 2.0 | TypeError | `handle_missing="value"`, the default |
| `LeaveOneOutEncoder(randomized=True, sigma=0.05)` | 2.0 | TypeError | `LeaveOneOutEncoder(sigma=0.05)` |
| `utils.get_obj_cols(df)` | 2.6.3 | AttributeError | `utils.get_categorical_cols(df)` |
| `utils.TransformerWithTargetMixin` | 2.8 | AttributeError | `utils.SupervisedTransformerMixin` |
| `encoded.drop(columns=["intercept"])` after a contrast encoder | 2.7 | KeyError | remove the drop |
| `encoder.feature_names_out_.index("a")` | 2.7 | AttributeError | `list(encoder.feature_names_out_).index("a")` |
| `encoder.get_feature_names_out().index("a")` | 2.6.1 | AttributeError | `list(encoder.get_feature_names_out()).index("a")` |

Until 2.6 the four contrast encoders, `BackwardDifferenceEncoder`, `HelmertEncoder`,
`PolynomialEncoder` and `SumEncoder`, added a column named `intercept` that holds 1 on every
row. 2.7 stopped adding it, so a drop of the column raises `KeyError: "['intercept'] not found
in axis"`. The rule reports the drop only in a file that names one of the four classes, and it
leaves the following drops alone:

- a drop with `errors="ignore"`
- a drop on a frame that the file gives a column named `intercept` itself, with
  `frame["intercept"] = 1.0` or `frame.insert`
- every drop in a file that calls `assign(intercept=...)`

The last two drops take out the file's own column.

`feature_names_out_` was a list until 2.6 and is a NumPy array since 2.7. The result of
`get_feature_names_out` is a NumPy array since 2.6.1. `index`, `remove`, `append`, `extend`,
`insert`, `count` and `pop` raise `AttributeError` on an array. `feature_names_in_` is still a
list.

`get_obj_cols` is still in the wheels of 2.6.0, 2.6.1 and 2.6.2 and went away in 2.6.3, so its
document carries `since: "2.6.3"`.

The fixes are unsafe, because each turns code that raises into code that runs. `get_obj_cols`
named the columns of the object dtype, and `get_categorical_cols` also names the columns of the category and string dtypes. A file that
defines its own `get_obj_cols` is left alone.

pandas 3 stores text in the `str` dtype, so a check for `object` is False for a text column. Use `pd.api.types.is_string_dtype`.

```diff lang="python"
-is_text = df["name"].dtype == object
+is_text = pd.api.types.is_string_dtype(df["name"])
```

## What the rule reports

The rule reports `df["k"].dtype == object`, `select_dtypes(include="object")`, `is_object_dtype(df["k"])`, `df["k"].dtype in ("object", "category")` and `df["k"].dtype.name == "object"`, which are False for a text column, because pandas 3 stores text in the `str` dtype. It leaves `dtype.kind == "O"` alone, which is still True for a text column.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `pd.api.types.is_string_dtype(df["k"])` for `dtype == object`, `is_object_dtype` and `dtype.name == "object"`, and `["object", "str"]` in `select_dtypes`, which selects the same columns on pandas 3 and raises TypeError on pandas 2. `dtypes == object`, a test with `in`, and any other spelling of object in `select_dtypes`, such as `np.dtype("O")`, have no fix.

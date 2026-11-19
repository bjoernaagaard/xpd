pandas 3 stores datetimes in microseconds by default, so a check for `datetime64[ns]` is False for them. Use `pd.api.types.is_datetime64_dtype`, which is True for every unit.

```diff lang="python"
-is_time = df["time"].dtype == "datetime64[ns]"
-times = df.select_dtypes("datetime64[ns]")
+is_time = pd.api.types.is_datetime64_dtype(df["time"])
+times = df.select_dtypes("datetime64")
```

## What the rule reports

The rule reports `df["d"].dtype == "datetime64[ns]"`, `df["d"].dtype == "datetime64[ns, UTC]"`, `select_dtypes("datetime64[ns]")` and `is_datetime64_ns_dtype(df["d"])`. Datetimes default to microseconds in pandas 3, so on pandas 3.1 each comparison was False for a column parsed from text, and `select_dtypes` selected no column.

The rule leaves alone the code in the table, where the old comparison can still be right.

| Left alone | Reason |
| --- | --- |
| a file that calls `as_unit("ns")` or `astype("datetime64[ns]")` | the file makes nanosecond columns itself |
| the dtype of a value other than a column taken by a label, as in `df["time"].dtype`, or a name such as `series`, `col` or `index` | the value can be a NumPy array, which keeps the unit it was made with |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `pd.api.types.is_datetime64_dtype(df["d"])` for `dtype`, `str(dtype)` and `dtype.name`, `pd.api.types.is_datetime64_any_dtype` for `is_datetime64_ns_dtype`, `select_dtypes("datetime64")`, `(isinstance(df["d"].dtype, pd.DatetimeTZDtype) and str(df["d"].dtype.tz) == "UTC")` for a dtype with a time zone, and `select_dtypes("datetimetz")`, which also selects columns of other zones. Each is True for every unit. `dtypes`, and `str(dtype)` or `dtype.name` with a time zone, have no fix.

For a column with a time zone, write `isinstance(df["time"].dtype, pd.DatetimeTZDtype)` and `select_dtypes("datetimetz")`.

A column made from a nanosecond source, such as a NumPy array of `datetime64[ns]`, keeps the nanosecond unit, and the old comparison is still True for it.

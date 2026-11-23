Use `xpd equiv` to check a refactor of data code. It runs two scripts and tells you whether the
variable `result` that each one leaves is equal. DataFrames and Series are compared with
`pandas.testing`, and anything else with `numpy.testing`, so a list and an array with the same
values count as equal.

## Examples

```console
$ xpd equiv samples/equiv/old.py samples/equiv/new.py
{"equivalent": true}
$ xpd equiv samples/equiv/old.py samples/equiv/wrong.py
{"equivalent": false, "detail": "DataFrame.iloc[:, 2] (column name=\"w\") are different\n\nDataFrame.iloc[:, 2] (column name=\"w\") values are different (100.0 %)\n[index]: [0, 1, 2]\n[left]:  [2, 4, 6]\n[right]: [3, 6, 9]"}
```

Each script runs in its own Python process of the project's Python, in the folder where you run
`xpd equiv`. The second script gets no module, no `sys.path` entry and no variable from the
first. What a script prints goes to stderr, so stdout holds only the JSON. xpd gets each `result`
back with Python's `pickle`, so it cannot be an object of a class that the script defines.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | `{"equivalent": true}` |
| 1 | The results differ, with the difference in `detail` |
| 2 | A script failed, exited or left no `result` that pickle can save, or pandas is missing. The JSON has an `error` |

## Compare pandas and Polars

Add `--contract migration.json` when the old script produces a pandas
DataFrame and the new script produces a native Polars DataFrame. Both scripts
must use the same input data. The old result must have an unnamed default
RangeIndex. Materialize any LazyFrame in the new proof script.

The ordered schema arrays contain the columns and their logical types.
The supported types are `Boolean`, `Float64`, `Int64` and `String`.
For example, this pipeline reads `grams` and returns both `grams` and `kg`.

```json
{
  "input_schema": [{"name": "grams", "dtype": "Float64"}],
  "output_schema": [
    {"name": "grams", "dtype": "Float64"},
    {"name": "kg", "dtype": "Float64"}
  ],
  "row_identity": "position",
  "materialization": "lazy",
  "float_atol": {"kg": 1e-12},
  "missing_float_columns": ["grams", "kg"]
}
```

`materialization` records the production caller boundary as `lazy` or
`eager`. `float_atol` and `missing_float_columns` are optional and apply
to named Float64 output columns. Values are exact by default. Declare a
tolerance to allow a finite absolute difference. List a column under
`missing_float_columns` to treat null and NaN as the same missing value there.
Column order, native types, row order and signed infinities stay strict.

With a contract, exit 3 means the result kind or pandas index is unsupported.
Exit 2 also covers invalid contracts and missing Polars. The JSON reports
the comparison policy on match, mismatch and unsupported results.

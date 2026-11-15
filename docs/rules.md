# Rules

xpd has six rule packs: [pandas](#pandas), [NumPy](#numpy), [SciPy](#scipy),
[scikit-learn](#scikit-learn), [Hamilton](#hamilton) and [Python](#python). A library's rules run only on files that import it. The
Python rules run on every file.

## pandas

Every rule below has a direct heading link, its severity, fix safety, and before and after code.
Examples assume `import pandas as pd` and `import numpy as np`. Names such as `df`,
`output_columns` and `mapping` represent values supplied by your program. The replacement may
change behavior when repairing a bug or using an unsafe fix. Read the notes before applying it.

The rules follow pandas 3, where copy-on-write is always on. A fix that writes `pd.` or `np.`
applies only in a file with `import pandas as pd` or `import numpy as np`. In any other file xpd
reports the issue without a fix.

One rule asks for an API that pandas 3.1 added, `concat-then-category`. xpd reads the installed
pandas version from the name of the `pandas-<version>.dist-info` folder in the project's venv,
which is `$VIRTUAL_ENV` or the nearest `.venv` above the working directory, without starting
Python. On pandas 3.0 it skips that rule. When the venv has no pandas, or there is no venv, every
rule runs.

### Code that is wrong or broken on pandas 3

Each old form gives a wrong result, raises, or warns on pandas 3.0 and 3.1.

#### chained-assignment

**Severity:** error. **Fix:** unsafe: `df.loc[i, "a"] = v`. A write through `.iloc` or `.iat` on a column, and `update` on a column, have no fix.

Reports `df["a"][i] = v`, `df[mask]["a"] = v`, `df.loc[mask]["a"] = v`, `df.a[mask] = v`, `df["a"].iloc[0] = v` and `df["a"].update(other)`, which write to a copy. Also reports `df[col][mask] = v` with the column named by a variable. A dict, list or set assigned to `x["a"][key]` is a nested dict being filled and is left alone.

Example:

```python
df["load"][row] = value
```

Use instead:

```python
df.loc[row, "load"] = value
```

#### inplace-on-column

**Severity:** error. **Fix:** unsafe: `df["b"] = df["b"].fillna(0)`.

Reports `df["b"].fillna(0, inplace=True)`, which changes a copy. Also reports a column named by a variable, `df[column].fillna(0, inplace=True)`.

Example:

```python
df["load"].fillna(0, inplace=True)
```

Use instead:

```python
df["load"] = df["load"].fillna(0)
```

#### inplace-result-assigned

**Severity:** error. **Fix:** unsafe: `df = df.dropna()`.

Reports `df = df.dropna(inplace=True)`, which leaves `df` as None.

Example:

```python
df = df.dropna(inplace=True)
```

Use instead:

```python
df = df.dropna()
```

#### no-inplace

**Severity:** info. **Fix:** unsafe: `df = df.dropna()`.

Reports any other `inplace=True`, which gives no speedup and blocks chaining.

Example:

```python
df.dropna(inplace=True)
```

Use instead:

```python
df = df.dropna()
```

#### epoch-from-int-cast

**Severity:** error. **Fix:** none.

Reports `df["d"].astype("int64") // 10**9`, `index.asi8 // 10**9` and `df["d"].values.astype("float64") / 1e9`, which are off by 1,000 now that datetimes default to microseconds. Also reports `.view(np.int64)` divided the same way.

Example:

```python
seconds = df["time"].astype("int64") // 10**9
```

Use instead:

```python
seconds = (df["time"] - pd.Timestamp("1970-01-01")) // pd.Timedelta("1s")
```

The epoch example assumes dates without a time zone. For dates with a time zone, use a UTC epoch. Missing timestamps remain missing.

The replacement is advice. xpd does not apply it automatically.

#### compare-to-missing

**Severity:** error. **Fix:** unsafe: `df["b"].isna()`.

Reports `df["b"] == np.nan`, `df["k"].astype(str) == "nan"` and `df[df["b"] == None]`, which are never true.

Example:

```python
missing = df["load"] == np.nan
```

Use instead:

```python
missing = df["load"].isna()
```

#### in-column

**Severity:** error. **Fix:** unsafe: `(df["a"] == 2).any()`.

Reports `2 in df["a"]`, which checks the index labels. The receiver must be named like a frame: `df`, `frame`, or a name with one of them as a word. In real code `key in data["k"]` is far more often a test on a nested dict or a string.

Example:

```python
found = 2 in df["load"]
```

Use instead:

```python
found = (df["load"] == 2).any()
```

#### mask-precedence

**Severity:** error. **Fix:** none.

Reports `df["a"] > 1 & df["b"] < 3`, where `&` binds first.

Example:

```python
selected = df[df["load"] > 1 & df["wind"] < 3]
```

Use instead:

```python
selected = df[(df["load"] > 1) & (df["wind"] < 3)]
```

The replacement is advice. xpd does not apply it automatically.

#### write-through-values

**Severity:** error. **Fix:** none.

Reports `df["a"].values[0] = 9`, which raises because the array is read-only.

Example:

```python
df["load"].values[0] = value
```

Use instead:

```python
df.loc[df.index[0], "load"] = value
```

The replacement is advice. xpd does not apply it automatically.

#### object-dtype-check

**Severity:** error. **Fix:** unsafe: `pd.api.types.is_string_dtype(df["k"])`. `is_object_dtype`, a test with `in`, `dtype.name` and `select_dtypes` have no fix: `"str"` alone leaves out object columns that hold other Python objects, so write `["object", "str"]` to keep both.

Reports `df["k"].dtype == object`, `select_dtypes(include="object")`, `is_object_dtype(df["k"])`, `df["k"].dtype in ("object", "category")` and `df["k"].dtype.name == "object"`, which miss the new str dtype.

Example:

```python
is_text = df["name"].dtype == object
```

Use instead:

```python
is_text = pd.api.types.is_string_dtype(df["name"])
```

#### groupby-apply-reads-key

**Severity:** error. **Fix:** none.

Reports `df.groupby("k").apply(lambda g: g["k"])`, which raises KeyError.

Example:

```python
keys = df.groupby("region").apply(lambda group: group["region"].iloc[0])
```

Use instead:

```python
keys = df.groupby("region").apply(lambda group: group.name)
```

Read group.name only inside the function passed to apply. Multiple group keys give a tuple.

The replacement is advice. xpd does not apply it automatically.

#### removed-in-pandas-3

**Severity:** error. **Fix:** safe: `map`, `"ME"`, `"h"`, `"min"`, `sep=r"\s+"`, `lineterminator=`, `sep=` for a separator passed by position, `is_monotonic_increasing`, `np.` for `pd.np.` and `pd.json_normalize`. unsafe: `ffill()` and `bfill()`, which label and order the rows of a group-by differently, `"M"` in `resample`, `dt.isocalendar().week`, `writer.close()` and `pd.Series(x).value_counts()`. The other forms have no fix.

Reports `applymap`, `fillna(method=)`, frequencies such as `"M"`, `"H"` and `"T"`, `delim_whitespace=True`, `dt.week`, and the removed arguments and names listed below.

Example:

```python
filled = df["load"].fillna(method="ffill")
months = df.resample("M", on="time")["load"].sum()
```

Use instead:

```python
filled = df["load"].ffill()
months = df.resample("ME", on="time")["load"].sum()
```

#### reduction-level-argument

**Severity:** error. **Fix:** unsafe: `df.groupby(level=0).sum()`.

Reports `df.sum(level=0)` and the other reductions with `level=`, which raise TypeError.

Example:

```python
totals = df["load"].sum(level="region")
```

Use instead:

```python
totals = df["load"].groupby(level="region").sum()
```

#### stack-dropna

**Severity:** error, or warning for `future_stack=False`. **Fix:** none.

Reports `df.stack(dropna=False)`, which raises ValueError, and `df.stack(future_stack=False)`, which is deprecated.

Example:

```python
long = wide.stack(dropna=True)
```

Use instead:

```python
long = wide.stack().dropna(how="all")
```

For dropna=False, write stack() alone. The new stack keeps the rows that are all missing, which the old default dropped.

The replacement is advice. xpd does not apply it automatically.

#### frame-append

**Severity:** error. **Fix:** none.

Reports `df = df.append(row)` and `df.append(row, ignore_index=True)`, which raise AttributeError. A module named like NumPy, such as `np`, `_np` or `anp`, is left alone.

Example:

```python
out = pd.DataFrame()
for part in parts:
    out = out.append(part)
```

Use instead:

```python
out = pd.concat(parts)
```

The example assumes parts is not empty. For single rows, collect them in a list and build the frame once.

The replacement is advice. xpd does not apply it automatically.

#### str-replace-regex-default

**Severity:** error. **Fix:** unsafe: `regex=True`.

Reports `df["s"].str.replace(r"\d+", "")`, which replaces nothing, because `str.replace` reads its pattern as plain text since pandas 2.0.

Example:

```python
digits = df["phone"].str.replace(r"\D", "")
```

Use instead:

```python
digits = df["phone"].str.replace(r"\D", "", regex=True)
```

#### positional-key-on-labels

**Severity:** error. **Fix:** unsafe: `.iloc[0]`.

Reports `df["k"].value_counts()[0]` and `df.iloc[0][0]`, which raise KeyError, because an integer key on a Series is a label on pandas 3. A negative key, as in `mean()[-1]`, is reported too.

Example:

```python
top = df["region"].value_counts()[0]
first = df.iloc[0][0]
```

Use instead:

```python
top = df["region"].value_counts().iloc[0]
first = df.iloc[0, 0]
```

#### deprecated-in-pandas-3

**Severity:** warning. **Fix:** safe: `pd.Timestamp.now("UTC")`, drop `copy=`. unsafe: delete the copy-on-write option line, because `pd.get_option` reads the value back, and drop `copy=` from `astype`.

Reports `pd.set_option("mode.copy_on_write", True)`, `copy=False` and `pd.Timestamp.utcnow()`. `astype(copy=)` is reported on a column taken by a label or a receiver named like a frame, because NumPy's `astype` has the same keyword.

Example:

```python
now = pd.Timestamp.utcnow()
```

Use instead:

```python
now = pd.Timestamp.now("UTC")
```


`removed-in-pandas-3` also reports these, each of which raises on pandas 3.1:

- Arguments of `read_csv`: `error_bad_lines`, `warn_bad_lines`, `squeeze`, `verbose`, `keep_date_col`,
  `infer_datetime_format`, `mangle_dupe_cols`, `prefix`, `date_parser`, and a separator passed by position.
  For bad lines, pass `on_bad_lines="skip"`.
- Writing: `to_csv(line_terminator=)`, a separator passed to `to_csv` by position, `to_excel(encoding=)`,
  `read_excel(squeeze=)` and `writer.save()` on a name that holds the word writer.
- Names: `pd.value_counts`, `is_monotonic`, `mad`, `as_matrix`, `ix`, `pd.np`, `pd.datetime`,
  `pd.Int64Index`, `pd.UInt64Index`, `pd.Float64Index`, `pd.io.json.json_normalize`, `pandas.util.testing`,
  `SettingWithCopyWarning`, the option `mode.use_inf_as_na` and `pd.to_numeric(errors="ignore")`.
- Time series: `tshift`, `pad()`, `first("3D")` and `last("3D")`, `resample(loffset=, base=, kind=)`,
  `date_range(closed=)`, `to_datetime(infer_datetime_format=)` and `to_datetime(errors="ignore")`,
  `pct_change(fill_method="pad")`, `pd.Timedelta("5T")`, and `astype("timedelta64[D]")`.
- `iteritems()`, `is_lexsorted()`, `lexsort_depth` and `groupby(axis=1)`.

The `writer.close()` fix is unsafe, because xpd knows the writer only by its name. `DataFrame.lookup`
was removed too and is not reported, because other libraries have a method of that name.

The new `chained-assignment` shapes: `df.a[mask] = v` is reported when the mask reads the same
frame, which tells it from `self.items[i > 3] = v`. `df["a"].update(other)` is reported only on a name such as `df` or
`frame`, because `state["seen"].update(extra)` is how a dict of dicts is updated. `compare-to-missing` reports `None` only
inside a subscript, because `config["mode"] == None` on a dict is valid Python. `object-dtype-check`
leaves `dtype.kind == "O"` alone, which is still true for a text column.

`str-replace-regex-default` reports only a pattern that cannot be meant as plain text: a class such
as `\d`, a set in brackets, `.*` or `.+`, a `^` at the start, or `a|b`. It leaves `"."`, `"$"` and
`"("` alone. Its fix is unsafe, because a set in brackets can be meant as text.
`positional-key-on-labels` reports `value_counts()[0]`, `df.iloc[i][0]`, `df.dtypes[0]` and a frame
reduction such as `df.isna().sum()[0]`. Its fix is unsafe: on labels that are whole numbers the old
form still works, and there `value_counts()[0]` is the count of the value 0. The frame reductions have no fix.

The `reduction-level-argument` fix applies when `level=` is the only argument, and it is unsafe
because the old form does not run on pandas 3, so the two cannot be compared. `stack-dropna` has no
fix, because the replacement depends on the value of `dropna`. `frame-append` reports an `append`
whose result is assigned, returned, passed on or read, or that passes `ignore_index`,
`verify_integrity` or `sort`, because `list.append` returns None and takes no keywords. It leaves
`df.index.append(x)` and `df.columns.append(x)` alone, because `Index.append` still exists. An
Index held in a plain name, as in `labels = labels.append(more)`, is reported by mistake.

The `chained-assignment` fix covers the three shapes above. A mask held in a name is fixed when
the enclosing function assigns that name only masks, such as a comparison or a call to `isin`.
Any other chain, such as `df.iloc[0]["a"] = v`, is reported without a fix. Two plain keys, as in
`rows[0]["name"] = v`, are left alone, because they are more often a list or a dict.

`in-column` and `inplace-on-column` can report a dict, such as `"x" in data["names"]`, because xpd
cannot tell it from a frame. Their fixes are unsafe for that reason.

`removed-in-pandas-3` rewrites a frequency where the call takes only datetimes, such as
`date_range`, `Grouper`, `floor` and `rolling`. Inside `resample` and `shift` the fix for `"M"`,
`"Q"` and `"Y"` is unsafe, because those aliases stay valid when the index holds periods. It
leaves `to_period("M")` and `asfreq("M")` alone, and it reports `"A"` inside `resample`, `asfreq`
and `shift` without a fix, because the new alias is `"YE"` for datetimes and `"Y"` for periods.

`deprecated-in-pandas-3` deletes the copy-on-write option only at the top level of a file, and
reports it without a fix inside a function or another block. Dropping `copy=` from `astype` is
unsafe, because a numpy array also has `astype(copy=False)`, and there it can return the same
array.

The `no-inplace` fix applies only to a call that stands alone as a statement, on a name, attribute
or subscript. The inplace fixes drop comments between the call's arguments.

### Shorter forms, including the pandas 3.0 and 3.1 API

These rules are warnings, apart from those marked info. Most of them make code shorter and do not change its speed.

#### sort-then-reset-index

**Severity:** info. **Fix:** unsafe: `df.sort_values("b", ignore_index=True)`. `reset_index` always gives a RangeIndex, and `ignore_index=True` keeps the old index when the method has nothing to do, as on an empty frame.

Reports `df.sort_values("b").reset_index(drop=True)`, also after `sort_index`, `dropna`, `drop_duplicates`, `explode` and `sample`.

Example:

```python
ordered = df.sort_values("load").reset_index(drop=True)
```

Use instead:

```python
ordered = df.sort_values("load", ignore_index=True)
```

#### set-index-then-resample

**Severity:** warning. **Fix:** unsafe: `df.resample("D", on="d").sum()`, which raises on an empty frame where the indexed form returned.

Reports `df.set_index("d").resample("D").sum()`.

Example:

```python
daily = df.set_index("time").resample("D")["load"].sum()
```

Use instead:

```python
daily = df.resample("D", on="time")["load"].sum()
```

#### subtract-shift

**Severity:** warning. **Fix:** unsafe: `df["b"].diff()`.

Reports `df["b"] - df["b"].shift()`.

Example:

```python
change = df["load"] - df["load"].shift()
```

Use instead:

```python
change = df["load"].diff()
```

#### range-mask

**Severity:** warning. **Fix:** unsafe: `df["a"].between(10, 20)`.

Reports `(df["a"] >= 10) & (df["a"] <= 20)`.

Example:

```python
inside = (df["load"] >= 10) & (df["load"] <= 20)
```

Use instead:

```python
inside = df["load"].between(10, 20)
```

`between` is a method of a Series. An index and a NumPy array have none, so comparisons on
`df.index`, `df.index.hour`, `df.columns`, `df["load"].values` and `df["load"].array` are left
alone.

#### groupby-then-reset-index

**Severity:** warning. **Fix:** unsafe: `df.groupby("k", as_index=False)[["a", "b"]].sum()`.

Reports `df.groupby("k")[["a", "b"]].sum().reset_index()`.

Example:

```python
totals = df.groupby("region")["load"].sum().reset_index()
```

Use instead:

```python
totals = df.groupby("region", as_index=False)["load"].sum()
```

#### rename-after-agg

**Severity:** warning. **Fix:** none.

Reports `g.agg({"b": "mean"}).rename(columns={"b": "b_mean"})`.

Example:

```python
summary = df.groupby("region").agg({"load": "mean"}).rename(columns={"load": "mean_load"})
```

Use instead:

```python
summary = df.groupby("region").agg(mean_load=("load", "mean"))
```

The replacement is advice. xpd does not apply it automatically.

#### merge-indicator-anti-join

**Severity:** warning. **Fix:** none.

Reports `m[m["_merge"] == "left_only"]` after `merge(..., indicator=True)`.

Example:

```python
merged = left.merge(right, on="key", how="left", indicator=True)
unmatched = merged[merged["_merge"] == "left_only"]
```

Use instead:

```python
unmatched = left.merge(right, on="key", how="left_anti")
```

The anti join returns the left columns, without the indicator or the right-hand columns. Use it when the result needs unmatched left rows rather than the full merged schema.

The replacement is advice. xpd does not apply it automatically.

#### value-counts-share

**Severity:** warning. **Fix:** unsafe: `df["k"].value_counts(normalize=True)`.

Reports `df["k"].value_counts() / len(df["k"])`.

Example:

```python
shares = df["region"].value_counts() / len(df["region"])
```

Use instead:

```python
shares = df["region"].value_counts(normalize=True)
```

#### nested-np-where

**Severity:** warning. **Fix:** none.

Reports `np.where(a < 20, "low", np.where(a < 70, "mid", "high"))`.

Example:

```python
band = np.where(df["load"] < 20, "low", np.where(df["load"] < 70, "mid", "high"))
```

Use instead:

```python
band = pd.Series("high", index=df.index).case_when([
    (df["load"] < 20, "low"),
    (df["load"] < 70, "mid"),
])
```

case_when uses the first matching condition. It returns a Series rather than a NumPy array.

The replacement is advice. xpd does not apply it automatically.

#### query-string

**Severity:** info. **Fix:** none.

Reports `df.query("a > 50 and b < 0")`.

Example:

```python
selected = df.query("load > 50 and wind < 3")
```

Use instead:

```python
selected = df.loc[(pd.col("load") > 50) & (pd.col("wind") < 3)]
```

The replacement is advice. xpd does not apply it automatically.

#### filter-items

**Severity:** warning. **Fix:** unsafe: `df[cols]`.

Reports `df.filter(items=cols)`.

Example:

```python
selected = df.filter(items=output_columns)
```

Use instead:

```python
selected = df[output_columns]
```

The replacement raises when a requested column is absent. filter silently skips it, which is why the fix is unsafe.

#### inline-column-list

**Severity:** info. **Fix:** none.

Reports `df[["k", "a", "b"]]` and `df.groupby(["k", "s", "raw"])`, three or more names written inside the expression.

Example:

```python
selected = df[["load", "wind", "solar"]]
```

Use instead:

```python
output_columns = ["load", "wind", "solar"]
selected = df[output_columns]
```

The replacement is advice. xpd does not apply it automatically.

#### concat-then-category

**Severity:** warning. **Fix:** unsafe: `pd.concat(parts, union_categories=True)`.

Reports `pd.concat(parts).astype("category")`, on pandas 3.1.

Example:

```python
combined = pd.concat(parts).astype("category")
```

Use instead:

```python
combined = pd.concat(parts, union_categories=True)
```


xpd has no rule for or against a lambda in `assign`. A lambda and `pd.col` gave the same result
at the same speed, and a lambda also works with `np.where` and with calculations across columns.
`set-index-then-resample` applies only when a named aggregation such as `sum` or `mean` follows,
because upsampling with `asfreq` raises with `on=`, and a function passed to `agg` sees other
data. The `range-mask` fix is unsafe, because the operand may be a numpy array, which has no
`between`. The `subtract-shift` fix is unsafe, because `diff` wraps around on an unsigned integer
column and returns booleans for a boolean column. The `groupby-then-reset-index` fix is unsafe,
because a key that is not a column comes back in another place. The `value-counts-share` fix names
the result proportion instead of count and leaves missing values out of the total.
`query-string` leaves a string that reads like SQL alone, such as `conn.query("SELECT ...")`, and
it asks for `df.loc[pd.col("a") > 50]`, which type checkers accept. The `filter-items` fix is
unsafe, because `df[cols]` raises on a label that is absent, where `filter` skipped it.
`inline-column-list` reports a list of three or more text literals in `df[[...]]`, in
`df.loc[rows, [...]]`, as an argument of `groupby`, `sort_values`, `set_index`, `select` or
`drop_duplicates`, and in `by=`, `subset=`, `on=`, `left_on=`, `right_on=`, `columns=`,
`id_vars=`, `value_vars=` or `usecols=`. It leaves `pd.DataFrame(data, columns=[...])` alone,
because the list there defines the frame. It asks for a named list, such as
`output_columns = ["k", "a", "b"]`, passed by name. The `concat-then-category`
fix is unsafe, because the category order differs, and parts that are not categorical stay so.

### Python work per row or per value

#### apply-known-function

**Severity:** warning. **Fix:** unsafe: `df["s"].str.len()`, which raises on a column that is not text, `df["b"].isna()`, `pd.to_datetime(df["raw"])`, `df["b"].round(2)`.

Reports `df["s"].apply(len)`, `df["b"].apply(pd.isna)`, `df["raw"].apply(pd.to_datetime)` and `df["b"].apply(lambda v: round(v, 2))`.

Example:

```python
lengths = df["name"].apply(len)
```

Use instead:

```python
lengths = df["name"].str.len()
```

#### literal-contains

**Severity:** warning. **Fix:** unsafe: `str.contains("ta_", regex=False)`. On a column of Python objects, a value that is not text gives NaN with a pattern and False with `regex=False`.

Reports `df["s"].str.contains("ta_")`, a pattern with no regex character.

Example:

```python
matches = df["name"].str.contains("load_")
```

Use instead:

```python
matches = df["name"].str.contains("load_", regex=False)
```

#### copy-after-indexing

**Severity:** warning. **Fix:** unsafe: drop `.copy()`. Without the copy, `.values` and `.to_numpy()` of the result give an array that is read-only or shares memory with the source.

Reports `df[["a", "b"]].copy()`, `df[mask].copy()` and `df.loc[...].copy()`.

Example:

```python
part = df[["load", "wind"]].copy()
```

Use instead:

```python
part = df[["load", "wind"]]
```

#### lookup-per-row

**Severity:** warning. **Fix:** unsafe: `df["k"].map(table["v"])`, and `df["k"].map(codes)` for `codes.get(key)` per value. A lookup through a mask has no fix.

Reports `df["k"].apply(lambda key: table.loc[key, "v"])` and `lambda key: table[table["k"] == key]["v"].iloc[0]`, which read another frame once per row.

Example:

```python
df["price"] = df["item"].apply(lambda item: prices.loc[item, "price"])
```

Use instead:

```python
df["price"] = df["item"].map(prices["price"])
```

#### apply-recomputes-aggregate

**Severity:** warning. **Fix:** none.

Reports `df["b"].apply(lambda v: v - df["b"].mean())`, which computes the mean of the whole column again for every row.

Example:

```python
centered = df["load"].apply(lambda value: value - df["load"].mean())
```

Use instead:

```python
centered = df["load"] - df["load"].mean()
```

The replacement is advice. xpd does not apply it automatically.

#### enumerate-map-encoding

**Severity:** warning. **Fix:** none.

Reports `df["k"].map({v: i for i, v in enumerate(df["k"].unique())})`, a dict of codes built by hand.

Example:

```python
codes = df["region"].map({name: code for code, name in enumerate(df["region"].unique())})
```

Use instead:

```python
codes = pd.factorize(df["region"])[0]
```

factorize numbers the values in the order they first appear, as the dict does. It returns a NumPy array, and it gives a missing value the code -1 where the dict gave it a code of its own. Pass `sort=True` for the order of `sorted`.

The replacement is advice. xpd does not apply it automatically.

#### copy-in-chain

**Severity:** warning. **Fix:** unsafe: drop `.copy()`. Without the copy, `.values` and `.to_numpy()` of the result give an array that is read-only or shares memory with the source.

Reports `df.copy().rename(...)` and `df.rename(...).copy()`, also with `assign`, `reset_index`, `set_index`, `astype`, `fillna`, `drop(columns=)` and `pd.concat`. Also reports `drop(name, axis=1).copy()`.

Example:

```python
clean = df.copy().rename(columns=names)
tidy = df.reset_index(drop=True).copy()
```

Use instead:

```python
clean = df.rename(columns=names)
tidy = df.reset_index(drop=True)
```

#### row-loop

**Severity:** warning. **Fix:** none.

Reports a loop over `iterrows()`, a loop over `itertuples()` or `to_dict("records")` that only computes from the row values, and `for i in range(len(df))` with `df.iloc[i]`. It does not run on test files.

Example:

```python
total = sum(row.load * row.weight for row in df.itertuples())
```

Use instead:

```python
total = (df["load"] * df["weight"]).sum()
```

The example sums numeric columns without missing values. Check missing values and floating point rounding when replacing a Python reduction. Keep a loop when its work cannot be expressed on columns.

A loop over `itertuples()` or `to_dict("records")` that hands the row or its values to a function,
as in `orders = [Order(id=row["id"], total=row["total"]) for row in df.to_dict("records")]`, turns
rows into Python objects. Columns cannot do that, so it is left alone. A cast such as
`int(row["a"])` and a list method such as `out.append(...)` do not count as a function here.
`iterrows()` is reported in every loop, because it builds a Series for each row: use
`itertuples()` or `to_dict("records")` when each row has to become an object.

The replacement is advice. xpd does not apply it automatically.

#### apply-axis-1

**Severity:** warning. **Fix:** none.

Reports `df.apply(f, axis=1)`, and `agg` and `transform` with `axis=1` and a lambda or a `"_".join`.

Example:

```python
total = df.apply(lambda row: row["load"] + row["wind"], axis=1)
```

Use instead:

```python
total = df["load"] + df["wind"]
```

The replacement is advice. xpd does not apply it automatically.

#### apply-text-method

**Severity:** warning. **Fix:** unsafe: `df["s"].str.startswith("A")`.

Reports `df["s"].map(lambda v: v.startswith("A"))`.

Example:

```python
names = df["name"].map(lambda name: name.upper())
```

Use instead:

```python
names = df["name"].str.upper()
```

#### apply-date-part

**Severity:** warning. **Fix:** unsafe: `df["d"].dt.year`.

Reports `df["d"].apply(lambda t: t.year)`.

Example:

```python
years = df["time"].apply(lambda time: time.year)
```

Use instead:

```python
years = df["time"].dt.year
```

#### apply-condition

**Severity:** warning. **Fix:** none.

Reports `df["a"].apply(lambda v: 1 if v > 50 else 0)` and `lambda v: min(v, 1.0)`. A lambda that reduces its argument, as in `x.isnull().all()`, receives a whole Series per group and is left alone.

Example:

```python
flag = df["load"].apply(lambda value: 1 if value > 50 else 0)
```

Use instead:

```python
flag = pd.Series(np.where(df["load"] > 50, 1, 0), index=df.index)
```

The explicit Series constructor preserves the row labels. Check the result name, dtype and missing-value behavior.

The replacement is advice. xpd does not apply it automatically.

#### apply-arithmetic

**Severity:** warning. **Fix:** unsafe: `df["b"] * 2 + 1`.

Reports `df["b"].apply(lambda v: v * 2 + 1)`.

Example:

```python
scaled = df["load"].apply(lambda value: value * 2 + 1)
```

Use instead:

```python
scaled = df["load"] * 2 + 1
```

#### builtin-on-column

**Severity:** warning. **Fix:** unsafe: `df["a"].sum()`, `df["k"].nunique()`, `df["k"].nunique(dropna=False)` for `len(df["k"].unique())`, `(df["a"] > 50).sum()`, `set(df["k"].unique())`, `df["a"].tolist()`, `df["a"].to_numpy()`. `Counter` has no fix.

Reports `sum(df["a"])`, `len(set(df["k"]))`, `sum(df["a"] > 50)`, `set(df["k"])`, `Counter(df["k"])`, `list(df["a"])` and `np.array(df["a"].tolist())`. Also reports `len(df["k"].unique())` and `set(df["k"].values)`. For `set` and `list` the receiver must be named like a frame, because both are also how a dict's value is copied. It does not run on test files.

Example:

```python
total = sum(df["load"])
```

Use instead:

```python
total = df["load"].sum()
```

#### list-over-column

**Severity:** warning. **Fix:** none.

Reports `[v.upper() for v in df["s"]]`, and a `for` loop over a column that appends to a list. The receiver must be named like a frame: `df`, `frame`, or a name with one of them as a word. `x["key"]` in a for clause is more often a list inside a dict. It does not run on test files.

Example:

```python
names = [name.upper() for name in df["name"]]
```

Use instead:

```python
names = df["name"].str.upper()
```

The result is a labeled Series rather than a Python list. Use tolist only at a boundary that needs a list.

The replacement is advice. xpd does not apply it automatically.

#### frame-growth-in-loop

**Severity:** warning. **Fix:** none.

Reports `out.loc[len(out)] = row` inside a loop.

Example:

```python
out = pd.DataFrame(columns=output_columns)
for row in rows:
    out.loc[len(out)] = row
```

Use instead:

```python
out = pd.DataFrame(rows, columns=output_columns)
```

The replacement is advice. xpd does not apply it automatically.

#### len-of-filter

**Severity:** warning. **Fix:** unsafe: `int((df["a"] > 50).sum())`.

Reports `len(df[df["a"] > 50])`. The frame must be a plain name. It does not run on test files.

Example:

```python
count = len(df[df["load"] > 50])
```

Use instead:

```python
count = int((df["load"] > 50).sum())
```

#### sort-then-head

**Severity:** warning. **Fix:** unsafe: `df.nsmallest(10, "b")`, `df.iloc[df["b"].argmin()]`, `df["b"].min()`, `df["b"].max()`, `df["b"].idxmin()`. The last rows of a frame, `iloc[-1]` on a sorted column and `sorted(df["b"])[0]` have no fix.

Reports `df.sort_values("b").head(10)`, `df.sort_values("b").iloc[0]`, `df["b"].sort_values().iloc[0]`, `df.sort_values("b").tail(10)` and `sorted(df["b"])[0]`. A negative count, `head(-3)`, is left alone: it keeps all but the last rows, and `nsmallest(-3)` returns none.

Example:

```python
largest = df.sort_values("load", ascending=False).head(10)
```

Use instead:

```python
largest = df.nlargest(10, "load")
```

#### replace-with-mapping

**Severity:** warning. **Fix:** none.

Reports `df["k"].replace(mapping)` with a mapping held in a name.

Example:

```python
names = df["name"].replace(mapping)
```

Use instead:

```python
names = df["name"].map(lambda value: mapping.get(value, value))
```

The example uses an ordinary dictionary and scalar values. A short mapping can be faster with replace. map does not reproduce regex replacement or every special mapping behavior.

The replacement is advice. xpd does not apply it automatically.

#### concat-in-loop

**Severity:** warning. **Fix:** none.

Reports a frame that grows inside a loop, as in `out = pd.concat([out, part])`. Each pass copies every earlier row. A concat with `axis=1` is left alone: on pandas 3 it shares the columns it already has.

Example:

```python
out = pd.DataFrame()
for path in paths:
    out = pd.concat([out, pd.read_csv(path)])
```

Use instead:

```python
frames = [pd.read_csv(path) for path in paths]
out = pd.concat(frames)
```

The example assumes paths is not empty and the starting frame has no schema to preserve. A single concat avoids repeatedly copying the growing result.

A `pd.concat` inside a loop that builds a frame from other objects, such as
`panel = pd.concat(parts_of_this_month)`, copies each row once and is left alone.

The replacement is advice. xpd does not apply it automatically.

#### apply-math-function

**Severity:** warning. **Fix:** unsafe: `np.sqrt(df["w"])`, `df["b"].abs()`.

Reports `df["w"].map(np.sqrt)`, `df["w"].apply(lambda v: np.log(v))`, `df["w"].apply(math.sqrt)` and `df["b"].apply(abs)`.

Example:

```python
root = df["wind"].map(np.sqrt)
```

Use instead:

```python
root = np.sqrt(df["wind"])
```

#### apply-cast

**Severity:** warning. **Fix:** unsafe: `df["b"].astype(int)`.

Reports `df["b"].apply(int)`, `df["raw"].map(float)` and `df["b"].apply(lambda v: int(v))`.

Example:

```python
whole = df["load"].apply(int)
```

Use instead:

```python
whole = df["load"].astype(int)
```

#### apply-comparison

**Severity:** warning. **Fix:** unsafe: `df["a"] > 50`, `df["a"].isin(wanted)`.

Reports `df["a"].apply(lambda v: v > 50)` and `df["a"].apply(lambda v: v in wanted)`.

Example:

```python
high = df["load"].apply(lambda value: value > 50)
```

Use instead:

```python
high = df["load"] > 50
```

#### apply-pd-series

**Severity:** warning. **Fix:** unsafe: `pd.DataFrame(df["pair"].tolist(), index=df["pair"].index)`.

Reports `df["pair"].apply(pd.Series)`, which builds one Series per row to split a column of lists or dicts.

Example:

```python
parts = df["pair"].apply(pd.Series)
```

Use instead:

```python
parts = pd.DataFrame(df["pair"].tolist(), index=df["pair"].index)
```

#### index-loop

**Severity:** warning. **Fix:** none.

Reports `for label in df.index` with `df.loc[label, "a"]` or `df.at[label, "a"]` in the body, also as a comprehension. It does not run on test files.

Example:

```python
out = []
for label in df.index:
    out.append(df.loc[label, "load"] * df.loc[label, "weight"])
```

Use instead:

```python
out = (df["load"] * df["weight"]).tolist()
```

The replacement is advice. xpd does not apply it automatically.

#### zip-over-columns

**Severity:** warning. **Fix:** none.

Reports a loop or comprehension over `zip(df["a"], df["w"])`. It does not run on test files.

Example:

```python
out = [load * weight for load, weight in zip(df["load"], df["weight"])]
```

Use instead:

```python
out = (df["load"] * df["weight"]).tolist()
```

Keep the result as a Series when the next step takes one. Use tolist only at a boundary that needs a list.

The replacement is advice. xpd does not apply it automatically.

On 1,000,000 rows `map` with a Series ran 50 to 64 times faster than the `.loc` lambda of
`lookup-per-row`, measured on 100,000 rows, and 777 to 848 times faster than the mask form, measured
on 20,000. Its fix is unsafe, because a key that is absent gives a missing value where the lambda
raised. `apply-recomputes-aggregate` grows with the square of the row count: 20,000 rows took 0.34
seconds against 0.2 milliseconds. It reports a reduction of a column taken by a text label, and
leaves `lambda v: v["scores"].mean()` alone, which reads the row's own value.
`enumerate-map-encoding` was 7 to 17 times slower than `pd.factorize`. `copy-in-chain` names only
methods that return a new frame at once: each copy cost 4 to 10 milliseconds on 6 columns, and
dropping it was 3 to 118 times faster. A copy next to a slow method such as `sort_values` or `merge`
is as wasted and is not reported, because it gained under 2 times. `df.copy().pipe(f)` is left
alone, because `f` may write to its argument.

The `apply-axis-1` shapes with `agg` and `transform` were 327 to 2,735 times slower than whole
columns on 100,000 rows. A named method, as in `agg("sum", axis=1)`, is left alone. The new
`builtin-on-column` shapes were 2.6 to 483 times slower, and `list` and `set` can report a dict of
lists, such as `list(config["names"])`, because xpd cannot tell it from a frame. The `for` loop of
`list-over-column` was 13 to 19 times slower. `sort-then-head` reports `sort_values().iloc[-1]`
without a fix, because the sort puts missing values last, so the old form returns NaN where `max()`
skips them. Its `idxmin` fix can name another row when the lowest value appears twice.

`apply-known-function` reports only a column taken by a text label, because on a frame `apply`
works per column. Its `pd.to_datetime` fix is unsafe, because a call per value parses each text
on its own, and the whole column must share one format. The `isna` fix is unsafe, because on a
column of lists `pd.isna` per value returned an array per cell. The `round` fix is unsafe, because
Python and numpy round some halfway values differently. `copy-after-indexing` leaves a single text
label and a slice alone, because on a dict or a numpy array the copy keeps a later write away
from the original.

`apply-math-function` leaves `df["w"].apply(np.sqrt)` alone, because `Series.apply` already hands
a NumPy function the whole column, and `map` does not. Its fix is unsafe, because `math.sqrt` and
`math.log` raise on a value outside their domain, where NumPy returns NaN. `apply-cast` covers
`int` and `float` only: on floats with missing values `astype(str)` was slower than `apply(str)`
and kept the missing values. `apply-comparison` leaves a chain such as `0 < v < 5` alone, and a
test on text such as `"ta" in v` or `v in "abc"`. Its `isin` fix is unsafe, because `in` asks the
container, and a dict, a range or a custom container can answer differently. The
`apply-pd-series` fix is unsafe, because the old form turned whole numbers into floats when a row
also held floats, and the fix keeps them whole. With a bare function name, `apply-math-function`,
`apply-cast` and `apply-pd-series` report `apply` only on a column taken by a text label, because
on a frame `apply` works per column. `index-loop` leaves a loop that never reads the frame by its
label alone. `zip-over-columns` reports only when every argument of `zip` is a column taken by a
text label, or a `.values`, `.to_numpy()` or `.tolist()`. It leaves `dict(zip(...))` alone, which
is not a loop.

`row-loop` replaces the `no-iterrows` rule of xpd 0.1.0. The fixes of `apply-text-method`,
`apply-date-part` and `apply-arithmetic` are unsafe, because a column method returns a missing
value where the lambda raised, and the dtype can change: `dt.year` is int32 where the lambda gave
int64. `builtin-on-column` skips missing values after the fix. `sort-then-head` is unsafe, because
`nsmallest` raises on a text column and orders ties differently, and `argmin` raises when every
value is missing. `replace-with-mapping` asks for
`df["k"].map(lambda v: mapping.get(v, v))`, which keeps the values and the dtype.

### groupby, agg and transform

#### groupby-lambda

**Severity:** warning. **Fix:** unsafe: `"max"` for min, max, median, count and nunique, `"sum"`, `"mean"`. The named method can give another dtype, and it raises on a column the lambda could reduce. A reduction with arguments, a count of a condition and `resample` have no fix.

Reports `g["b"].agg(lambda v: v.max())`, `g.agg(np.sum)`, `pivot_table(aggfunc=np.mean)`, `g.agg(lambda v: v.quantile(0.9))`, `g.agg(lambda v: (v > 0).sum())` and a lambda or NumPy function on `resample`.

Example:

```python
maximum = df.groupby("region")["load"].agg(lambda group: group.max())
```

Use instead:

```python
maximum = df.groupby("region")["load"].agg("max")
```

#### groupby-lambda-window

**Severity:** warning. **Fix:** unsafe: `g["b"].shift(1)`, `g["b"].cumsum()`. The method can give another dtype or index type on an empty or all-missing group.

Reports `g["b"].transform(lambda v: v.shift(1))`.

Example:

```python
previous = df.groupby("region")["load"].transform(lambda group: group.shift(1))
```

Use instead:

```python
previous = df.groupby("region")["load"].shift(1)
```


`groupby-lambda` leaves a lambda inside a list alone, because the result column would get a new
name. For `g.agg(lambda v: v.quantile(0.9))` write `g.quantile(0.9)`, which was 4.8 times faster
with 1,000 groups and 345 times with 100,000. For a count of a condition write
`(df["b"] > 0).groupby(df["k"]).sum()`. On a resampler the named method was 3.4 to 82 times faster.
The `groupby-lambda-arithmetic` shapes `v - v.mean()` and `v / v.max()` were 6 times slower than
`transform` with a name with 1,000 groups and 390 times slower with 100,000. Min-max scaling has no
fix. It also leaves `np.median`, `np.std` and `np.var` alone, because they treat missing values
and degrees of freedom differently from the named methods. The fixes for sum, mean, std, var, prod,
cumsum and cumprod are unsafe, because pandas adds up each group with compensated summation. On
values such as 1e16, 1, 1 and -1e16 the sum changes from 0 to 2. `groupby-lambda-window` leaves
`shift(freq=)` alone, because it moves the index.

#### groupby-lambda-arithmetic

**Severity:** warning. **Fix:** unsafe: `df["w"] / df.groupby("k")["w"].transform("sum")`.

Reports `g["w"].transform(lambda v: v / v.sum())`, `v - v.mean()`, `v / v.max()`, the z-score, min-max scaling and `v.fillna(v.mean())`.

Example:

```python
share = df.groupby("region")["load"].transform(lambda group: group / group.sum())
```

Use instead:

```python
share = df["load"] / df.groupby("region")["load"].transform("sum")
```

#### groupby-filter-lambda

**Severity:** warning. **Fix:** unsafe: `df[df.groupby("k")["k"].transform("size") > 10]`.

Reports `df.groupby("k").filter(lambda g: len(g) > 10)`.

Example:

```python
selected = df.groupby("region").filter(lambda group: len(group) > 10)
```

Use instead:

```python
selected = df[df.groupby("region")["region"].transform("size") > 10]
```

The example assumes the group keys are not missing. Check dropna and missing-key behavior before rewriting a filter.

#### groupby-apply

**Severity:** warning. **Fix:** none.

Reports any `groupby(...).apply` with a lambda. A lambda that sorts each group, or builds a dict, a list or a joined text per group, is left alone.

Example:

```python
totals = df.groupby("region")["load"].apply(lambda group: group.sum())
```

Use instead:

```python
totals = df.groupby("region")["load"].sum()
```

The built-in reduction can change the index name, the dtype and floating point rounding. Check the result before replacing a general apply.

The replacement is advice. xpd does not apply it automatically.

#### aggregate-then-merge-back

**Severity:** warning. **Fix:** none.

Reports `df.merge(df.groupby("k")["b"].sum()...)` and `df["k"].map(df.groupby("k")["b"].sum())`.

Example:

```python
result = df.merge(
    df.groupby("region")["load"].sum().rename("total").reset_index(),
    on="region",
    how="left",
    validate="many_to_one",
)
```

Use instead:

```python
result = df.assign(total=lambda frame: frame.groupby("region")["load"].transform("sum"))
```

transform keeps the original index and row order. A merge can instead create a fresh index. The report covers an inline aggregation on the same frame, as well as mapping an aggregate back to its source column. An aggregate stored in a separate variable is not detected.

The replacement is advice. xpd does not apply it automatically.

#### groupby-first-skips-missing

**Severity:** warning. **Fix:** none.

Reports `df.groupby("k").first()` and `last()`. A group of one selected column, `df.groupby("k")["b"].last()`, is left alone, because one column cannot mix rows.

Example:

```python
first = df.groupby("region").first()
```

Use instead:

```python
first = df.groupby("region").first(skipna=False)
```

skipna=False preserves missing values from the first row. Without it, first can combine values from different rows.

The replacement is advice. xpd does not apply it automatically.


### MultiIndex

#### multiindex-from-tuples

**Severity:** warning. **Fix:** unsafe: `pd.MultiIndex.from_arrays([df["k"], df["a"]])`, `pd.MultiIndex.from_product([keys, numbers])`. The comprehension has no fix.

Reports `pd.MultiIndex.from_tuples(zip(df["k"], df["a"]))`, `from_tuples(itertools.product(keys, numbers))` and `from_tuples([(k, n) for k in keys for n in numbers])`, which build one Python tuple per row.

Example:

```python
index = pd.MultiIndex.from_tuples(list(zip(df["region"], df["year"])), names=["region", "year"])
```

Use instead:

```python
index = pd.MultiIndex.from_arrays([df["region"], df["year"]], names=["region", "year"])
```

#### level-values-unique

**Severity:** warning. **Fix:** unsafe: `df.index.unique("k")`, `df.index.unique("k").nunique()`.

Reports `df.index.get_level_values("k").unique()` and `.nunique()`, which build one value per row first. It does not run on test files.

Example:

```python
regions = df.index.get_level_values("region").unique()
```

Use instead:

```python
regions = df.index.unique("region")
```


The `multiindex-from-tuples` fix is unsafe, because without `names=` `from_arrays` takes the level
names from the Series it is given, where `from_tuples` left them empty. It leaves
`itertools.product(keys, repeat=2)` alone. The `level-values-unique` fix is unsafe, because a level
built from a range comes back as a RangeIndex, with the same values, where the old form gave an
Index. On 1,000,000 rows `from_arrays` ran 27 times faster than `from_tuples` over `zip`,
`from_product` 118 times faster than the comprehension, and `unique` with a level 22 times faster.

### Time series

#### rolling-apply

**Severity:** warning. **Fix:** safe: `rolling(20).max()` for min, max and median. unsafe: sum, mean, std and var. The `raw=True` case, which reports as info, and a NumPy function have no fix.

Reports `rolling(20).apply(lambda w: w.max())`, `rolling(20).apply(f)` without `raw=True`, `rolling(20).agg(np.max)` and `rolling(20).apply(np.mean, raw=True)`.

Example:

```python
average = df["load"].rolling(20).apply(lambda window: window.mean())
```

Use instead:

```python
average = df["load"].rolling(20).mean()
```

#### text-time-key

**Severity:** warning. **Fix:** none.

Reports `df.groupby(df["d"].dt.strftime("%Y-%m"))`.

Example:

```python
monthly = df.groupby(df["time"].dt.strftime("%Y-%m"))["load"].sum()
```

Use instead:

```python
monthly = df.groupby(df["time"].dt.to_period("M"))["load"].sum()
```

The result has a PeriodIndex instead of text labels. Converting time zones to periods also removes the time zone.

The replacement is advice. xpd does not apply it automatically.

#### object-dates

**Severity:** warning. **Fix:** none.

Reports `df["d"].dt.date` and `dt.time`. A `dt.date` that is one side of a comparison is left alone: `dt.normalize()` keeps datetime64, and datetime64 compared with a Python `date` raises TypeError.

Example:

```python
days = df["time"].dt.date
```

Use instead:

```python
days = df["time"].dt.normalize()
```

normalize returns timestamps at midnight, not Python date objects. Keep dt.date when an external interface needs Python dates. For dt.time, choose the required datetime representation rather than normalizing away the time.

The replacement is advice. xpd does not apply it automatically.

#### apply-on-dates

**Severity:** warning. **Fix:** unsafe: `df["d"] + pd.Timedelta(days=7)`.

Reports `df["d"].apply(lambda t: t + pd.Timedelta(days=7))`, `t.replace(day=1)` and `t.strftime(...)`.

Example:

```python
next_week = df["time"].apply(lambda time: time + pd.Timedelta(days=7))
```

Use instead:

```python
next_week = df["time"] + pd.Timedelta(days=7)
```

#### strftime-date-part

**Severity:** warning. **Fix:** unsafe: `df["d"].dt.year`, `df["d"].dt.day_name()`.

Reports `df["d"].dt.strftime("%Y").astype(int)`, the same with `%m`, `%H` and `%j`, and `dt.strftime("%A")` and `"%B"`.

Example:

```python
year = df["time"].dt.strftime("%Y").astype(int)
weekday = df["time"].dt.strftime("%A")
```

Use instead:

```python
year = df["time"].dt.year
weekday = df["time"].dt.day_name()
```

#### apply-date-expression

**Severity:** warning. **Fix:** none.

Reports `df["d"].apply(lambda t: t.weekday() >= 5)` and `lambda t: t.hour * 60 + t.minute`, a lambda that compares a date part or computes with parts.

Example:

```python
weekend = df["time"].apply(lambda time: time.weekday() >= 5)
```

Use instead:

```python
weekend = df["time"].dt.weekday >= 5
```

The replacement is advice. xpd does not apply it automatically.

#### strftime-round-trip

**Severity:** warning. **Fix:** unsafe: `df["d"].dt.normalize()`, `df["d"].dt.floor("h")`. The comparison has no fix.

Reports `pd.to_datetime(df["d"].dt.strftime("%Y-%m-%d"))` and `df["d"].dt.strftime("%Y-%m-%d") == "2023-06-01"`, which turn dates into text to drop the time.

Example:

```python
day = pd.to_datetime(df["time"].dt.strftime("%Y-%m-%d"))
```

Use instead:

```python
day = df["time"].dt.normalize()
```

#### index-year-mask

**Severity:** warning. **Fix:** none.

Reports `df[df.index.year == 2023]`.

Example:

```python
year = df[df.index.year == 2023]
```

Use instead:

```python
year = df.loc["2023"]
```

Use the label example on a sorted DatetimeIndex. It is a label selection, not a rewrite for every kind of index.

The replacement is advice. xpd does not apply it automatically.

#### apply-date-method

**Severity:** warning. **Fix:** unsafe: `df["d"].dt.floor("h")`, `df["d"].dt.weekday`, `df["d"].dt.date`, `df["d"].dt.isocalendar().week`. `timestamp` and `toordinal` have no fix.

Reports `df["d"].apply(lambda t: t.floor("h"))`, and the same with `ceil`, `normalize`, `tz_localize`, `tz_convert`, `day_name`, `month_name`, `to_period`, `total_seconds`, `weekday`, `date`, `isocalendar`, `timestamp` and `toordinal`.

Example:

```python
hours = df["time"].apply(lambda time: time.floor("h"))
```

Use instead:

```python
hours = df["time"].dt.floor("h")
```

#### apply-parse-date

**Severity:** warning. **Fix:** unsafe: `pd.to_datetime(df["raw"], format="%Y-%m-%d")`.

Reports `df["raw"].apply(lambda v: datetime.strptime(v, "%Y-%m-%d"))`, and a lambda or `map` that calls `pd.to_datetime`, `pd.Timestamp` or `parser.parse` per value.

Example:

```python
when = df["raw"].apply(lambda text: datetime.strptime(text, "%Y-%m-%d"))
```

Use instead:

```python
when = pd.to_datetime(df["raw"], format="%Y-%m-%d")
```

#### timedelta-per-value

**Severity:** warning. **Fix:** unsafe: `pd.to_timedelta(df["n"], unit="D")` for `pd.Timedelta(v, unit="D")`. The other shapes have no fix.

Reports `df["n"].apply(lambda v: pd.Timedelta(days=v))`, the same with `timedelta` and `pd.DateOffset`, and `pd.to_timedelta(df["n"].astype(str) + " days")`.

Example:

```python
shift = df["days"].apply(lambda days: pd.Timedelta(days=days))
```

Use instead:

```python
shift = pd.to_timedelta(df["days"], unit="D")
```

The units are "W", "D", "h", "min", "s", "ms" and "us". On fractional values the two forms can differ in the last digits.

The `rolling-apply` fixes for sum, mean, std and var are unsafe, because the window methods keep
running totals, so values far apart in size can give another result. For a function with no
window method the message names `raw=True` and `engine="numba"`. `apply-on-dates` writes
`df["d"] - pd.to_timedelta(df["d"].dt.day - 1, unit="D")` for `replace(day=1)`, which keeps the
time of day. Its fixes are unsafe, because a missing date gives a missing result where the
lambda raised.

`strftime-date-part` was 33 to 198 times slower than the part on 1,000,000 rows. Its fixes are
unsafe: a part is int32 where `astype(int)` gave int64, and `day_name` is English where `strftime`
follows the locale. `apply-date-expression` was 22 to 94 times slower. It has no fix, because the
rewrite depends on the operator. `strftime-round-trip` was 18 to 202 times slower. Its fix is
unsafe, because on dates with a time zone the round trip drops the zone. For `timestamp` in a
lambda write `(df["d"] - pd.Timestamp("1970-01-01")) / pd.Timedelta("1s")`, which was 229 times
faster. `isocalendar().week` is UInt32 where the lambda gave int64. `rolling(60).agg(np.max)` was
1,031 times slower than `rolling(60).max()` on 200,000 rows.

The `apply-date-method` fix is unsafe, because the dtype can change: `dt.weekday` is int32 where
the lambda gave int64. It leaves `round` alone, because `lambda v: v.round(2)` is more often a
number. The `apply-parse-date` fix is unsafe for the reason `apply-known-function` gives, and
because `strptime` raises on a missing value where `pd.to_datetime` returns NaT. It reports
`strptime` only on `datetime`, because `time.strptime` returns another type. `timedelta-per-value`
has no fix for a keyword such as `days=v`, because the keyword must become a unit. It leaves
`pd.DateOffset(months=v)` alone, because a month has no fixed length. On 1,000,000 rows `dt.floor`
ran 1,354 times faster than the lambda, `pd.to_datetime` with a format 24 times faster than
`strptime`, and `pd.to_timedelta` 1,549 times faster.

### Stated intent

#### merge-without-validate

**Severity:** warning. **Fix:** none.

Reports `merge` without `validate=`, which can duplicate rows. A merge with `how="cross"` is left alone, because it pairs every row on purpose.

Example:

```python
joined = left.merge(right, on="key")
```

Use instead:

```python
joined = left.merge(right, on="key", validate="many_to_one")
```

Choose validate to match the data relationship. many_to_one is only the example, not a default xpd inserts. many_to_many does not check key uniqueness.

The replacement is advice. xpd does not apply it automatically.

#### merge-without-on

**Severity:** warning. **Fix:** none.

Reports `merge` without `on=`, which joins on every shared column name.

Example:

```python
joined = left.merge(right, validate="many_to_one")
```

Use instead:

```python
joined = left.merge(right, on="key", validate="many_to_one")
```

Choose the actual join keys. Omitting on makes every shared column name a key.

The replacement is advice. xpd does not apply it automatically.

#### to-csv-index

**Severity:** warning. **Fix:** none.

Reports `df.to_csv(path)` without `index=`. A call with no argument and a call that forwards `**options` are left alone. A named index is written under its name, so the message applies to an index with no name.

Example:

```python
df.to_csv(path)
```

Use instead:

```python
df.to_csv(path, index=False)
```

The replacement is advice. xpd does not apply it automatically.


### Reading and writing files

#### read-csv-slow-option

**Severity:** warning. **Fix:** none.

Reports `read_csv(..., engine="python")`, `sep=None` and `low_memory=False`, which turn off reading on several threads.

Example:

```python
df = pd.read_csv(path, engine="python")
```

Use instead:

```python
df = pd.read_csv(path)
```

The example needs no Python-engine-only options. Keep engine="python" when the separator or skipfooter requires it. For low_memory=False, specify the required column dtypes rather than dropping it blindly.

The replacement is advice. xpd does not apply it automatically.



#### read-then-head

**Severity:** warning. **Fix:** unsafe: `nrows=1000`.

Reports `pd.read_csv(path).head(1000)` and `.iloc[:1000]`, which parse the whole file to keep the first rows.

Example:

```python
sample = pd.read_csv(path).head(1000)
```

Use instead:

```python
sample = pd.read_csv(path, nrows=1000)
```

#### read-csv-converters

**Severity:** warning. **Fix:** none.

Reports `read_csv(..., converters=...)`, which calls a Python function once per value.

Example:

```python
df = pd.read_csv(path, converters={"price": float, "name": str.strip})
```

Use instead:

```python
df = pd.read_csv(path, dtype={"price": float})
df["name"] = df["name"].str.strip()
```

The replacement is advice. xpd does not apply it automatically.

#### read-json-lines-engine

**Severity:** warning. **Fix:** unsafe: `engine="pyarrow"`. The `json.loads` loop has no fix.

Reports `pd.read_json(path, lines=True)` without an engine, and `pd.DataFrame([json.loads(line) for line in f])`.

Example:

```python
df = pd.read_json(path, lines=True)
```

Use instead:

```python
df = pd.read_json(path, lines=True, engine="pyarrow")
```

#### to-csv-append-header

**Severity:** error. **Fix:** none.

Reports `df.to_csv(path, mode="a")` without `header=`, which writes the header again on every call.

Example:

```python
for part in parts:
    part.to_csv(path, mode="a", index=False)
```

Use instead:

```python
for part in parts:
    part.to_csv(path, mode="a", index=False, header=not os.path.exists(path))
```

The repeated header rows are read back as data, so every number in the file comes back as text.

The replacement is advice. xpd does not apply it automatically.

`read-csv-slow-option` leaves `engine="python"` alone when the call needs it, for `skipfooter` or
a separator of several characters other than `\s+`. `sep=None` was 54 times slower than a named
separator on 1,000,000 rows. `nrows` was 58 times faster than `head`. Its fix is unsafe, because
pandas guesses the dtypes from the rows it reads. `converters` was 3.9 to 7.8 times slower than
`dtype` or a column method after the read. `read_json` with the pyarrow engine was 18 times faster,
and 27 times faster than the `json.loads` loop. Its fix is unsafe, because pyarrow can pick another
dtype for a column. A call that reads from a `StringIO` is left alone, because the pyarrow engine
raises on a text buffer.

### Packages

`xpd check` also looks at the project's venv once per run. When the venv has pandas, it warns
about each package below that the venv lacks. The warning points at the project's
`pyproject.toml`, or at the venv when there is none. A project without a venv gets no package
warning, and `xpd hook` never gives one.

#### missing-pyarrow

**Severity:** warning. **Fix:** none.

Text columns are stored as Python strings without it. The `.str` methods ran 3 to 11 times faster with it on 1,000,000 rows.

The warning appears when pyarrow is not installed.

For a project managed with uv:

```sh
uv add pyarrow
```

Add the package to the project environment, not to xpd itself.

#### missing-bottleneck

**Severity:** warning. **Fix:** none.

`std` ran 4 times faster and `median` 1.7 times faster with it on 10,000,000 rows.

The warning appears when bottleneck is not installed.

For a project managed with uv:

```sh
uv add bottleneck
```

Add the package to the project environment, not to xpd itself.

#### missing-pandas-stubs

**Severity:** warning. **Fix:** none.

Without the stubs a type checker does not check calls to pandas.

The warning appears when pandas-stubs is not installed, and the project uses mypy, pyright, basedpyright, ty or pyrefly.

For a project managed with uv:

```sh
uv add --dev pandas-stubs
```

Add the package to the project environment, not to xpd itself.


A project uses a type checker when the venv has it or `pyproject.toml` has a `[tool.<name>]`
table for it. numexpr and numba have no check. numexpr changed little in measurements, and numba
only helps a function passed to `rolling(...).apply`, where the `rolling-apply` message names it.

## NumPy

These rules run on files that import `numpy`. Examples assume `import numpy as np`. The rules
follow NumPy 2.5. Every claim was run on NumPy 2.5.4: a form that is called removed raises there, a
form that is called wrong gives the wrong value there, and each speedup is the median of 7 runs on
one core, at 1,000,000 values unless the entry says otherwise.

A fix that writes `np.` applies only in a file with `import numpy as np`. A safe fix returns a bit
equal value of the same dtype. An unsafe fix names what can differ. A rule that asks for an API
added in NumPy 2.0 or later is skipped when the project's venv holds an older NumPy.

### Code that gives a wrong result or raises on NumPy 2.5

Each old form gives a wrong value without an error, or raises on every input in a way the text
already shows. Examples assume `import numpy as np`. Every fix in this group changes the result to
the one the code meant, so each one is unsafe. Read the notes before applying it.

#### compare-to-nan

**Severity:** error. **Fix:** unsafe: `np.isnan(x)`. `np.nan in np.unique(x)` has no fix.

Reports `x == np.nan`, `x != np.nan`, `x == float("nan")`, `x == math.nan` and `np.nan in np.unique(x)`, which never find NaN, because NaN equals nothing.

Example:

```python
missing = x == np.nan
```

Use instead:

```python
missing = np.isnan(x)
```

A file that imports pandas is left to the pandas rule `compare-to-missing`. A comparison passed to an assert function is left alone, because a test may check how a comparison with NaN behaves.

#### argument-in-wrong-slot

**Severity:** error. **Fix:** unsafe: `np.zeros((3, 4))`. The linspace form and a quantile call with more arguments have no fix.

Reports `np.zeros(3, 4)`, `np.full(3, 4, 0)`, `np.reshape(a, 2, 3)`, `np.vstack(a, b)`, `np.concatenate(a, [4, 5])`, `np.random.rand((3, 4))`, `np.random.random(3, 4)`, `np.linspace(0, 1, 0.1)` and `np.quantile(x, 95)`, which raise because an argument sits in the slot of another parameter.

Example:

```python
grid = np.zeros(3, 4)
```

Use instead:

```python
grid = np.zeros((3, 4))
```

#### percentile-of-fraction

**Severity:** warning. **Fix:** unsafe: `np.quantile(x, 0.95)`.

Reports `np.percentile(x, 0.95)` and `np.percentile(x, [0.25, 0.75])`, which ask for percentiles near the minimum, because np.percentile takes percents from 0 to 100.

Example:

```python
upper = np.percentile(x, 0.95)
```

Use instead:

```python
upper = np.quantile(x, 0.95)
```

A list such as `[0.5, 99.5]`, where one number is a percent, is left alone.

#### positional-tolerance

**Severity:** warning. **Fix:** none.

Reports `np.isclose(a, b, 1e-3)` and `np.allclose(a, b, 1e-3)`, where the third argument is rtol and atol stays 1e-8.

Example:

```python
same = np.isclose(a, b, 1e-3)
```

Use instead:

```python
same = np.isclose(a, b, atol=1e-3)
```

A call that also passes atol, or a value named like rtol, is left alone. The replacement is advice. xpd does not apply it automatically.

#### arange-float-step

**Severity:** warning. **Fix:** none.

Reports `np.arange(0, 1, 0.1)`, whose length depends on rounding, so `np.arange(1, 1.3, 0.1)` ends with 1.3000000000000003. The [NumPy docs for arange](https://numpy.org/doc/stable/reference/generated/numpy.arange.html) recommend np.linspace for a step that is not whole.

Example:

```python
ticks = np.arange(0, 1, 0.1)
```

Use instead:

```python
ticks = np.linspace(0, 0.9, 10)
```

A step that is a multiple of 1/64, such as 0.5 or 0.25, is exact and left alone. The replacement is advice. xpd does not apply it automatically.

#### randint-single-value

**Severity:** error. **Fix:** none.

Reports `np.random.randint(0, 1, n)`, `np.random.randint(1)` and `rng.integers(0, 1)`, which always return 0, because the high end is exclusive.

Example:

```python
coin = rng.integers(0, 1, size=10)
```

Use instead:

```python
coin = rng.integers(0, 2, size=10)
```

The replacement is advice. xpd does not apply it automatically.

#### result-is-none

**Severity:** error. **Fix:** unsafe: `x.sort()`. A result assigned to another name, or returned, has no fix.

Reports `x = x.sort()`, `m = np.fill_diagonal(m, 0)`, `y = rng.shuffle(y)`, `w = w.fill(0)` and `return v.sort()`, which leave None, because these calls change the array in place.

Example:

```python
values = values.sort()
```

Use instead:

```python
values.sort()
```

A sort with an argument other than an axis, such as `xp.sort(x)` in the array API, returns a value and is left alone.

#### result-discarded

**Severity:** error. **Fix:** unsafe: `x = np.sort(x)`.

Reports `np.sort(x)`, `np.append(x, 4)`, `np.clip(x, 0, 1)`, `x.astype(float)` and `m.reshape(3, 2)` standing alone as a statement, which do nothing, because they return a new array.

Example:

```python
np.append(lengths, 1)
```

Use instead:

```python
lengths = np.append(lengths, 1)
```

A call with `out=`, a call in a try block or in a `with pytest.raises` block, and a call in an asv benchmark method are left alone. Other names bound to the old array keep its old values after the fix.

#### write-to-copy

**Severity:** error. **Fix:** none.

Reports `x[x > 2][0] = v`, `x[[0, 1]][0] = v`, `m.flatten()[0] = v` and `x.astype(int)[0] = v`, which write to a copy, so the array keeps its old values.

Example:

```python
x[x > 2][0] = -1
```

Use instead:

```python
x[np.flatnonzero(x > 2)[0]] = -1
```

A slice or a whole number returns a view, so `x[1:4][0] = v` and `m[0][mask] = v` are left alone. The replacement is advice. xpd does not apply it automatically.

#### array-truth-value

**Severity:** error. **Fix:** none.

Reports `x[(x > 2) and (x < 8)]`, `x[not (x > 2)]` and `x[2 < x < 8]`, which raise ValueError, because and, or, not and a chained comparison ask for the truth of a whole array.

Example:

```python
inside = x[(x > 2) and (x < 8)]
```

Use instead:

```python
inside = x[(x > 2) & (x < 8)]
```

The replacement is advice. xpd does not apply it automatically.

#### mask-operator-precedence

**Severity:** error. **Fix:** none.

Reports `x > 2 & x < 8`, where & binds first, so Python compares x with `2 & x`.

Example:

```python
inside = x[x > 2 & x < 8]
```

Use instead:

```python
inside = x[(x > 2) & (x < 8)]
```

A file that imports pandas is left to the pandas rule `mask-precedence`. The replacement is advice. xpd does not apply it automatically.

#### int-mask-as-index

**Severity:** error. **Fix:** unsafe: `y[y > 15]`. The `~` form has no fix.

Reports `y[(y > 15).astype(int)]`, which picks element 0 and element 1 over and over, and `~(y > 15).astype(int)`, which gives -1 and -2.

Example:

```python
large = y[(y > 15).astype(int)]
```

Use instead:

```python
large = y[y > 15]
```

A lookup such as `labels[(score > t).astype(int)]`, where the mask comes from another array, maps each value to one of two labels and is left alone.

#### iterator-to-array

**Severity:** error. **Fix:** unsafe: `np.array([v * 2 for v in y])`. A set has no fix.

Reports `np.array(v * 2 for v in y)`, `np.array(map(float, y))`, `np.array(d.values())`, `np.array({1, 2})`, `np.max(v for v in y)` and `np.sum(map(f, y))`, because NumPy does not consume an iterator. np.array wraps it in a zero dimensional object array, np.max returns the iterator itself, np.any returns True, and np.sum of a generator raises TypeError.

Example:

```python
doubled = np.array(v * 2 for v in values)
```

Use instead:

```python
doubled = np.array([v * 2 for v in values])
```

`np.fromiter` consumes an iterator and is left alone.

#### nan-to-int

**Severity:** warning. **Fix:** none.

Reports `np.where(m, x, np.nan).astype(int)`, which turns each NaN into -9223372036854775808.

Example:

```python
counts = np.where(seen, totals, np.nan).astype(int)
```

Use instead:

```python
counts = np.where(seen, totals, 0).astype(int)
```

The replacement is advice. xpd does not apply it automatically.

#### read-uninitialized

**Severity:** error. **Fix:** none.

Reports `np.empty(n).sum()` and `np.empty_like(x) + 1`, which compute with whatever bytes were in memory.

Example:

```python
total = np.empty(n) + offset
```

Use instead:

```python
total = np.zeros(n) + offset
```

`np.empty(n) * np.nan` gives NaN everywhere and is left alone. The replacement is advice. xpd does not apply it automatically.

#### array-default-argument

**Severity:** warning. **Fix:** none.

Reports `def f(acc=np.zeros(2))` when the function writes into `acc`, because every call shares the one default array.

Example:

```python
def add_one(acc=np.zeros(2)):
    acc += 1
    return acc
```

Use instead:

```python
def add_one(acc=None):
    acc = np.zeros(2) if acc is None else acc
    acc += 1
    return acc
```

The replacement is advice. xpd does not apply it automatically.

#### append-flattens

**Severity:** error. **Fix:** none.

Reports `np.append(m, [[5, 5]])` and `np.insert(m, 0, [[5, 5]])` without axis, which flatten the array, so the row becomes loose values.

Example:

```python
grown = np.append(m, [[5, 5]])
```

Use instead:

```python
grown = np.append(m, [[5, 5]], axis=0)
```

The replacement is advice. xpd does not apply it automatically.

#### compare-reductions

**Severity:** error. **Fix:** unsafe: `np.array_equal(a, b)`.

Reports `a.all() == b.all()` and `np.any(a) == np.any(b)`, which compare two single booleans, not the arrays.

Example:

```python
assert a.all() == b.all()
```

Use instead:

```python
assert np.array_equal(a, b)
```

#### where-tuple-length

**Severity:** error. **Fix:** unsafe: `np.count_nonzero(z > 0)`.

Reports `len(np.where(z > 0))`, `len(np.nonzero(z))` and `if np.where(z > 9):`, because np.where with one argument returns a tuple with one array per dimension. Its length is the number of dimensions, and it is always true.

Example:

```python
matches = len(np.where(z > 0))
```

Use instead:

```python
matches = np.count_nonzero(z > 0)
```

### Code that raises or warns on NumPy 2.5

#### removed-in-numpy-2

**Severity:** error. **Fix:** safe: the builtin for `np.int`, `np.float`, `np.complex`, `np.object`, `np.str` and `np.unicode`, `np.float64` for `np.float_` and the other scalar aliases, `np.nan` and `np.inf` for `np.NaN`, `np.Inf` and their kin, `np.prod`, `np.cumprod`, `np.any`, `np.all`, `np.round` and `np.vstack` for the renamed functions, `np.exceptions.AxisError` and the other warnings and errors, `np.trapezoid`, `np.sort(a, axis=0)`, `a.item()`, `np.pad` for `np.lib.pad` and the other functions that `np.lib` re-exported, a shape by position for `newshape=`, `method=` for `interpolation=` and `minlength=0`. unsafe: `"S"` for the dtype code `"a"`, because a `dtype=` keyword of another library can give the text another meaning, deleting `fix_imports=`, `bias=` and `ddof=`, because a call in the deleted value no longer runs, `np.ptp(a)`, `a.tobytes()` and `a[i, j] = v` for `a.itemset`, because the pattern cannot tell an array from another object with a method of that name, `np.isin` for `np.in1d`, `density=` for `normed=`, `np.asarray` for `np.array(x, copy=False)`, `np.frombuffer` for `np.fromstring`, deleting `style=`, and the module path for `numpy.linalg.linalg`, `numpy.fft.helper`, `numpy.testing.utils` and the private modules of `numpy.lib`. The other forms have no fix.

Reports the names, methods, modules and keywords that NumPy 1.24 through 2.5 removed, each with what to write instead. Code under a NumPy version check, a `hasattr(np, ...)` check, or a `try` that catches `ImportError` or `AttributeError` is left alone, because it exists for old NumPy.

Example:

```python
values = np.array(rows, dtype=np.float_, copy=False)
low = np.quantile(values, 0.1, interpolation="nearest")
clean = np.where(values > 0, values, np.NaN)
warning = np.ComplexWarning
```

Use instead:

```python
values = np.asarray(rows, dtype=np.float64)
low = np.quantile(values, 0.1, method="nearest")
clean = np.where(values > 0, values, np.nan)
warning = np.exceptions.ComplexWarning
```

#### deprecated-in-numpy-2

**Severity:** warning. **Fix:** safe: `np.trunc` for `np.fix`, `np.ma.round` for `np.ma.round_`, `out=` for an output passed by position to `np.maximum` and `np.minimum`, and a datetime64 string without its `Z`. unsafe: `np.rec` and `np.char` for `np.core.records` and `np.core.defchararray`, which export fewer names, `np.<name>` for `np.core.<name>`, `randint(low, high + 1)` for `random_integers(low, high)`, with or without a size, `x = x.reshape(shape)` for `x.shape = shape`, and `x = x.view(dtype)` for `x.dtype = dtype`. The other forms have no fix.

Reports `numpy.core`, `np.typename`, `np.char.array`, `np.char.asarray` and `np.char.chararray`, `np.testing.assert_warns` and `suppress_warnings`, a timedelta64 without a unit, a time zone in a datetime64 string, `fftn` and its kin with `s=` and no `axes=`, and setting `shape`, `dtype` or `strides` on a name bound from an `np.` call in the same function. Each warns on NumPy 2.5.

Example:

```python
from numpy.core.numeric import normalize_axis_tuple
whole = np.fix(values)
np.maximum(values, 0.0, out_buffer)
grid = np.arange(6.0)
grid.shape = (2, 3)
```

Use instead:

```python
from numpy.lib.array_utils import normalize_axis_tuple
whole = np.trunc(values)
np.maximum(values, 0.0, out=out_buffer)
grid = np.arange(6.0)
grid = grid.reshape((2, 3))
```

### Python work per value on arrays

Each old form calls Python once per value, or copies the whole array once per pass. Speedups are
medians of 7 runs on one core with NumPy 2.5.4, on float64 and int64.

#### element-loop

**Severity:** warning. **Fix:** none.

Reports a `for` loop over `range`, `np.arange` or `np.ndindex`, and a `while i < n` loop with `i += 1`, whose only statement writes one element of a NumPy array from arithmetic on the loop position, and a loop over `np.nditer` or `np.ndenumerate` without `external_loop`. A body that calls another function, reads the array it writes, or writes a slice is left alone.

Example:

```python
out = np.empty_like(a)
for i in range(len(a)):
    out[i] = a[i] * 2 + np.sqrt(b[i])
```

Use instead:

```python
out = a * 2 + np.sqrt(b)
```

#### growth-in-loop

**Severity:** warning. **Fix:** none.

Reports `out = np.append(out, x)`, `out = np.concatenate((out, x))` and the same with `vstack`, `hstack`, `column_stack`, `dstack` and `stack` inside a loop, when the loop does not reset `out` or read it elsewhere.

Example:

```python
out = np.array([])
for part in parts:
    out = np.append(out, part)
```

Use instead:

```python
out = np.concatenate(parts)
```

#### per-value-comprehension

**Severity:** warning. **Fix:** none.

Reports `np.array([x * 2 + 1 for x in a])`, `np.array([math.sqrt(x) for x in a])`, `np.array([i * 0.5 for i in range(n)])` and `map(np.sqrt, a)`, where the element is arithmetic or a NumPy or math function on the loop variable.

Example:

```python
roots = np.array([math.sqrt(x) for x in values])
```

Use instead:

```python
roots = np.sqrt(np.asarray(values))
```

#### builtin-on-array

**Severity:** warning. **Fix:** unsafe: `np.abs(a).sum(axis=0)`, `np.abs(a).max(axis=0)`. `sorted` and `list` have no fix.

Reports `sum`, `min`, `max`, `sorted` and `list` around a NumPy call, as in `sum(np.abs(a))`, or around a comparison of one, as in `sum(np.abs(a) > 1)`.

Example:

```python
peak = max(np.abs(signal))
```

Use instead:

```python
peak = np.abs(signal).max(axis=0)
```

#### np-vectorize

**Severity:** warning. **Fix:** none.

Reports `np.vectorize`, `np.frompyfunc` and `np.apply_along_axis`, called or used as a decorator.

Example:

```python
@np.vectorize
def clipped(x):
    return min(x, 3.0)
```

Use instead:

```python
def clipped(x):
    return np.minimum(x, 3.0)
```

### Arrays built, copied and stored the slow way

#### array-construction

**Severity:** warning. **Fix:** safe: `np.zeros(n, dtype=np.float32)`. unsafe: `np.full(n, 3, dtype=np.float64)`, which gives a zero dimensional array where the product of an empty shape gave a scalar, `np.arange(n)`, `np.full(n, 0)`, `np.arange(n, dtype=float)`.

Reports `np.array(range(n))`, `np.array([0] * n)`, `np.ones(n) * 3`, `np.zeros(n).astype(np.float32)` and `np.arange(n).astype(float)`.

Example:

```python
weights = np.ones(n) * 0.5
```

Use instead:

```python
weights = np.full(n, 0.5, dtype=np.float64)
```

#### needless-copy

**Severity:** warning. **Fix:** unsafe: `np.abs(m).ravel()`, `x.astype(np.float32)`, `np.array(x)`. ravel can return a view of an output array that was passed by position, and the other two can change the memory order.

Reports `.flatten()` on the result of a NumPy call that returns a new array, `x.copy().astype(t)` and `np.array(x).copy()`.

Example:

```python
flat = np.concatenate([a, b]).flatten()
```

Use instead:

```python
flat = np.concatenate([a, b]).ravel()
```

#### file-roundtrip

**Severity:** warning. **Fix:** none.

Reports `np.loadtxt(f)` and `np.genfromtxt(f)` in a file that also calls `np.savetxt(f, ...)`, and `np.fromfile(f)` in a file that also calls `x.tofile(f)`, with the same file argument.

Example:

```python
np.savetxt("cache.txt", data)
data = np.loadtxt("cache.txt")
```

Use instead:

```python
np.save("cache.npy", data)
data = np.load("cache.npy")
```

### Whole-array math with a faster or more exact function

These rules are warnings. Each one names the NumPy function that does the same work without a
full sort, a temporary array or a step that loses digits. The speedups were measured on one core
with BLAS and LAPACK on one thread, NumPy 2.5.4.

#### sort-then-select

**Severity:** warning. **Fix:** unsafe: `np.min(x)`, `np.max(x)`, `np.sort(np.partition(x, 5)[:5])`, `np.sort(np.partition(x, -5)[-5:])`, `np.sort(x)`, `np.argmin(x)`, `np.argmax(x)`. The positions of the k smallest or largest and the rank by a double argsort have no fix.

Reports `np.sort(x)[0]`, `np.sort(x)[-1]`, `np.sort(x)[:5]`, `np.sort(x)[-5:]`, `np.sort(x)[::-1][:5]`, `x[np.argsort(x)]`, `np.argsort(x)[0]`, `np.argsort(x)[-1]`, `np.argsort(x)[:5]`, `np.argsort(x)[-5:]` and `np.argsort(np.argsort(x))`, with a literal count under 100.

Example:

```python
top = np.sort(scores)[-5:]
```

Use instead:

```python
top = np.sort(np.partition(scores, -5)[-5:])
```

#### count-nonzero

**Severity:** warning. **Fix:** safe: `np.any(x > 2)`. unsafe: `np.count_nonzero(x > 0)`. `len(x[x > 0])` has no fix.

Reports `np.sum(x > 0)`, `len(np.where(c)[0])`, `len(np.nonzero(c)[0])`, `len(np.flatnonzero(c))`, `len(x[x > 0])` and `np.sum(x > 2) > 0`.

Example:

```python
changed = np.sum(before != after)
```

Use instead:

```python
changed = np.count_nonzero(before != after)
```

#### where-shortcut

**Severity:** warning. **Fix:** unsafe: `np.asarray(x > 0)`, `np.asarray(x > 0, dtype=int)`, `np.maximum(x, 0)`, `np.flatnonzero(x > 1)`.

Reports `np.where(x > 0, True, False)`, `np.where(x > 0, 1, 0)`, `np.where(x > 0, x, 0)`, `np.where(x < 0, 0, x)` and `np.arange(len(x))[x > 1]`.

Example:

```python
relu = np.where(z > 0, z, 0)
```

Use instead:

```python
relu = np.maximum(z, 0)
```

#### expm1-log1p

**Severity:** warning. **Fix:** unsafe: `np.expm1(x)`, `-np.expm1(x)`, `np.log1p(x)`, `np.log1p(-p)`.

Reports `np.exp(x) - 1`, `1 - np.exp(x)`, `np.log(1 + x)`, `np.log(x + 1)` and `np.log(1 - p)`, but not on a number literal.

Example:

```python
log_survival = np.log(1 - p)
```

Use instead:

```python
log_survival = np.log1p(-p)
```

#### logaddexp

**Severity:** warning. **Fix:** unsafe: `np.logaddexp(a, b)`, `np.logaddexp(0, x)`. The log of a sum of exponentials and the softmax have no fix.

Reports `np.log(np.exp(a) + np.exp(b))`, `np.log(1 + np.exp(x))`, `np.log1p(np.exp(x))`, `np.log(np.sum(np.exp(x)))` and `np.exp(x) / np.sum(np.exp(x))`.

Example:

```python
softplus = np.log(1 + np.exp(z))
```

Use instead:

```python
softplus = np.logaddexp(0, z)
```

#### log-of-product

**Severity:** warning. **Fix:** unsafe: `np.sum(np.log(x))`.

Reports `np.log(np.prod(x))`.

Example:

```python
log_likelihood = np.log(np.prod(probabilities))
```

Use instead:

```python
log_likelihood = np.sum(np.log(probabilities))
```

#### reduce-last

**Severity:** warning. **Fix:** unsafe: `np.sum(x)`, `np.prod(x)`.

Reports `np.cumsum(x)[-1]` and `np.cumprod(x)[-1]`.

Example:

```python
total = np.cumsum(amounts)[-1]
```

Use instead:

```python
total = np.sum(amounts)
```

#### power-half

**Severity:** warning. **Fix:** unsafe: `np.sqrt(x)`.

Reports `np.power(x, 0.5)`.

Example:

```python
root = np.power(area, 0.5)
```

Use instead:

```python
root = np.sqrt(area)
```

#### sum-of-products

**Severity:** warning. **Fix:** unsafe: `np.vdot(a, b)`, `np.vdot(x, x)`. The mean of squared differences has no fix.

Reports `np.sum(a * b)`, `np.sum(x ** 2)`, `np.sum(np.square(x))` and `np.mean((a - b) ** 2)`, but not when an operand is a number, a comparison, a conjugate or a new axis.

Example:

```python
energy = np.sum(signal ** 2)
```

Use instead:

```python
energy = np.vdot(signal, signal)
```

#### norm-by-hand

**Severity:** warning. **Fix:** unsafe: `np.linalg.norm(x)`.

Reports `np.sqrt(np.sum(x ** 2))`, `np.sqrt(np.sum(x * x))`, `np.sqrt(np.sum(np.square(x)))`, `np.sqrt((x ** 2).sum())` and `np.sqrt((x * x).sum())`.

Example:

```python
distance = np.sqrt(np.sum((a - b) ** 2))
```

Use instead:

```python
distance = np.linalg.norm(a - b)
```

#### vecdot

**Severity:** warning. **Fix:** unsafe: `np.vecdot(x, x)` and `np.vecdot(a, b)`. Floats can differ in the last bits, int8 products can wrap where `np.sum` counted in int64, two bool arrays give bool, and vecdot conjugates a complex first argument. Needs NumPy 2.0.

Reports `np.sum(x ** 2, axis=-1)` and `np.sum(np.square(x), axis=-1)`. It also reports `np.sum(a * b, axis=-1)` and `np.sum(a.conj() * b, axis=-1)` where `a` and `b` are names or subscripts. `np.vecdot` multiplies and sums in one pass without the temporary array, and ran 2 to 3.5 times faster on 1,000,000 values.

Example:

```python
squared_length = np.sum(points ** 2, axis=-1)
```

Use instead:

```python
squared_length = np.vecdot(points, points)
```

#### inv-then-multiply

**Severity:** warning. **Fix:** unsafe: `np.linalg.solve(a, b)`, `np.linalg.solve(a, b @ c)`.

Reports `np.linalg.inv(a) @ b`, `np.linalg.inv(a).dot(b)`, `np.dot(np.linalg.inv(a), b)` and `np.linalg.inv(a) @ b @ c`, as in the normal equations.

Example:

```python
beta = np.linalg.inv(X.T @ X) @ X.T @ y
```

Use instead:

```python
beta = np.linalg.solve(X.T @ X, X.T @ y)
```

#### log-of-det

**Severity:** warning. **Fix:** unsafe: `np.linalg.slogdet(a).logabsdet`. Needs NumPy 2.0.

Reports `np.log(np.linalg.det(a))` and `np.log(np.abs(np.linalg.det(a)))`.

Example:

```python
log_det = np.log(np.linalg.det(cov))
```

Use instead:

```python
log_det = np.linalg.slogdet(cov).logabsdet
```

#### eig-on-symmetric

**Severity:** warning. **Fix:** unsafe: `np.linalg.eigh(m)`, `np.linalg.eigvalsh(m)`.

Reports `np.linalg.eig` and `np.linalg.eigvals` on a matrix the code made symmetric: `x @ x.T`, `x.T @ x`, `np.dot(x, x.T)`, `np.dot(x.T, x)`, `a + a.T`, `np.cov(...)` and `np.corrcoef(...)`.

Example:

```python
values, vectors = np.linalg.eig(np.cov(data, rowvar=False))
```

Use instead:

```python
values, vectors = np.linalg.eigh(np.cov(data, rowvar=False))
```

#### matmul-diagonal

**Severity:** warning. **Fix:** unsafe: `np.einsum("ij,ji->i", a, b)`, `np.einsum("ij,ij->i", a, b)`, `np.einsum("ij,ji->", a, b)`.

Reports `np.diag(a @ b)`, `np.diag(np.dot(a, b))`, `np.diagonal(a @ b)`, `np.diag(a @ b.T)`, `np.trace(a @ b)` and `np.trace(np.dot(a, b))`.

Example:

```python
leverage = np.diag(X @ cov @ X.T)
```

Use instead:

```python
leverage = np.einsum("ij,ij->i", X @ cov, X)
```

#### einsum-optimize

**Severity:** warning. **Fix:** unsafe: add `optimize=True`.

Reports `np.einsum` with three or more 2-D operands and a 2-D result, such as `"ij,jk,kl->il"`, and no `optimize=`.

Example:

```python
chain = np.einsum("ij,jk,kl->il", a, b, c)
```

Use instead:

```python
chain = np.einsum("ij,jk,kl->il", a, b, c, optimize=True)
```

### The API that NumPy's docs prefer on NumPy 2.5

Each old form still runs without a warning on NumPy 2.5.4. NumPy's docs call it legacy, not recommended or kept for backwards compatibility, or the newer form measured at least 1.5 times faster. The rules that state only a documented preference report as info, below the warnings. The rules that name an API added in NumPy 2.0 are skipped on an older NumPy.

#### legacy-random

**Severity:** warning. **Fix:** none. A Generator draws a different stream from the same seed, and `np.random.seed` elsewhere would no longer reach the rewritten call.

Reports `np.random.seed`, `np.random.rand`, `np.random.randint`, `np.random.randn`, `np.random.choice`, every other distribution function in `np.random`, `np.random.get_state`, `np.random.RandomState(...)` outside files that import scikit-learn, and imports of these names from `numpy.random`. The message gives the Generator method, such as `integers` for `randint`, and its measured speed where it was at least 1.5 times faster: `random` 2 times, `integers` 2.5 times, `exponential` 2.4 times, and `choice(..., replace=False)` 280 times.

Example:

```python
np.random.seed(0)
idx = np.random.choice(len(x), 100, replace=False)
```

Use instead:

```python
rng = np.random.default_rng(0)
idx = rng.choice(len(x), 100, replace=False)
```

#### rng-in-loop

**Severity:** warning. **Fix:** none.

Reports `np.random.default_rng()` without a seed inside a `for` or `while` body or a comprehension. Each call gathers entropy from the operating system, about 5 microseconds. A loop that seeds each pass from its loop variable is left alone.

Example:

```python
noise = [np.random.default_rng().normal() for _ in range(n)]
```

Use instead:

```python
rng = np.random.default_rng()
noise = rng.normal(size=n)
```

#### clock-seed

**Severity:** warning. **Fix:** unsafe: `np.random.default_rng()`, `np.random.seed()`, `np.random.RandomState()`, `np.random.SeedSequence()`. The numbers change, and a seed recovered from a logged start time no longer repeats the run.

Reports a seed taken from `time.time()`, `time.time_ns()`, `time.perf_counter()`, `datetime.now()` and similar clocks. Two runs that start in the same tick get the same numbers, and NumPy recommends entropy from the operating system, logged when a run must be repeatable.

Example:

```python
rng = np.random.default_rng(int(time.time()))
```

Use instead:

```python
rng = np.random.default_rng()
print(rng.bit_generator.seed_seq.entropy)
```

#### legacy-polynomial

**Severity:** info. **Fix:** none, because `numpy.polynomial` takes coefficients from degree zero up, the reverse of the old order.

Reports `np.polyfit`, `np.poly1d`, `np.polyval`, `np.roots`, `np.poly`, `np.polyder`, `np.polyint`, `np.polyadd`, `np.polysub`, `np.polymul` and `np.polydiv`.

Example:

```python
slope, intercept = np.polyfit(x, y, 1)
```

Use instead:

```python
intercept, slope = np.polynomial.Polynomial.fit(x, y, 1).convert().coef
```

#### almost-equal-assert

**Severity:** info. **Fix:** unsafe: `np.testing.assert_allclose(a, b, rtol=0, atol=1.5e-7)`, with `1.5e-6` for `assert_array_almost_equal` and `1.5e-<decimal>` for a literal decimal. A difference of exactly `atol` failed before and passes after, dicts and ragged lists are no longer compared item by item, and complex values are compared by magnitude. A bare imported name, an `err_msg` argument, a computed decimal and `assert_approx_equal` have no fix.

Reports `assert_almost_equal`, `assert_array_almost_equal` and `assert_approx_equal`, which NumPy lists under "Asserts (not recommended)".

Example:

```python
np.testing.assert_array_almost_equal(result, expected, decimal=4)
```

Use instead:

```python
np.testing.assert_allclose(result, expected, rtol=0, atol=1.5e-4)
```

#### assert-on-arrays

**Severity:** info. **Fix:** unsafe: `np.testing.assert_allclose(a, b, rtol=1e-05, atol=1e-08, equal_nan=False)` and `np.testing.assert_array_equal(a, b)`. assert_allclose fails on shapes that `np.allclose` broadcasts, assert_array_equal treats NaN in the same place as equal, and both still run under `python -O`. An assert with a message, or `np.allclose` with more arguments, has no fix.

Reports `assert np.allclose(a, b)`, `assert np.array_equal(a, b)`, `assert np.all(a == b)` and `assert (a == b).all()`. A failed assert shows only False, and the testing functions show the differing elements.

Example:

```python
assert np.allclose(weights, expected)
```

Use instead:

```python
np.testing.assert_allclose(weights, expected, rtol=1e-05, atol=1e-08)
```

#### legacy-testing-helper

**Severity:** info. **Fix:** unsafe: `assert x` and `assert x, msg`. The statement is skipped under `python -O`, and its message is built only on failure. `assert_raises` has no fix.

Reports `numpy.testing.assert_`, which NumPy's testing guidelines call the legacy form of the assert statement, and `numpy.testing.assert_raises` and `assert_raises_regex`, where they prefer `pytest.raises`. A bare `assert_raises` counts only in a file that imports it from `numpy.testing`, because SciPy imports `pytest.raises` under that name.

Example:

```python
assert_(res.success, res.message)
```

Use instead:

```python
assert res.success, res.message
```

#### char-module

**Severity:** info. **Fix:** safe: `np.strings.upper` and every other name that is the same function in both modules. Unsafe: the comparisons, such as `np.strings.equal`, which do not strip trailing whitespace, and `np.strings.multiply`, which raises a different error on bad input. `partition` and `rpartition` have no fix, because `np.strings` returns a tuple of three arrays.

Reports `np.char` functions, which NumPy's docs call legacy, where `np.strings` from NumPy 2.0 has the same name.

Example:

```python
names = np.char.upper(names)
```

Use instead:

```python
names = np.strings.upper(names)
```

#### dot-on-matrices

**Severity:** info. **Fix:** unsafe: `a @ b` when both operands are names, attributes, subscripts, calls or parenthesized. `@` raises on a scalar and on two Python lists, and on arrays of more than two dimensions it works on stacks of matrices, where `np.dot` does not.

Reports `np.dot(a, b)`. The np.dot docs prefer `matmul` or `a @ b` for two 2-D arrays.

Example:

```python
cov = np.dot(X.T, X) / n
```

Use instead:

```python
cov = X.T @ X / n
```

#### mergesort-kind

**Severity:** info. **Fix:** safe: `kind="stable"` in `np.sort` and `np.argsort`. unsafe: the same in a `sort` or `argsort` method, because the pattern cannot tell an array from another object.

Reports `kind="mergesort"` in `np.sort`, `np.argsort` and the `sort` and `argsort` methods. The docs keep "mergesort" only for backwards compatibility, and both names select the same stable sort.

Example:

```python
order = np.argsort(scores, kind="mergesort")
```

Use instead:

```python
order = np.argsort(scores, kind="stable")
```

#### printoptions-restore

**Severity:** info. **Fix:** none, because the code between the two calls has to move into the `with` block.

Reports `np.set_printoptions(**saved)`, which restores options changed for a while. The docs say to use `np.printoptions` as a context manager for that, which also restores the options when an error is raised.

Example:

```python
saved = np.get_printoptions()
np.set_printoptions(precision=3)
print(table)
np.set_printoptions(**saved)
```

Use instead:

```python
with np.printoptions(precision=3):
    print(table)
```

#### as-strided

**Severity:** info. **Fix:** none, because the shape and strides decide which safe function fits.

Reports `as_strided(...)`. Its docs ask for extreme care, because a wrong shape or stride reads memory outside the array, and point to `sliding_window_view`, `broadcast_to` and `reshape`.

Example:

```python
windows = as_strided(x, shape=(len(x) - 2, 3), strides=(x.strides[0], x.strides[0]))
```

Use instead:

```python
windows = np.lib.stride_tricks.sliding_window_view(x, 3)
```

#### numpy-import-alias

**Severity:** info. **Fix:** none, because every use of the old name changes with the import.

Reports `import numpy as <name>` with a name other than `np` or `xp`, and `from numpy import *`, which replaces Python's `sum`, `min`, `max`, `round`, `abs`, `all` and `any`.

Example:

```python
import numpy as N
```

Use instead:

```python
import numpy as np
```

#### unpackbits-popcount

**Severity:** warning. **Fix:** none, because the view, reshape and axis around unpackbits vary.

Reports `np.unpackbits(...).sum()`, `np.sum(np.unpackbits(...))` and `np.count_nonzero(np.unpackbits(...))`. np.bitwise_count, added in NumPy 2.0, counts the set bits of each value, and ran 37 times faster on 1,000,000 uint64 values.

Example:

```python
ones = np.unpackbits(masks.view(np.uint8)).sum()
```

Use instead:

```python
ones = np.bitwise_count(masks).sum()
```

## SciPy

These rules run on files that import `scipy`. Examples assume `import numpy as np`. The rules
follow SciPy 1.18. Every claim was run on SciPy 1.18.1 with NumPy 2.5.4. A form that is called
removed or deprecated raises there under `python -W error`. A form that is called inexact lost at
least 3 decimal digits against mpmath at 50 digits, where the form to write lost fewer than 1. Each
speedup is the range over repeated runs on one core, at 1,000,000 values unless the entry says
otherwise, and the faster form was not slower at 1,000. A rule with severity info states only
what SciPy's own source calls legacy or prefers, and claims nothing about speed or accuracy.

A rule reports a name only when it is rooted at `scipy` or the file's own imports bind it from
`scipy`. A form made only of NumPy names, such as `np.linalg.inv(a) @ b`, belongs to the
[NumPy](#numpy) rules. A safe fix gives the same object or a bit equal value of the same type, and
is not offered where the file binds the name again or the call spreads `*args` or `**kwargs`. An
unsafe fix names what can differ. A rule that asks for an API newer than the project's SciPy is
skipped.

### Names that are gone or that warn

#### removed-in-scipy

**Severity:** error. **Fix:** unsafe: `trapezoid`, `cumulative_trapezoid` and `simpson` for `trapz`, `cumtrapz` and `simps`, `pinv` for `pinv2`, `eye` for `speye`, `median_abs_deviation` and `gibrat` for `median_absolute_deviation` and `gilbrat`, each in the module the code already names, `from scipy.integrate import trapezoid as trapz` for an import of the old name, `scipy.signal.windows.<name>` for a window on the `scipy.signal` namespace, `stats.mstats.find_repeats` and `kendalltau_seasonal`, and `np.<name>` for a NumPy name on the `scipy` namespace. Each is unsafe because the old body is not in SciPy 1.18.1, so the fix cannot be shown to give the same result. The other forms have no fix.

Reports the SciPy names that raise on SciPy 1.18.1, each with what to write instead: the integration, window, wavelet, Legendre, spherical Bessel, statistics and linear algebra functions that SciPy removed, `interp2d`, which raises `NotImplementedError` when called, everything in `scipy.misc`, which is empty and warns when imported, and the NumPy functions and constants that the `scipy` namespace no longer re-exports. A name without a public replacement says so. Code under a SciPy version check, a `hasattr` check, or a `try` that catches `ImportError` or `AttributeError` is left alone, because it exists for old SciPy.

Example:

```python
from scipy import integrate, signal, stats
from scipy.misc import comb
area = integrate.trapz(y, x)
window = signal.hann(64)
p = stats.binom_test(k, n)
angle = scipy.pi * frac
```

Use instead:

```python
from scipy import integrate, signal, stats
from scipy.special import comb
area = integrate.trapezoid(y, x)
window = signal.windows.hann(64)
p = stats.binomtest(k, n).pvalue
angle = np.pi * frac
```

#### deprecated-in-scipy

**Severity:** warning. **Fix:** safe: the public namespace for a name taken from one of the 107 private modules that SciPy deprecated, such as `scipy.sparse.csr_matrix` for `scipy.sparse.csr.csr_matrix` and `from scipy.ndimage import gaussian_filter` for `from scipy.ndimage.filters import gaussian_filter`, which give the same object, and `tri.find_simplex(xi)` for `scipy.spatial.tsearch(tri, xi)`, which is what `tsearch` returns. The other forms have no fix.

Reports the names that warn with `DeprecationWarning` on SciPy 1.18.1: every name taken from a private module such as `scipy.stats.stats`, `scipy.linalg.basic`, `scipy.ndimage.filters` or `scipy.sparse.linalg.isolve`, an import of such a module, `scipy.odr`, which goes away in SciPy 1.19, and calls of `minkowski_distance`, `minkowski_distance_p`, `distance_matrix`, `tsearch`, `lagrange`, `pade`, `approximate_taylor_polynomial` and `py_vq`, each with the replacement that SciPy's warning names.

Example:

```python
from scipy.ndimage.filters import gaussian_filter
from scipy.stats.stats import pearsonr
smooth = scipy.sparse.linalg.isolve.cg(A, b)
cell = scipy.spatial.tsearch(tri, points)
D = scipy.spatial.distance_matrix(X, Y)
```

Use instead:

```python
from scipy.ndimage import gaussian_filter
from scipy.stats import pearsonr
smooth = scipy.sparse.linalg.cg(A, b)
cell = tri.find_simplex(points)
D = scipy.spatial.distance.cdist(X, Y)
```

### Arguments, defaults and legacy entry points

#### removed-keyword

**Severity:** error. **Fix:** unsafe: `rtol=` for `tol=` on the sparse.linalg solvers, because the old tol came with an atol default that SciPy 1.18 rejects, `restart=` for gmres `restrt=`, `assume_a='pos'` for linalg.solve `sym_pos=True`, `subset_by_index=` for eigh and eigvalsh `eigvals=`, `workers=` for `n_jobs=` on the KDTree and cKDTree query methods, and `scale=1.0` for iqr and median_abs_deviation `scale='raw'`, because no source on SciPy 1.18 shows the old code. The other forms have no fix.

Reports keywords and argument values that SciPy removed and that raise on SciPy 1.18.1: `tol=`, `atol='legacy'` and a fourth positional argument on cg, gmres and the other iterative solvers, gmres `restrt=`, linalg.solve `sym_pos=`, eigh and eigvalsh `eigvals=` and `turbo=`, pinv and pinvh `cond=` and `rcond=`, integrate.simpson `even=`, cumulative_trapezoid with an `initial` other than 0, `n_jobs=` on the methods of a KDTree or cKDTree, kendalltau `initial_lexsort=`, iqr and median_abs_deviation `scale='raw'`, fmin_l_bfgs_b `iprint=` and `disp=`, and the metric `'wminkowski'` of cdist and pdist.

Example:

```python
x, info = cg(A, b, tol=1e-8)
w = linalg.eigh(a, eigvals=(0, 2), eigvals_only=True)
dist, idx = tree.query(points, k=3, n_jobs=-1)
```

Use instead:

```python
x, info = cg(A, b, rtol=1e-8)
w = linalg.eigh(a, subset_by_index=(0, 2), eigvals_only=True)
dist, idx = tree.query(points, k=3, workers=-1)
```

#### deprecated-argument

**Severity:** warning. **Fix:** none.

Reports arguments that warn on SciPy 1.18.1 and that SciPy will remove: lombscargle `precenter=` and a fourth or fifth positional argument, linalg.qr `lwork=` by keyword or by position, anderson_ksamp `midrank=` by keyword or by position, and linprog with `method='simplex'`, `'interior-point'` or `'revised simplex'`. The rules that name `variant=` are skipped on SciPy older than 1.17.

Example:

```python
power = signal.lombscargle(t, y, freqs, precenter=True)
res = optimize.linprog(c, A_ub=A, b_ub=b, method="interior-point")
```

Use instead:

```python
power = signal.lombscargle(t, y - y.mean(), freqs)
res = optimize.linprog(c, A_ub=A, b_ub=b, method="highs")
```

#### default-that-warns

**Severity:** warning. **Fix:** safe: `callback_type='legacy'` for gmres with a callback, and `spmatrix=True` for hb_read and mmread. Each is what SciPy 1.18.1 does after it warns. The other forms have no fix.

Reports calls that rely on a default that warns on SciPy 1.18.1: gmres with `callback=` and no `callback_type=`, stats.anderson without `method=`, anderson_ksamp without `variant=`, io.hb_read and io.mmread without `spmatrix=`, and ndimage.zoom with `grid_mode=True` and mode `'constant'`, the default, or `'wrap'`. The rules that name an argument added in SciPy 1.17 or 1.18 are skipped on an older SciPy.

Example:

```python
x, info = gmres(A, b, callback=log)
A = mmread("matrix.mtx")
result = stats.anderson(sample)
```

Use instead:

```python
x, info = gmres(A, b, callback=log, callback_type="legacy")
A = mmread("matrix.mtx", spmatrix=True)
result = stats.anderson(sample, method="interpolate")
```

#### minimize-unknown-option

**Severity:** warning. **Fix:** none.

Reports a key in the `options=` dict of `optimize.minimize` that the chosen `method=` does not take, when another method takes it. Each such pair warns OptimizeWarning and is ignored on SciPy 1.18.1, or raises TypeError for `'trust-constr'`. Examples are `disp` and `iprint` for L-BFGS-B, `maxiter` for TNC and `maxfun` for BFGS. The keys of every method were measured by running each pair.

Example:

```python
res = minimize(f, x0, method="L-BFGS-B", options={"disp": True, "maxiter": 200})
```

Use instead:

```python
res = minimize(f, x0, method="L-BFGS-B", options={"maxiter": 200})
```

#### minimize-unused-argument

**Severity:** warning. **Fix:** none.

Reports `jac=` with Nelder-Mead, Powell, COBYLA or COBYQA, `hess=` and `hessp=` with a method that uses no Hessian, `constraints=` with a method that cannot handle them, and `bounds=` with CG, BFGS, Newton-CG or a trust-region method other than trust-constr. minimize ignores the argument and warns RuntimeWarning on SciPy 1.18.1.

Example:

```python
res = minimize(f, x0, method="BFGS", jac=grad, bounds=[(0, 1)] * n)
```

Use instead:

```python
res = minimize(f, x0, method="L-BFGS-B", jac=grad, bounds=[(0, 1)] * n)
```

#### rng-keyword

**Severity:** info. **Fix:** none. The same integer gives a different stream under `rng=` than under `random_state=` or `seed=`.

Reports `random_state=` and `seed=` passed by keyword to the functions and classes that SciPy 1.15 moved to `rng=`, such as sparse.random, svds, bootstrap, permutation_test, kmeans2, differential_evolution, basinhopping, dual_annealing, directed_hausdorff, Rotation.random and the qmc engines. SciPy's docs say new code should pass `rng`. The old keywords run without a warning on SciPy 1.18.1. Skipped on SciPy older than 1.15.

Example:

```python
engine = qmc.Sobol(d=2, seed=0)
X = sparse.random(1000, 100, density=0.01, random_state=0)
```

Use instead:

```python
engine = qmc.Sobol(d=2, rng=0)
X = sparse.random(1000, 100, density=0.01, rng=0)
```

#### legacy-optimize

**Severity:** info. **Fix:** none, because the newer functions take other argument names and return an OptimizeResult.

Reports the functions that scipy.optimize lists as legacy and not recommended for new scripts: fmin, fmin_powell, fmin_cg, fmin_bfgs, fmin_ncg, fmin_l_bfgs_b, fmin_tnc, fmin_cobyla, fmin_slsqp, fminbound, brent, golden, leastsq, fsolve, broyden1, broyden2, newton_krylov, anderson, excitingmixing, linearmixing and diagbroyden. The message names the `minimize`, `minimize_scalar`, `least_squares` or `root` call that replaces each.

Example:

```python
xopt = optimize.fmin_bfgs(f, x0, fprime=grad, disp=False)
```

Use instead:

```python
xopt = optimize.minimize(f, x0, jac=grad, method="BFGS").x
```

#### legacy-odeint

**Severity:** info. **Fix:** none, because solve_ivp calls `fun(t, y)` where odeint calls `func(y, t)`, and returns an object.

Reports `integrate.odeint`, which SciPy's docs ask new code to replace with `solve_ivp`. Neither solver was faster on every problem measured, so the message makes no speed claim. It says to keep solve_ivp's default method for a large system, because `method='LSODA'` stores a full Jacobian and raised MemoryError at 1,000,000 equations.

Example:

```python
ys = odeint(lambda y, t: -k * y, y0, t)
```

Use instead:

```python
ys = solve_ivp(lambda t, y: -k * y, (t[0], t[-1]), y0, t_eval=t).y.T
```

#### legacy-interpolate

**Severity:** info. **Fix:** none, because each replacement takes other arguments and returns a BSpline or another object.

Reports the interpolation classes and functions that SciPy marks as legacy: interp1d, Rbf, UnivariateSpline, InterpolatedUnivariateSpline, LSQUnivariateSpline, splrep, splprep, splev, splint, sproot, spalde, splder and splantider. The message names the replacement SciPy recommends, such as make_splrep for splrep and RBFInterpolator for Rbf. Documents that name an API added in SciPy 1.7 or 1.15 are skipped on an older SciPy.

Example:

```python
tck = interpolate.splrep(x, y, s=0.1)
smooth = interpolate.splev(grid, tck)
```

Use instead:

```python
spline = interpolate.make_splrep(x, y, s=0.1)
smooth = spline(grid)
```

#### legacy-signal

**Severity:** info. **Fix:** safe: `freqz_sos` for `sosfreqz`, whose body only calls freqz_sos. The other forms have no fix.

Reports signal.stft, istft, spectrogram and check_COLA, which SciPy marks as legacy in favor of the ShortTimeFFT class, and signal.sosfreqz, a legacy alias of freqz_sos. The sosfreqz document is skipped on SciPy older than 1.15.

Example:

```python
w, h = signal.sosfreqz(sos, worN=512)
```

Use instead:

```python
w, h = signal.freqz_sos(sos, worN=512)
```

#### legacy-api

**Severity:** info. **Fix:** safe: `ndimage.sum_labels` for `ndimage.sum`, whose body only calls sum_labels. The other forms have no fix.

Reports ndimage.sum, which SciPy keeps for backwards compatibility, stats.ranksums, which SciPy marks as legacy in favor of mannwhitneyu with the same p-value and the U statistic in place of z, and sparse.bmat, which returns a sparse matrix where block_array returns a sparse array.

Example:

```python
totals = ndimage.sum(image, labels, index)
stat, p = stats.ranksums(x, y)
```

Use instead:

```python
totals = ndimage.sum_labels(image, labels, index)
u, p = stats.mannwhitneyu(x, y, use_continuity=False, method="asymptotic")
```

### Forms that keep digits

#### log-of-special

**Severity:** warning. **Fix:** unsafe: `special.log_ndtr(x)`, `special.gammaln(x)`, `special.gammaln(n + 1)`, `special.betaln(a, b)`, `special.log_expit(x)`, `special.log_softmax(x)`. `log_expit` needs SciPy 1.8 and `log_softmax` SciPy 1.5.

Reports `np.log` of `special.ndtr`, `special.gamma`, `special.factorial`, `special.beta`, `special.expit` and `special.softmax`, through any import of `scipy.special`, and without a fix when the function was imported by name.

Example:

```python
loglike = np.log(special.ndtr(q * xb)).sum()
```

Use instead:

```python
loglike = special.log_ndtr(q * xb).sum()
```

#### log-of-dist-method

**Severity:** warning. **Fix:** unsafe: `dist.logpdf(...)`, `dist.logpmf(...)`, `dist.logcdf(...)`, `dist.logsf(...)`.

Reports `np.log` of the `pdf`, `pmf`, `cdf` or `sf` of a `scipy.stats` distribution, written as `stats.norm`, imported by name as `norm`, or frozen in the same expression as `stats.norm(0, 2)`. Each method has its own list of distributions, measured one by one, so `np.log(stats.binom.pmf(k, n, p))` is not reported.

Example:

```python
ll = np.log(stats.norm.pdf(resid, scale=sigma)).sum()
```

Use instead:

```python
ll = stats.norm.logpdf(resid, scale=sigma).sum()
```

#### cdf-complement

**Severity:** warning. **Fix:** unsafe: `dist.sf(...)`, `dist.cdf(...)`.

Reports `1 - dist.cdf(...)` and `1 - dist.sf(...)` for a `scipy.stats` distribution from a measured list.

Example:

```python
pvalue = 2 * (1 - stats.norm.cdf(np.abs(z)))
```

Use instead:

```python
pvalue = 2 * stats.norm.sf(np.abs(z))
```

#### special-complement

**Severity:** warning. **Fix:** unsafe: `special.erfc(x)`, `special.erf(x)`, `special.gammaincc(a, x)`, `special.gammainc(a, x)`, `special.betaincc(a, b, x)`, `special.erfc(-x)`, `special.ndtr(-x)`, `special.expit(-x)`. `betaincc` needs SciPy 1.11.

Reports `1 - special.erf(x)`, `1 - special.erfc(x)`, `1 - special.gammainc(a, x)`, `1 - special.gammaincc(a, x)`, `1 - special.betainc(a, b, x)`, `1 + special.erf(x)`, `1 - special.ndtr(x)` and `1 - special.expit(x)`.

Example:

```python
cdf = 0.5 * (1 + special.erf(x / np.sqrt(2)))
```

Use instead:

```python
cdf = 0.5 * special.erfc(-x / np.sqrt(2))
```

#### ppf-of-complement

**Severity:** warning. **Fix:** unsafe: `dist.isf(p, ...)`, `dist.ppf(p, ...)`, `-special.ndtri(p)`.

Reports `dist.ppf(1 - p, ...)` and `dist.isf(1 - p, ...)` for a continuous `scipy.stats` distribution from a measured list, and `special.ndtri(1 - p)`. A probability written as a number, such as `1 - 0.05`, is left alone unless it is below 5e-7.

Example:

```python
crit = stats.norm.ppf(1 - alpha / 2)
```

Use instead:

```python
crit = stats.norm.isf(alpha / 2)
```

#### comb-by-factorials

**Severity:** warning. **Fix:** unsafe: `special.comb(n, k)`.

Reports `special.factorial(n) / (special.factorial(k) * special.factorial(n - k))` and the same with the factors in another order or divided one at a time.

Example:

```python
ways = special.factorial(n) / (special.factorial(k) * special.factorial(n - k))
```

Use instead:

```python
ways = special.comb(n, k)
```

#### log-of-scipy-det

**Severity:** warning. **Fix:** unsafe: `np.linalg.slogdet(a)[1]`.

Reports `np.log(linalg.det(a))`, `np.log(np.abs(linalg.det(a)))` and `np.log(abs(linalg.det(a)))` where `linalg` is `scipy.linalg`, or `det` was imported from it. `np.log(np.linalg.det(a))` belongs to the NumPy rule `log-of-det`.

Example:

```python
logdet = np.log(linalg.det(cov))
```

Use instead:

```python
logdet = np.linalg.slogdet(cov)[1]
```

### SciPy calls made once per value or once per pair

Each old form calls SciPy from Python once per value, once per pair of points or once per pass of a loop, where one call on whole arrays does the same work in compiled code. Speedups are medians of 7 runs on one core with SciPy 1.18.1, on float64 and float32, at 1,000,000 values unless the entry says what the 1,000,000 counts.

#### distance-per-pair

**Severity:** warning. **Fix:** none.

Reports a metric function of `scipy.spatial.distance`, such as `euclidean`, `cityblock` or `cosine`, called inside a comprehension, a generator or a loop body. `cdist` ran 960 to 9,600 times faster on 1,000 by 1,000 points.

Example:

```python
D = np.array([[distance.euclidean(a, b) for b in Y] for a in X])
```

Use instead:

```python
D = distance.cdist(X, Y, "euclidean")
```

#### callable-metric

**Severity:** warning. **Fix:** unsafe: the metric's name as a string. `cosine` and `correlation` differ in the last bits, and complex input is cast to float.

Reports a metric function of `scipy.spatial.distance` passed to `cdist` or `pdist` as the metric, by position or as `metric=`. The name runs the same metric in C, 720 to 7,600 times faster on 1,000 by 1,000 points.

Example:

```python
D = cdist(X, Y, distance.euclidean)
```

Use instead:

```python
D = cdist(X, Y, "euclidean")
```

#### per-value-scipy-call

**Severity:** warning. **Fix:** none.

Reports a comprehension or generator whose element is a call to a `scipy.special` ufunc, a `scipy.stats` distribution method such as `norm.cdf`, a distribution frozen from the loop name such as `norm(m, 2.0).pdf(x)`, or `stats.scoreatpercentile(x, p)` over the percentiles, with the loop name as an argument. One call on the array ran 7 to 23 times faster for `special`, 34 to 1,045 times faster for `stats` methods, 4.9 to 23 times faster for frozen distributions, and 2.1 to 3.2 times faster for 1,000 percentiles, because `scoreatpercentile` sorts the data on every call. A filter with `if`, a second `for` and an element that does more than the call are left alone.

Example:

```python
p = np.array([stats.norm.cdf(v) for v in x])
```

Use instead:

```python
p = stats.norm.cdf(x)
```

#### query-per-point

**Severity:** warning. **Fix:** none.

Reports `tree.query(q)` inside a comprehension or a `for` loop, where `q` is the loop name or a row indexed by it and `tree` comes from a `KDTree` or `cKDTree` call in the same function. One query of all points ran 17 to 36 times faster for 1,000,000 points on a tree of 10,000.

Example:

```python
tree = KDTree(points)
nearest = [tree.query(q)[1] for q in queries]
```

Use instead:

```python
tree = KDTree(points)
nearest = tree.query(queries)[1]
```

#### solve-in-loop

**Severity:** warning. **Fix:** none.

Reports `scipy.sparse.linalg.spsolve(A, b)` and `scipy.linalg.solve(A, b)` inside a comprehension or a loop that does not assign `A` or take it as its loop name. Each call factors `A` again. Factoring once ran 17 to 20 times faster for 100 right sides of a sparse matrix of 10,000 rows, and 18 to 23 times faster for 20 right sides of a dense 1,000 by 1,000 matrix.

Example:

```python
xs = [spsolve(A, b) for b in sides]
```

Use instead:

```python
lu = splu(A.tocsc())
xs = [lu.solve(b) for b in sides]
```

#### sparse-inv-then-multiply

**Severity:** warning. **Fix:** unsafe: `spla.spsolve(A, b)` when `inv` is reached through its module. The last bits change, and a `b` of shape `(n, 1)` gives a 1-D result. `inv` imported by name has no fix.

Reports `scipy.sparse.linalg.inv(A) @ b` and `inv(A).dot(b)`. The inverse of most sparse matrices is dense. `spsolve` ran 260 to 620 times faster on a tridiagonal matrix of 1,000 rows, whose inverse holds 1,000,000 values.

Example:

```python
x = spla.inv(A) @ b
```

Use instead:

```python
x = spla.spsolve(A, b)
```

### Sparse matrices

#### empty-csr-write

**Severity:** warning. **Fix:** none.

Reports a write into a `csr_matrix`, `csc_matrix`, `csr_array` or `csc_array` built from a shape alone, as in `A = csr_matrix((n, m))` followed by `A[i, j] = v`, `A[i, j] += v` or a slice write, in a function that does not assign `A` again. Each write adds a stored value, which emits `SparseEfficiencyWarning` and raises under `python -W error` on SciPy 1.18.1, and copies the whole matrix. 1,000 writes ran 30 times slower than into a `lil_matrix`.

Example:

```python
A = sparse.csr_matrix((n, n))
for i, j, v in triples:
    A[i, j] = v
```

Use instead:

```python
A = sparse.lil_array((n, n))
for i, j, v in triples:
    A[i, j] = v
A = A.tocsr()
```

#### sparse-matrix-api

**Severity:** info. **Fix:** none, because `*` and `**` work elementwise on a sparse array, a row slice of one is 1-D and `todense` gives an ndarray. Skipped on SciPy older than 1.8 for the classes and 1.12 for the builder functions.

Reports a call to `csr_matrix`, `csc_matrix`, `coo_matrix`, `lil_matrix`, `dok_matrix`, `bsr_matrix` or `dia_matrix`, whose SciPy docstrings say the sparse matrix interface is expected to be deprecated, and to `eye`, `identity`, `diags`, `spdiags`, `rand`, `random` and `bmat`, whose docstrings encourage `eye_array`, `diags_array`, `dia_array`, `random_array` and `block_array`. A function that also names a sparse array class, and a branch taken on a condition that mentions a matrix, are left alone, because such code builds the matrix on purpose.

Example:

```python
X = sparse.csr_matrix(X)
I = sparse.eye(n)
```

Use instead:

```python
X = sparse.csr_array(X)
I = sparse.eye_array(n)
```

### The API that SciPy's docs prefer on SciPy 1.18

#### ckdtree

**Severity:** info. **Fix:** none, because the default `leafsize` differs, 16 against 10. Skipped on SciPy older than 1.6.

Reports a call to `cKDTree`. Its docstring says it is functionally identical to `KDTree` and kept only for backward compatibility.

Example:

```python
tree = cKDTree(points)
```

Use instead:

```python
tree = KDTree(points)
```

#### legacy-fftpack

**Severity:** info. **Fix:** none, because `rfft` packs its output differently and the pseudo differential operators have no `scipy.fft` form.

Reports an import of `scipy.fftpack`, whose module docstring is a legacy directive that says new code should use `scipy.fft`. `scipy.fft` was no faster in the measurements, so the rule claims only the preference.

Example:

```python
from scipy import fftpack
y = fftpack.dct(x)
```

Use instead:

```python
from scipy import fft
y = fft.dct(x)
```

## scikit-learn

These rules run on files that import `sklearn`. Each old form gives a wrong or leaked score, or a
score that changes from run to run. Speed plays no part. The numbers are means of 30 seeds on
scikit-learn 1.9.1. On data with no signal, an honest accuracy is 0.5.

Several of these rules read the names of variables, such as `X_test` and `y_pred`, because that
is all a pattern can see. A name that misleads gives a wrong report.

### Leakage

#### fit-on-test

**Severity:** error. **Fix:** unsafe: `scaler.transform(X_test)`. `fit` on test data has no fix.

Reports `scaler.fit_transform(X_test)` and `model.fit(X_test, y_test)`, which learn from the test rows.

Example:

```python
X_train = scaler.fit_transform(X_train)
X_test = scaler.fit_transform(X_test)
```

Use instead:

```python
X_train = scaler.fit_transform(X_train)
X_test = scaler.transform(X_test)
```

#### fit-before-split

**Severity:** error. **Fix:** none.

Reports `X = selector.fit_transform(X, y)` followed by `train_test_split(X, y)`, and `fit_resample` before the split.

Example:

```python
X = selector.fit_transform(X, y)
X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=0)
```

Use instead:

```python
X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=0)
X_train = selector.fit_transform(X_train, y_train)
X_test = selector.transform(X_test)
```

The replacement is advice. xpd does not apply it automatically.

#### cross-val-on-fitted-data

**Severity:** error. **Fix:** none.

Reports `cross_val_score(model, scaler.fit_transform(X), y)`, and a fitted result passed to it from an earlier line.

Example:

```python
X_picked = selector.fit_transform(X, y)
scores = cross_val_score(model, X_picked, y, cv=5)
```

Use instead:

```python
pipeline = make_pipeline(selector, model)
scores = cross_val_score(pipeline, X, y, cv=5)
```

The replacement is advice. xpd does not apply it automatically.

#### get-dummies-on-test

**Severity:** error. **Fix:** none.

Reports `pd.get_dummies(X_test)`, which makes one column per value in the test rows.

Example:

```python
X_train = pd.get_dummies(X_train)
X_test = pd.get_dummies(X_test)
```

Use instead:

```python
encoder = OneHotEncoder(handle_unknown="ignore")
X_train = encoder.fit_transform(X_train)
X_test = encoder.transform(X_test)
```

The replacement is advice. xpd does not apply it automatically.


`fit-on-test` reads the first argument's name: `test`, `val`, `valid`, `validation` or `holdout`
for `fit_transform`, and `test` only for `fit`, because a fit on a validation set can be meant, as
in probability calibration. A scaler refitted on test rows that arrive 5 at a time scored 0.865
where `transform` scored 0.976. An encoder refitted on the test rows scored 0.795 against 0.905.
A model fitted on its test rows scored 1.000 on noise. The fix is unsafe, because a name can
mislead. `StandardScaler().fit_transform(X_test)` has no fix, because that scaler was never fitted
on the training data.

`fit-before-split` reports the fit when a later statement of the same block passes its result to
`train_test_split`. Feature selection before the split scored 0.781 on noise, where the honest
value was 0.503. SMOTE before the split gave an F1 of 0.842 on noise, where the honest value was
0.164. For `StandardScaler` the leak is real and small: 0.9673 against 0.9667. xpd cannot tell a
scaler from a selector, so it reports both. It leaves a label encoder on the target alone, when the
result is named `y`, `target` or `labels`, or the encoder's name holds `label`.

`cross-val-on-fitted-data` covers `cross_val_score`, `cross_validate`, `cross_val_predict`,
`learning_curve` and `validation_curve`. Cross-validated accuracy on noise was 0.781 where the
Pipeline gave 0.510. `GridSearchCV(...).fit(X_fitted, y)` leaks in the same way and is not
reported, because xpd cannot tell a search from any other object with `fit`.

`get-dummies-on-test` reads the first argument's name, as `fit-on-test` does. In 15 of 30 splits
`predict` raised on the columns. Passed on as arrays, 102 of 200 splits raised and 17 ran on the
wrong columns. A call followed by `reindex` or `align` is left alone.

### Metrics

#### auc-from-labels

**Severity:** error. **Fix:** none.

Reports `roc_auc_score(y_test, model.predict(X_test))`, which ranks class labels where the metric needs a score per row.

Example:

```python
auc = roc_auc_score(y_test, model.predict(X_test))
```

Use instead:

```python
auc = roc_auc_score(y_test, model.predict_proba(X_test)[:, 1])
```

The replacement is advice. xpd does not apply it automatically.

#### metric-arguments-swapped

**Severity:** error. **Fix:** unsafe: `precision_score(y_test, y_pred)`.

Reports `precision_score(y_pred, y_test)`, with the predictions first.

Example:

```python
precision = precision_score(y_pred, y_test)
```

Use instead:

```python
precision = precision_score(y_test, y_pred)
```


`auc-from-labels` covers `roc_auc_score`, `average_precision_score`, `roc_curve`,
`precision_recall_curve`, `log_loss` and `brier_score_loss`. The AUC was 0.755 from labels and
0.875 from probabilities. It also reports `predict_proba(X)[:, 0]`, which scored 0.126, and a
variable that holds the result of `predict` and reaches the metric on a later line. Files that
import keras, tensorflow or torch are left alone, because `predict` returns probabilities there.

`metric-arguments-swapped` covers `precision_score`, `recall_score`, `confusion_matrix`,
`classification_report`, `r2_score`, `explained_variance_score`, `mean_absolute_percentage_error`,
`balanced_accuracy_score` and `fbeta_score`. The swapped precision is the recall: 0.817 against
0.967. R2 went from 0.48 to -3.19. `accuracy_score` and `f1_score` return the same number either
way and are left alone. The rule reports when the first name holds `pred` or `hat` and the second
holds `true`, `test`, `val`, `actual`, `target` or `label`, or is `y`.

### Reproducible results

#### missing-random-state

**Severity:** warning. **Fix:** none.

Reports `train_test_split(X, y)`, `RandomForestClassifier()` and `KFold(shuffle=True)` without `random_state`.

Example:

```python
X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.2)
model = RandomForestClassifier()
```

Use instead:

```python
X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.2, random_state=0)
model = RandomForestClassifier(random_state=0)
```

The value 0 is only the example. Any fixed number makes the run repeatable.

The replacement is advice. xpd does not apply it automatically.


The same script, run 30 times, scored between 0.930 and 1.000. The rule names the classes whose
output changed between two runs: the random forests, extra trees, bagging and gradient boosting
models, `IsolationForest`, the decision trees, `KMeans`, `MiniBatchKMeans`, `GaussianMixture`, the
MLP and SGD models, `TruncatedSVD`, `NMF`, `TSNE`, `ShuffleSplit`, `StratifiedShuffleSplit` and
`RepeatedKFold`, and `KFold`, `StratifiedKFold`, `GroupKFold` and `StratifiedGroupKFold` with
`shuffle=True`. `LogisticRegression`, `SVC`, `LinearSVC` and `PCA` gave the same output twice and
are left alone. A call with `shuffle=False` or `**params` is left alone, and so is a file that
calls `np.random.seed`, because scikit-learn then draws from that seed.

## Hamilton

These rules run on files that import `hamilton`, the module of
[Apache Hamilton](https://hamilton.apache.org/), which PyPI ships as `apache-hamilton` and
`sf-hamilton`. Hamilton builds a dataflow from plain functions: the function name is the output
and the parameter names are its inputs. The results below are from sf-hamilton 1.90.0 with pandas
3.1.1.

A node module that uses no Hamilton decorator has no `hamilton` import, and xpd does not check
it.

### Shared inputs

#### node-mutates-input

**Severity:** error. **Fix:** none.

Reports a node that writes to one of its parameters, such as `raw["uk"] = ...`, `raw.loc[mask, "uk"] = 0`, `raw.columns = ...`, `del raw["dk"]`, `raw.pop("uk")` and `rows.append(...)`.

Example:

```python
def clipped(raw: pd.DataFrame) -> pd.DataFrame:
    raw.loc[raw["uk"] < 0, "uk"] = 0
    return raw


def total_uk(raw: pd.DataFrame) -> float:
    return float(raw["uk"].sum())
```

Use instead:

```python
def clipped(raw: pd.DataFrame) -> pd.DataFrame:
    return raw.assign(uk=raw["uk"].clip(lower=0))


def total_uk(raw: pd.DataFrame) -> float:
    return float(raw["uk"].sum())
```

The replacement is advice. xpd does not apply it automatically.

Hamilton calls each function once and hands its result, the same object, to every node that
takes it. With the example above, `execute(["total_uk"])` gave 2.0 and
`execute(["clipped", "total_uk"])` gave 4.0. With the replacement both gave 2.0. The Hamilton
documentation asks for the same under
[output immutability](https://hamilton.apache.org/concepts/best-practices/output-immutability/).

A node is a function at module level with a return hint and a name that does not start with an
underscore. The rule also reports a write through a second name for the input, as in
`gdf = buildings` followed by `gdf["area"] = ...`. It leaves these alone:

- A parameter the function rebinds first, as in `raw = raw.copy()`.
- A function under `@load_from` or `@dataloader`. No other node can take the frame they inject.
- A function under `@mutate` or `@hamilton_exclude`.
- `update`, `append` and the like when the result is used, as in `return state.update(row)`,
  because an immutable object returns a new one.
- `inplace=True`, which [no-inplace](#no-inplace) reports.

A write that reaches the input through a helper function is not seen.

## Python

These rules are about Python itself, so they run on every file, whatever it imports. They cover
style choices that no formatter or type checker makes. The `python-best-practices` skill in
[`skills/`](https://github.com/bjoernaagaard/xpd/tree/main/skills) holds the rest of the style,
and `python-review` reviews code against it.

### Names

#### module-private-name

**Severity:** warning. **Fix:** none.

Reports a function, class or variable at module level whose name starts with an underscore.

Example:

```python
_LIMIT = 100


def _prices(rows):
    return [row["price"] for row in rows][:_LIMIT]


def total(rows):
    return sum(_prices(rows))
```

Use instead:

```python
__all__ = ["total"]

LIMIT = 100


def prices(rows):
    return [row["price"] for row in rows][:LIMIT]


def total(rows):
    return sum(prices(rows))
```

The replacement is advice. xpd does not apply it automatically.

`__all__` says in one place which names callers may import, and `from module import *`, type
checkers and documentation tools all read it. An underscore says the same thing once per name
and makes every use of the name harder to read.

The rule leaves these alone:

- A method or attribute of a class, such as `def _load(self)`.
- A name inside a function.
- `_` alone, and names with two leading underscores such as `__all__` and `__version__`.
- `_T = TypeVar("_T")`, and the same for `ParamSpec` and `TypeVarTuple`. A type variable is
  never part of a module's API, and typeshed and the typing documentation write it with an
  underscore. On Python 3.12, `def first[T](xs: list[T]) -> T` needs no module level name.
- A file that imports `hamilton`, where an underscore tells Hamilton that a function is not a
  node.

#### vague-module-name

**Severity:** warning. **Fix:** none.

Reports a module named `util`, `utils`, `helper`, `helpers`, `common` or `misc`, and a package
with one of those names.

Example:

```text
orders/
├── utils.py        # parse_date, round_price, retry
└── report.py
```

Use instead:

```text
orders/
├── dates.py        # parse_date
├── prices.py       # round_price
└── report.py
```

The replacement is advice. xpd does not apply it automatically.

The name says nothing about what the module holds, so unrelated code collects in it and every
other module imports it. xpd reports the file on line 1.

### Modules

#### import-side-effect

**Severity:** warning. **Fix:** none.

Reports a call that stands alone at module level, and a read or write of `os.environ` or
`os.getenv` at module level.

Example:

```python
import logging
import os

import dspy

logging.basicConfig(level=logging.INFO)
dspy.configure(lm=dspy.LM("openai/gpt-4o-mini"))
REGION = os.environ.get("REGION", "eu")


def answer(question: str) -> str:
    return dspy.Predict("question -> answer")(question=question).answer
```

Use instead:

```python
import logging
import os

import dspy


def answer(question: str) -> str:
    return dspy.Predict("question -> answer")(question=question).answer


def main() -> None:
    logging.basicConfig(level=logging.INFO)
    dspy.configure(lm=dspy.LM("openai/gpt-4o-mini"))
    print(answer(os.environ.get("QUESTION", "What is xpd?")))
```

The replacement is advice. xpd does not apply it automatically.

A module level call runs on the first import, in an order nobody chose, and again in every
process that imports the module. A program that imports the module to use one function gets
its logging and its library settings changed, and the environment is read before the program
has parsed its own arguments.

The rule leaves these alone:

- A file with no function and no class. It is a script, which runs from top to bottom.
- A `__main__.py`, which is run and never imported.
- A method that edits a collection the module defines, such as `NAMES.append("id")`.
- A call whose name starts with `register`, and `include_router`, `add_typer`,
  `add_middleware` and `mount` on a web or command line app.
- `pytest.importorskip`, which is how a test module skips itself.
- A call inside `if`, `try`, `with` or a class body, and a call whose result is assigned.

### Types

#### typing-cast

**Severity:** warning. **Fix:** none.

Reports a call to `typing.cast`.

Example:

```python
from typing import cast


def port(settings: dict[str, object]) -> int:
    return cast(int, settings["port"])
```

Use instead:

```python
def port(settings: dict[str, object]) -> int:
    value = settings["port"]
    if not isinstance(value, int):
        raise TypeError(f"port must be an int: {value!r}")
    return value
```

The replacement is advice. xpd does not apply it automatically.

`cast` tells the type checker to believe a type that nothing checks, so a wrong value fails at
run time, far from the cast. `isinstance`, `match` and a `TypeIs` function narrow the type for
the checker and check it in the program.

xpd reports `typing.cast(...)`, and `cast(...)` in a file that imports `cast` from `typing` or
`typing_extensions`. ruff has no rule that reports a cast. Its `TC006` only asks for quotes
around the type.

#### dict-str-any

**Severity:** warning. **Fix:** none.

Reports `dict[str, Any]` as the type of a parameter or a return value. `Dict`, `Mapping` and
`MutableMapping` with the same arguments are reported too.

Example:

```python
from typing import Any


def total(order: dict[str, Any]) -> float:
    return order["price"] * order["quantity"]
```

Use instead:

```python
from dataclasses import dataclass


@dataclass(frozen=True, slots=True, kw_only=True)
class Order:
    price: float
    quantity: int


def total(order: Order) -> float:
    return order.price * order.quantity
```

The replacement is advice. xpd does not apply it automatically.

The type says nothing about the keys or the values, so every function that takes the dict has
to know its shape by heart, and the type checker cannot find a misspelled key. Decode the data
once, where it enters the program, into a dataclass or a `TypedDict`.

ruff's `ANN401` reports a parameter annotated with `Any` alone, and leaves `dict[str, Any]`.

### Tests

#### mock-import

**Severity:** warning. **Fix:** none.

Reports an import of `unittest.mock`, `mock` or `pytest_mock`.

Example:

```python
from unittest.mock import patch


def test_total():
    with patch("orders.load", return_value=[{"price": 2.0}]):
        assert total() == 2.0
```

Use instead:

```python
def test_total(tmp_path):
    path = tmp_path / "orders.csv"
    path.write_text("price\n2.0\n", encoding="utf-8")
    assert total(path) == 2.0
```

The replacement is advice. xpd does not apply it automatically.

A test that replaces a function with a mock passes when the mock is called the way the test
expects, whatever the real function does. Run the real code on real input.

### Comprehensions

#### comprehension-merges-dicts

**Severity:** warning. **Fix:** none.

Reports a comprehension or generator expression that merges dicts with `**` for every item.

Example:

```python
def table(models, catalog):
    return [{"id": m.id, **owner(m), **limits(catalog, m)} for m in models]
```

Use instead:

```python
def table(models, catalog):
    rows = []
    for m in models:
        row = {"id": m.id}
        row.update(owner(m))
        row.update(limits(catalog, m))
        rows.append(row)
    return rows
```

The replacement is advice. xpd does not apply it automatically.

The comprehension builds a whole row in one expression, and every part of the row has to be a
helper that returns a dict. The loop reads from top to bottom, and a step in it can grow without
the row being rewritten. A comprehension that maps or filters one value per item is still the
right tool.

`{**d for d in dicts}`, which Python 3.15 added, is not reported.

#### unpacked-comprehension

**Severity:** warning. **Fix:** none.

Reports a dict literal that unpacks a dict comprehension with `**`.

Example:

```python
def row(model, limits):
    return {"id": model.id, **{f"limit_{k}": v for k, v in limits.items()}}
```

Use instead:

```python
def row(model, limits):
    out = {"id": model.id}
    for name, value in limits.items():
        out[f"limit_{name}"] = value
    return out
```

The replacement is advice. xpd does not apply it automatically.

The literal builds a dict, unpacks it and merges it in one expression, so a reader has to run
the comprehension in their head to know which keys the result has.

#### comprehension-two-fors

**Severity:** warning. **Fix:** none.

Reports a comprehension or generator expression with more than one `for`.

Example:

```python
def checks(cases):
    return [check for case in cases for check in case.checks]
```

Use instead:

```python
def checks(cases):
    found = []
    for case in cases:
        found.extend(case.checks)
    return found
```

The replacement is advice. xpd does not apply it automatically.

The clauses run in the order of a nested loop, and the result stands before both, so a reader
has to reorder the expression. Python 3.15 flattens with `[*case.checks for case in cases]`.

## Write a rule

Rules are [ast-grep rules](https://ast-grep.github.io/guide/rule-config.html) in `.yml` files,
compiled into the binary. A pack lives at `rules/<module>/`, where `<module>` is the name you
import.

```text
rules/pandas/
├── pack.yml        # the pack's manifest: import aliases and package checks. A pack may leave it out
├── row-loop.yml    # one file per rule, named after the rule id
└── ...
```

A rule file holds the documents of one rule, one per shape, and each document has the file's name
as its `id`. The build fails on any other id.

A rule may have one ast-grep `fix:`. It must then say whether the fix keeps what the code does,
with `metadata: { fix: safe }` or `metadata: { fix: unsafe }`. A fix may write an alias that
`pack.yml` lists under `aliases`, such as `pd.`, and xpd applies it only in a file that imports
the module under that alias.

A rule that needs a newer library version says so with `metadata: { since: "3.1" }`. xpd skips it
when the project's venv holds an older version of the pack's library. It finds the version for any
pack the same way: from the dist-info folder whose name is the module, or whose `top_level.txt`
lists the module.

Add a file, rebuild, and check it against a sample:

```sh
cargo build --release
target/release/xpd check samples/bad.py
cp samples/bad.py /tmp/bad.py && target/release/xpd check /tmp/bad.py --fix --unsafe-fixes
```

The files in `samples/pandas/`, `samples/numpy/`, `samples/scipy/`, `samples/sklearn/`, `samples/hamilton/` and
`samples/python/` mark each line a rule reports with a comment that names the rules.

`pack.yml` may list package checks under `packages`. Each entry has an `id`, the `package` to
look for, a `message`, and optionally `onlyWith`, a list of tools. xpd warns when the venv has the
pack's library and lacks the package. With `onlyWith`, it warns only when the project also uses
one of the tools.

A rule may limit the files it runs on with ast-grep's
[`files` and `ignores`](https://ast-grep.github.io/reference/yaml.html#files) globs. xpd matches
a glob against the path as it was given, so start it with `**/` to match at any depth, as in
`ignores: ["**/__main__.py"]`. A rule about the file itself, such as `vague-module-name`, matches
`kind: module` and is reported on line 1.

A new folder under `rules/` is a new pack, with no Rust change. Its rules run only on files that
import the module. A pack about Python itself, such as `rules/python/`, sets `everyFile: true` in
`pack.yml`, and its rules run on every file.

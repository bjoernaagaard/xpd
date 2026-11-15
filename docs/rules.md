# Rules

xpd has five rule packs: [pandas](#pandas), [NumPy](#numpy), [scikit-learn](#scikit-learn),
[Hamilton](#hamilton) and [Python](#python). A library's rules run only on files that import it. The
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

Reports `df["a"][i] = v`, `df[mask]["a"] = v`, `df.loc[mask]["a"] = v`, `df.a[mask] = v`, `df["a"].iloc[0] = v` and `df["a"].update(other)`, which write to a copy.

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

Reports `df["b"].fillna(0, inplace=True)`, which changes a copy.

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

**Severity:** warning. **Fix:** unsafe: `df = df.dropna()`.

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

Reports `df["d"].astype("int64") // 10**9`, `index.asi8 // 10**9` and `df["d"].values.astype("float64") / 1e9`, which are off by 1,000 now that datetimes default to microseconds.

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

Reports `2 in df["a"]`, which checks the index labels.

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

**Severity:** error. **Fix:** unsafe: `pd.api.types.is_string_dtype(df["k"])`. `is_object_dtype`, a test with `in`, `dtype.name` and `select_dtypes("object")` by position have no fix.

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

**Severity:** error. **Fix:** safe: `map`, `ffill()`, `"ME"`, `"h"`, `"min"`, `sep=r"\s+"`, `lineterminator=`, `sep=` for a separator passed by position, `is_monotonic_increasing`, `np.` for `pd.np.` and `pd.json_normalize`. unsafe: `"M"` in `resample`, `dt.isocalendar().week`, `writer.close()` and `pd.Series(x).value_counts()`. The other forms have no fix.

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

Reports `df = df.append(row)` and `df.append(row, ignore_index=True)`, which raise AttributeError.

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

Reports `df["k"].value_counts()[0]` and `df.iloc[0][0]`, which raise KeyError, because an integer key on a Series is a label on pandas 3.

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

**Severity:** warning. **Fix:** safe: delete the line, drop `copy=`, `pd.Timestamp.now("UTC")`. unsafe: drop `copy=` from `astype`.

Reports `pd.set_option("mode.copy_on_write", True)`, `copy=False` and `pd.Timestamp.utcnow()`.

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

These rules are warnings. Most of them make code shorter and do not change its speed.

#### sort-then-reset-index

**Severity:** warning. **Fix:** safe: `df.sort_values("b", ignore_index=True)`.

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

**Severity:** warning. **Fix:** safe: `df.resample("D", on="d").sum()`.

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

**Severity:** warning. **Fix:** none.

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

**Severity:** warning. **Fix:** none.

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

**Severity:** warning. **Fix:** safe: `df["s"].str.len()`. unsafe: `df["b"].isna()`, `pd.to_datetime(df["raw"])`, `df["b"].round(2)`.

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

**Severity:** warning. **Fix:** safe: `str.contains("ta_", regex=False)`.

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

**Severity:** warning. **Fix:** safe: drop `.copy()`.

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

**Severity:** warning. **Fix:** unsafe: `df["k"].map(table["v"])`. A lookup through a mask has no fix.

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

**Severity:** warning. **Fix:** safe: drop `.copy()`.

Reports `df.copy().rename(...)` and `df.rename(...).copy()`, also with `assign`, `reset_index`, `set_index`, `astype`, `fillna`, `drop(columns=)` and `pd.concat`.

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

Reports a loop over `iterrows()`, a loop over `itertuples()` or `to_dict("records")` that only computes from the row values, and `for i in range(len(df))` with `df.iloc[i]`.

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

Reports `df["a"].apply(lambda v: 1 if v > 50 else 0)` and `lambda v: min(v, 1.0)`.

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

**Severity:** warning. **Fix:** unsafe: `df["a"].sum()`, `df["k"].nunique()`, `(df["a"] > 50).sum()`, `set(df["k"].unique())`, `df["a"].tolist()`, `df["a"].to_numpy()`. `Counter` has no fix.

Reports `sum(df["a"])`, `len(set(df["k"]))`, `sum(df["a"] > 50)`, `set(df["k"])`, `Counter(df["k"])`, `list(df["a"])` and `np.array(df["a"].tolist())`.

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

Reports `[v.upper() for v in df["s"]]`, and a `for` loop over a column that appends to a list.

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

Reports `len(df[df["a"] > 50])`.

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

Reports `df.sort_values("b").head(10)`, `df.sort_values("b").iloc[0]`, `df["b"].sort_values().iloc[0]`, `df.sort_values("b").tail(10)` and `sorted(df["b"])[0]`.

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

Reports a frame that grows inside a loop, as in `out = pd.concat([out, part])`. Each pass copies every earlier row.

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

Reports `for label in df.index` with `df.loc[label, "a"]` or `df.at[label, "a"]` in the body, also as a comprehension.

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

Reports a loop or comprehension over `zip(df["a"], df["w"])`.

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

**Severity:** warning. **Fix:** safe: `"max"` for min, max, median, count and nunique. unsafe: `"sum"`, `"mean"`. A reduction with arguments, a count of a condition and `resample` have no fix.

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

**Severity:** warning. **Fix:** safe: `g["b"].shift(1)`. unsafe: `g["b"].cumsum()`.

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

Reports any `groupby(...).apply` with a lambda.

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

Reports `df.groupby("k").first()` and `last()`.

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

Reports `df.index.get_level_values("k").unique()` and `.nunique()`, which build one value per row first.

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

**Severity:** warning. **Fix:** safe: `rolling(20).max()` for min, max and median. unsafe: sum, mean, std and var. The `raw=True` case and a NumPy function have no fix.

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

Reports `df["d"].dt.date` and `dt.time`.

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

Reports `merge` without `validate=`, which can duplicate rows.

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

Reports `df.to_csv(path)` without `index=`.

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

## scikit-learn

These rules run on files that import `sklearn`. They report code that gives a leaked or wrong
score, code that raises or warns on scikit-learn 1.9.1, and code that does the same work twice.
Every number in this section was measured on scikit-learn 1.9.1. Examples assume that the
scikit-learn names they use are imported. Names such as `X_train`, `y_test` and `model` represent
values supplied by your program.

Several rules read the names of variables, such as `X_test` and `y_pred`, because a pattern sees
nothing else, so a name that misleads gives a wrong report. A rule document whose advice needs a
newer release of scikit-learn carries `since`. xpd reads the installed version from the project's
venv, as it does for pandas, and skips that document on an older release. When the venv has no
scikit-learn, or there is no venv, every document runs.

### How a rule earns its place

Every rule document ships on a saved proof that its old form is wrong, and a proof is one of
three kinds:

- A score proof runs the old and the new form over 30 seeds on data with no signal, where the
  honest score is 0.5. It passes when the old form's mean is more than three standard errors
  above the honest score and the new form's mean is within 0.02 of it.
- An interpreter proof runs the old form under `python -W error`. It passes when the old form
  raises an exception or a warning and the new form runs clean.
- A speed proof times both forms on one thread, as the median of 7 runs, at two data sizes. It
  passes when the new form is at least 1.5 times faster at both sizes.

A document is `info` when no proof of the three kinds passed for it. Either it changes a score in
a way the three proofs cannot show, such as a ranking metric given class labels, which scores
0.5 on data with no signal in both forms, or it changes nothing that can be measured, such as a
missing seed.

The rules were then judged on 1,414 held-back files from 268 public repositories, which were set
aside before any rule was written. Two readers who did not write the rules labelled 478 reports
of the rules as they stood then, each apart from the other, and marked every report whose advice
was wrong. Recall was measured by planting 2,749 old forms in copies of the held-back files and
counting the plants that a rule reported. Every fix was run before and after, on its held-back
sites and on cases built for each form, and the fitted model and its predictions were compared
bit for bit.

| Rule | Severity | Proof | Held-back reports | Wrong advice | Recall | Fix |
| --- | --- | --- | --- | --- | --- | --- |
| [`fit-on-test`](#fit-on-test) | error, info | score: 0.997 against 0.525 AUC | 20 | 0 and 0 of 20 | 103 of 130 | unsafe |
| [`fit-before-split`](#fit-before-split) | error | score: 0.771 against 0.497 AUC | 24 | 0 and 0 of 24 | 91 of 162 | none |
| [`cross-val-on-fitted-data`](#cross-val-on-fitted-data) | error | score: 0.764 against 0.518 AUC | 10 | 0 and 0 of 10 | 72 of 78 | none |
| [`tune-on-test`](#tune-on-test) | error | score: 0.597 against 0.507 accuracy | 1 | 0 and 0 of 1 | 15 of 50 | none |
| [`stateful-function-transformer`](#stateful-function-transformer) | info | none of the three | 0 | no reports | 24 of 30 | none |
| [`groups-ignored`](#groups-ignored) | error | score: 0.849 against 0.480 accuracy | 0 | no reports | 56 of 57 | none |
| [`missing-groups`](#missing-groups) | error | interpreter: ValueError | 0 | no reports | 48 of 50 | none |
| [`leave-one-out-score`](#leave-one-out-score) | error | interpreter: UndefinedMetricWarning | 0 | no reports | 14 of 33 | none |
| [`auc-from-labels`](#auc-from-labels) | error, info | score: log loss 18.00 against 0.70 | 20 | 2 and 2 of 20 | 55 of 72 | unsafe |
| [`metric-arguments-swapped`](#metric-arguments-swapped) | error, info | score: precision 0.471 against 0.294 | 43 | 0 and 0 of 30 | 66 of 100 | unsafe |
| [`score-on-training-rows`](#score-on-training-rows) | error | score: 1.000 against 0.510 accuracy | 0 | no reports | 10 of 27 | none |
| [`unknown-scoring-name`](#unknown-scoring-name) | error | interpreter: InvalidParameterError, ValueError | 4 | 0 and 0 of 4 | 16 of 32 | none |
| [`removed-in-sklearn`](#removed-in-sklearn) | error | interpreter: ModuleNotFoundError, ImportError, TypeError, InvalidParameterError | 321 | 1 and 0 of 58 | 517 of 600 | unsafe |
| [`deprecated-in-sklearn`](#deprecated-in-sklearn) | warning | interpreter: FutureWarning | 101 | 3 and 0 of 57 | 274 of 308 | safe, unsafe |
| [`removed-metric-api`](#removed-metric-api) | error | interpreter: ImportError, TypeError | 13 | 0 and 0 of 13 | 74 of 74 | none |
| [`deprecated-metric-api`](#deprecated-metric-api) | warning | interpreter: FutureWarning | 0 | no reports | 48 of 48 | safe |
| [`fit-params-keyword`](#fit-params-keyword) | error | interpreter: TypeError | 4 | 0 and 0 of 4 | 32 of 32 | none |
| [`column-transformer-removed`](#column-transformer-removed) | error | interpreter: TypeError | 3 | 0 and 0 of 3 | 30 of 30 | none |
| [`metadata-routing`](#metadata-routing) | error | interpreter: RuntimeError, ValueError, TypeError, UnsetMetadataPassedError | 0 | no reports | 34 of 34 | none |
| [`pipeline-fit-params`](#pipeline-fit-params) | error | interpreter: ValueError | 0 | no reports | 23 of 30 | none |
| [`pipeline-step-form`](#pipeline-step-form) | error | interpreter: ValueError, TypeError | 0 | no reports | 29 of 35 | none |
| [`transformed-target-pair`](#transformed-target-pair) | error | interpreter: ValueError | 0 | no reports | 31 of 36 | none |
| [`pandas-output-sparse`](#pandas-output-sparse) | error | interpreter: ValueError | 0 | no reports | 29 of 31 | none |
| [`predict-per-row`](#predict-per-row) | warning | speed: 2.77 times | 2 | 0 and 2 of 2 | 28 of 32 | none |
| [`score-after-predict`](#score-after-predict) | warning | speed: 1.62 times | 17 | 0 and 0 of 17 | 31 of 32 | unsafe |
| [`predict-and-predict-proba`](#predict-and-predict-proba) | warning | speed: 1.83 times | 22 | 0 and 0 of 22 | 30 of 30 | none |
| [`cross-val-score-per-metric`](#cross-val-score-per-metric) | warning | speed: 1.60 times | 14 | 0 and 0 of 14 | 30 of 30 | none |
| [`vectorizer-fit-then-transform`](#vectorizer-fit-then-transform) | warning | speed: 1.63 times | 12 | 0 and 4 of 12 | 19 of 25 | unsafe |
| [`refit-per-n-estimators`](#refit-per-n-estimators) | warning | speed: 1.64 times | 0 | no reports | 30 of 30 | none |
| [`tsne-exact`](#tsne-exact) | info | none of the three | 0 | no reports | 21 of 30 | none |
| [`random-state-without-shuffle`](#random-state-without-shuffle) | error | interpreter: ValueError | 9 | 0 and 0 of 9 | 54 of 54 | none |
| [`missing-random-state`](#missing-random-state) | info | none of the three | 527 | 1 and 1 of 52 | 79 of 80 | none |

A score proof gives the old form's mean against the new form's mean. A speed proof gives the
lowest ratio over the classes the rule reports and both sizes. Wrong advice gives the reports
that reader A and reader B called wrong, out of the reports they labelled. A rule with more
reports than labels was sampled.

The rules were narrowed after the readers' labels came in, so the wrong advice of a narrowed rule
counts how many of its labelled reports are still made and still wrong, and it is not a clean
measurement. Recall was measured on the rules before that narrowing. Most plants were inserted
into a file in shapes the judges chose, which sit closer to the documented examples than real
code does, so recall on real code is likely lower. For the metric and pipeline rules, recall was
190 of 280 on plants turned from a right form already in the file, against 311 of 357 on inserted
plants. A rule with no report on the held-back files has no wrong-advice number.

### Leakage

Each old form below lets the test rows, or every row, shape what the model learns, so a score on
the test rows is too high. The score proofs in this group use data whose target is drawn apart
from every feature, where an honest AUC is 0.5, and give the mean over 30 seeds. A step that does
not read the target, such as a scaler, an imputer, PCA, a binner, a one-hot encoder or a text
vectorizer, moved the test AUC by at most 0.006 when it was fitted before the split, so these
rules leave it alone.

#### fit-on-test

**Severity:** error for `fit` with a target and for `partial_fit`, and info for `fit_transform` and for `fit` without a target. **Fix:** unsafe: `scaler.transform(X_test)`. The other forms have no fix.

Reports `model.fit(X_test, y_test)` and `model.partial_fit(X_test, y_test)`, which train on the test rows, and `scaler.fit_transform(X_test)` and `scaler.fit(X_test)`, which refit a step on them.

Example:

```python
model = LogisticRegression().fit(X_test, y_test)
X_test = scaler.fit_transform(X_test)
```

Use instead:

```python
model = LogisticRegression().fit(X_train, y_train)
X_test = scaler.transform(X_test)
```

#### fit-before-split

**Severity:** error. **Fix:** none.

Reports a step that learns from the target and is fitted on every row before `train_test_split`, such as `selector.fit_transform(X, y)`, `encoder.fit(X, y).transform(X)`, `fit_resample`, `resample(..., replace=True)`, a target mean per group from `groupby(...)[target].transform("mean")`, and columns picked by `df.corr()[target]`.

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

Reports a step that learns from the target, or `fit_resample`, on every row before `cross_val_score`, another cross-validation function, or a search fitted on the result.

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

#### tune-on-test

**Severity:** error. **Fix:** none.

Reports a loop that builds one model per setting, scores each model on the test rows and keeps the best score.

Example:

```python
for k in range(1, 30):
    model = KNeighborsClassifier(n_neighbors=k).fit(X_train, y_train)
    scores.append(model.score(X_test, y_test))
best = max(scores)
```

Use instead:

```python
search = GridSearchCV(KNeighborsClassifier(), {"n_neighbors": range(1, 30)}, cv=5)
search.fit(X_train, y_train)
test_score = search.score(X_test, y_test)
```

The replacement is advice. xpd does not apply it automatically.

#### stateful-function-transformer

**Severity:** info. **Fix:** none.

Reports `FunctionTransformer(lambda X: (X - X.mean(axis=0)) / X.std(axis=0))`, a function that takes statistics of the rows it gets, so at predict time it scales by the test rows.

Example:

```python
pipe = make_pipeline(FunctionTransformer(lambda X: (X - X.mean(axis=0)) / X.std(axis=0)), model)
```

Use instead:

```python
pipe = make_pipeline(StandardScaler(), model)
```

The replacement is advice. xpd does not apply it automatically.


`fit-on-test` reads the name of the first argument. For `fit` the name must hold `test`. For
`partial_fit` and `fit_transform` it may also hold `holdout`, or be a validation name such as
`X_val`. A `LogisticRegression` fitted on 60 test rows and scored
on them gave 0.997 AUC, where the fit on the training rows gave 0.525. Five `partial_fit` calls on
the test rows gave 0.924 against 0.518. A refitted scaler cannot move a score on data with no
signal, so the forms without a target are info. On data with signal, a `StandardScaler` refitted
on batches of 5 test rows lost 0.048 AUC, and an `OrdinalEncoder` refitted on 30 test rows lost
0.058. With the target, a category_encoders `TargetEncoder` refitted on the test rows scored 0.992
on noise, but `SelectKBest` scored 0.438 and the scikit-learn `TargetEncoder` 0.499, so that form
is info as well. The `fit_transform` forms read the step's name too, such as `scaler`, `imputer`,
`encoder`, `ohe` or `tfidf`, because `PolynomialFeatures` and plots refit on purpose. The rule
leaves alone `PermutationImportance` and `permutation_importance`, a fit on train and test rows
joined, a `LabelBinarizer` or `MultiLabelBinarizer`, and a `LabelEncoder` whose result goes
straight into another call, such as a plot. The fix scales or codes the test rows with the
statistics of the training rows, so it changes the test matrix by up to 0.43 and the predicted
probabilities by up to 0.18, and a test level the step has not seen now raises. At 4 of 17
held-back sites the fixed code raised, because the scaler was built again on the line before, or
one `LabelEncoder` was last fitted on another column. The frozen rule gave wrong advice in 3 and
3 of 23 labelled reports, and the narrowed rule no longer makes those reports. Recall was 41 of
45 for the error forms and 62 of 85 for the info forms. The rule misses a step whose name it does
not read, such as `rs = RobustScaler()` or `imp`, a test name with no separator such as `Xtest`,
and held-out rows named `X_dev`, `X_te` or `X_cv`.

`fit-before-split` reports the fit when a later statement of the same block passes its result to
`train_test_split`. Before the split, `SelectKBest` scored 0.771 against 0.497, `SelectFromModel`
0.684 against 0.519, `RFE` 0.750 against 0.495, `RFECV` 0.739 against 0.506,
`SequentialFeatureSelector` 0.551 against 0.477, and a category_encoders `TargetEncoder` 0.880
against 0.495. `SMOTE` scored 0.815, `RandomOverSampler` 0.737, `ADASYN` 0.819,
`EditedNearestNeighbours` 0.678 and `resample(replace=True)` 0.970, against 0.478 to 0.493 after
the split. `TargetEncoder().fit(X, y).transform(X)` scored 0.889 against 0.509, a target mean per
group 0.887 against 0.497, and 10 columns picked by `df.corr()` 0.771 against 0.497. `NearMiss`
scored 0.448 and `TomekLinks` 0.516, which did not pass the proof, and the rule reports them
because they pick rows by their neighbours. The rule leaves `RandomUnderSampler` alone, which drops
rows at random and scored 0.512 against 0.491. It leaves the scikit-learn
`TargetEncoder.fit_transform` alone too, which cross-fits and scored 0.522 against 0.504. A label
encoder on the target is left alone, when the result is named `y`, `target` or `labels`, or the
encoder's name holds `label`. The target mean and the `df.corr()` forms need the same column to
become the target later in the block, as in `y = df["target"]`. The frozen rule gave wrong advice
in 2 and 0 of 26 labelled reports. Recall was 91 of 162. The rule found none of 24 columns picked
by correlation in other shapes, such as `corr.index[1:11]` read back as `df[top_features]`. It
also misses a target mean written with `.map` or `.mean()`, `resample` imported under another
name, and `SelectFromModel` fitted in one statement and applied in the next.

`cross-val-on-fitted-data` covers `cross_val_score`, `cross_validate`, `cross_val_predict`,
`learning_curve`, `validation_curve` and `permutation_test_score` when the fitted result is the
second argument, and a `GridSearchCV`, `RandomizedSearchCV`, `HalvingGridSearchCV` or
`HalvingRandomSearchCV` that is assigned in the same file and fitted on the result.
Cross-validated AUC after `SelectKBest` on every row was 0.764 against 0.518 for a Pipeline.
`SMOTE` before `cross_val_score` gave 0.838 against 0.510. `best_score_` after `SelectKBest` was
0.770 against 0.522, and after `SMOTE` 0.854 against 0.521. `RandomUnderSampler` before the folds
scored 0.519 against 0.511, so the rule leaves it alone. The frozen rule gave wrong advice in 1
and 0 of 11 labelled reports.

`tune-on-test` needs the loop variable as a keyword of the call that built the scored model, a
`score`, `predict` or `predict_proba` on a name with `test` in it, and a pick. A pick is a `>`
comparison that reads or sets a name with `best` or `max` in it, or whose branch reads the
setting, or a later `max`, `argmax` or `idxmax` of the list the loop appends to. A keyword named
like `random_state`, `seed`, `method` or `model` is left alone, because such a loop compares or
averages. The best of 30 test accuracies for `n_neighbors` from 1 to 30 was 0.597, where a
`GridSearchCV` on the training rows gave 0.507. The frozen rule gave wrong advice in 2 and 2 of 3
labelled reports, on loops that printed whether the training score was far above the test score,
and the narrowed rule no longer makes those reports. Recall was 15 of 50. The rule found none of
20 thresholds picked by the test score. It also misses scores kept in a dict and picked with
`max(results, key=results.get)`, a loop over `ParameterGrid` that builds the model with
`**params`, and a list comprehension picked with `np.argmax`.

`stateful-function-transformer` is info, because on data with no signal it does not move a
score. Predicted in batches of 5 rows it scored 0.484 against 0.494 for `StandardScaler`. On data
with signal, `LogisticRegression` scored 0.773 against 0.847 with batches of 5 test rows, and
0.824 against 0.847 with batches of 20, over 30 seeds. A reduction along axis 1 works per row
and is left alone. Recall was 24 of 30. The rule found none of 6 functions defined with `def` and
passed by name.

### Splits and groups

Each old form below splits the rows in a way that leaks or gives no score.

#### groups-ignored

**Severity:** error. **Fix:** none.

Reports `groups=` given to a cross-validation function or a search whose `cv` ignores it, and the groups given to `split` on a splitter that ignores them, such as `KFold(5).split(X, y, g)`.

Example:

```python
scores = cross_val_score(model, X, y, groups=patient_id, cv=5)
```

Use instead:

```python
scores = cross_val_score(model, X, y, groups=patient_id, cv=GroupKFold(n_splits=5))
```

The replacement is advice. xpd does not apply it automatically.

#### missing-groups

**Severity:** error. **Fix:** none.

Reports a group splitter that gets no groups, such as `cross_val_score(model, X, y, cv=GroupKFold(5))` or `GroupKFold(5).split(X, y)`, which raises ValueError.

Example:

```python
search = GridSearchCV(model, grid, cv=GroupKFold(n_splits=5))
search.fit(X, y)
```

Use instead:

```python
search = GridSearchCV(model, grid, cv=GroupKFold(n_splits=5))
search.fit(X, y, groups=patient_id)
```

The replacement is advice. xpd does not apply it automatically.

#### leave-one-out-score

**Severity:** error. **Fix:** none.

Reports `cv=LeaveOneOut()` with a score that needs more than one row, which is R2 or ROC AUC given as `scoring`, or the R2 that a regressor scores by default.

Example:

```python
scores = cross_val_score(Ridge(), X, y, cv=LeaveOneOut())
```

Use instead:

```python
r2 = r2_score(y, cross_val_predict(Ridge(), X, y, cv=LeaveOneOut()))
```

The replacement is advice. xpd does not apply it automatically.


`groups-ignored` covers `cross_val_score`, `cross_validate`, `cross_val_predict`,
`learning_curve`, `validation_curve` and `permutation_test_score` with `groups=` and a `cv` that
is missing, a whole number, or one of `KFold`, `StratifiedKFold`, `ShuffleSplit`,
`StratifiedShuffleSplit`, `RepeatedKFold`, `RepeatedStratifiedKFold`, `TimeSeriesSplit`,
`LeaveOneOut`, `LeavePOut` and `PredefinedSplit`, given in the call or assigned to a name earlier.
It covers `GridSearchCV`, `RandomizedSearchCV`, `HalvingGridSearchCV` and `HalvingRandomSearchCV`
built in the same block as a `fit` that gets `groups=`, and such a splitter's
`split(X, y, groups)`, called directly or through a name. scikit-learn warns "The groups parameter
is ignored by KFold" and splits the rows as if no groups were given. On 60 groups of 8 rows, with
one label per group drawn apart from the features and the rows in random order, a
`RandomForestClassifier` scored 0.849 accuracy with `cv=5` and `groups=`, where `GroupKFold(5)`
scored 0.480. When the rows of each group sit next to each other, folds that are not shuffled keep
most groups whole and the score was 0.497, so the leak depends on the row order. On 40 groups of 5
near copies, a 1-nearest-neighbour model scored 0.998 with `cv=5` and 0.487 with `GroupKFold(5)`.
A `cv` held in a name that xpd cannot trace to one of these splitters is left alone. Recall was
56 of 57.

`missing-groups` covers `GroupKFold`, `StratifiedGroupKFold`, `GroupShuffleSplit`,
`LeaveOneGroupOut` and `LeavePGroupsOut` given as `cv` to the six cross-validation functions
above, their own `split(X, y)` called directly or through a name, and `GridSearchCV`,
`RandomizedSearchCV`, `HalvingGridSearchCV`, `HalvingRandomSearchCV` and `RFECV` whose `fit` gets
no `groups=`. Each raises "The 'groups' parameter should not be None" on scikit-learn 1.9.1, and
`StratifiedGroupKFold` raises that it found 1 group. A call with `params=` or `**` is left alone,
because it can carry the groups. `SequentialFeatureSelector` and `LogisticRegressionCV` take
groups only through metadata routing, so they are not listed. Recall was 48 of 50.

`leave-one-out-score` covers `cross_val_score`, `cross_validate`, `GridSearchCV`,
`RandomizedSearchCV`, `validation_curve`, `learning_curve`, `permutation_test_score` and `RFECV`.
With one row per fold, R2 warns "R^2 score is not well-defined with less than two samples" and
ROC AUC warns "Only one class is present in y_true", every fold scores nan, and a search over nan
scores picks its first candidate. Under `python -W error` each warning is an
`UndefinedMetricWarning` that raises. The rule knows a regressor only by its class name in the
same call. `explained_variance` and `average_precision` score 1 or 0 on a fold of one row and give
no nan, so they are left alone. Recall was 14 of 33, and the plants included those two scorings,
which the rule left out after the recall run. The rule misses a splitter kept in a name, as in
`loo = LeaveOneOut()` and `cv=loo`, `LeaveOneOut` imported under another name, and a regressor
passed by a variable with no `scoring`. `cv=len(X)`, which is leave one out under another name,
is left alone by design.

### Metrics and scoring

Each old form below passes a metric the wrong thing, scores the wrong rows, or names a scorer that
does not exist.

#### auc-from-labels

**Severity:** error for `log_loss` and `brier_score_loss` given labels, for the whole `predict_proba` matrix where one score per row is needed, for `decision_function` given to a loss, and for `roc_auc_score` with `multi_class` given labels, and info for ranking metrics given labels, column 0 of `predict_proba`, and `make_scorer(roc_auc_score)` without `response_method`. **Fix:** unsafe: `log_loss(y_test, model.predict_proba(X_test))`. The other forms have no fix.

Reports `log_loss(y_test, model.predict(X_test))`, `roc_auc_score(y_test, model.predict_proba(X_test))` and `roc_auc_score(y_test, model.predict(X_test))`, where a metric that needs one probability or score per row gets class labels or a whole matrix.

Example:

```python
loss = log_loss(y_test, model.predict(X_test))
auc = roc_auc_score(y_test, model.predict_proba(X_test))
```

Use instead:

```python
loss = log_loss(y_test, model.predict_proba(X_test))
auc = roc_auc_score(y_test, model.predict_proba(X_test)[:, 1])
```

#### metric-arguments-swapped

**Severity:** error for `precision_score`, `classification_report` and `precision_recall_fscore_support`, and for scores passed first to a ranking metric or a loss, and info for the other metrics that change with the order. **Fix:** unsafe: `precision_score(y_test, y_pred)`. The scores-first form has no fix.

Reports `precision_score(y_pred, y_test)` and `roc_auc_score(y_score, y_test)`, where the predictions come first.

Example:

```python
precision = precision_score(y_pred, y_test)
```

Use instead:

```python
precision = precision_score(y_test, y_pred)
```

#### score-on-training-rows

**Severity:** error. **Fix:** none.

Reports `model.score(X_train, y_train)` and `accuracy_score(y_train, model.predict(X_train))` when nothing around them scores held-out rows.

Example:

```python
model.fit(X_train, y_train)
print(model.score(X_train, y_train))
```

Use instead:

```python
model.fit(X_train, y_train)
print(model.score(X_test, y_test))
```

The replacement is advice. xpd does not apply it automatically.

#### unknown-scoring-name

**Severity:** error. **Fix:** none.

Reports `scoring="mean_squared_error"` and any other string that `get_scorer` refuses, in a search, `cross_val_score`, `cross_validate`, the other scikit-learn functions that take `scoring`, and `get_scorer` itself.

Example:

```python
scores = cross_val_score(model, X, y, scoring="mean_squared_error")
```

Use instead:

```python
scores = cross_val_score(model, X, y, scoring="neg_mean_squared_error")
```

The replacement is advice. xpd does not apply it automatically.


`auc-from-labels` was measured with `LogisticRegression` over 30 seeds. On data with no signal,
`log_loss` given the labels came out at 17.998, where the probabilities gave 0.699 and the honest
loss is ln 2, 0.693. `brier_score_loss` given the labels came out at 0.499 against 0.253, where
the honest loss is 0.25. Passing the whole `predict_proba` matrix to `roc_auc_score` without
`multi_class`, or to `roc_curve`, `precision_recall_curve` or `det_curve`, raises ValueError. So
does `roc_auc_score(multi_class="ovr")` given labels on three classes, and `decision_function`
given to `log_loss` or `brier_score_loss`. Ranking metrics given labels are info, because on
noise the labels scored 0.495 and the probabilities 0.498. On data with signal the labels gave an
AUC of 0.735 against 0.822 and an average precision of 0.677 against 0.827, and column 0 of
`predict_proba` gave an AUC of 0.178. `make_scorer(roc_auc_score)` without `response_method`
scored 0.739 in `cross_val_score` against 0.824. The variable form reports
`y_pred = model.predict(X_test)` when a later line passes `y_pred` to the metric. The rule leaves
alone a `predict` on a Booster that `xgb.train` or `lgb.train` returned, a `predict` whose
argument is named like predictions or probabilities, as on a calibrator, and files that import
keras, tensorflow or torch, because `predict` returns probabilities there. The fix changes the
loss on purpose, from 12.0 to 0.63 on the case built for it. On three classes the old form raised
and the fixed form runs, and on an `SVC` without probabilities the fixed form raises
AttributeError. The readers called 2 and 2 of the 20 held-back reports wrong advice. Recall was
55 of 72. The rule found none of 13 plants that keep column 0 or the whole `predict_proba` matrix
in a variable before the metric. It also misses a `predict` variable whose argument is wrapped, as
in `model.predict(np.array(X_test.values))`.

`metric-arguments-swapped` reports when the first name holds `pred` or `hat` and the second holds
`true`, `test`, `val`, `actual`, `target` or `label`, or is `y`. With the arguments swapped, the
precision a metric reports is the recall. On noise where 30 percent of the rows are positive, a
`LogisticRegression(class_weight="balanced")` gave a swapped precision of 0.471, where the honest
precision was 0.294, and `classification_report` gave the same numbers for class 1. The other
documents are info, because their swapped numbers on noise are not above the honest value. The
swapped recall was 0.294 against 0.471, the balanced accuracy 0.496 against 0.495, and R2 -186.06
against -0.022. On data
with signal a swapped R2 was -0.069 against 0.486. `accuracy_score`, `f1_score`, `jaccard_score`,
`matthews_corrcoef`, `cohen_kappa_score` and the error metrics such as `mean_squared_error` return
the same number in either order, so they are left alone. Scores passed first to `roc_auc_score`,
`average_precision_score`, `roc_curve`, `precision_recall_curve`, `log_loss` or
`brier_score_loss` raise ValueError, because the true values then look continuous. The fix
changes the number on purpose. Precision becomes the old recall, and a confusion matrix is
transposed. Of 43 held-back reports, 30 were labelled, and neither reader called one wrong.
Recall was 66 of 100. The rule found none of 9 swapped keyword values such as
`confusion_matrix(y_true=y_pred, y_pred=y_true)`, and 2 of 7 `log_loss` calls with the scores
first. It also misses a column slice first, such as `roc_auc_score(y_prob[:, 1], y_test)`, and
true values named with `train` or `Y_`.

`score-on-training-rows` reads the names of the rows. On noise, a `DecisionTreeClassifier` with
default settings scored 1.000 accuracy on its training rows, where the test rows gave 0.510, and a
1-nearest-neighbour model scored 1.000 against 0.495. A `LogisticRegression` scored 0.545 on its
training rows against 0.501. The rule stays silent when the function around the score, the
functions next to it, or the file for code outside functions, scores a name with `test`, `val` or
`holdout` in it, or calls cross-validation or a search. Recall was 10 of 27. The rule found none of
9 metrics given a `predict` variable of the training rows, as in `y_pred = knn.predict(X_train)`
and `accuracy_score(y_train, y_pred)`. It also misses rows whose names do not say train, as in
`model.score(X, y)` after `model.fit(X, y)`.

`unknown-scoring-name` checks each string against `sklearn.metrics.get_scorer_names()` on
scikit-learn 1.9.1. `scoring="mean_squared_error"`, `scoring="log_loss"` and `scoring="auc"`
raise InvalidParameterError, and `"roc_auc_score"` in a list given to `cross_validate` raises
ValueError. A string with `{` or `%` in it, or an f-string, is left alone, because code fills it
in later. Recall was 16 of 32. The rule found none of 16 scoring strings or dicts of scorer names
kept in a variable.

### Removed and deprecated forms

Each form below raises or warns on scikit-learn 1.9.1. Each was run under `python -W error`, where
the old form raised and the new form ran clean. The messages take each replacement from the 1.9.1
source, such as the docstring that says `sparse` was renamed `sparse_output` in 1.2, or from the
error text. Every fix of a removed form turns code that raises into code that runs, so none of
those fixes can be bit equal and none is safe. A fix that drops a keyword deletes the comments
inside the argument list and joins a call written over several lines onto one line.

#### removed-in-sklearn

**Severity:** error. **Fix:** unsafe: `sklearn.model_selection` for `sklearn.cross_validation`, `sklearn.grid_search` and `sklearn.learning_curve`, `import joblib`, the public module for a private module path such as `sklearn.linear_model.logistic`, `get_feature_names_out()`, renamed keywords such as `sparse_output=`, `estimator=`, `max_iter=`, `store_cv_results=`, `ensure_all_finite=`, `metric=`, `alphas=` and `n_splits=`, dropped keywords such as `n_jobs`, `behaviour`, `algorithm`, `iid`, `multi_class`, `min_impurity_split=None` and `normalize=False`, and new values for refused ones, such as `max_features="sqrt"` or `1.0` for `"auto"`, `loss="log_loss"`, `criterion="squared_error"` and `"absolute_error"`, and `loss="squared_error"`. The other forms have no fix.

Reports imports of removed modules and names, such as `sklearn.cross_validation`, `sklearn.externals.joblib` and `six`, `load_boston`, `preprocessing.Imputer`, private module paths and `sklearn.feature_extraction.stop_words`, and reports `get_feature_names()`, keywords the signature no longer has, values that parameter validation refuses, positional arguments past the keyword-only marker, and splitters given the data the way `sklearn.cross_validation` took it.

Example:

```python
from sklearn.cross_validation import train_test_split

encoder = OneHotEncoder(sparse=False)
names = vectorizer.get_feature_names()
forest = RandomForestClassifier(max_features="auto", random_state=0)
folds = StratifiedKFold(y, n_folds=5)
```

Use instead:

```python
from sklearn.model_selection import train_test_split

encoder = OneHotEncoder(sparse_output=False)
names = vectorizer.get_feature_names_out()
forest = RandomForestClassifier(max_features="sqrt", random_state=0)
folds = StratifiedKFold(n_splits=5).split(X, y)
```

#### deprecated-in-sklearn

**Severity:** warning. **Fix:** safe: drop `probability=False` from `SVC` and `NuSVC`, drop `penalty="elasticnet"` beside an `l1_ratio`, drop `n_jobs=` from `LogisticRegression`, `criterion="squared_error"` for `"friedman_mse"` on regression trees and forests, drop `criterion=` from GradientBoosting, `metric=` for MDS `dissimilarity=`, `metric_mds=` for MDS `metric=True` or `False`, `alphas=` for `n_alphas=`, and `alphas=100` for `alphas=None`. Unsafe: drop `penalty="l2"`, `l1_ratio=1` for `penalty="l1"`, and `l1_ratio=0` for `l1_ratio=None` on `LogisticRegression`. The other forms have no fix.

Reports `SVC(probability=)` and `NuSVC(probability=)`, `probA_` and `probB_`, `penalty=`, `l1_ratio=None` and `n_jobs=` on `LogisticRegression`, `penalty=` and the three defaults that warn on `LogisticRegressionCV`, `criterion="friedman_mse"`, GradientBoosting `criterion=`, the PassiveAggressive classes, `TargetEncoder(shuffle=, random_state=)`, MDS `dissimilarity=`, `metric=True` and a missing `init`, `HDBSCAN` without `copy`, `n_alphas=` and `alphas=None` on the path functions, a negative `power_t`, and `stable_cumsum`.

Example:

```python
model = LogisticRegression(penalty="l1", solver="liblinear", n_jobs=-1)
svm = SVC(probability=True, random_state=0).fit(X_train, y_train)
tree = DecisionTreeRegressor(criterion="friedman_mse", random_state=0)
```

Use instead:

```python
model = LogisticRegression(l1_ratio=1, solver="liblinear")
svm = CalibratedClassifierCV(SVC(), ensemble=False).fit(X_train, y_train)
tree = DecisionTreeRegressor(criterion="squared_error", random_state=0)
```

#### removed-metric-api

**Severity:** error. **Fix:** none.

Reports metric APIs that scikit-learn removed: `SCORERS`, `plot_confusion_matrix` and the other `plot_*` functions, `jaccard_similarity_score`, `squared=` on `mean_squared_error` and `mean_squared_log_error`, `eps=` on `log_loss`, `squared=`, `needs_proba=` and `needs_threshold=` passed to `make_scorer`, and the `RocCurveDisplay` keywords that 1.7 replaced.

Example:

```python
rmse = mean_squared_error(y_test, y_pred, squared=False)
scorer = make_scorer(roc_auc_score, needs_proba=True)
```

Use instead:

```python
rmse = root_mean_squared_error(y_test, y_pred)
scorer = make_scorer(roc_auc_score, response_method="predict_proba")
```

The replacement is advice. xpd does not apply it automatically.

#### deprecated-metric-api

**Severity:** warning. **Fix:** safe: `log_loss(y_test, y_proba=proba)` and `PrecisionRecallDisplay(precision=precision, recall=recall, name="model")`. The other forms have no fix.

Reports metric APIs that warn on scikit-learn 1.9.1: `y_pred=` on `log_loss` and `d2_log_loss_score`, `estimator_name=` on `PrecisionRecallDisplay`, `y_pred=` on `PrecisionRecallDisplay.from_predictions` and `DetCurveDisplay.from_predictions`, matplotlib keywords passed straight to `PrecisionRecallDisplay`, and `sklearn.metrics.cluster.entropy`.

Example:

```python
loss = log_loss(y_test, y_pred=proba)
```

Use instead:

```python
loss = log_loss(y_test, y_proba=proba)
```

#### fit-params-keyword

**Severity:** error. **Fix:** none.

Reports `fit_params=` on `cross_val_score`, `cross_validate`, `cross_val_predict`, `learning_curve`, `validation_curve`, `permutation_test_score` and the four search classes.

Example:

```python
scores = cross_val_score(model, X, y, fit_params={"sample_weight": w})
```

Use instead:

```python
scores = cross_val_score(model, X, y, params={"sample_weight": w})
```

The replacement is advice. xpd does not apply it automatically.

#### column-transformer-removed

**Severity:** error. **Fix:** none.

Reports `force_int_remainder_cols=` on `ColumnTransformer` and `make_column_transformer`.

Example:

```python
ColumnTransformer(transformers, remainder="passthrough", force_int_remainder_cols=False)
```

Use instead:

```python
ColumnTransformer(transformers, remainder="passthrough")
```

The replacement is advice. xpd does not apply it automatically.


`removed-in-sklearn` ran 49 forms on scikit-learn 1.9.1. Each old form raised
ModuleNotFoundError, ImportError, AttributeError, TypeError for an unexpected keyword or too many
positional arguments, or InvalidParameterError, and its new form ran clean. A grid with
`max_features: ["auto", "sqrt"]` raised through the FitFailedWarning that `GridSearchCV` gives.
`SCORERS`, the `plot_*` functions, `jaccard_similarity_score`, `squared=` and `fit_params=` are
left to `removed-metric-api` and `fit-params-keyword`. A backslash line break inside a call is
not counted as a positional argument. Every fixed form ran on the case built for it, and each
renamed keyword and refused value gave the same model as a reference written from the 1.9.1
docstrings. Some fixes change what the code means. `get_feature_names_out()` returns an array
where `get_feature_names()` returned a list, so `print` shows `array([...])`, and at 1 held-back
site the fix broke a project class that defined its own `get_feature_names`. Dropping
`multi_class` makes every multiclass fit use the multinomial loss, and with liblinear on three or
more classes the fixed call raises ValueError. Dropping `algorithm="SAMME.R"` turns a SAMME.R
model into a SAMME model. At 2 held-back sites the fixed `OneHotEncoder(sparse_output=False,
dtype=np.int)` still raised, because `np.int` is gone from NumPy. Of 321 held-back reports, 58
were labelled, and reader A called 1 wrong. Recall was 517 of 600. Of the misses, 36 were
`SCORERS`, the `plot_*` functions and `jaccard_similarity_score`, which `removed-metric-api`
reports. The rule found none of 18 refused values in a parameter grid for keywords other than
`max_features`, and none of 16 refused values held in a variable. It also misses
`sklearn.feature_extraction.stop_words` imported as a module.

`deprecated-in-sklearn` ran 30 forms on scikit-learn 1.9.1, and each old form raised its
FutureWarning under `-W error` while its new form ran clean. The warning gives the release that
removes each form, 1.10 or 1.11. `LogisticRegression(n_jobs=None)` gives no warning and is left
alone. Every safe fix gave a bit equal fitted model and bit equal predictions in every run. The
runs were 16 for dropping `probability=False`, 16 for dropping `penalty="elasticnet"`, 38 for
dropping `n_jobs`, 16 for `friedman_mse`, 6 for the GradientBoosting `criterion`, 4 and 6 for the
two MDS renames, 8 for `n_alphas`, and 6 for `alphas=None`. The stored constructor parameter
changes in a fix that drops a keyword, which is not counted as a change of the model. The three
unsafe fixes were bit equal in 48 runs, 32 runs and 8 runs, and they differ only with `C=np.inf`,
which 1.9.1 turns into no penalty only when `penalty` is left at its default. With liblinear and
`C=np.inf`, the fixed `l1_ratio=1` raises where the old form ran, and the fixed form of
`penalty="l2"` raises ValueError where the old form did not finish in 60 s. With saga and
`C=np.inf`, the fixed `l1_ratio=0` runs where the old form raised TypeError. Of 101 held-back
reports, 57 were
labelled, and reader A called 3 wrong. Recall was 274 of 308. Of the misses, 12 were
`sklearn.metrics.cluster.entropy`, which `deprecated-metric-api` reports. The rule also misses
`HDBSCAN`, `stable_cumsum` and the PassiveAggressive classes reached through a module or another
name, and a deprecated value in a parameter grid or a variable.

`removed-metric-api` raises on scikit-learn 1.9.1 for every form, ImportError for `SCORERS`, the
`plot_*` functions and `jaccard_similarity_score`, and TypeError for the keywords. `make_scorer`
passes `squared=`, `needs_proba=` and `needs_threshold=` on to the metric, so the error comes when
the scorer runs, and a search with the default `error_score` then scores every fold as nan. The
rule reports `squared=` and `eps=` only on the scikit-learn functions, imported from
`sklearn.metrics` or called as `metrics.<name>`, because torchmetrics has a `mean_squared_error`
with `squared=`. Old module paths such as `sklearn.metrics.ranking` are left to
`removed-in-sklearn`. The four `RocCurveDisplay` forms raised TypeError in the first run, with
matplotlib 3.11.2 added to scikit-learn 1.9.1, and the saved rerun had no matplotlib, so the
saved results hold no passing proof for them. The forms raise, so a fix could not be compared bit
for bit, and they have none. Neither reader called one of the 13 held-back reports wrong. Recall
was 74 of 74.

`deprecated-metric-api` gives the release that removes each form, from the warning text.
`y_pred=` on `log_loss` goes in 1.11, `estimator_name=` and the display `y_pred=` go in 1.10, the
display matplotlib keywords go in 1.11, and `entropy` goes in 1.10. The four display forms passed
in the first run with matplotlib 3.11.2, and the saved rerun had no matplotlib. The two fixes
rename a keyword and gave bit equal results, the `log_loss` rename in 12 of 12 runs and the
`PrecisionRecallDisplay` rename in 2 of 2 with the same plot label. `scipy.stats.entropy` of the
class counts differs from `entropy` in the last bits, so that form has no fix. Recall was 48 of
48.

`fit-params-keyword` reports a keyword that raises TypeError on scikit-learn 1.9.1. The documents
for `cross_val_score`, `cross_validate` and `cross_val_predict` carry `since: "1.4"`, the release
that added `params`, and the documents for the other three functions carry `since: "1.6"`. On
`GridSearchCV`, `RandomizedSearchCV`, `HalvingGridSearchCV` and `HalvingRandomSearchCV` the
message says to drop the keyword and pass the values to `fit`. A rename has no fix, because the
old form raises and leaves nothing to compare. Neither reader called one of the 4 held-back
reports wrong. Recall was 32 of 32.

`column-transformer-removed` reports a keyword that raises TypeError on scikit-learn 1.9.1, with
either value. The remainder columns now come back in the form the other columns were given in.
Neither reader called one of the 3 held-back reports wrong. Recall was 30 of 30.

### Pipeline and metadata routing

Each form below raises at construction or at fit on scikit-learn 1.9.1.

#### metadata-routing

**Severity:** error. **Fix:** none.

Reports forms that raise because metadata routing is off, such as `set_fit_request` and `Pipeline(transform_input=...)`, and forms that raise because it is on, such as `groups=`, `step__sample_weight=` and `params=` with no request in the file.

Example:

```python
model = LogisticRegression().set_fit_request(sample_weight=True)
cross_validate(model, X, y, params={"sample_weight": w})
```

Use instead:

```python
sklearn.set_config(enable_metadata_routing=True)
model = LogisticRegression().set_fit_request(sample_weight=True).set_score_request(sample_weight=True)
cross_validate(model, X, y, params={"sample_weight": w})
```

The replacement is advice. xpd does not apply it automatically.

#### pipeline-fit-params

**Severity:** error. **Fix:** none.

Reports `pipe.fit(X, y, sample_weight=w)` on a Pipeline built in the same block, with metadata routing off.

Example:

```python
pipe = make_pipeline(StandardScaler(), LogisticRegression())
pipe.fit(X, y, sample_weight=w)
```

Use instead:

```python
pipe = make_pipeline(StandardScaler(), LogisticRegression())
pipe.fit(X, y, logisticregression__sample_weight=w)
```

The replacement is advice. xpd does not apply it automatically.

#### pipeline-step-form

**Severity:** error. **Fix:** none.

Reports step names with `__`, `make_pipeline` given one list, `Pipeline` given bare estimators, and an imbalanced-learn sampler before another step in the Pipeline of scikit-learn.

Example:

```python
from sklearn.pipeline import Pipeline

pipe = Pipeline([("smote", SMOTE()), ("model", LogisticRegression())])
```

Use instead:

```python
from imblearn.pipeline import Pipeline

pipe = Pipeline([("smote", SMOTE()), ("model", LogisticRegression())])
```

The replacement is advice. xpd does not apply it automatically.

#### transformed-target-pair

**Severity:** error. **Fix:** none.

Reports `TransformedTargetRegressor` with `func` and no `inverse_func`, the other way round, or with both a `transformer` and functions.

Example:

```python
model = TransformedTargetRegressor(regressor=Ridge(), func=np.log1p)
```

Use instead:

```python
model = TransformedTargetRegressor(regressor=Ridge(), func=np.log1p, inverse_func=np.expm1)
```

The replacement is advice. xpd does not apply it automatically.

#### pandas-output-sparse

**Severity:** error. **Fix:** none.

Reports pandas output on a OneHotEncoder that keeps its default `sparse_output=True`, set with `set_output` or with `set_config(transform_output="pandas")`.

Example:

```python
encoder = OneHotEncoder(handle_unknown="ignore")
encoder.set_output(transform="pandas")
```

Use instead:

```python
encoder = OneHotEncoder(handle_unknown="ignore", sparse_output=False)
encoder.set_output(transform="pandas")
```

The replacement is advice. xpd does not apply it automatically.


No held-back file uses metadata routing, a misnamed step, `TransformedTargetRegressor` in a wrong
mix, or pandas output on a sparse encoder, so the rules in this group made no report there and
have no wrong-advice number.

`metadata-routing` counts a file as routed when any call in it passes `enable_metadata_routing`
with a value other than `False`. With routing off, `set_fit_request` and the other request
setters raise RuntimeError, and `Pipeline(transform_input=...)` raises ValueError at fit. With
routing on, `groups=` on a cross-validation function raises ValueError, a `step__sample_weight`
keyword on `fit` raises TypeError, and `params=` raises UnsetMetadataPassedError when no
estimator asked for the metadata. A function or a script that never calls a `set_*_request`
method has asked for nothing. On scikit-learn 1.9.1 a weighted `cross_validate` with routing on
also needs `set_score_request`, or it raises for the scorer. Recall was 34 of 34.

`pipeline-fit-params` reads the Pipeline from an assignment earlier in the same block, or from
the call itself. With routing off, `Pipeline.fit` raises "ValueError: Pipeline.fit does not
accept the sample_weight parameter". A file that turns routing on is left alone, because there a
plain name is the right form. Recall was 23 of 30. The rule found none of 6 `sample_weight=`
keywords given to the `fit` of a search over a Pipeline, which raises the same ValueError. It also
misses a Pipeline built outside a loop and fitted inside it.

`pipeline-step-form` reports a step name with two underscores in a row in the list given to
`Pipeline`, `ColumnTransformer` or `FeatureUnion`, which raises ValueError at fit.
`make_pipeline([a, b])` raises TypeError at fit, and `Pipeline([a, b])` raises "TypeError: cannot
unpack non-iterable". A sampler such as `SMOTE` before another step raises "TypeError: All
intermediate steps should be transformers" in the Pipeline of scikit-learn. The first three forms
are reported only when the name comes from scikit-learn, imported from `sklearn.pipeline` or
`sklearn.compose` or called through those modules, because other libraries such as xfeat have a
Pipeline that takes a plain list. The frozen rule made 4 reports on the held-back files, on such
Pipelines, and both readers called all 4 wrong. Files that import from `imblearn.pipeline` are
left alone, and so is a sampler as the last step, which fits. Recall was 29 of 35, measured
before the rule was narrowed to the Pipeline of scikit-learn. The rule found 4 of 10 plants of
`Pipeline` given bare estimators, and it misses them given by `steps=`, by names that hold the
estimators, or as a single step.

`transformed-target-pair` reports the three mixes that raise ValueError at fit. Recall was 31 of
36. The rule found none of 5 plants that import the class under another name, as in
`TransformedTargetRegressor as TTR`.

`pandas-output-sparse` reports `set_output(transform="pandas")` on a OneHotEncoder, or on a
Pipeline, ColumnTransformer or FeatureUnion that holds one, either in the same expression or
assigned in an earlier statement of the block. With `set_config` or `config_context` it reports
every OneHotEncoder in the file. A OneHotEncoder with `sparse_output=False` is left alone. Each
form raises "ValueError: Pandas output does not support sparse data" on scikit-learn 1.9.1.
Recall was 29 of 31.

### Work done twice

Each old form below repeats work that scikit-learn already did, and the new form does it once.
The numbers are speed proofs on scikit-learn 1.9.1, on one thread, as the median of 7 runs of
each form at two data sizes. A rule reports only the classes where both sizes passed 1.5 times.

#### predict-per-row

**Severity:** warning. **Fix:** none.

Reports `model.predict([row])`, `model.predict_proba(X[i].reshape(1, -1))`, `model.predict(df.iloc[[i]])` and the same calls of `decision_function` and `predict_log_proba` in a loop, a comprehension or a lambda over the rows.

Example:

```python
labels = [model.predict([row])[0] for row in X_test]
```

Use instead:

```python
labels = model.predict(X_test)
```

The replacement is advice. xpd does not apply it automatically.

#### score-after-predict

**Severity:** warning. **Fix:** unsafe: `f1_score(y_test, y_pred)`. `model.score(X_test, y_test)` has no fix.

Reports `model.score(X_test, y_test)` and `f1_score(y_test, model.predict(X_test))` after `y_pred = model.predict(X_test)`, when the file builds `model` as a forest, a boosting or bagging model, k-nearest neighbours, an SVM, an MLP or `GaussianNB`.

Example:

```python
forest = RandomForestClassifier(random_state=0).fit(X_train, y_train)
y_pred = forest.predict(X_test)
accuracy = forest.score(X_test, y_test)
```

Use instead:

```python
forest = RandomForestClassifier(random_state=0).fit(X_train, y_train)
y_pred = forest.predict(X_test)
accuracy = accuracy_score(y_test, y_pred)
```

#### predict-and-predict-proba

**Severity:** warning. **Fix:** none.

Reports `model.predict(X_test)` next to `model.predict_proba(X_test)` when the file builds `model` as a `RandomForestClassifier`, `ExtraTreesClassifier` or `BaggingClassifier`.

Example:

```python
forest = RandomForestClassifier(random_state=0).fit(X_train, y_train)
y_pred = forest.predict(X_test)
y_score = forest.predict_proba(X_test)[:, 1]
```

Use instead:

```python
forest = RandomForestClassifier(random_state=0).fit(X_train, y_train)
proba = forest.predict_proba(X_test)
y_pred = forest.classes_[proba.argmax(axis=1)]
y_score = proba[:, 1]
```

The replacement is advice. xpd does not apply it automatically.

#### cross-val-score-per-metric

**Severity:** warning. **Fix:** none.

Reports `cross_val_score(model, X, y, scoring="accuracy")` followed by `cross_val_score(model, X, y, scoring="precision")` on the same model, data and `cv`.

Example:

```python
accuracy = cross_val_score(model, X, y, cv=5, scoring="accuracy")
precision = cross_val_score(model, X, y, cv=5, scoring="precision")
```

Use instead:

```python
scores = cross_validate(model, X, y, cv=5, scoring=["accuracy", "precision"])
accuracy, precision = scores["test_accuracy"], scores["test_precision"]
```

The replacement is advice. xpd does not apply it automatically.

#### vectorizer-fit-then-transform

**Severity:** warning. **Fix:** unsafe: `vectorizer.fit_transform(docs)`. The form in two statements has no fix.

Reports `vectorizer.fit(docs).transform(docs)`, and `vectorizer.fit(docs)` followed by `vectorizer.transform(docs)`, on a `CountVectorizer` or `TfidfVectorizer`.

Example:

```python
vectorizer = TfidfVectorizer()
X = vectorizer.fit(docs).transform(docs)
```

Use instead:

```python
vectorizer = TfidfVectorizer()
X = vectorizer.fit_transform(docs)
```

#### refit-per-n-estimators

**Severity:** warning. **Fix:** none.

Reports a `RandomForest`, `ExtraTrees`, `GradientBoosting` or `Bagging` model or an `IsolationForest` built with `n_estimators=n` in a loop over `n` and fitted there.

Example:

```python
for n in [10, 50, 100]:
    forest = RandomForestClassifier(n_estimators=n, random_state=0)
    forest.fit(X_train, y_train)
    scores.append(forest.score(X_test, y_test))
```

Use instead:

```python
forest = RandomForestClassifier(warm_start=True, random_state=0)
for n in [10, 50, 100]:
    forest.set_params(n_estimators=n)
    forest.fit(X_train, y_train)
    scores.append(forest.score(X_test, y_test))
```

The replacement is advice. xpd does not apply it automatically.

#### tsne-exact

**Severity:** info. **Fix:** none.

Reports `TSNE(method="exact")`.

Example:

```python
embedding = TSNE(method="exact", random_state=0).fit_transform(X)
```

Use instead:

```python
embedding = TSNE(random_state=0).fit_transform(X)
```

The replacement is advice. xpd does not apply it automatically.


`predict-per-row` reports a call whose rows come from the loop variable, either the row itself or
the data indexed by it. Each call checks the input and sets up the model again. One call on all
rows measured 2.77 and 2.79 times faster for `SVC` at 500 and 5,000 rows, 216 and 356 times for
`RandomForestClassifier`, and 104 and 816 times for `LogisticRegression.predict_proba`. The
labels were bit equal, and the probabilities of `LogisticRegression` differed by up to 3.3e-16,
so the rule has no fix. A loop that writes into the data it predicts from, as recursive
forecasting does, is left alone. Reader B called both held-back reports wrong advice, and reader
A called neither wrong. Recall was 28 of 32. The 4 misses were `DataFrame.apply` with the row's
values in a list, as in `X_test.apply(lambda row: model.predict([row.values])[0], axis=1)`.

`score-after-predict` reads the class from the line that builds the model, because `score` costs
the most where `predict` does. The score call measured 1.62 to 879 times the metric alone over
the 13 reported classes that were timed, at 1,000 and 10,000 test rows, and gave the same number.
`GaussianNB` was the slowest gain, 1.62 times at 1,000 rows. `DecisionTreeClassifier` measured
1.45 times at 1,000 rows in the first run and 1.54 times in the saved rerun, and
`DecisionTreeRegressor` 1.52 and 1.66 times. The rule leaves both decision trees alone, as it does
`LogisticRegression` and `LinearRegression`. A name the file also binds to another class is left
alone. The fix on the metric form is unsafe, because code between the two lines can change the
saved predictions. It gave the same score in 12 of 12 runs on both held-back shapes, and the
score changed only when the model was fitted again between the two lines. Neither reader called
one of the 17 held-back reports wrong. Recall was 31 of 32.

`predict-and-predict-proba` covers the classes whose `predict` takes the argmax of
`predict_proba`. Deriving the labels from one `predict_proba` measured 1.83 to 2.00 times faster
for `RandomForestClassifier`, `ExtraTreesClassifier` and `BaggingClassifier` at 1,000 and 10,000
rows, with bit equal labels and probabilities. Other classes decide the label from a raw score,
and a tie at 0.5 can give another label, so they are left alone. Neither reader called one of the
22 held-back reports wrong. Recall was 30 of 30.

`cross-val-score-per-metric` reports the first call when a later call in the same block passes the
same model, data and `cv` with another `scoring`. Two scorings in one `cross_validate` measured
1.60 and 1.84 times faster at 2,000 and 20,000 rows with `LogisticRegression`, and the scores were
bit equal. Neither reader called one of the 14 held-back reports wrong. Recall was 30 of 30.

`vectorizer-fit-then-transform` covers `CountVectorizer` and `TfidfVectorizer`, whose `fit` runs
`fit_transform` and drops the matrix. `fit_transform` measured 1.63 and 2.06 times faster for
`CountVectorizer` and 1.82 and 2.03 times faster for `TfidfVectorizer`, at 2,000 and 20,000
documents. The fitted vectorizer is bit equal. The fix is unsafe, because `fit_transform` leaves
the column indices of each row unsorted, and `TfidfVectorizer` values differ by up to 1.1e-16. A
`MultinomialNB` fitted on the matrix gave probabilities that differ by up to 3e-15, and a
`LogisticRegression` coefficients that differ by up to 1.3e-15. Reader B called 4 of the 12
held-back reports wrong advice, and reader A called none wrong. Recall was 19 of 25. The 6 misses
were a fitted vectorizer assigned back to its own name, as in `cv = cv.fit(corpus)` followed by
`cv.transform(corpus)`.

`refit-per-n-estimators` relies on `warm_start=True`, which keeps the trees already grown and
draws every seed from `random_state` in the same order, so the grown models were bit equal to
models fitted from scratch. For `n_estimators` 10, 20, 50 and 100, warm start measured 1.64 to
2.75 times faster at 1,000 and 5,000 rows for `RandomForestClassifier`, `RandomForestRegressor`,
`ExtraTreesClassifier`, `GradientBoostingClassifier`, `BaggingClassifier` and `IsolationForest`.
For 10 to 100 trees in steps of 10, it measured 6.82 and 3.59 times faster for
`RandomForestClassifier`. A model passed to a cross-validation function or to `clone` is cloned,
so warm start cannot help it, and that loop is left alone. So is a loop with another loop inside
it, where another setting changes between fits, and a loop that reads a clock, because it times
each fit. The frozen rule made 1 report on the held-back files, on a loop that timed each fit,
and both readers called it wrong. Recall was 30 of 30.

`tsne-exact` is info, because the default Barnes-Hut method approximates the forces between
distant points, so its embedding is another one and no speed proof applies. Over 20 seeds on
`make_classification` data, Barnes-Hut gave a trustworthiness with 5 neighbours within 0.001 of
the exact method and fitted a median 2.2 times faster at 300 rows and 4.0 times faster at 1,000
rows. At 300 rows the slowest seed took 0.39 times as long with the exact method, so the gain is
not certain on small data. Recall was 21 of 30. The rule misses `TSNE` imported under another
name and a method held in a variable.

### Reproducible results

#### random-state-without-shuffle

**Severity:** error. **Fix:** none.

Reports `KFold(n_splits=5, random_state=42)` without `shuffle=True`, which raises ValueError.

Example:

```python
folds = KFold(n_splits=5, random_state=42)
```

Use instead:

```python
folds = KFold(n_splits=5, shuffle=True, random_state=42)
```

The replacement is advice. xpd does not apply it automatically.

#### missing-random-state

**Severity:** info. **Fix:** none.

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


`random-state-without-shuffle` covers `KFold`, `StratifiedKFold` and `StratifiedGroupKFold`, and
`GroupKFold` from scikit-learn 1.6, which added its `shuffle`. Each raises "Setting a random_state
has no effect since shuffle is False" on scikit-learn 1.9.1. A file that imports the removed
`sklearn.cross_validation` is left alone unless the call passes `n_splits`, because the old
classes took other arguments and `removed-in-sklearn` reports the import. The frozen rule gave
wrong advice in 2 and 2 of 11 labelled reports, on such files, and the narrowed rule no longer
makes those reports. Recall was 54 of 54.

`missing-random-state` is info, because a seed does not move the expected score. What changes is
which model wins a comparison. With two models whose true accuracy differs a little, the worse
model won 25.0 percent of 300 trials when each was scored on its own split without a seed, and
14.3 percent when both were scored on one shared split. The classes the rule names gave another
output in five fresh runs on scikit-learn 1.9.1. They are the forests, extra trees, bagging,
gradient boosting, `AdaBoostRegressor`, `IsolationForest`, `RandomTreesEmbedding`, the decision
trees, `KMeans`, `MiniBatchKMeans`, `SpectralClustering`, `AffinityPropagation`, the MLPs, the SGD
models, `TruncatedSVD`, `NMF`, `LatentDirichletAllocation`, `FastICA`,
`MiniBatchDictionaryLearning`, `MDS`, `SpectralEmbedding`, `Nystroem`, `RBFSampler`, the random
projections, `ShuffleSplit`, `StratifiedShuffleSplit`, `RepeatedKFold` and
`RepeatedStratifiedKFold`, and `KFold`, `StratifiedKFold`, `GroupKFold` and
`StratifiedGroupKFold` with `shuffle=True`. `TSNE` with `init="random"`, `SVC(probability=True)`,
a HistGradientBoosting model with `early_stopping=True`, and a Gaussian mixture with more than one
component are reported too. A HistGradientBoosting model without `random_state` is reported with
a message that names 10,000 rows, because above that size it stops early on a validation part
drawn at random. `LogisticRegression`, `PCA`, `Perceptron`, `AdaBoostClassifier` and `TSNE` with
its default `init` gave the same output and are not listed. The rule leaves alone a file that
calls `np.random.seed`, or a seed helper such as `set_seed` or `seed_everything` that the file
does not define and does not import from TensorFlow, PyTorch or `random`. It also leaves alone a
call with `shuffle=False` or `**`, a `train_test_split` whose result is thrown away, and a
constructor that only `isinstance` reads. Of 527 held-back reports, 52 were labelled, and each
reader called 1 wrong. Recall was 79 of 80.

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

The files in `samples/pandas/`, `samples/numpy/`, `samples/sklearn/`, `samples/hamilton/` and
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

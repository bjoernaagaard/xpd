# Rules

xpd has nine rule packs: [pandas](#pandas), [NumPy](#numpy), [SciPy](#scipy),
[scikit-learn](#scikit-learn), [scikit-lego](#scikit-lego), [MAPIE](#mapie),
[Category Encoders](#category-encoders), [Hamilton](#hamilton) and [Python](#python). A library's
rules run only on files that import it. The Python rules run on every file.

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

#### datetime-unit-check

**Severity:** error. **Fix:** unsafe: `pd.api.types.is_datetime64_dtype(df["d"])` and `select_dtypes("datetime64")`, which are True for every unit. A dtype with a time zone, `str(dtype)`, `dtype.name`, `dtypes` and `is_datetime64_ns_dtype` have no fix.

Reports `df["d"].dtype == "datetime64[ns]"`, `df["d"].dtype == "datetime64[ns, UTC]"`, `select_dtypes("datetime64[ns]")` and `is_datetime64_ns_dtype(df["d"])`. Datetimes default to microseconds in pandas 3, so on pandas 3.1 each comparison was False for a column parsed from text and `select_dtypes` selected no column.

Example:

```python
is_time = df["time"].dtype == "datetime64[ns]"
times = df.select_dtypes("datetime64[ns]")
```

Use instead:

```python
is_time = pd.api.types.is_datetime64_dtype(df["time"])
times = df.select_dtypes("datetime64")
```

For a column with a time zone, write `isinstance(df["time"].dtype, pd.DatetimeTZDtype)` and `select_dtypes("datetimetz")`.

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

**Severity:** error. **Fix:** safe: `map`, `"ME"`, `"h"`, `"min"`, a Timedelta unit such as `"min"` for `"T"`, `sep=r"\s+"`, `lineterminator=`, `sep=` for a separator passed by position, `is_monotonic_increasing`, `np.` for `pd.np.` and `pd.json_normalize`. unsafe: `ffill()` and `bfill()`, which label and order the rows of a group-by differently, `"M"` in `resample`, `dt.isocalendar().week`, `writer.close()`, `pd.Series(x).value_counts()` and the alias in a comparison with `infer_freq` or `freqstr`. The other forms have no fix.

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

#### day-frequency-with-time-zone

**Severity:** warning. **Fix:** unsafe: `"24h"` for `"D"` and `"1D"` in `shift` and `pct_change`. Several days, the `Day` offset, `resample` and `Grouper` have no fix.

Reports `ts.shift(freq="D")`, `ts.index + pd.offsets.Day(1)` and `ts.resample("D", offset="6h")` in a file that names a time zone. `"D"` was 24 hours on pandas 2 and is a calendar day on pandas 3.

On quarter-hourly data in `Europe/Copenhagen`, pandas 2.3 shifted by 24 hours. pandas 3.1 raised ValueError when the data held 31 March or 27 October 2024, because a shifted time fell in the daylight saving change. With `resample("D", offset="6h")`, pandas 2.3 started each bin at 06:00. pandas 3.1 started it at 00:00 and gave only a RuntimeWarning, also on an index in UTC. `origin=` and a frequency of several days, such as `"7D"`, are ignored in the same way.

Example:

```python
ts = df.set_index("time").tz_localize("Europe/Copenhagen")
day_before = ts.shift(freq="D")
```

Use instead:

```python
ts = df.set_index("time").tz_localize("Europe/Copenhagen")
day_before = ts.shift(freq="24h")
```

`"24h"` gives the pandas 2 result on both versions. It reads the value 24 hours earlier, which is one hour off by the clock on the day after a daylight saving change. In `resample`, bins of `"24h"` keep the offset, and they start an hour later by the clock after a change to summer time.

xpd cannot see whether an index has a time zone. It reports only in a file that holds `tz_localize`, `tz_convert`, `tz=` or `utc=True`, so it misses a frame that got its time zone in another file. On an index without a time zone both versions give the same result. A PeriodIndex of days in such a file is reported by mistake, unless
its name holds the word period, and there `"24h"` raises.

#### mixed-offsets-without-utc

**Severity:** warning. **Fix:** none.

Reports `pd.to_datetime(s, format="%Y-%m-%d %H:%M%z")` without `utc=`. Local times across a daylight saving change carry two offsets, such as `+0100` and `+0200`. For them pandas 2.3 returned an object Index with a FutureWarning, and pandas 3.1 raises ValueError.

Example:

```python
times = pd.to_datetime(df["time"], format="%Y-%m-%d %H:%M%z")
```

Use instead:

```python
times = pd.to_datetime(df["time"], format="%Y-%m-%d %H:%M%z", utc=True).dt.tz_convert("Europe/Copenhagen")
```

The replacement is advice. xpd does not apply it automatically, because `utc=True` changes the result of a column that holds one offset, from that offset to UTC.

#### deprecated-in-pandas-3

**Severity:** warning. **Fix:** safe: `pd.Timestamp.now("UTC")`, `pd.Timestamp.fromtimestamp(t, "UTC")`, `"h"` and `"s"` for the Timedelta units `"H"` and `"S"`, drop `copy=`. unsafe: delete the copy-on-write option line, because `pd.get_option` reads the value back, and drop `copy=` from `astype`.

Reports `pd.set_option("mode.copy_on_write", True)`, `copy=False`, `pd.Timestamp.utcnow()`, `pd.Timestamp.utcfromtimestamp(t)`, and the units `"H"` and `"S"` in `pd.Timedelta("1H")`, `pd.to_timedelta(x, unit="H")` and `resample(offset="1H")`. `astype(copy=)` is reported on a column taken by a label or a receiver named like a frame, because NumPy's `astype` has the same keyword.

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
  `pct_change(fill_method="pad")`, `between_time(include_start=, include_end=)`, `pd.Timestamp(freq=)`,
  and `astype("timedelta64[D]")`.
- Timedelta units: `"T"`, `"L"`, `"U"` and `"N"` in `pd.Timedelta("5T")`, `pd.Timedelta(1, unit="T")`,
  `pd.to_timedelta(x, unit="T")` and `resample(offset="30T")`.
- A handler for `pytz.exceptions.AmbiguousTimeError` or `NonExistentTimeError`. pandas 3 raises ValueError for
  those times, also when the zone is a pytz object, and the pytz errors are not ValueErrors.
- An old alias compared with `pd.infer_freq(index)` or `index.freqstr`, as in `pd.infer_freq(index) == "H"`,
  which is never equal, because pandas returns `"h"`.
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
`date_range`, `timedelta_range`, `interval_range`, `to_offset`, `Grouper`, `floor`, `snap` and
`rolling`. Inside `resample`, `shift` and `pct_change` the fix for `"M"`, `"Q"` and `"Y"` is
unsafe, because those aliases stay valid when the index holds periods. It leaves `asfreq("M")`
alone, and it reports `"A"` inside `resample`, `asfreq`, `shift` and `pct_change` without a fix,
because the new alias is `"YE"` for datetimes and `"Y"` for periods. Where the call takes
periods, in `to_period`, `period_range`, `pd.Period` and `pd.PeriodIndex`, it rewrites `"H"`,
`"T"`, `"S"`, `"L"`, `"U"` and `"N"`, writes `"Y"` for `"A"`, and leaves `"M"`, `"Q"` and `"Y"`
alone.

The handler for a pytz error is reported when its try block holds a pandas call, such as
`tz_localize`, `.dt.`, `pd.` or `floor`. It is left alone when it also names ValueError or
Exception, and when the block calls pytz's own `localize`, which still raises the pytz errors. The fix for an
alias compared with `infer_freq` or `freqstr` is unsafe, because the comparison can then be
equal. `"M"` is compared only with `infer_freq`, because the `freqstr` of a monthly PeriodIndex
is still `"M"`.

`datetime-unit-check` leaves a file alone that calls `as_unit("ns")` or
`astype("datetime64[ns]")`, because that file makes nanosecond columns itself. It compares a dtype
only on a column taken by a label, as in `df["time"].dtype`, or on a name such as `series`, `col`
or `index`, because a NumPy array keeps the unit it was made with. A column read
from a nanosecond source, such as a NumPy array of `datetime64[ns]`, keeps that unit, and there
the old comparison is still True.

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

The result has a PeriodIndex instead of text labels. On a column with a time zone, `to_period` drops the zone and gives a UserWarning, so group on `pd.Grouper(key="time", freq="MS")` there.

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

**Severity:** warning. **Fix:** unsafe: `df["d"].dt.floor("h")`, `df["d"].dt.weekday`, `df["d"].dt.date`, `df["d"].dt.isocalendar().week`, `df["d"].dt.tz_localize(None)` for `replace(tzinfo=None)` and `df["d"].dt.tz_convert(zone)` for `astimezone(zone)`. `timestamp` and `toordinal` have no fix.

Reports `df["d"].apply(lambda t: t.floor("h"))`, and the same with `ceil`, `normalize`, `tz_localize`, `tz_convert`, `day_name`, `month_name`, `to_period`, `total_seconds`, `weekday`, `date`, `isocalendar`, `timestamp`, `toordinal`, `replace(tzinfo=None)` and `astimezone`.

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

#### tz-localize-without-dst

**Severity:** warning. **Fix:** none.

Reports `df["d"].dt.tz_localize("Europe/Copenhagen")` without `ambiguous=` or without `nonexistent=`. A zone with daylight saving has a stretch each year that happens twice and one that never happens, an hour in most zones. `tz_localize` raises on a time in either stretch unless the call says what to do.

Example:

```python
local = df["time"].dt.tz_localize("Europe/Copenhagen")
```

Use instead:

```python
local = df["time"].dt.tz_localize("Europe/Copenhagen", ambiguous="infer", nonexistent="shift_forward")
```

`ambiguous="infer"` reads the order of the rows and needs both copies of the repeated hour. `"NaT"` makes the time missing, and a boolean array says which rows are summer time. `nonexistent=` takes `"shift_forward"`, `"shift_backward"`, `"NaT"` or a Timedelta.

The replacement is advice. xpd does not apply it automatically.

#### date-range-then-localize

**Severity:** warning. **Fix:** unsafe: `pd.date_range(start, end, freq="h", tz=zone)`.

Reports `pd.date_range(start, end, freq="h").tz_localize(zone)`.

Example:

```python
hours = pd.date_range("2024-10-26", "2024-10-28", freq="h").tz_localize("Europe/Copenhagen")
```

Use instead:

```python
hours = pd.date_range("2024-10-26", "2024-10-28", freq="h", tz="Europe/Copenhagen")
```

#### to-datetime-then-localize

**Severity:** warning. **Fix:** unsafe: `pd.to_datetime(df["raw"], utc=True)`.

Reports `pd.to_datetime(df["raw"]).dt.tz_localize("UTC")`.

Example:

```python
when = pd.to_datetime(df["raw"]).dt.tz_localize("UTC")
```

Use instead:

```python
when = pd.to_datetime(df["raw"], utc=True)
```

#### convert-then-drop-zone

**Severity:** warning. **Fix:** safe: `df["d"].dt.tz_convert(None)`.

Reports `df["d"].dt.tz_convert("UTC").dt.tz_localize(None)`, and the same on an index or a frame.

Example:

```python
plain = df["time"].dt.tz_convert("UTC").dt.tz_localize(None)
```

Use instead:

```python
plain = df["time"].dt.tz_convert(None)
```

#### index-plus-offset

**Severity:** warning. **Fix:** unsafe: `ts = ts.shift(freq=pd.Timedelta(hours=6))`.

Reports `ts.index = ts.index + pd.Timedelta(hours=6)` and `ts.index += timedelta(days=1)`.

Example:

```python
ts.index = ts.index + pd.Timedelta(hours=6)
```

Use instead:

```python
ts = ts.shift(freq=pd.Timedelta(hours=6))
```

#### date-range-slice

**Severity:** warning. **Fix:** unsafe: `pd.date_range(start, end, freq="15min", inclusive="left")`.

Reports `pd.date_range(start, end, freq="15min")[:-1]`.

Example:

```python
steps = pd.date_range("2024-03-30", "2024-04-01", freq="15min")[:-1]
```

Use instead:

```python
steps = pd.date_range("2024-03-30", "2024-04-01", freq="15min", inclusive="left")
```

#### index-time-mask

**Severity:** warning. **Fix:** safe: `ts.at_time(time(12))`. unsafe: `ts.between_time("8:00", "16:00", inclusive="left")`.

Reports `ts[ts.index.time == time(12)]` and `ts[(ts.index.hour >= 8) & (ts.index.hour < 16)]`.

Example:

```python
noon = ts[ts.index.time == time(12)]
working = ts[(ts.index.hour >= 8) & (ts.index.hour < 16)]
```

Use instead:

```python
noon = ts.at_time(time(12))
working = ts.between_time("8:00", "16:00", inclusive="left")
```

#### reindex-to-date-range

**Severity:** warning. **Fix:** unsafe: `ts.asfreq("15min")`.

Reports `ts.reindex(pd.date_range(ts.index.min(), ts.index.max(), freq="15min"))`.

Example:

```python
full = ts.reindex(pd.date_range(ts.index.min(), ts.index.max(), freq="15min"))
```

Use instead:

```python
full = ts.asfreq("15min")
```

#### divide-by-shift

**Severity:** warning. **Fix:** unsafe: `df["load"].pct_change(96)`.

Reports `df["load"] / df["load"].shift(96) - 1` and `(df["load"] - df["load"].shift(96)) / df["load"].shift(96)`.

Example:

```python
change = df["load"] / df["load"].shift(96) - 1
```

Use instead:

```python
change = df["load"].pct_change(96)
```

#### date-part-key

**Severity:** warning. **Fix:** none.

Reports `df.groupby([df["d"].dt.year, df["d"].dt.month])`, and the same with `index.year` and `index.month`.

Example:

```python
monthly = df.groupby([df["time"].dt.year, df["time"].dt.month])["load"].sum()
```

Use instead:

```python
monthly = df.groupby(pd.Grouper(key="time", freq="MS"))["load"].sum()
```

The result has one timestamp per month in place of a year and a month, and it also holds the months without rows.

The replacement is advice. xpd does not apply it automatically.

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

The time zone rules were run on pandas 2.3.3 and 3.1.1 with quarter-hourly data in
`Europe/Copenhagen`. `tz-localize-without-dst` raised on half-hourly data for 2024 in that zone.
It leaves `"UTC"`, `"GMT"`, a zone under `Etc/` and an offset such as `"+01:00"` alone, which did
not raise. It also leaves a zone held in a name alone, and it reports a zone without daylight
saving, such as `"Asia/Tokyo"`, by mistake. `date-range-then-localize` is unsafe, because hourly
steps from 26 to 28 October 2024 raised when localized and gave 50 hours with `tz=`. Without a
daylight saving change in the range the two forms gave an equal index.
`to-datetime-then-localize` is unsafe, because on text that carries an offset `utc=True` converts
to UTC where `tz_localize` raised TypeError. `index-plus-offset` is unsafe, because the
assignment changes the object in place and `shift` returns a new one. `date-range-slice` is
unsafe: when the end is not a step of the range, the slice drops the last real step and
`inclusive="left"` keeps it. It leaves a range with periods alone. These three rules also leave
a call alone that ends in a trailing comma or holds a comment, because the fix adds an argument
after the last one.

`index-time-mask` took 241 ms as a mask on `index.time` and 25 ms as `at_time` on 2,000,000 rows.
The two hour masks took 51 ms and `between_time` took 28 ms. The `between_time` fix applies to
whole hours written as numbers, and it is unsafe: when the first hour is later than the second,
the masks select nothing and `between_time` selects the hours across midnight. The `at_time` fix
applies to a time object, such as `time(12)`, and a comparison with text is left alone, because
text never equals a time.
`reindex-to-date-range` is unsafe, because `asfreq` keeps the name of the index and the reindex
dropped it. `divide-by-shift` is unsafe: `(s - s.shift(n)) / s.shift(n)` rounds differently, so
the result is close and not bit equal, with or without a number of rows, and on pandas 2 `pct_change` filled missing values forward
first. `date-part-key` took 25 ms with two keys and 6 ms with one monthly key on 2,000,000 rows.
For `replace(tzinfo=None)` in a lambda, `dt.tz_localize(None)` took 2.3 ms where the lambda took
746 ms on 200,000 rows. `apply-date-method` leaves `replace(tzinfo=zone)` alone, because `replace`
skips the daylight saving check that `tz_localize` makes.

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

Reports `random_state=` and `seed=` passed by keyword to the functions and classes that SciPy 1.15 moved to `rng=`, such as sparse.random, svds, bootstrap, permutation_test, kmeans2, differential_evolution, basinhopping, dual_annealing, directed_hausdorff, Rotation.random and the qmc engines. SciPy's docs say new code should pass `rng`. The old keywords run without a warning on SciPy 1.18.1. A call under a SciPy version check is left alone, because it is kept for an older SciPy, where `rng=` raises. The message says to keep the old keyword when the code must also run on a SciPy older than 1.15, because xpd reads only the installed SciPy and cannot see which versions a library supports. Skipped on SciPy older than 1.15.

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

Reports `integrate.odeint`, which SciPy's docs ask new code to replace with `solve_ivp`. Neither solver was faster on every problem measured, so the message makes no speed claim. It says that solve_ivp raises ValueError when `t_eval` repeats a time, which odeint accepts, and that the values differ in the last digits. It says to keep solve_ivp's default method for a large system, because `method='LSODA'` stores a full Jacobian and raised MemoryError at 1,000,000 equations.

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

Reports the interpolation classes and functions that SciPy marks as legacy: interp1d, Rbf, UnivariateSpline, InterpolatedUnivariateSpline, LSQUnivariateSpline, splrep, splprep, splev, splint, sproot, spalde, splder and splantider. The message names the replacement SciPy recommends, such as make_splrep for splrep and RBFInterpolator for Rbf. For interp1d it names `make_interp_spline(x, y, k=1)`, with `axis=-1` when `y` has more than one dimension, and says that outside the data interp1d raises where make_interp_spline extrapolates and `numpy.interp` repeats the end value. For splev it says to pass `nu=n` where splev took `der=n`. Documents that name an API added in SciPy 1.7 or 1.15 are skipped on an older SciPy.

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
group 0.887 against 0.497, and 10 columns picked by `df.corr()` 0.771 against 0.497. The rule
leaves three under-samplers alone, because they only drop rows and did not pass the proof:
`RandomUnderSampler` scored 0.512 against 0.491, `NearMiss` 0.448 and `TomekLinks` 0.516. It
leaves the scikit-learn `TargetEncoder.fit_transform` alone too, which cross-fits and scored 0.522
against 0.504. A label
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
scored 0.519 against 0.511, `NearMiss` 0.458 and `TomekLinks` 0.530 against 0.508, which did not
pass the proof, so the rule leaves the three alone. The frozen rule gave wrong advice in 1 and 0 of
11 labelled reports.

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

## scikit-lego

These rules run on files that import `sklego`, the module of scikit-lego. They report arguments
that give a wrong number without an error, steps that read the wrong rows or cannot be fitted,
and code that raises or warns on scikit-lego 0.9.10. Every number in this section was measured
on scikit-lego 0.9.10 with scikit-learn 1.9.1, NumPy 2.5.5 and pandas 3.1.1. Examples assume that
the names they use are imported. Names such as `X_train`, `y_test` and `model` represent values
supplied by your program.

A rule document whose advice needs a newer release of scikit-lego carries `since`. xpd reads the
installed version from the project's venv, as it does for pandas, and skips that document on an
older release. When the venv has no scikit-lego, or there is no venv, every document runs.

The pack has no package checks. cvxpy, umap-learn and formulaic are extras that only some
estimators need, and a package check cannot see which estimators a project uses. scikit-lego
raises an ImportError that names the extra when one is missing.

### How a rule earns its place

Every rule is an `error` only when real code was run that shows the wrong number or the
exception, and its message quotes what was measured. A proof is one of three kinds:

- A number proof runs the old and the new form on the same data and compares what they return.
- An interpreter proof runs the old form and records the exception, or the warning under
  `python -W error`, and runs the new form clean.
- A speed proof times the old form at two settings.

A rule is a `warning` when the proof passed and a pattern cannot see whether the code wants the
old form, and `info` when no proof of the three kinds applies.

The rules were then run over scikit-lego's own source, docs scripts and tests at the 0.9.10 tag,
134 Python files, and every report was read. The pack made 46 reports there. 42 are on a form the
rule describes, most of them in tests that build a deprecated or superseded class or check that a
wrong call raises. 4 are on code that is right as it stands: the package's own export of
`PandasTypeSelector`, two tests of `add_lags` with a positive lag, and one test that passes
`threshold=0.1` to `GMMOutlierDetector` only to build an object. Two rules were narrowed after
that reading. `debug-pipeline-default-log` now leaves a file alone when it imports `logging` or
uses pytest's `caplog`, which removed 5 reports, and `unseeded-random-estimator` leaves test
files alone, which removed 7.

| Rule | Severity | Proof | Reports on scikit-lego's files | On code that is right | Fix |
| --- | --- | --- | --- | --- | --- |
| [`thresholder-score`](#thresholder-score) | error | number: 0.875 at three thresholds against 0.700, 0.875 and 0.850 | 0 | 0 | none |
| [`estimator-transformer-proba`](#estimator-transformer-proba) | error | number: shape (800, 1) for 400 rows | 0 | 0 | none |
| [`column-capper-fraction`](#column-capper-fraction) | error | number: 9,910 of 10,000 values changed against 1,000 | 0 | 0 | none |
| [`dict-mapper-not-int`](#dict-mapper-not-int) | error | number: 0.5 and 1.7 became 0 and 1. Interpreter: ValueError | 0 | 0 | none |
| [`regression-outlier-column-name`](#regression-outlier-column-name) | error | number: coefficients 0, 0 and 1, and 30 rows flagged against 10 | 2 | 0 | none |
| [`pca-outlier-after-scaler`](#pca-outlier-after-scaler) | error | number: 287 rows flagged against 20 | 0 | 0 | none |
| [`outlier-threshold`](#outlier-threshold) | error | interpreter: ValueError. Number: 380 of 400 rows flagged against 20 | 1 | 1 | none |
| [`grouped-shrinkage-arguments`](#grouped-shrinkage-arguments) | error | number: probabilities bit equal with and without the keyword. Interpreter: TypeError | 0 | 0 | unsafe |
| [`add-lags-positive`](#add-lags-positive) | warning | number: accuracy 0.992 against 0.531 on noise | 2 | 2 | none |
| [`decay-on-shuffled-rows`](#decay-on-shuffled-rows) | warning | number: slope 1.648 against 2.989 | 0 | 0 | none |
| [`outlier-remover-before-model`](#outlier-remover-before-model) | error | interpreter: ValueError | 0 | 0 | none |
| [`decay-without-sample-weight`](#decay-without-sample-weight) | error | interpreter: ValueError, TypeError | 0 | 0 | none |
| [`zero-inflated-roles`](#zero-inflated-roles) | error | interpreter: ValueError | 2 | 0 | none |
| [`scorer-called-as-metric`](#scorer-called-as-metric) | error | interpreter: IndexError, TypeError. Number: a function where a score is expected | 0 | 0 | unsafe |
| [`dataset-frame-and-xy`](#dataset-frame-and-xy) | error | interpreter: ValueError | 0 | 0 | none |
| [`log-step-extra-bare`](#log-step-extra-bare) | error | number: a function where a frame is expected. Interpreter: ValueError | 1 | 0 | none |
| [`selector-without-feature-names`](#selector-without-feature-names) | error | interpreter: ValueError, AttributeError | 0 | 0 | none |
| [`debug-pipeline-default-log`](#debug-pipeline-default-log) | info | number: 0 characters printed | 0 | 0 | none |
| [`removed-in-sklego`](#removed-in-sklego) | error | interpreter: ImportError, ModuleNotFoundError, TypeError | 0 | 0 | unsafe |
| [`deprecated-in-sklego`](#deprecated-in-sklego) | warning | interpreter: DeprecationWarning | 10 | 1 | safe |
| [`superseded-estimator`](#superseded-estimator) | warning, info | interpreter: UserWarning | 28 | 0 | none |
| [`group-time-series-split-folds`](#group-time-series-split-folds) | warning | speed: 1.08 s at 5 folds against 10.48 s at 6 | 0 | 0 | none |
| [`unseeded-random-estimator`](#unseeded-random-estimator) | info | none of the three | 0 | 0 | none |

A rule with no report on scikit-lego's files was checked only on the pack's sample files, so its
rate of wrong reports on real code is not measured. The pack was not judged on held-back files
from other projects, and it has no recall number.

### Arguments that give a wrong number

Each old form below runs without an error and returns a number that is not the one the code asks
for, or raises where the argument is read.

#### thresholder-score

**Severity:** error. **Fix:** none.

Reports `GridSearchCV`, `RandomizedSearchCV`, `cross_val_score`, `cross_validate` and the other search and validation functions when they hold a `Thresholder(...)` and no `scoring=`, and reports `.score(...)` on a `Thresholder`. `Thresholder.score` returns the score of the inner model, so it does not change with `threshold`.

Example:

```python
search = GridSearchCV(Thresholder(LogisticRegression(), threshold=0.5), {"threshold": [0.1, 0.5, 0.9]})
accuracy = Thresholder(LogisticRegression(), threshold=0.1).fit(X_train, y_train).score(X_test, y_test)
```

Use instead:

```python
search = GridSearchCV(Thresholder(LogisticRegression(), threshold=0.5), {"threshold": [0.1, 0.5, 0.9]}, scoring="accuracy")
model = FixedThresholdClassifier(LogisticRegression(), threshold=0.1, response_method="predict_proba")
accuracy = model.fit(X_train, y_train).score(X_test, y_test)
```

The replacement is advice. xpd does not apply it automatically. On 400 rows with 20 percent
positives, the grid search gave a mean test score of 0.875 at each of the three thresholds, and
0.700, 0.875 and 0.850 with `scoring="accuracy"`. `score` on the test rows gave 0.890 at each
threshold, where the accuracy of `predict` was 0.770, 0.890 and 0.810. scikit-learn's
`FixedThresholdClassifier` made the same predictions as `Thresholder` at each threshold, and its
`score` gave 0.770, 0.890 and 0.810.

#### estimator-transformer-proba

**Severity:** error. **Fix:** none.

Reports `EstimatorTransformer(..., predict_func="predict_proba")` and `"predict_log_proba"`. The transformer reshapes the output to one column when the target has one dimension, so the probabilities of the two classes are stacked into twice as many rows.

Example:

```python
features = EstimatorTransformer(LogisticRegression(), predict_func="predict_proba")
```

Use instead:

```python
features = EstimatorTransformer(LogisticRegression(), predict_func="decision_function")
stack = StackingClassifier([("lr", LogisticRegression())], stack_method="predict_proba")
```

The replacement is advice. xpd does not apply it automatically. A binary model on 400 rows came
back from `transform` with the shape (800, 1). Inside a `FeatureUnion` and inside a pipeline
before a model, `fit` raised ValueError on the row counts. `decision_function` gave (400, 1), and
`StackingClassifier(stack_method="predict_proba").transform` gave one row per input row.

#### column-capper-fraction

**Severity:** error. **Fix:** none.

Reports `ColumnCapper(quantile_range=(0.05, 0.95))` and other ranges whose upper number is 1 or less. `ColumnCapper` reads the range in percent, as its default `(5.0, 95.0)` shows, so fractions cap every value at a percentile below 1.

Example:

```python
capper = ColumnCapper(quantile_range=(0.05, 0.95))
```

Use instead:

```python
capper = ColumnCapper(quantile_range=(5, 95))
```

The replacement is advice. xpd does not apply it automatically. On 10,000 normal values
`(0.05, 0.95)` changed 9,910 of them and left 92 distinct values between -3.272 and -2.379.
`(5, 95)` changed 1,000 values and kept the range -1.628 to 1.640.

#### dict-mapper-not-int

**Severity:** error. **Fix:** none.

Reports `DictMapper` with a dict literal that maps to a float with a fraction or to text, and a `default` that is such a float, text or NaN. `DictMapper` casts every mapped value and the default to int.

Example:

```python
mapper = DictMapper({"low": 0.5, "high": 1.7}, default=-1)
```

Use instead:

```python
mapper = DictMapper({"low": 0, "high": 1}, default=-1)
weights = frame["level"].map({"low": 0.5, "high": 1.7}).fillna(-1)
```

The replacement is advice. xpd does not apply it automatically. `{"a": 0.5, "b": 1.7}` with
`default=-1` came back as 0, 1 and -1 with no warning. A text value raised ValueError "invalid
literal for int() with base 10", and `default=np.nan` raised ValueError "cannot convert float NaN
to integer". A float with no fraction, such as `1.0`, keeps its value, and the rule leaves it
alone. It also leaves a string that holds a whole number alone: `DictMapper({"a": "1"}, default=0)`
gave 1.

#### regression-outlier-column-name

**Severity:** error. **Fix:** none.

Reports `RegressionOutlierDetector(model, column="y")`, where the column is a name. The detector finds the target of a frame by name, and then drops the column whose position equals `column` from the features. A name equals no position, so the target stays among the features and the inner model predicts it from itself.

Example:

```python
detector = RegressionOutlierDetector(LinearRegression(), column="y").fit(frame)
```

Use instead:

```python
position = frame.columns.get_loc("y")
detector = RegressionOutlierDetector(LinearRegression(), column=position).fit(frame.to_numpy())
```

The replacement is advice. xpd does not apply it automatically. On 500 rows with 10 planted
outliers, `column="y"` on the frame fitted 3 features with the coefficients 0, 0 and 1, had a
residual spread of 8.08e-16, and flagged 30 rows. `column=2` on `frame.to_numpy()` fitted 2
features, had a spread of 1.48, and flagged the 10 planted rows and no other. A position on a
frame is wrong in another way, which a pattern cannot see: the detector compares the frame's
column names with the number, none matches, and the first column becomes the target. scikit-lego's
own test of the name form passes because the planted rows have the largest rounding errors.

#### pca-outlier-after-scaler

**Severity:** error. **Fix:** none.

Reports `PCAOutlierDetection` and `UMAPOutlierDetection` without `variant="absolute"` in a pipeline that holds `StandardScaler`, `RobustScaler` or `PowerTransformer`. The default `variant="relative"` divides the reconstruction error of a row by the sum of the row's values, which a centring step puts near zero.

Example:

```python
detector = make_pipeline(StandardScaler(), PCAOutlierDetection(n_components=1, threshold=0.05))
```

Use instead:

```python
detector = make_pipeline(StandardScaler(), PCAOutlierDetection(n_components=1, threshold=2.0, variant="absolute"))
```

The replacement is advice. xpd does not apply it automatically, because the threshold has other
units. On 1,000 rows with 20 planted outliers the relative variant flagged the 20 planted rows on
the raw data. After `StandardScaler` it flagged 287 rows, 10 of them planted, and 505 rows had a
negative ratio. `variant="absolute"` flagged the 20 planted rows in both cases. A scaler built
with `with_mean=False`, `with_centering=False` or `standardize=False` does not centre, and the
rule leaves it alone.

#### outlier-threshold

**Severity:** error. **Fix:** none.

Reports `PCAOutlierDetection` and `UMAPOutlierDetection` built without a `threshold`, and `GMMOutlierDetector` and `BayesianGMMOutlierDetector` whose `threshold` is below 0.5, or above 1, with `method="quantile"` written out or left as the default.

Example:

```python
pca = PCAOutlierDetection(n_components=2)
gmm = GMMOutlierDetector(threshold=0.05)
```

Use instead:

```python
pca = PCAOutlierDetection(n_components=2, threshold=0.1)
gmm = GMMOutlierDetector(threshold=0.95)
```

The replacement is advice. xpd does not apply it automatically. Without a threshold, `fit` of the
PCA and UMAP detectors raised ValueError "The `threshold` value cannot be `None`." The mixture
detectors keep the share `threshold` of the training rows as inliers. `threshold=0.05` flagged 380
of 400 training rows, 0.4 flagged 240, 0.5 flagged 200, 0.95 flagged 20, and the default 0.99
flagged 4. `threshold=2` raised ValueError, and ran with `method="stddev"`, which counts standard
deviations. A call whose `method` is anything but the text `"quantile"`, such as a variable, is
left alone, because it may be `"stddev"`. A detector inside a search is left alone, because the
search may set the threshold.

#### grouped-shrinkage-arguments

**Severity:** error. **Fix:** unsafe: `shrinkage_kwargs={"alpha": 0.1}` for `alpha=0.1`, and the same for `min_n_obs=`. The other forms have no fix.

Reports three spellings of shrinkage that fail on the grouped and hierarchical meta estimators: any extra keyword on `GroupedClassifier`, which drops it, `shrinkage="constant"` or `"min_n_obs"` without `shrinkage_kwargs`, and a bare `alpha=` or `min_n_obs=` keyword.

Example:

```python
classifier = GroupedClassifier(LogisticRegression(), groups=["shop"], shrinkage="relative")
regressor = GroupedRegressor(LinearRegression(), groups=["region", "shop"], shrinkage="constant", alpha=0.1)
```

Use instead:

```python
classifier = HierarchicalClassifier(LogisticRegression(), groups=["shop"], shrinkage="relative")
regressor = GroupedRegressor(LinearRegression(), groups=["region", "shop"], shrinkage="constant", shrinkage_kwargs={"alpha": 0.1})
```

`GroupedClassifier` ends its signature with `**shrinkage_kwargs` and never reads it. With
`shrinkage="relative"` its probabilities were bit equal to those without the keyword, and they
differed by up to 0.17 from `HierarchicalClassifier(shrinkage="relative")`, which does shrink.
`shrinkage="constant"` without a setting raised TypeError "constant_shrinkage() missing 1 required
positional argument: 'alpha'" in `fit`. The bare keyword raised TypeError "got an unexpected
keyword argument 'alpha'". `GroupedPredictor` took it until scikit-lego 0.9.2, so that document
carries `since: "0.9.2"`.

### Rows and fits

Each old form below fits a step on rows it should not see in that order, or builds a meta
estimator around a model it cannot fit.

#### add-lags-positive

**Severity:** warning. **Fix:** none.

Reports `add_lags` with a positive lag, as a number, in a list or from `range`. `add_lags` shifts by the negative of the lag, so lag 1 puts the value of the next row on each row. pandas `shift(1)` goes the other way.

Example:

```python
features = add_lags(frame, "sales", [1, 2])
```

Use instead:

```python
features = add_lags(frame, "sales", [-1, -2])
```

The replacement is advice. xpd does not apply it automatically. On a frame with sales 10 to 50,
`add_lags(frame, "sales", 1)` gave `sales1` = 20, 30, 40, 50, and lag -1 gave 10, 20, 30, 40 from
the second row on. On a series of pure noise, a model given the column from lag 1 predicted the
sign of the next row with accuracy 0.992, and a model given the column from lag -1 predicted the
sign of its own row with accuracy 0.531. A positive lag is right when the new column is the
target of a forecast, so the rule is a warning. A `range` is reported when its numbers are
written out and it holds a positive lag: it starts at 1 or more, or it counts up to a stop of 2
or more. `range(0, -3, -1)` holds 0, -1 and -2 and is left alone.

#### decay-on-shuffled-rows

**Severity:** warning. **Fix:** none.

Reports `train_test_split` without `shuffle=False` in a file that builds a `DecayEstimator`. The estimator weights each row by its position, the last row most, and never reads a date, so it is right only on rows in time order.

Example:

```python
X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=0)
model = DecayEstimator(Ridge(), decay_kwargs={"decay_rate": 0.99}).fit(X_train, y_train)
```

Use instead:

```python
X_train, X_test, y_train, y_test = train_test_split(X, y, shuffle=False)
model = DecayEstimator(Ridge(), decay_kwargs={"decay_rate": 0.99}).fit(X_train, y_train)
```

The replacement is advice. xpd does not apply it automatically. On 2,000 rows whose slope changed
from 1 to 3 at row 1,000, the model fitted on the first 1,500 rows in order had the slope 2.989
and a mean absolute error of 0.079 on the last 500 rows. Fitted on the same 1,500 rows in
shuffled order it had the slope 1.648 and an error of 1.135, as the mean of 30 shuffles. A
pattern cannot see which rows the estimator is fitted on, so the rule is a warning.

#### outlier-remover-before-model

**Severity:** error. **Fix:** none.

Reports `OutlierRemover` in a pipeline that also holds a step named like a regressor or a classifier. The remover drops rows of X while the pipeline is fitted, and a scikit-learn pipeline cannot drop the same rows of y.

Example:

```python
pipeline = make_pipeline(OutlierRemover(IsolationForest(), refit=True), LinearRegression())
pipeline.fit(X, y)
```

Use instead:

```python
kept = IsolationForest().fit_predict(X) != -1
model = LinearRegression().fit(X[kept], y[kept])
```

The replacement is advice. xpd does not apply it automatically. `fit` raised ValueError "Found
input variables with inconsistent numbers of samples: [356, 400]". A pipeline that ended in
`KMeans`, which reads no y, fitted on the 356 kept rows, and the rule leaves it alone.

#### decay-without-sample-weight

**Severity:** error. **Fix:** none.

Reports `DecayEstimator` around a `Pipeline`, and around a scikit-learn model whose `fit` takes no `sample_weight`, such as `KNeighborsRegressor`, `GaussianProcessRegressor` and `LinearDiscriminantAnalysis`. `DecayEstimator` passes its weights as `fit(X, y, sample_weight=...)`.

Example:

```python
model = DecayEstimator(make_pipeline(StandardScaler(), Ridge()))
```

Use instead:

```python
model = make_pipeline(StandardScaler(), DecayEstimator(Ridge()))
```

The replacement is advice. xpd does not apply it automatically. The pipeline raised ValueError
"Pipeline.fit does not accept the sample_weight parameter", and `KNeighborsRegressor` raised
TypeError "Model KNeighborsRegressor.fit() does not have 'sample_weight'". The list of models is
every classifier and regressor of `sklearn.utils.all_estimators()` on scikit-learn 1.9.1 that
takes no argument to build and whose `fit` raised that TypeError inside a `DecayEstimator`. It
has 22 names, among them `PassiveAggressiveClassifier`, `PassiveAggressiveRegressor`, `CCA` and
`PLSCanonical`. With the `DecayEstimator` around the last step,
the pipeline fitted.

#### zero-inflated-roles

**Severity:** error. **Fix:** none.

Reports `ZeroInflatedRegressor` with a class named like a regressor as the classifier, or a class named like a classifier as the regressor. The classifier comes first and decides zero or not, and the regressor comes second.

Example:

```python
model = ZeroInflatedRegressor(LinearRegression(), LogisticRegression())
```

Use instead:

```python
model = ZeroInflatedRegressor(classifier=LogisticRegression(), regressor=LinearRegression())
```

The replacement is advice. xpd does not apply it automatically. `fit` raised ValueError
"`classifier` has to be a classifier" and, with a classifier in second place, "`regressor` has to
be a regressor". The rule reads the class names, and it knows that `LogisticRegression` is a
classifier.

### Scorers, datasets and helpers

#### scorer-called-as-metric

**Severity:** error. **Fix:** unsafe: `p_percent_score("sex")` for `make_scorer(p_percent_score("sex"))`. The other forms have no fix.

Reports `make_scorer` around `p_percent_score`, `equal_opportunity_score`, `correlation_score` or `subset_score`, and reports `p_percent_score(y_true, y_pred)` and `equal_opportunity_score(y_true, y_pred)`. Each function takes the column to look at and returns a scorer with the signature `(estimator, X, y)`.

Example:

```python
search = GridSearchCV(model, grid, scoring=make_scorer(p_percent_score("sex")))
fairness = p_percent_score(y_test, y_pred)
```

Use instead:

```python
search = GridSearchCV(model, grid, scoring=p_percent_score("sex"))
fairness = p_percent_score("sex")(model, X_test, y_test)
```

`make_scorer` calls the scorer with `(y_true, y_pred)`, which raised IndexError for three of the
functions and TypeError for `equal_opportunity_score`. Inside `GridSearchCV` scikit-learn caught
the error and the best score was NaN. `p_percent_score(y, y_pred)` raised nothing and returned a
function. The scorer itself, given to `scoring=`, gave a score.

#### dataset-frame-and-xy

**Severity:** error. **Fix:** none.

Reports `load_penguins`, `load_arrests`, `load_chicken`, `load_abalone`, `load_heroes` and `load_hearts` called with both `return_X_y=True` and `as_frame=True`. The loaders test `as_frame` first and return the whole frame, with the target in it.

Example:

```python
X, y = load_penguins(return_X_y=True, as_frame=True)
```

Use instead:

```python
frame = load_penguins(as_frame=True)
X, y = frame.drop(columns="species"), frame["species"]
```

The replacement is advice. xpd does not apply it automatically. Unpacking raised ValueError "too
many values to unpack (expected 2)" for all six loaders. scikit-learn's own loaders return a pair
of frames for the same two keywords. `fetch_creditcard` passes both keywords to `fetch_openml`
and is left alone.

#### log-step-extra-bare

**Severity:** error. **Fix:** none.

Reports `@log_step_extra` without arguments, as a bare decorator or with empty parentheses. `log_step` works both ways. `log_step_extra` takes the log functions as its arguments, so as a bare decorator it takes the decorated function as a log function.

Example:

```python
@log_step_extra
def add_total(frame):
    return frame.assign(total=frame["price"] * frame["count"])
```

Use instead:

```python
@log_step_extra(len)
def add_total(frame):
    return frame.assign(total=frame["price"] * frame["count"])
```

The replacement is advice. xpd does not apply it automatically. Calling the function under the
bare decorator returned another function, not a frame, and raised nothing. `@log_step_extra()`
raised ValueError "Supply at least one log_function for log_step_extra".

#### selector-without-feature-names

**Severity:** error. **Fix:** none.

Reports `set_output(...)` and `get_feature_names_out()` on a pipeline that holds `ColumnSelector`, `ColumnDropper` or `TypeSelector`, called on the pipeline itself or on a name the file assigns it to. The three classes have `get_feature_names` and neither of the two scikit-learn methods.

Example:

```python
pipeline = make_pipeline(ColumnSelector(["age", "fare"]), StandardScaler())
pipeline.set_output(transform="pandas")
```

Use instead:

```python
pipeline = make_pipeline(ColumnTransformer([("keep", "passthrough", ["age", "fare"])]), StandardScaler())
pipeline.set_output(transform="pandas")
```

The replacement is advice. xpd does not apply it automatically. `set_output` raised ValueError
"Unable to configure output for ColumnSelector(...) because `set_output` is not available." and
`get_feature_names_out` raised AttributeError "Estimator columnselector does not provide
get_feature_names_out" for each of the three classes. The `ColumnTransformer` form did both, and
names the columns `keep__age` and `keep__fare`.

#### debug-pipeline-default-log

**Severity:** info. **Fix:** none.

Reports `log_callback="default"` on `DebugPipeline` and `make_debug_pipeline` in a file that neither imports `logging` nor uses pytest's `caplog`. The default callback logs each step at level INFO, which Python hides until logging is set up.

Example:

```python
pipeline = DebugPipeline(steps, log_callback="default")
```

Use instead:

```python
import logging

logging.basicConfig(level=logging.INFO)
pipeline = DebugPipeline(steps, log_callback="default")
```

The replacement is advice. xpd does not apply it automatically. Without the setup, fitting the
pipeline printed 0 characters. After `logging.basicConfig(level=logging.INFO)` it logged one line
for each step, such as `[StandardScaler()] shape=(400, 6) time=0s`. Logging may be set up in
another module, so the rule is info.

### Removed, deprecated and superseded forms

Each form below raises or warns on scikit-lego 0.9.10. The removed forms were run and raised. The
deprecated and superseded forms were run under `python -W error`, where the old form raised its
warning and the new form ran clean. The messages take each replacement from the text of the
warning or from the release that made the change. Every fix of a removed form turns code that
raises into code that runs, so none of those fixes is safe.

#### removed-in-sklego

**Severity:** error. **Fix:** unsafe: `DemographicParityClassifier` for `FairClassifier`, `GroupedPredictor` for `GroupedEstimator`, `from sklego.preprocessing import OutlierRemover` for the import from `sklego.meta`, `as_frame=` for `give_pandas=`, and `decay_kwargs={"decay_rate": 0.99}` for `decay=0.99`. The other forms have no fix.

Reports imports of `PatsyTransformer`, `FairClassifier`, `GroupedEstimator` and `sklego.meta.OutlierRemover` and of the modules that held them, which scikit-lego 0.8.0 removed, the `give_pandas=` keyword of the dataset loaders, which 0.6.6 removed, and the `decay=` keyword of `DecayEstimator`, which 0.7.0 removed. An import inside a `try` block that catches the error is left alone.

Example:

```python
from sklego.meta import GroupedEstimator
from sklego.meta import OutlierRemover

frame = load_chicken(give_pandas=True)
model = DecayEstimator(Ridge(), decay=0.99)
```

Use instead:

```python
from sklego.meta import GroupedPredictor
from sklego.preprocessing import OutlierRemover

frame = load_chicken(as_frame=True)
model = DecayEstimator(Ridge(), decay_kwargs={"decay_rate": 0.99})
```

The imports raised ImportError or ModuleNotFoundError, and the keywords raised TypeError.
`PatsyTransformer` has no fix, because `FormulaicTransformer` reads the formula with formulaic
and not with patsy. The weights that `decay=0.97` gave in scikit-lego 0.6 are equal to those of
`decay_kwargs={"decay_rate": 0.97}` to rounding, and not bit for bit. Each document carries
`since` with the release that removed its form.

#### deprecated-in-sklego

**Severity:** warning. **Fix:** safe: `ClusterFoldValidation` for `KlusterFoldValidation`, `TypeSelector` for `PandasTypeSelector`, `date_series=` for `date_serie=`, `n_features_in_` for `dim_`, `num_fit_cols_`, `n_columns_` and `num_cols_`, `n_fitted_levels_` for `n_levels_`, `coef_` for `coefs_`, and `penalty=None` for `penalty="none"`. `allowed_strategies`, `allowed_methods` and `allowed_effects` have no fix.

Reports the names, keywords and fitted attributes that raise a DeprecationWarning: `KlusterFoldValidation`, `PandasTypeSelector`, `TimeGapSplit(date_serie=)`, the attributes `dim_`, `num_fit_cols_`, `n_columns_`, `num_cols_`, `n_levels_` and `coefs_`, the properties `allowed_strategies`, `allowed_methods` and `allowed_effects`, and `penalty="none"` on the fairness classifiers.

Example:

```python
folds = KlusterFoldValidation(cluster_method=KMeans(3))
splitter = TimeGapSplit(date_serie=dates, train_duration=timedelta(days=10), valid_duration=timedelta(days=5))
width = capper.n_columns_
```

Use instead:

```python
folds = ClusterFoldValidation(cluster_method=KMeans(3))
splitter = TimeGapSplit(date_series=dates, train_duration=timedelta(days=10), valid_duration=timedelta(days=5))
width = capper.n_features_in_
```

Every fix was run before and after on the pack's sample file, and the folds, the selected
columns, the attribute values, the coefficients and the predictions were equal. An old attribute
name is reported only on an object that the file builds from a scikit-lego class with that
property: a call chain that starts at the class, as in `ColumnCapper().fit(X).n_columns_`, or a
name that the file assigns such a chain to. The classes are `RandomRegressor`, `RandomAdder`,
`DictMapper` and `OutlierRemover` for `dim_`, the two mixture naive Bayes classifiers for
`num_fit_cols_`, `ColumnCapper` for `n_columns_`, `IntervalEncoder` for `num_cols_`, and
`HierarchicalRegressor` and `HierarchicalClassifier` for `n_levels_`. The same name on any other
object, such as `cfg.dim_` on your own class, stays silent, because the rename would raise
AttributeError there. An assignment to the name, and the definition of a function or class of
the name, are the file's own and stay silent. `coefs_` is reported only in a file that names `ProbWeightRegression`
or `DeadZoneRegressor` and no MLP, because scikit-learn's MLP models have a real `coefs_`. Each
document carries `since` with the release that began to warn.

#### superseded-estimator

**Severity:** warning for the classes that warn, and info for `IdentityTransformer()`. **Fix:** none.

Reports the classes that scikit-lego itself points away from since 0.9.8, where building one raises a UserWarning that names the replacement: `QuantileRegression` and `LADRegression`, `RepeatingBasisFunction`, and `DemographicParityClassifier` and `EqualOpportunityClassifier`. It also reports `IdentityTransformer()` without arguments.

Example:

```python
model = QuantileRegression(quantile=0.8)
season = RepeatingBasisFunction(n_periods=12, column="day_of_year")
keep = IdentityTransformer()
```

Use instead:

```python
model = QuantileRegressor(quantile=0.8, alpha=0)
season = ColumnTransformer([("season", SplineTransformer(n_knots=13, extrapolation="periodic"), ["day_of_year"])])
keep = FunctionTransformer()
```

The replacement is advice. xpd does not apply it automatically, because each replacement is
another model with other parameter names. The warnings ask for scikit-learn's `QuantileRegressor`
and `SplineTransformer(extrapolation="periodic")`, and for fairlearn's `ThresholdOptimizer`
around a `LogisticRegression`. On 500 rows `QuantileRegression(quantile=0.8)` and
`QuantileRegressor(quantile=0.8, alpha=0)` gave the same coefficients to three decimals. The
spline is another basis than the radial one, so its columns are other numbers.
`IdentityTransformer()` and `FunctionTransformer()` returned the same array.

### Speed and repeatable runs

#### group-time-series-split-folds

**Severity:** warning. **Fix:** none.

Reports `GroupTimeSeriesSplit` with 6 or more splits. The class tries every combination of split points over the groups to find the most even folds, so each extra fold multiplies the time.

Example:

```python
folds = GroupTimeSeriesSplit(n_splits=6)
```

Use instead:

```python
folds = GroupTimeSeriesSplit(n_splits=5)
```

The replacement is advice. xpd does not apply it automatically. On 3,000 rows in 60 groups,
`split` took 0.09 s at 4 splits, 1.08 s at 5 and 10.48 s at 6. With 40 groups it took 0.21 s at 5
and 1.07 s at 6. The class warns itself above 5 splits from 30 groups on. With a few groups the
search is short, so the rule is a warning.

#### unseeded-random-estimator

**Severity:** info. **Fix:** none.

Reports `RandomRegressor` and `RandomAdder` built without `random_state`, outside test files. Each call then draws new numbers.

Example:

```python
baseline = RandomRegressor(strategy="normal")
```

Use instead:

```python
baseline = RandomRegressor(strategy="normal", random_state=0)
```

The replacement is advice. xpd does not apply it automatically. Two `predict` calls of one fitted
`RandomRegressor()` differed, and so did two `transform` calls of one `RandomAdder()` on its
training rows. Both pairs were equal with `random_state=0`.

### Habits the pack does not report

- `MaximumRelevanceMinimumRedundancy` with the default `kind="auto"` scores a whole-number
  regression target as classes. On one such target it picked the features 1, 2 and 3, and 1, 2
  and 5 with `kind="regression"`. A pattern cannot see the target, and a rule on every call
  without `kind` would report the library's own tests.
- `RegressionOutlierDetector` with a column position on a frame, and `TimeGapSplit`,
  `RandomAdder` and `OutlierRemover` given an input type they do not take, depend on what `fit`
  is given, which a pattern cannot see.
- The extras cvxpy, umap-learn and formulaic have no rule, for the reason at the top of this
  section.
## MAPIE

These rules run on files that import `mapie`, the module of
[MAPIE](https://mapie.readthedocs.io/), which builds prediction intervals and prediction sets
around a scikit-learn model. They report the MAPIE 0.x API, which raises on MAPIE 1, code that
breaks the coverage a conformal method promises, arguments that MAPIE refuses, and code that runs
the model twice. Every number in this section was measured on MAPIE 1.6.0 with scikit-learn 1.9.1.
Examples assume that the MAPIE and scikit-learn names they use are imported. Names such as
`X_train`, `X_conf`, `y_test` and `model` represent values supplied by your program.

MAPIE 1.0 replaced `MapieRegressor` and `MapieClassifier` with one class per method, moved `alpha`
to the constructor as `confidence_level`, which is `1 - alpha`, and split `fit` into `fit` and
`conformalize`. A rule document about that change carries `since`. xpd reads the installed
version from the project's venv, as it does for pandas, and skips the document on an older
release, so a project that still has MAPIE 0.9 gets no report about the 0.x API. When the venv
has no MAPIE, or there is no venv, every document runs.

Two rules read the names of variables, such as `X_train`, because a pattern sees nothing else, so
a name that misleads gives a wrong report. The other rules tie a call to an object by the
variable the file assigns it to, and to rows by the variable given to an earlier call.

### How a rule earns its place

Every rule document ships on something that was run, and it is one of four kinds:

- A coverage proof runs the old and the new form over 30 seeds with `confidence_level=0.9` and
  gives the mean share of test rows that the intervals or sets cover.
- An interpreter proof runs the old form on MAPIE 1.6.0, under `python -W error` for a warning.
  It passes when the old form raises and the new form runs clean.
- A speed proof times both forms on one thread, as the median of 5 or 7 runs, at two data sizes,
  and compares the arrays they return.
- A rerun proof runs the old form twice in fresh objects. It passes when the two results differ
  and the new form gives the same result twice.

A document is `error` only on a coverage proof or an interpreter proof, and its message quotes
what was measured. A document that can also match code that is meant, such as a level below 0.5
in a plot of coverage against the level, is `warning`.

The rules were then run over MAPIE's own code at the 1.6.0 tag: its 162 Python files, which are
the library, its tests and its examples, and the Python code of its 21 documentation pages and
notebooks. Every report was read. A rule that reported correct code there was narrowed, and the
table gives what is left.

| Rule | Severity | Proof | Reports on MAPIE's own code | Wrong advice | Fix |
| --- | --- | --- | --- | --- | --- |
| [`removed-in-mapie`](#removed-in-mapie) | error | interpreter: ImportError, ModuleNotFoundError, TypeError, ValueError, AttributeError | 1 | 0 | unsafe |
| [`deprecated-in-mapie`](#deprecated-in-mapie) | warning | interpreter: FutureWarning, and intervals equal with and without `gamma` | 2 | 0 | safe |
| [`alpha-as-confidence-level`](#alpha-as-confidence-level) | warning, error | coverage: 0.101 against 0.905, and interpreter: ValueError | 0 | no reports | none |
| [`calibrate-on-training-rows`](#calibrate-on-training-rows) | error | coverage: 0.483 against 0.905 | 6 | 1 | none |
| [`coverage-on-seen-rows`](#coverage-on-seen-rows) | error, warning | coverage: 1.000 against 0.914 | 0 | no reports | none |
| [`shuffled-time-series-split`](#shuffled-time-series-split) | warning | coverage: 0.923 against 0.153 | 0 | no reports | none |
| [`prefit-estimator-not-fitted`](#prefit-estimator-not-fitted) | error | interpreter: NotFittedError, AttributeError, ValueError | 0 | no reports | none |
| [`quantile-estimator-loss`](#quantile-estimator-loss) | error | interpreter: ValueError | 0 | no reports | none |
| [`predict-function-returns-labels`](#predict-function-returns-labels) | error | interpreter: IndexError | 0 | no reports | unsafe |
| [`risk-control-method`](#risk-control-method) | error, warning | interpreter: ValueError, UserWarning | 0 | no reports | none |
| [`predict-and-predict-interval`](#predict-and-predict-interval) | warning | speed: 1.97 times | 3 | 3 | none |
| [`refit-per-confidence-level`](#refit-per-confidence-level) | warning | speed: 2.87 times | 1 | 1 | none |
| [`unseeded-resampling`](#unseeded-resampling) | info | rerun: the result differs | 36 | 0 | none |

A coverage proof gives the old form's mean against the new form's mean. A speed proof gives the
lowest ratio over the forms the rule reports and both sizes. Wrong advice counts the reports
where following the message would not fix a real problem in that code.

All 5 reports with wrong advice are in MAPIE's tests. Three tests call `predict` beside
`predict_interval` or `predict_set` to assert that the two agree, one test builds one object per
level to compare it with the list form, and one test conformalizes on the rows of a
`DummyRegressor` that predicts a constant, which learns nothing from them. Four more reports of
`calibrate-on-training-rows` are tests that fit and conformalize on the same rows to check the
mechanics, where coverage is not the point. The sixth is an example in MAPIE's gallery that
conformalizes on its training rows. No rule reported the library's own source, and the examples,
documentation pages and notebooks got 9 reports, none with wrong advice.

Recall was not measured. No set of real MAPIE code was held back, so the table says how often a
rule is wrong when it reports, and not how often it misses. The pack has no rule for
`mapie.exchangeability_testing`, `mapie.experimental` and the FWER procedures: their misuse is in
the data, such as rows shuffled before an exchangeability test, and a pattern cannot see it.

### The 0.x API

Each form below raises or warns on MAPIE 1.6.0. Each was run there, and the messages take each
replacement from the v1 release notes or from the error text. Every fix of a removed form turns
code that raises into code that runs, so none of those fixes can be compared with the old form
and none is safe.

#### removed-in-mapie

**Severity:** error. **Fix:** unsafe: `numpy.typing` for `mapie._typing`, `mapie.metrics.regression`, `mapie.metrics.classification` or `mapie.metrics.calibration` for `mapie.metrics` when every imported name is in that module, `confidence_level=1 - alpha` for `alpha=` on a constructor, on `TimeSeriesRegressor.predict`, on `regression_mwi_score` and on `coverage_width_based` when the value is a number or a name, `fit_params={"sample_weight": w}` for `sample_weight=w`, `update` for `partial_fit`, and one stacked array for the two bounds given to `regression_coverage_score` and `regression_mean_width_score`. The other forms have no fix.

Reports imports of `MapieRegressor`, `MapieClassifier`, `MapieQuantileRegressor`, `MapieTimeSeriesRegressor`, `MapieCalibrator`, `MapieMultiLabelClassifier`, `PrecisionRecallController`, `MondrianCP` and `ConformityScore`, imports of the modules that are gone, such as `mapie.quantile_regression`, `mapie.control_risk` and `mapie._typing`, and metric functions imported from `mapie.metrics` itself. It also reports `alpha=` on the MAPIE 1 constructors, on `predict_interval` and `predict_set`, on `predict` of an object that the file builds from a MAPIE 1 class and on the metrics, `fit` on a `CrossConformalRegressor`, a `CrossConformalClassifier` or a `JackknifeAfterBootstrapRegressor`, `cv=` on the split classes and `cv="prefit"` or `cv="split"` on the cross classes, `method=` on the classifiers and on `TopLabelCalibrator`, `method="naive"`, `sample_weight=` given to `fit` or `fit_conformalize`, `partial_fit` on a `TimeSeriesRegressor`, and the lower and upper bounds given to a metric as two arguments.

Example:

```python
from mapie.regression import MapieRegressor
from mapie.metrics import regression_coverage_score

mapie = MapieRegressor(model, cv="prefit")
mapie.fit(X_conf, y_conf)
y_pred, y_pis = mapie.predict(X_test, alpha=0.1)
coverage = regression_coverage_score(y_test, y_pis[:, 0, 0], y_pis[:, 1, 0])
```

Use instead:

```python
from mapie.regression import SplitConformalRegressor
from mapie.metrics.regression import regression_coverage_score

mapie = SplitConformalRegressor(model, confidence_level=0.9, prefit=True)
mapie.conformalize(X_conf, y_conf)
y_pred, y_pis = mapie.predict_interval(X_test)
coverage = regression_coverage_score(y_test, y_pis)[0]
```

#### deprecated-in-mapie

**Severity:** warning. **Fix:** safe: `BinaryRisk` for `BinaryClassificationRisk`, and `aggregate_point_predictions=` for `aggregate_predictions=` and for `ensemble=` on a `JackknifeAfterBootstrapRegressor`. `update` has no fix.

Reports `BinaryClassificationRisk`, which MAPIE 1.4 renamed, the `aggregate_predictions=` keyword of `CrossConformalRegressor.predict` and `predict_interval` and the `ensemble=` keyword of the same methods on `JackknifeAfterBootstrapRegressor`, which MAPIE 1.4.1 renamed, and `gamma=`, `confidence_level=` and `optimize_beta=` given to `TimeSeriesRegressor.update`, which has read none of them since MAPIE 1.3.

Example:

```python
risk = BinaryClassificationRisk(risk_occurrence=missed, risk_condition=positive, higher_is_better=False)
y_pred, y_pis = cross.predict_interval(X_test, aggregate_predictions="median")
series.update(X_new, y_new, gamma=0.05)
```

Use instead:

```python
risk = BinaryRisk(risk_occurrence=missed, risk_condition=positive, higher_is_better=False)
y_pred, y_pis = cross.predict_interval(X_test, aggregate_point_predictions="median")
series.update(X_new, y_new)
series.adapt_conformal_inference(X_new, y_new, gamma=0.05)
```


`removed-in-mapie` leaves alone an import inside a `try` block that catches the error, code
behind a version check, and code inside `pytest.raises`. Each of its documents carries the
release that removed the form: 1.0 for most, 1.2 for `partial_fit`, 1.3 for
`PrecisionRecallController` and 1.4.1 for `sample_weight=` on `TimeSeriesRegressor.fit`.
`TimeSeriesRegressor` still had `partial_fit` in the 1.1.0 wheel and lost it in 1.2.0. Its `fit`
took `sample_weight` and ran through 1.4.0, and raises TypeError since 1.4.1, and
`fit_params={"sample_weight": w}` raised on 1.0.1 to 1.3.0 and ran from 1.4.0. The `fit` of the
split classes raised on `sample_weight` on 1.0.1 already.

The fix `confidence_level=1 - alpha` is written only for a number or a name. `alpha=[0.05, 0.2]`
was common in 0.x code, and `1 - [0.05, 0.2]` raises TypeError, so a list or another expression
is reported without a fix, and the message says to write 1 minus each value.
`confidence_level=[0.95, 0.8]` gave intervals of shape (3, 2, 2) for 3 rows.

`predict(X, alpha=0.1)` raised TypeError on each of the split, cross, jackknife and quantile
classes. Their level is set in the constructor, so that document has no fix. `fit` raised
AttributeError on `CrossConformalRegressor`, `CrossConformalClassifier` and
`JackknifeAfterBootstrapRegressor`, on 1.0.1 and on 1.6.0. It is reported on a name that the file
assigns from one of the three and from nothing else. `CrossConformalizedQuantileRegressor` has a
`fit` on 1.6.0 and is left alone. The rule reports an import and does not rename the class, because
the constructors differ: `MapieRegressor(model, cv="prefit")` becomes
`SplitConformalRegressor(model, prefit=True)`, and `alpha` moves from `predict` to the
constructor. Two fixes write `np.` and apply only in a file with `import numpy as np`. On the
same bounds, the fixed calls of `regression_coverage_score`, `regression_mean_width_score` and
`coverage_width_based` on MAPIE 1.6.0 returned the value that the old calls returned on MAPIE
0.9.2, bit for bit, and `regression_mwi_score` returned it to 15 digits.

The fixes of `deprecated-in-mapie` are safe: a controller built with `BinaryRisk` picked the same
thresholds and gave the same predictions as one built with `BinaryClassificationRisk`, and the
intervals with the new keywords were bit equal to the ones with the old keywords. A file that
imports both risk classes ends with `BinaryRisk` twice in its import. `ensemble=` is reported only
on an object that the file assigns from `JackknifeAfterBootstrapRegressor` and from no other
class, because `TimeSeriesRegressor.predict` still takes it. With `method="aci"`, the intervals
after `update(X, y, gamma=0.5)` were bit equal to the ones after `update(X, y)` on MAPIE 1.3.0,
1.4.0 and 1.6.0, and they differed on MAPIE 1.2.0, so that document carries `since: "1.3"`.

### Coverage that does not hold

A conformal method promises that its intervals cover a share of new rows. The promise needs
conformity scores from rows the model did not learn from, a level that is the share to cover, and
test rows that the method has not seen. Each number below is the mean share of test rows covered
over 30 seeds, with a `RandomForestRegressor` unless another model is named.

#### alpha-as-confidence-level

**Severity:** warning for a level below 0.5 and for a variable named `alpha` or `delta`, and error for a percent. **Fix:** none.

Reports `confidence_level=0.1`, `confidence_level=[0.05, 0.1]` and `confidence_level=alpha`, where the value is the share to miss, the same values given to `regression_mwi_score` and `coverage_width_based` in the place of the level, and `confidence_level=90`.

Example:

```python
alpha = 0.1
mapie = SplitConformalRegressor(model, confidence_level=alpha)
score = regression_mwi_score(y_test, y_pis, alpha)
```

Use instead:

```python
alpha = 0.1
mapie = SplitConformalRegressor(model, confidence_level=1 - alpha)
score = regression_mwi_score(y_test, y_pis, 1 - alpha)
```

The replacement is advice. xpd does not apply it automatically.

#### calibrate-on-training-rows

**Severity:** error. **Fix:** none.

Reports `conformalize` on the training rows, by the name of its first argument or because an earlier `fit` got the same variable, `calibrate` of a risk controller on the training rows, and `fit` of a `TopLabelCalibrator` or `VennAbersCalibrator` with `cv="prefit"` on the training rows.

Example:

```python
model.fit(X_train, y_train)
mapie = SplitConformalRegressor(model, confidence_level=0.9, prefit=True)
mapie.conformalize(X_train, y_train)
```

Use instead:

```python
X_train, X_conf, X_test, y_train, y_conf, y_test = train_conformalize_test_split(
    X, y, train_size=0.6, conformalize_size=0.2, test_size=0.2, random_state=0
)
model.fit(X_train, y_train)
mapie = SplitConformalRegressor(model, confidence_level=0.9, prefit=True)
mapie.conformalize(X_conf, y_conf)
```

The replacement is advice. xpd does not apply it automatically.

#### coverage-on-seen-rows

**Severity:** error for rows given to `fit` or `fit_conformalize`, and warning for rows given to `conformalize`. **Fix:** none.

Reports `predict_interval` and `predict_set` on the variable that an earlier statement gave to `fit`, `fit_conformalize` or `conformalize` of the same object, where the result goes into a MAPIE coverage metric.

Example:

```python
cross.fit_conformalize(X_train, y_train)
y_pred, y_pis = cross.predict_interval(X_train)
coverage = regression_coverage_score(y_train, y_pis)[0]
```

Use instead:

```python
cross.fit_conformalize(X_train, y_train)
y_pred, y_pis = cross.predict_interval(X_test)
coverage = regression_coverage_score(y_test, y_pis)[0]
```

The replacement is advice. xpd does not apply it automatically.

#### shuffled-time-series-split

**Severity:** warning. **Fix:** none.

Reports `train_test_split` and `train_conformalize_test_split` without `shuffle=False`, where the first result goes into `fit` of a `TimeSeriesRegressor`.

Example:

```python
X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.3, random_state=0)
series = TimeSeriesRegressor(model, cv=blocks)
series.fit(X_train, y_train)
```

Use instead:

```python
X_train, X_test, y_train, y_test = train_test_split(X, y, test_size=0.3, shuffle=False)
series = TimeSeriesRegressor(model, cv=blocks)
series.fit(X_train, y_train)
```

The replacement is advice. xpd does not apply it automatically.


`alpha-as-confidence-level` rests on this measurement: with `confidence_level=0.1` the intervals
covered 0.101 of the test rows and were 0.289 wide, against 0.905 and 3.854 with 0.9. MAPIE runs
both without a warning. A literal below 0.5 is read on the constructors of the conformal classes
and on `TimeSeriesRegressor.predict`. A risk controller is left alone, because its level is
`1 - delta` and a low one is a choice. 0.5 itself and a list with a level of 0.5 or more are left
alone. A percent raises "Invalid confidence_level or alpha. Allowed values are between 0 and 1",
but only from `predict_interval` or `predict_set`, after the model is fitted, and a risk
controller raises "delta must be in (0, 1]".

`calibrate-on-training-rows` rests on four measurements. A regressor conformalized on its
training rows covered 0.483 of the test rows against 0.905 with held-out rows, and a
`RandomForestClassifier` covered 0.248 against 0.920. A model fitted on rows that include the
conformalization rows covered 0.478, and a `GridSearchCV` fitted on the conformalization rows
covered 0.819 against 0.902. A `BinaryClassificationController` with a target precision of 0.85
that was calibrated on the training rows gave a precision of 0.666 on test rows and was below the
target in 30 of 30 runs. With held-out rows it found a threshold in 10 of 30 runs, and none of
those was below the target. On test rows a `TopLabelCalibrator` with `cv="prefit"` that was
fitted on the training rows had a top-label calibration error of 0.525, against 0.077 with
held-out rows and 0.119 without calibration, and a `VennAbersCalibrator` had 0.272 against 0.051.
The rule reads the name for `train`, `trn`, `tr`, `fit` and `training`, and leaves alone a name
with `conf`, `calib`, `cal`, `val`, `hold`, `test` or `res` in it. The form without a name needs
an earlier `fit` with a target in the same block or the same chain, so a scaler fitted on the
conformalization rows is left alone, and so is a class that calls `fit` and `conformalize` on
itself. A `fit` is left out when a statement between it and the `conformalize` assigns to the
variable, as in `X, y = load("conformalize")`, because the name then holds other rows. A model picked from six settings by its score on the conformalization rows covered 0.898
against 0.899, so the rule does not report it.

`coverage-on-seen-rows` rests on these: the intervals of a `CrossConformalRegressor` covered
1.000 of the rows given to `fit_conformalize` and 0.914 of the test rows, and the ones of a
`SplitConformalRegressor` covered 1.000 of the rows given to `fit` and 0.905 of the test rows.
The rows given to `conformalize` were covered at 0.903 in each of the 30 runs, with a standard
error of 0.000, because `conformalize` picks the quantile that covers them, so that number shows
nothing and the document is a warning. The rule reports only where the result reaches a coverage
metric: the call is an argument of `regression_coverage_score`, `classification_coverage_score`,
the SSC functions, `regression_mwi_score` or `hsic`, or the second name it is assigned to is. A
prediction for seen rows that is only plotted is left alone, and so is one for a variable that
a statement in between assigns other rows to.

`shuffled-time-series-split` rests on a random walk of 900 rows. A `TimeSeriesRegressor` covered
0.923 of a shuffled test part and 0.153 of a test part that came after the training rows. A
`SplitConformalRegressor` on a shuffled split covered 0.894 of its own test rows and 0.128 of the
300 rows that came after them. The rule cannot see that the rows are in time order, so it is a
warning.

### Arguments that raise

Each form below raises when the object is used, on MAPIE 1.6.0. Code inside `pytest.raises` or
`pytest.warns` is left alone.

#### prefit-estimator-not-fitted

**Severity:** error. **Fix:** none.

Reports `SplitConformalRegressor` and `SplitConformalClassifier` with an estimator built in the argument list and no `prefit=False`, `fit` on such an object without `prefit=False`, and `ConformalizedQuantileRegressor(prefit=True)` with one estimator.

Example:

```python
mapie = SplitConformalRegressor(RandomForestRegressor(random_state=0), confidence_level=0.9)
mapie.fit(X_train, y_train)
```

Use instead:

```python
mapie = SplitConformalRegressor(RandomForestRegressor(random_state=0), confidence_level=0.9, prefit=False)
mapie.fit(X_train, y_train)
```

The replacement is advice. xpd does not apply it automatically.

#### quantile-estimator-loss

**Severity:** error. **Fix:** none.

Reports `ConformalizedQuantileRegressor` and `CrossConformalizedQuantileRegressor` with an estimator built in the argument list that has no quantile loss: `GradientBoostingRegressor` or `HistGradientBoostingRegressor` without `loss="quantile"`, `LGBMRegressor` without `objective="quantile"`, or a scikit-learn regressor that has none, such as `RandomForestRegressor`.

Example:

```python
mapie = ConformalizedQuantileRegressor(GradientBoostingRegressor(random_state=0), confidence_level=0.9)
```

Use instead:

```python
mapie = ConformalizedQuantileRegressor(GradientBoostingRegressor(loss="quantile", random_state=0), confidence_level=0.9)
```

The replacement is advice. xpd does not apply it automatically.

#### predict-function-returns-labels

**Severity:** error. **Fix:** unsafe: `model.predict_proba`.

Reports `BinaryClassificationController` with `model.predict` as its `predict_function`.

Example:

```python
controller = BinaryClassificationController(model.predict, "precision", target_level=0.8)
```

Use instead:

```python
controller = BinaryClassificationController(model.predict_proba, "precision", target_level=0.8)
```

#### risk-control-method

**Severity:** error for a method the risk does not take and for a missing `confidence_level`, and warning for an argument that `"crc"` ignores. **Fix:** none.

Reports `MultiLabelClassificationController` and `SemanticSegmentationController` with `risk="precision"` and `method="crc"` or `"rcps"`, with `method="ltt"` and recall, with `method="rcps"` or `risk="precision"` and no `confidence_level`, and with `method="crc"` beside a `confidence_level` or an `rcps_bound`.

Example:

```python
controller = MultiLabelClassificationController(model.predict_proba, risk="precision", method="crc")
```

Use instead:

```python
controller = MultiLabelClassificationController(model.predict_proba, risk="precision", confidence_level=0.9)
```

The replacement is advice. xpd does not apply it automatically.


`prefit` is `True` by default on the split classes, which says the estimator is already fitted.
With an estimator built in the argument list, `conformalize` raised NotFittedError for a regressor
and "Invalid classifier. Fitted classifier does not contain 'classes_' attribute" for a
classifier, and `fit` raised "The fit method must be skipped when the prefit parameter is set to
True". `prefit-estimator-not-fitted` reads a class name or `make_pipeline` as a new estimator, so
`LinearRegression().fit(X, y)` and a model loaded from a file are left alone. A wrapper around a
fitted model is fitted too: `SplitConformalRegressor(FrozenEstimator(fitted))` ran, so
`FrozenEstimator` is left alone, and so is a class that is given a `.fit(...)` chain. `fit` is reported on
an object that the file assigns from a split class without `prefit=False` and from nothing else.
`ConformalizedQuantileRegressor` has `prefit=False` as its default. With `prefit=True` it needs
three fitted estimators, for the lower quantile, the upper quantile and the median, and one
estimator raised "Estimator for prefit must be an iterable object". That document reads only a
class built in the argument list, with or without a `fit` chained to it, as one estimator. A
call of a function may return the list of three, and
`ConformalizedQuantileRegressor(fit_three(X, y), prefit=True)` ran. The conditional classes
`ConditionalSplitConformalRegressor` and `ConditionalSplitConformalClassifier` raised the same
errors and are read by their `estimator` keyword.

`quantile-estimator-loss` rests on two errors. A boosting model without the quantile loss raised
"You need to set the loss/objective argument of your base model to `quantile`", and a model of
another class raised "The base model is not supported". MAPIE sets the quantile itself, so
`GradientBoostingRegressor(loss="quantile")` and `HistGradientBoostingRegressor(loss="quantile")`
ran as they are. `LGBMRegressor(objective="quantile")` raised "The matching parameter
`alpha_name` for estimator does not exist" until it also got an `alpha`. The rule reads only an
estimator built in the argument list. A `loss` or `objective` that is not written as text, such
as `loss=loss`, may hold `"quantile"` and is left alone.

With `model.predict`, `calibrate` raised IndexError, "Error when calling the predict_function",
because the controller reads the probability of the positive class from the second column.
`MultiLabelClassificationController` ran with `predict`, so `predict-function-returns-labels`
leaves it alone.

`risk-control-method` rests on these errors from `calibrate`: "Invalid method 'crc' for risk
'precision'. Valid methods are: ['ltt']", "Invalid method 'ltt' for risk 'recall'. Valid methods
are: ['crc', 'rcps']", and "Invalid confidence_level. confidence_level cannot be `None` when
using method 'rcps'", and the same for `'ltt'`. With `method="crc"`, a `confidence_level` or an
`rcps_bound` raised a UserWarning that it is ignored.

### Work done twice

#### predict-and-predict-interval

**Severity:** warning. **Fix:** none.

Reports `predict` on the same object and the same rows as a `predict_interval` or `predict_set` in the same block.

Example:

```python
y_pred = mapie.predict(X_test)
_, y_pis = mapie.predict_interval(X_test)
```

Use instead:

```python
y_pred, y_pis = mapie.predict_interval(X_test)
```

The replacement is advice. xpd does not apply it automatically.

#### refit-per-confidence-level

**Severity:** warning. **Fix:** none.

Reports a conformal object built inside a loop or a comprehension with the loop variable as its `confidence_level`.

Example:

```python
for level in (0.8, 0.9, 0.95):
    cross = CrossConformalRegressor(model, confidence_level=level, cv=5, random_state=0)
    cross.fit_conformalize(X_train, y_train)
    intervals[level] = cross.predict_interval(X_test)[1][:, :, 0]
```

Use instead:

```python
levels = [0.8, 0.9, 0.95]
cross = CrossConformalRegressor(model, confidence_level=levels, cv=5, random_state=0)
cross.fit_conformalize(X_train, y_train)
_, y_pis = cross.predict_interval(X_test)
intervals = {level: y_pis[:, :, i] for i, level in enumerate(levels)}
```

The replacement is advice. xpd does not apply it automatically.


`predict_interval` and `predict_set` return the point predictions as their first value. With a
forest of 50 trees on one thread, `predict` and `predict_interval` together took 1.99 times as
long as `predict_interval` alone on 2,000 and on 20,000 rows, and `predict` and `predict_set`
took 1.98 and 1.97 times as long as `predict_set` alone. Both forms returned the same arrays, bit
for bit. The two calls must be statements of the same block and name the same object and the
same rows.

`confidence_level` takes a list, and the intervals then have one column per level. With three
levels, one seeded `CrossConformalRegressor` per level took 2.87 times as long as one for the
list on 1,000 rows and 2.95 times on 5,000, and `SplitConformalRegressor` took 2.99 and 3.01
times. The intervals were bit equal in both. Without `random_state` a cross class draws other
folds for each object, so the per-level intervals then differ from the list form.
`ConformalizedQuantileRegressor` takes one level only, and `refit-per-confidence-level` leaves it
alone.

### Reproducible results

#### unseeded-resampling

**Severity:** info. **Fix:** none.

Reports `CrossConformalRegressor`, `CrossConformalClassifier` and `CrossConformalizedQuantileRegressor` with a number of folds, `JackknifeAfterBootstrapRegressor`, `Subsample`, `BlockBootstrap`, `train_conformalize_test_split`, and `SplitConformalClassifier` with the `"aps"` or `"raps"` score, each without `random_state`.

Example:

```python
cross = CrossConformalRegressor(model, confidence_level=0.9, cv=5)
X_train, X_conf, X_test, y_train, y_conf, y_test = train_conformalize_test_split(
    X, y, train_size=0.6, conformalize_size=0.2, test_size=0.2
)
```

Use instead:

```python
cross = CrossConformalRegressor(model, confidence_level=0.9, cv=5, random_state=0)
X_train, X_conf, X_test, y_train, y_conf, y_test = train_conformalize_test_split(
    X, y, train_size=0.6, conformalize_size=0.2, test_size=0.2, random_state=0
)
```

The value 0 is only the example. Any fixed number makes the run repeatable.

The replacement is advice. xpd does not apply it automatically.


`unseeded-resampling` is info, because a seed does not move the expected coverage. Each form it
names gave another result in two fresh runs and the same result with `random_state`. The cross
classes shuffle their folds when `cv` is a number, which the default is, and gave the same
intervals with `cv=KFold(5)` and with `cv=-1`, so a splitter and a negative number are left
alone. `SplitConformalClassifier` gave the same sets twice with the `"lac"` and `"top_k"` scores
and is reported only with `"aps"` and `"raps"`, which draw a random number for each row. The rule
leaves alone a file that calls `np.random.seed`, after which the same objects gave the same
result twice, a call with `shuffle=False` or `**`, a
`JackknifeAfterBootstrapRegressor` whose `resampling` is a `Subsample`, which is reported itself,
and a call whose result is thrown away. A splitter that shuffles draws its own seed, and
`random_state` of the cross class does not reach it:
`CrossConformalRegressor(model, cv=KFold(5, shuffle=True), random_state=0)` gave other intervals
in two runs, and the same intervals with `KFold(5, shuffle=True, random_state=0)`. This rule does
not report that splitter, because the scikit-learn rule
[`missing-random-state`](#missing-random-state) reports the estimator and a `KFold` with
`shuffle=True` in the same file.
## Category Encoders

These rules run on files that import `category_encoders`. They report code that leaks the
target into the test rows, arguments that give wrong codes without an error, code that raises
on category_encoders 2.11.2, and forms that a release removed or deprecated. Every number in
this section was measured on category_encoders 2.11.2 with scikit-learn 1.9.1, pandas 3.1.1 and
NumPy 2.5.5 on Python 3.13. Examples assume `import category_encoders as ce` and that the
scikit-learn names they use are imported. Names such as `X_train`, `y_test` and `encoder`
represent values supplied by your program.

Three class names exist in scikit-learn as well: `OneHotEncoder`, `OrdinalEncoder` and
`TargetEncoder`. A rule about one of them reports a call only with proof that the class comes
from category_encoders. The proof is the module in front of the name, as in
`ce.OneHotEncoder`, or an import of the name from `category_encoders` in the same file. A file
that imports the same name from two libraries gives no proof, so those rules stay silent there.
The other 20 encoder names belong to category_encoders alone and need no proof.

A rule document whose advice needs a newer release carries `since`. xpd reads the installed
version from the project's venv, as it does for pandas, and skips that document on an older
release. When the venv has no category_encoders, or there is no venv, every document runs.

The scikit-learn pack already reports a target encoder that is fitted before the split, fitted
before cross-validation, or refitted on the test rows, through
[`fit-before-split`](#fit-before-split),
[`cross-val-on-fitted-data`](#cross-val-on-fitted-data) and [`fit-on-test`](#fit-on-test). A
file that uses category_encoders with a split nearly always imports scikit-learn, so this pack
does not repeat those rules. It adds the one form they leave alone, in
[`encode-before-split`](#encode-before-split).

### How a rule earns its place

Every `error` and `warning` document ships on a proof that was run, and the message quotes what
the proof measured. A proof is one of four kinds:

- A score proof runs the old and the new form over 30 seeds on data whose target is drawn apart
  from every feature, where the honest AUC is 0.5. It passes when the old form's mean is more
  than three standard errors above 0.5 and the new form's mean is within 0.02 of it.
- A result proof runs the old form and prints the codes it gives, next to the codes of the new
  form. It passes when the old form gives a code that is wrong for every input of that shape,
  such as -1.0 for every row.
- An interpreter proof runs the old form under `python -W error`. It passes when the old form
  raises an exception or a warning and the new form runs clean. category_encoders 2.11.2 itself
  sets a pandas option that pandas 3.1 deprecates, so that one warning was ignored.
- A speed proof times both forms on one thread, as the median of 7 runs, at 10,000 and 100,000
  rows. It passes when the new form is at least 1.5 times faster at both sizes.

A document is `info` when no proof passed for it, or when its advice depends on something the
code does not show, such as the dtype of a column. `sklearn-ships-encoder` passed its speed
proof and is still `info`, because the two libraries return different things and the swap is a
choice.

The rules were run over two sets of code. The first is the library's own code at its latest
release: the 86 Python files of the repository, and 35 more blocks taken from its docstrings,
its `.rst` pages and its README. The second is 55 files from public repositories that import
`category_encoders`. Four rules were narrowed after these runs, and the table gives the
reports that are left:

- `invalid-handle-option` reported three scikit-learn `OneHotEncoder(handle_unknown="ignore")`
  calls in files that import the name from both libraries, which is why a second import now
  removes the proof.
- `hashing-default-components` reported `HashingEncoder().hashing_trick(df, N=3)`, which passes
  the number of columns itself.
- `zero-regularization` reported `CountTargetEncoder(smoothing=0)` inside `assertRaises`. Code
  that expects the error is now left alone by every rule about a form that raises.
- `sklearn-ships-encoder` reported 6 encoders handed to `PolynomialWrapper` or
  `NestedCVWrapper`, which take only the encoders of this library.

| Rule | Severity | Proof | Reports on the library | Reports on 55 real files | Fix |
| --- | --- | --- | --- | --- | --- |
| [`transform-with-test-target`](#transform-with-test-target) | error, info | score: 0.582 against 0.498 AUC | 4 | 0 | unsafe |
| [`encode-before-split`](#encode-before-split) | error | score: 0.624 against 0.500 AUC | 0 | 0 | none |
| [`ordinal-mapping-pairs`](#ordinal-mapping-pairs) | error | result: every code is -1.0 | 0 | 0 | unsafe |
| [`ordinal-mapping-unknown`](#ordinal-mapping-unknown) | info | result: -1.0 for a category the mapping lacks | 3 | 1 | none |
| [`zero-regularization`](#zero-regularization) | error | result: inf. Interpreter: ValueError | 0 | 0 | none |
| [`hashing-default-components`](#hashing-default-components) | warning | result: 100 values give 8 distinct rows | 10 | 1 | none |
| [`cols-left-to-dtype`](#cols-left-to-dtype) | info | result: 1 column where 5 were meant | 1 | 0 | none |
| [`nan-into-pipeline`](#nan-into-pipeline) | info | interpreter: ValueError at predict | 0 | 0 | none |
| [`supervised-encoder-without-target`](#supervised-encoder-without-target) | error | interpreter: TypeError, ValueError | 0 | 0 | none |
| [`invalid-handle-option`](#invalid-handle-option) | error | interpreter: ValueError, TypeError | 0 | 0 | none |
| [`named-cols-after-array-step`](#named-cols-after-array-step) | error | interpreter: ValueError | 0 | 0 | none |
| [`wrapper-fit-returns-none`](#wrapper-fit-returns-none) | error | interpreter: AttributeError | 0 | 0 | none |
| [`removed-in-category-encoders`](#removed-in-category-encoders) | error | interpreter: TypeError, AttributeError, KeyError | 0 | 0 | unsafe |
| [`deprecated-in-category-encoders`](#deprecated-in-category-encoders) | warning, info | interpreter: FutureWarning. Result: equal frames | 0 | 0 | safe |
| [`sklearn-ships-encoder`](#sklearn-ships-encoder) | info | speed: 1.67 times at the lowest | 61 | 16 | none |
| [`encoder-noise-without-seed`](#encoder-noise-without-seed) | info | result: two runs differ by up to 0.14 | 4 | 0 | none |

A score proof gives the old form's mean against the new form's mean. The speed proof gives the
lowest ratio over the three classes and both sizes.

None of the 83 reports on the library names a line where the rule's claim is false. The advice
still does not apply to most of them, because the library is using or testing its own classes:

- All 61 reports of `sklearn-ships-encoder` are the library's tests, examples and documentation
  building its own `OneHotEncoder`, `OrdinalEncoder` or `TargetEncoder`.
- The 4 reports of `transform-with-test-target` are tests that pass a target to `transform` on
  purpose, to check what the encoder does with it.
- The 10 reports of `hashing-default-components` are calls that do run with 8 columns: 4 in
  tests, 4 in benchmark examples and 2 in the documentation.
- The report of `cols-left-to-dtype` is an example that hands a `TargetEncoder()` two text
  columns, which it encodes.

The 18 reports on the 55 real files all name a category_encoders class. Recall was not
measured, and most rules have no report on the real files, so nothing here says how often the
old forms occur.

### Leakage

A target encoder codes a category by the labels of its rows. When it sees the labels of the
test rows, the code of a test row holds part of its own label, and the score on the test rows is
too high. The score proofs in this group use 600 rows with one text column of 100 levels and a
target drawn apart from it, a random forest of 50 trees, and give the mean test AUC over 30
seeds.

#### transform-with-test-target

**Severity:** error when the encoder is a `LeaveOneOutEncoder`, and info for the other supervised encoders. **Fix:** unsafe: `encoder.transform(X_test)`.

Reports `encoder.transform(X_test, y_test)`. The supervised encoders of category_encoders take the target in `transform`, which no scikit-learn transformer does. It is meant for the training rows.

Example:

```python
encoder = ce.LeaveOneOutEncoder(cols=["city"])
X_train = encoder.fit_transform(X_train, y_train)
X_test = encoder.transform(X_test, y_test)
```

Use instead:

```python
encoder = ce.LeaveOneOutEncoder(cols=["city"])
X_train = encoder.fit_transform(X_train, y_train)
X_test = encoder.transform(X_test)
```

With the target, `LeaveOneOutEncoder` takes each row's own label out of the mean of its
category, so the code of a test row moves with its label. That scored 0.582 AUC on data with no
signal, 7.3 standard errors above 0.5, against 0.498 for `transform(X_test)`.

The other supervised encoders read the target in `transform` only to add the noise of `sigma`
or `randomized=True`. `CatBoostEncoder` scored 0.507 with the test labels, 0.8 standard errors
from 0.5. `TargetEncoder`, `WOEEncoder`, `JamesSteinEncoder`, `MEstimateEncoder`,
`QuantileEncoder` and `CountTargetEncoder` scored the same with and without them. The form is
still not meant for test rows, so it is info for them.

The rule reads the name of the first argument, which must hold `test` or `holdout`, or be a
validation name such as `X_val`. It needs the encoder assigned from a supervised class in the
same function, or in the module for code outside a function. The fix changes the codes of the
test rows of a `LeaveOneOutEncoder` or a `CatBoostEncoder`, and it takes the noise off the test
rows of an encoder with `randomized=True`, so it is unsafe. On a `TargetEncoder` and a
`WOEEncoder` without noise, the fixed line gave an equal frame.

#### encode-before-split

**Severity:** error. **Fix:** none.

Reports `TargetEncoder(...).fit_transform(X, y)` on every row, when the file imports `TargetEncoder` from `category_encoders` and the result is later split by `train_test_split` or passed to `cross_val_score` or another cross-validation function.

Example:

```python
from category_encoders import TargetEncoder

X_encoded = TargetEncoder(cols=["city"]).fit_transform(X, y)
X_train, X_test, y_train, y_test = train_test_split(X_encoded, y, random_state=0)
```

Use instead:

```python
from category_encoders import TargetEncoder

X_train, X_test, y_train, y_test = train_test_split(X, y, random_state=0)
encoder = TargetEncoder(cols=["city"])
X_train = encoder.fit_transform(X_train, y_train)
X_test = encoder.transform(X_test)
```

The replacement is advice. xpd does not apply it automatically.

On data with no signal the old form scored 0.624 AUC, 13.4 standard errors above 0.5, against
0.500 when the split came first. Before `cross_val_score` it scored 0.610 against 0.495 for a
Pipeline that holds the encoder and the model. Fitted on every row before the split,
`LeaveOneOutEncoder`, `CatBoostEncoder`, `WOEEncoder`, `JamesSteinEncoder`, `MEstimateEncoder`
and `CountTargetEncoder` scored between 0.620 and 0.623.

[`fit-before-split`](#fit-before-split) and
[`cross-val-on-fitted-data`](#cross-val-on-fitted-data) of the scikit-learn pack report this
for every step but one: a bare `TargetEncoder(...)` built in the same expression, which they
take for the scikit-learn class, because that class cross-fits in `fit_transform`. The
category_encoders class does not, so this rule reports the same form when the import shows
which class it is. Every other form, such as `ce.TargetEncoder(...)`, an encoder kept in a
name, or another encoder class, is reported by the scikit-learn pack.

### Codes that are wrong without an error

Each old form below runs and gives a number, and the number is not the one the code asks for.

#### ordinal-mapping-pairs

**Severity:** error. **Fix:** unsafe: the list of pairs written as a dict.

Reports an `OrdinalEncoder` mapping whose `"mapping"` entry is a list of `(category, code)` pairs. That was the form before category_encoders 2.0, and old answers and notebooks still show it.

Example:

```python
mapping = [{"col": "size", "mapping": [("small", 1), ("medium", 2), ("large", 3)]}]
encoder = ce.OrdinalEncoder(cols=["size"], mapping=mapping)
```

Use instead:

```python
mapping = [{"col": "size", "mapping": {"small": 1, "medium": 2, "large": 3}}]
encoder = ce.OrdinalEncoder(cols=["size"], mapping=mapping)
```

2.11.2 takes the list of pairs without an error and finds no category in it. With
`[("a", 1), ("b", 2), ("c", 3)]` every row of a column that holds a, b and c was coded -1.0. The
fix changes every code of the column, from -1.0 to the code the pair names, so it is unsafe. The
rule reports the `"mapping"` entry of a dict that also has a `"col"` entry.

#### ordinal-mapping-unknown

**Severity:** info. **Fix:** none.

Reports `OrdinalEncoder(mapping=...)` without `handle_unknown`. A category that the mapping does not name is then coded -1, with no error and no warning.

Example:

```python
encoder = ce.OrdinalEncoder(cols=["size"], mapping=[{"col": "size", "mapping": {"small": 1, "medium": 2}}])
```

Use instead:

```python
encoder = ce.OrdinalEncoder(
    cols=["size"],
    mapping=[{"col": "size", "mapping": {"small": 1, "medium": 2, "large": 3}}],
    handle_unknown="error",
)
```

The replacement is advice. xpd does not apply it automatically.

With the mapping `{"a": 1, "b": 2}`, the column a, b, c came out as 1.0, 2.0, -1.0. A key with
a typing error, or a category that appears later, reaches the model as -1, which is below
every real code. With `handle_unknown="error"` the same fit raised `ValueError: Unexpected
categories found in column c`. Whether the mapping names every category cannot be read from
the code, so the rule is info. A call that states `handle_unknown`, with any value, is left
alone.

#### zero-regularization

**Severity:** error. **Fix:** none.

Reports `WOEEncoder(regularization=0)` and `CountTargetEncoder(smoothing=0)`.

Example:

```python
encoder = ce.WOEEncoder(cols=["city"], regularization=0)
```

Use instead:

```python
encoder = ce.WOEEncoder(cols=["city"], regularization=0.5)
```

The replacement is advice. xpd does not apply it automatically.

`WOEEncoder` adds `regularization`, 1.0 by default, to the counts of both classes before it
takes the logarithm of their ratio. With 0, a category that holds one class alone divides by
zero. A category of five rows of class 1 was coded `inf`, with no error and no warning, where
the default gave 1.204. On a column where every category held both classes the codes were
finite. `CountTargetEncoder(smoothing=0)` raised `ValueError: smoothing must be strictly bigger
than 0.` at fit. That document carries `since: "2.11"`, the release that added the class.

`TargetEncoder(smoothing=0)` and `MEstimateEncoder(m=0)` gave the plain mean of each category
and are left alone.

#### hashing-default-components

**Severity:** warning. **Fix:** none.

Reports `HashingEncoder(...)` without `n_components`. The encoder hashes every value into `n_components` columns, 8 by default, and all encoded columns share them.

Example:

```python
encoder = ce.HashingEncoder(cols=["city", "shop"])
```

Use instead:

```python
encoder = ce.HashingEncoder(cols=["city", "shop"], n_components=1024)
```

The replacement is advice. xpd does not apply it automatically.

| `n_components` | Distinct encoded rows for a column of 100 distinct values |
| --- | --- |
| 8, the default | 8 |
| 64 | 48 |
| 1024 | 95 |

Two columns encoded by one `HashingEncoder()` gave 8 output columns in all, and the rows
`(x, y)` and `(y, x)` got the same code, so the encoder also loses which column a value came
from. The rule has no fix, because the right number depends on how many distinct values the
columns hold. A call that passes the number by position, a call inside `isinstance`, and
`HashingEncoder().hashing_trick(...)`, which takes the number as `N`, are left alone.

Two ideas about this class were measured and dropped. `max_process=1` was 9.81 times faster
than the default at 1,000 rows and 2.2 times slower at 100,000 rows, so no setting is right for
both. `FeatureHasher` of scikit-learn was 5.4 times faster at 10,000 and 100,000 rows, but it
returns a sparse matrix and takes no column names, so it is no drop-in.

#### cols-left-to-dtype

**Severity:** info. **Fix:** none.

Reports an encoder built without `cols` inside a `ColumnTransformer` or `make_column_transformer`. Without `cols`, an encoder encodes the columns of the object, category and string dtypes and passes every other column through.

Example:

```python
columns = make_column_transformer((ce.BinaryEncoder(), ["zip_code"]), remainder="passthrough")
```

Use instead:

```python
columns = make_column_transformer((ce.BinaryEncoder(cols="all"), ["zip_code"]), remainder="passthrough")
```

The replacement is advice. xpd does not apply it automatically.

Inside a `ColumnTransformer` the columns are already chosen, so a numeric column that was
chosen on purpose comes back as it went in. `(ce.OneHotEncoder(), ["zip"])` on a whole-number
column of 5 values gave 1 output column, equal to the input, and printed `Warning: No
categorical columns found` with `print`, where a warning would be raised. With `cols="all"` it
gave 5 columns. A `TargetEncoder()` gave the column back unchanged in the same way.

`cols="all"` was added in category_encoders 2.10, so the rule carries `since: "2.10"`. The rule
cannot see the dtype of the chosen columns, and on text columns the old form is right, so it is
info. For `OneHotEncoder`, `OrdinalEncoder` and `TargetEncoder` the message also names the
scikit-learn class, which encodes every column it is given.

#### nan-into-pipeline

**Severity:** info. **Fix:** none.

Reports an encoder with `handle_unknown="return_nan"` or `handle_missing="return_nan"` inside a `Pipeline` or `make_pipeline` call, and a `CountEncoder(handle_unknown=None)` there.

Example:

```python
model = make_pipeline(ce.OneHotEncoder(cols=["city"], handle_unknown="return_nan"), LogisticRegression())
```

Use instead:

```python
model = make_pipeline(ce.OneHotEncoder(cols=["city"], handle_unknown="value"), LogisticRegression())
```

The replacement is advice. xpd does not apply it automatically.

`return_nan` codes a category that fit did not see as NaN, in every output column of that row.
In a Pipeline the NaN goes to the next step. With a `OneHotEncoder` or a `TargetEncoder` in
front of a `LogisticRegression`, `predict` on a row with an unseen city raised `ValueError:
Input X contains NaN`, and so did an `OrdinalEncoder` with `handle_missing="return_nan"` on a
row with a missing city. The fit passed, because the training rows had no
unseen category, so the error first shows on new data. With `"value"`, the default, the same
`predict` ran.

| Encoder | Code of an unseen category with `"value"` | With `"return_nan"` |
| --- | --- | --- |
| `OrdinalEncoder` | -1.0 | NaN |
| `OneHotEncoder` | 0 in every column | NaN in every column |
| `BinaryEncoder` | 0 in every column | NaN in every column |
| `TargetEncoder` | the mean of the target, 0.5 here | NaN |
| `CountEncoder` | 0.0 | NaN |

The rule stays silent when the same pipeline call holds an imputer or a model that takes NaN,
such as `HistGradientBoostingClassifier`. Outside a pipeline call the option is left alone,
because the code may handle the NaN itself.

### Code that raises

Each old form below raises on category_encoders 2.11.2. Code inside `assertRaises`,
`pytest.raises` or a `try` block with an `except` clause expects the error and stays silent.

#### supervised-encoder-without-target

**Severity:** error. **Fix:** none.

Reports `fit(X)` and `fit_transform(X)` without the target on a supervised encoder.

Example:

```python
X_encoded = ce.TargetEncoder(cols=["city"]).fit_transform(X)
```

Use instead:

```python
X_encoded = ce.TargetEncoder(cols=["city"]).fit_transform(X, y)
```

The replacement is advice. xpd does not apply it automatically.

Ten encoders learn from the target: `CatBoostEncoder`, `CountTargetEncoder`, `GLMMEncoder`,
`JamesSteinEncoder`, `LeaveOneOutEncoder`, `MEstimateEncoder`, `QuantileEncoder`,
`SummaryEncoder`, `TargetEncoder` and `WOEEncoder`. On each of them `fit_transform(X)` raised
`TypeError: fit_transform() missing argument: y`. `fit(X)` raised `ValueError: Supervised
encoders need a target for the fitting. The target cannot be None` on nine of them, and
`TypeError` on `SummaryEncoder`, whose `fit` has no default for `y`. A scikit-learn transformer
takes `fit(X)`, so the form is easy to carry over from an `OrdinalEncoder` or a
`OneHotEncoder`.

The rule reports the call on an encoder built in the same expression, and on a name that is
assigned from a supervised class in the same function and from nothing else there.

#### invalid-handle-option

**Severity:** error. **Fix:** none.

Reports a `handle_unknown` or `handle_missing` string that the encoder does not take, such as `handle_unknown="ignore"`, which belongs to the scikit-learn `OneHotEncoder`.

Example:

```python
encoder = ce.OneHotEncoder(cols=["city"], handle_unknown="ignore")
```

Use instead:

```python
encoder = ce.OneHotEncoder(cols=["city"], handle_unknown="value")
```

The replacement is advice. xpd does not apply it automatically.

| Encoders | `handle_unknown` | `handle_missing` |
| --- | --- | --- |
| The ten supervised encoders, and `OrdinalEncoder` | `error`, `return_nan`, `value` | `error`, `return_nan`, `value` |
| `BackwardDifferenceEncoder`, `BaseNEncoder`, `BinaryEncoder`, `GrayEncoder`, `HelmertEncoder`, `PolynomialEncoder`, `SumEncoder` | the three, and `indicator` | the three, and `indicator` |
| `OneHotEncoder`, `MultiHotEncoder` | the three, and `indicator` | the three, `indicator` and `ignore` |
| `RankHotEncoder` | the three | the three, and `ignore` |
| `HashingEncoder` | not a parameter | not a parameter |
| `CountEncoder` | not checked | not checked |

The table was read from 2.11.2 by fitting every class with every string. Since 2.10 an unknown
string raises at fit, as in `ValueError: Unexpected handle_unknown value 'ignore' for
OneHotEncoder. Supported values: ['error', 'indicator', 'return_nan', 'value']`. Before 2.10 the
string was taken in silence and did not act as the default. On 2.9.0, `handle_unknown="ignore"`
coded an unseen category as NaN on `OneHotEncoder`, `OrdinalEncoder`, `BinaryEncoder`,
`TargetEncoder` and `WOEEncoder`. The form is wrong on both sides of 2.10, so the rule runs on
every version.

`"value"` on a category_encoders `OneHotEncoder` codes an unseen category as zeros, which is
what `"ignore"` does in scikit-learn. `HashingEncoder` keeps no list of categories, so it has
neither keyword and raised `TypeError: HashingEncoder.__init__() got an unexpected keyword
argument 'handle_unknown'`. A value that is not a string, such as a number or a function, is
allowed since 2.11 and is left alone, and so is `CountEncoder`, which checks nothing.

#### named-cols-after-array-step

**Severity:** error. **Fix:** none.

Reports an encoder with names in `cols` that comes after an imputer, a scaler or a `ColumnTransformer` in a pipeline. Those steps hand the next step a NumPy array, and the names are gone.

Example:

```python
model = make_pipeline(SimpleImputer(strategy="most_frequent"), ce.OneHotEncoder(cols=["city"]))
```

Use instead:

```python
imputer = SimpleImputer(strategy="most_frequent").set_output(transform="pandas")
model = make_pipeline(imputer, ce.OneHotEncoder(cols=["city"]))
```

The replacement is advice. xpd does not apply it automatically.

The old form raised at fit: `ValueError: X does not contain the columns listed in cols:
['city'] The encoder received a non-DataFrame input at fit, where named columns cannot be
recovered positionally`. With `set_output(transform="pandas")` on the imputer the same
pipeline ran, and so did the old form with the column given by position, as in `cols=[0]`.

The rule knows these steps to give an array: `SimpleImputer`, `KNNImputer`, `IterativeImputer`,
`StandardScaler`, `MinMaxScaler`, `MaxAbsScaler`, `RobustScaler`, `Normalizer`,
`PowerTransformer`, `QuantileTransformer`, `KBinsDiscretizer`, `PolynomialFeatures`,
`ColumnTransformer` and `make_column_transformer`. It reads `make_pipeline(...)` and
`Pipeline([...])` calls whose steps are written in the call. It stays silent in a file that
calls `set_output` or sets `transform_output`, anywhere.

#### wrapper-fit-returns-none

**Severity:** error. **Fix:** none.

Reports a method call on the result of `fit` of a `PolynomialWrapper` or a `NestedCVWrapper`, and a name assigned from that `fit`. Their `fit` has no return statement, so it returns `None` where every scikit-learn estimator returns itself.

Example:

```python
encoded = PolynomialWrapper(ce.TargetEncoder(cols=["city"])).fit(X, y).transform(X)
```

Use instead:

```python
wrapper = PolynomialWrapper(ce.TargetEncoder(cols=["city"]))
wrapper.fit(X, y)
encoded = wrapper.transform(X)
```

The replacement is advice. xpd does not apply it automatically.

The old form raised `AttributeError: 'NoneType' object has no attribute 'transform'` on both
wrappers. `fit_transform` returns the encoded frame on both and is left alone.

### Removed and deprecated forms

The release of each removal was found by reading the constructor of every class in the wheels
of 1.3.0, 2.2.2, 2.5.1, 2.6.4, 2.7.0, 2.8.0, 2.8.1, 2.9.0, 2.10.0 and 2.11.2, and each exception
was raised on 2.11.2. Code inside `assertRaises`, `pytest.raises` or a `try` block with an `except`
clause expects the error or handles both releases, and stays silent.

#### removed-in-category-encoders

**Severity:** error. **Fix:** unsafe for the two renamed names in `category_encoders.utils`. The other forms have no fix.

Reports forms that a release of category_encoders removed and that raise on 2.11.2.

Example:

```python
encoder = ce.OneHotEncoder(impute_missing=True)
```

Use instead:

```python
encoder = ce.OneHotEncoder(handle_missing="value")
```

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
row. 2.7 stopped adding it, so a drop by that name raises `KeyError: "['intercept'] not found
in axis"`. The rule reports the drop only in a file that names one of the four classes, and it
leaves a drop with `errors="ignore"` alone. It also leaves alone a drop on a frame that the file
gives a column of that name itself, with `frame["intercept"] = 1.0` or `frame.insert`, and every
drop in a file that calls `assign(intercept=...)`, because such a drop takes out the file's own
column.

`feature_names_out_` was a list until 2.6 and is a NumPy array since 2.7. The result of
`get_feature_names_out` is a NumPy array since 2.6.1. `index`, `remove`, `append`, `extend`,
`insert`, `count` and `pop` raise `AttributeError` on an array. `feature_names_in_` is still a
list.

`get_obj_cols` is still in the wheels of 2.6.0, 2.6.1 and 2.6.2 and went away in 2.6.3, so its
document carries `since: "2.6.3"`.

The two fixes are unsafe. `get_obj_cols` named the columns of the object dtype, and
`get_categorical_cols` also names the columns of the category and string dtypes. A file that
defines its own `get_obj_cols` is left alone.

#### deprecated-in-category-encoders

**Severity:** warning for `get_feature_names`, and info for `max_sample`. **Fix:** safe.

Reports `encoder.get_feature_names()` and `HashingEncoder(max_sample=...)`.

Example:

```python
names = encoder.get_feature_names()
hasher = ce.HashingEncoder(cols=["city"], max_sample=1000, n_components=64)
```

Use instead:

```python
names = encoder.get_feature_names_out()
hasher = ce.HashingEncoder(cols=["city"], n_components=64)
```

`get_feature_names` raised ``FutureWarning: `get_feature_names` is deprecated in all of
sklearn. Use `get_feature_names_out` instead.`` Both methods returned the same NumPy array, so
the fix is safe. The method `get_feature_names_out` was added in 2.6, so the document carries
`since: "2.6"`.

scikit-learn removed its own `get_feature_names`, and
[`removed-in-sklearn`](#removed-in-sklearn) reports every call in a file that imports sklearn.
This rule therefore runs only in a file that does not import it. It needs the receiver built
from a class whose name ends in `Encoder` or `Wrapper`, in the same expression or in an
assignment of the same function.

The changelog of 2.6.4 says of `max_sample` that "it has no use anymore". `HashingEncoder` still
takes the keyword and raises no warning, so the document is info. With `max_sample=10` and
without it the encoded frame was equal, with one process and with two, so the fix that drops the
keyword is safe.

### Encoders that scikit-learn ships

#### sklearn-ships-encoder

**Severity:** info. **Fix:** none.

Reports a category_encoders `OneHotEncoder`, `OrdinalEncoder` or `TargetEncoder` that uses nothing the scikit-learn class of the same name lacks.

Example:

```python
encoder = ce.OneHotEncoder(cols=["city", "shop"])
X_encoded = encoder.fit_transform(X)
```

Use instead:

```python
encoder = make_column_transformer(
    (OneHotEncoder(handle_unknown="ignore", sparse_output=False), ["city", "shop"]),
    remainder="passthrough",
)
X_encoded = encoder.fit_transform(X)
```

The replacement is advice. xpd does not apply it automatically.

`fit_transform` on 4 text columns of 50 levels each, as the median of 7 runs on one thread:

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
[`cols-left-to-dtype`](#cols-left-to-dtype).

One idea was measured and dropped: that a Pipeline with the scikit-learn `TargetEncoder`
scores better on the test rows than one with the category_encoders class. On 2,000 rows with a
target that depends on a column of 300 levels, the category_encoders `TargetEncoder` scored
0.599 AUC and the scikit-learn class 0.544, over 30 seeds with a random forest. The rule
therefore claims the speed and nothing about the score.

### Reproducible results

#### encoder-noise-without-seed

**Severity:** info. **Fix:** none.

Reports an encoder that adds random noise without `random_state`, and a `NestedCVWrapper` that shuffles its folds without it.

Example:

```python
encoder = ce.LeaveOneOutEncoder(cols=["city"], sigma=0.05)
```

Use instead:

```python
encoder = ce.LeaveOneOutEncoder(cols=["city"], sigma=0.05, random_state=0)
```

The replacement is advice. xpd does not apply it automatically.

Two `fit_transform` calls on the same 400 rows, and the largest gap between their codes:

| Form | Largest gap between two runs |
| --- | --- |
| `WOEEncoder(randomized=True)` | 0.07 |
| `JamesSteinEncoder(randomized=True)` | 0.11 |
| `MEstimateEncoder(randomized=True)` | 0.13 |
| `LeaveOneOutEncoder(sigma=0.05)` | 0.14 |
| `CatBoostEncoder(sigma=0.05)` | 0.12 |
| `NestedCVWrapper(TargetEncoder())` | 0.07 |

With `random_state=0` the two calls were equal in each row of the table, and a
`NestedCVWrapper` with `shuffle=False` was equal too. A seed does not move the expected score,
so the rule is info, as [`missing-random-state`](#missing-random-state) of the scikit-learn
pack is. A `NestedCVWrapper` given a splitter as `cv` is left alone, because the splitter
brings its own seed. A file that calls `np.random.seed` is left alone, because the encoders
then draw from that seed.

### What the pack leaves out

These habits were looked at and have no rule.

- A target of more than two classes given to `TargetEncoder` as whole numbers is coded as the
  mean of the class numbers, which means nothing. `PolynomialWrapper` is the answer, but a
  pattern cannot see how many classes a target has. `WOEEncoder` raises `ValueError: The target
  column y must be binary` in the same case.
- `encoder.fit(X_train, y_train)` followed by `encoder.transform(X_train)` gives the training
  rows of a `LeaveOneOutEncoder` the plain mean of their category. With a random forest that
  scored 0.599 AUC on the test rows against 0.541 with the target passed, so the form the
  documentation asks for was the worse one here, and no rule reports the other.
- `LeaveOneOutEncoder()` without `sigma` scored 0.541 against 0.551 with `sigma=0.05` in a
  forest, and the same in a logistic regression, which is no proof.
- One encoder per column in a loop was 1.03 to 1.16 times slower than one encoder with `cols`.
- `GLMMEncoder` needs statsmodels, which category_encoders stopped installing in 2.11, and
  raises `ImportError` without it. A package check runs for every project that has
  category_encoders and cannot ask whether a file uses that class, so the pack has none.
- `X` and `y` with indexes that do not match raise `ValueError` at fit. A pattern cannot see an
  index.

## arch

These rules run on files that import `arch`, the module of
[arch](https://bashtage.github.io/arch/), which fits ARCH and GARCH volatility models, tests for
unit roots and cointegration, and runs bootstraps and multiple comparison tests. They report API
that arch removed, string options that arch refuses, arguments that it accepts and does not read,
returns in units that the optimizer handles badly, forecasts that raise or that are computed many
times over, and random draws without a seed. Every number in this section was measured on arch
9.0.0 with NumPy 2.5.5, pandas 3.1.1, SciPy 1.18.1 and statsmodels 0.15.0, unless another release
is named. Examples assume that the arch names they use are imported. Names such as `returns`,
`prices`, `losses` and `x` represent values supplied by your program.

arch 8.0 removed the `RandomState` paths of the bootstraps and the distributions, and arch 9.0
removed `arch.__future__` and the `type` keyword of `hedgehog_plot`, made some attributes read
only and started to check every argument. A rule document about such a change carries `since`.
xpd reads the installed version from the project's venv, as it does for pandas, and skips the
document on an older release, so a project that still has arch 7.2 gets no report about what 8.0
removed. When the venv has no arch, or there is no venv, every document runs.

A pattern has no types, so each rule ties its report to a name that the file binds to arch. A
call is read by the name of its class or function, such as `arch_model` or `IIDBootstrap`. A
short name that another library may have, such as `SPA`, `MCS` or `Normal`, is read only when the
file imports it from arch. A method such as `fit`, `forecast` or `conf_int` is read only on a
model, a result or a bootstrap that the file builds from an arch name and assigns to a variable,
and the rule is silent when the same variable is assigned anything else anywhere in the file. An
option or a value is read only when it is written in the call. A variable in its place is never
taken for the bad value.

### Removed API

Each form below raises on arch 9.0.0. Each was run on the release before the removal and on the
release that made it, both installed from PyPI, because the release notes of 8.0 do not list what
8.0 removed. Every fix of a removed form turns code that raises into code that runs, so none of
those fixes can be compared with the old form and none is safe.

#### removed-in-arch

**Severity:** error. **Fix:** unsafe: `seed=` for `random_state=` on a bootstrap or a distribution, `plot_type=` for `type=` on `hedgehog_plot` of a result that the file gets from an arch model, and `from arch.utility.cov import cov_nw` for the import from `arch.utility` when `cov_nw` is the only name. The other forms have no fix.

Reports `from arch.__future__ import reindexing`, `type=` given to `hedgehog_plot`, `random_state=` given to `IIDBootstrap`, `IndependentSamplesBootstrap`, `StationaryBootstrap`, `CircularBlockBootstrap`, `MovingBlockBootstrap` or `LocalBlockBootstrap`, `random_state=` or a positional argument given to `Normal`, `StudentsT`, `SkewStudent` or `GeneralizedError`, the methods `seed`, `get_state` and `set_state` and the property `random_state` on a bootstrap, `seed` on an `SPA`, `StepM`, `MCS` or `RealityCheck`, `cov_nw` imported from `arch.utility`, an assignment to `block_size` of a block bootstrap or to `bootstrap` of a multiple comparison test, and `nested=True` with `studentize=False` on `SPA`, `StepM` or `RealityCheck`.

Example:

```python
from arch.__future__ import reindexing

bs = IIDBootstrap(x, random_state=np.random.RandomState(0))
bs.seed(1234)
dist = Normal(random_state=np.random.RandomState(0))
fig = res.hedgehog_plot(type="mean")
```

Use instead:

```python
bs = IIDBootstrap(x, seed=1234)
dist = Normal(seed=np.random.default_rng(0))
fig = res.hedgehog_plot(plot_type="mean")
```

The table gives what each form did on the release before the removal and what it does from that
release on.

| Form | Before | From | Now |
| --- | --- | --- | --- |
| `IIDBootstrap(x, random_state=rs)` | 7.2.0: FutureWarning | 8.0.0 | TypeError, because `random_state` is read as data |
| `Normal(random_state=rs)`, `Normal(rs)` | 7.2.0: FutureWarning | 8.0.0 | TypeError |
| `bs.seed(1)`, `bs.get_state()`, `bs.set_state(s)`, `bs.random_state` | 7.2.0: FutureWarning | 8.0.0 | AttributeError |
| `SPA(...).seed(1)`, the same on `StepM` and `MCS` | 7.2.0: FutureWarning | 8.0.0 | AttributeError |
| `from arch.utility import cov_nw` | 7.2.0: runs | 8.0.0 | ImportError |
| `from arch.__future__ import reindexing` | 8.0.0: runs and does nothing | 9.0.0 | ModuleNotFoundError |
| `res.hedgehog_plot(type="mean")` | 8.0.0: FutureWarning | 9.0.0 | TypeError |
| `bs.block_size = 5`, `spa.bootstrap = bs` | 8.0.0: runs | 9.0.0 | AttributeError |
| `SPA(..., studentize=False, nested=True)` | 8.0.0: runs | 9.0.0 | ValueError |

The bootstraps take their data as keyword arguments, so 8.0.0 and 9.0.0 read `random_state` as
one more series and raise because a `RandomState` is not an array. `seed` takes the same
`RandomState`, a `Generator` or an integer, and exists since arch 5.0: on 4.19
`IIDBootstrap(x, seed=0)` raised TypeError. `IIDBootstrap(x, seed=np.random.RandomState(0))` drew
the same samples on 6.1.0, 7.2.0, 8.0.0 and 9.0.0. The state of the generator is read and set
through the `state` property, and `generator` gives the generator itself.

`seed`, `get_state`, `set_state`, `random_state`, `block_size` and `bootstrap` are names that
other objects have too, such as a NumPy `RandomState`. They are reported only on a variable that
the file assigns from one of the arch classes and from nothing else. `hedgehog_plot` is reported
on any object, because no other library that was looked at has a method of that name, and it is
fixed only on a result that the file gets from `fit` or `fix` of an arch model.

The rule leaves alone an import inside a `try` block that catches the error, code behind a
version check, and code inside `pytest.raises`. arch 9.0.0 has no deprecation of its own left:
its source raises no `DeprecationWarning` and no `FutureWarning`, so the pack has no rule for
deprecated API.

### Options and arguments

arch 9.0 checks every argument when a function is called, and it still ignores an argument that
does not apply to the model that was asked for. The first rule reports a string that the check
refuses. The second reports an argument that is accepted and never read, so the code runs and
describes another model or test than the one it estimates.

#### arch-unknown-option

**Severity:** error. **Fix:** none.

Reports a string that is not one of the option's values: `mean`, `vol` and `dist` of `arch_model`, `trend` of `ADF`, `DFGLS`, `KPSS`, `PhillipsPerron`, `VarianceRatio`, `ZivotAndrews`, `engle_granger`, `phillips_ouliaris`, `DynamicOLS`, `FullyModifiedOLS` and `CanonicalCointegratingReg`, `method` of `ADF`, `DFGLS`, `ZivotAndrews`, `engle_granger` and `DynamicOLS`, `test_type` of `PhillipsPerron` and `phillips_ouliaris`, `kernel` of `auto_bandwidth`, `method` of `MCS`, `bootstrap` of `SPA`, `StepM`, `MCS` and `RealityCheck`, `cov_type` and `disp` of `fit`, `method` and `align` of `forecast`, `plot_type` and `method` of `hedgehog_plot`, and `method` and `tail` of `conf_int`.

Example:

```python
gjr = arch_model(returns, vol="gjr", dist="student")
res = gjr.fit(cov_type="mle")
adf = ADF(prices, trend="nc")
```

Use instead:

```python
gjr = arch_model(returns, vol="garch", o=1, dist="t")
res = gjr.fit(cov_type="classic")
adf = ADF(prices, trend="n")
```

arch checks each option against a table of accepted spellings, without regard to case. A script
wrapped that check on arch 9.0.0, called each API once and kept the table it was handed, so the
lists in the rule are the ones the installed release uses and none is typed by hand. The script
then ran each API with every spelling of its table, in lower and in upper case, and with 28 other
strings, and each of those raised ValueError. The tables hold more than the signatures show:
`vol="constant"` builds a model with a constant variance, `conf_int` also takes `"var"`, `"cov"`,
`"debiased"` and `"bias-corrected"`, `method` of the unit root tests also takes `"t"` and
`"tstat"`, and `bootstrap="moving_block"` is the same as `"moving block"`.

| Option | Values on arch 9.0.0 |
| --- | --- |
| `arch_model(mean=)` | `constant`, `zero`, `ls`, `ar`, `arx`, `har`, `harx` |
| `arch_model(vol=)` | `garch`, `arch`, `egarch`, `figarch`, `fiaparch`, `aparch`, `harch`, `ngarch`, `constant` |
| `arch_model(dist=)` | `normal`, `gaussian`, `t`, `studentst`, `skewstudent`, `skewt`, `ged`, `generalized error` |
| `trend=` of `ADF`, `engle_granger`, `phillips_ouliaris`, `DynamicOLS`, `FullyModifiedOLS`, `CanonicalCointegratingReg` | `n`, `c`, `ct`, `ctt` |
| `trend=` of `DFGLS`, `KPSS` | `c`, `ct` |
| `trend=` of `PhillipsPerron` | `n`, `c`, `ct` |
| `trend=` of `VarianceRatio` | `n`, `c` |
| `trend=` of `ZivotAndrews` | `c`, `t`, `ct` |
| `method=` of `ADF`, `DFGLS`, `ZivotAndrews`, `engle_granger` | `aic`, `bic`, `t-stat`, `maic`, `t`, `tstat` |
| `method=` of `DynamicOLS` | `aic`, `bic`, `hqic` |
| `PhillipsPerron(test_type=)` | `tau`, `rho` |
| `phillips_ouliaris(test_type=)` | `za`, `zt`, `pu`, `pz` |
| `auto_bandwidth(kernel=)` | `ba`, `bartlett`, `nw`, `pa`, `parzen`, `gallant`, `qs`, `andrews` |
| `MCS(method=)` | `r`, `max` |
| `bootstrap=` of `SPA`, `StepM`, `MCS`, `RealityCheck` | `stationary`, `sb`, `circular`, `cbb`, `moving block`, `mbb` |
| `fit(cov_type=)` | `robust`, `classic` |
| `fit(disp=)` | `off`, `final`, or a bool |
| `forecast(method=)`, `hedgehog_plot(method=)` | `analytic`, `simulation`, `bootstrap` |
| `forecast(align=)` | `origin`, `target`, `horizon` |
| `hedgehog_plot(plot_type=)` | `volatility`, `mean` |
| `conf_int(method=)` | `basic`, `percentile`, `studentized`, `norm`, `var`, `cov`, `bc`, `debiased`, `bias-corrected`, `bca` |
| `conf_int(tail=)` | `two`, `lower`, `upper` |

Before 9.0 some of these options took any string and fell back to one value without a word. On
arch 4.19, 5.0, 6.1.0, 6.2.0 and 8.0.0, `fit(cov_type="mle")` gave the standard errors of
`"classic"`, 0.00843 for omega on 1,500 daily S&P 500 returns where `"robust"` gave 0.01329, and
`PhillipsPerron(y, test_type="z")` gave the statistic of `"tau"`, -1.136, where `"rho"` gave
-2.0998. On 5.0, 6.1.0, 6.2.0 and 8.0.0, `MCS(losses, 0.1, method="range")` gave the p-values of
`"max"` and not those of `"R"`. The documents have no `since`, so the report also reaches a
project on an older arch, where the value is the wrong one in silence.

Releases before 9.0 took some options in one case only: `ADF(y, method="AIC")` raised on 8.0.0
and runs on 9.0.0. The rule follows 9.0.0 and does not report it. A value that only 9.0 has, such
as `method="maic"` or `align="horizon"`, is not reported either.

The public code had 13 reports: `trend="nc"`, which is the old statsmodels name, `vol="AVGARCH"`,
`disp="on"`, `method="stationary"` on `MCS`, and `ZivotAndrews(y, method="c")`, where the trend
was written as the method.

#### arch-ignored-argument

**Severity:** warning. **Fix:** none.

Reports `lags=` given to `arch_model` with `mean` `"constant"`, `"zero"` or `"ls"`, or with no `mean`, `x` given to `arch_model` with `mean` `"constant"`, `"zero"`, `"ar"` or `"har"`, or with no `mean`, `o=`, `q=` and `power=` given with `vol="arch"` or `vol="harch"`, `power=` given with `vol` `"egarch"`, `"aparch"`, `"fiaparch"` or `"ngarch"`, `o=` given with `vol="figarch"`, `method=` and `max_lags=` given to `ADF`, `DFGLS` or `ZivotAndrews` together with a number for `lags`, `random_state=` given to `forecast` with `method="simulation"`, and `rng=` given to `forecast` with `method="bootstrap"`.

Example:

```python
ar = arch_model(returns, lags=2)
with_regressors = arch_model(returns, x=x, vol="garch", p=1, q=1)
arch3 = arch_model(returns, vol="arch", q=3)
adf = ADF(prices, lags=4, method="bic")
paths = res.forecast(horizon=5, method="simulation", random_state=np.random.RandomState(0))
```

Use instead:

```python
ar = arch_model(returns, mean="AR", lags=2)
with_regressors = arch_model(returns, x=x, mean="ARX", lags=0, vol="garch", p=1, q=1)
arch3 = arch_model(returns, vol="arch", p=3)
adf = ADF(prices, method="bic")
paths = res.forecast(horizon=5, method="simulation", rng=np.random.default_rng(0).standard_normal)
```

Each form was fitted with and without the argument on 1,500 daily S&P 500 returns, on arch 9.0.0
and on 8.0.0, and the two fits had the same parameters. `"fiaparch"` and `"ngarch"` are new in
9.0 and were run there only.

| Call | Compared with | Parameters |
| --- | --- | --- |
| `arch_model(y, lags=2)`, and with `mean="zero"` or `"ls"` | the same call without `lags` | equal |
| `arch_model(y, mean="ar", lags=2)` | without `lags` | differ |
| `arch_model(y, x=x)`, and with `mean="zero"`, `"ar"` or `"har"` | without `x` | equal |
| `arch_model(y, x=x, mean="ls")`, and with `"arx"` or `"harx"` | without `x` | differ |
| `vol="arch"` or `"harch"` with `o=1`, `q=2` or `power=1.0` | `o=0`, `q=1`, `power=2.0` | equal |
| `vol="egarch"`, `"aparch"`, `"fiaparch"` or `"ngarch"` with `power=1.0` | `power=2.0` | equal |
| `vol="figarch"` with `o=1` | `o=0` | equal |
| `vol="garch"` with `o=1`, `q=2` or `power=1.0` | `o=0`, `q=1`, `power=2.0` | differ |

`ADF`, `DFGLS` and `ZivotAndrews` search for a lag length only when `lags` is not given. With
`lags=4` the statistic of 1,000 log prices was the same for `method` `"aic"`, `"bic"` and
`"t-stat"`, and the same for `max_lags` 2 and 20. Without `lags`, `"t-stat"` chose 8 lags where
`"aic"` chose 0, and the statistic moved from -1.2674 to -1.0838 for `ADF`.

`forecast` reads `random_state` only for `method="bootstrap"` and `rng` only for
`method="simulation"`. Two calls with `method="simulation"` and
`random_state=np.random.RandomState(0)` gave 2.586 and 2.598 for the variance five steps ahead,
and two calls with a seeded `rng` gave 2.571 twice. Two calls with `method="bootstrap"` and the
same seeded `rng` gave 2.588 and 2.470, and with `random_state=np.random.RandomState(0)` they
gave 2.595 twice. The page on volatility forecasts in arch's own documentation passes
`random_state` to a simulation. A call that passes both seeds is left alone.

A value that changes nothing in a GARCH model either, such as `o=0`, `q=0` or `power=2.0`, is not
reported. `lags=0`, `x=None` and a `mean` or a `vol` that is a variable are not reported. A call
with three or more positional arguments is left alone, because its `mean` may be positional.

The public code had 21 reports and none with wrong advice. Twelve were `x=` or a positional `x`
with a constant mean, seven of them copies of one function that fits
`arch_model(resid, x=exog, mean="Constant", ...)` as if the regressors were in the model. Four
were `vol="ARCH"` with `q=3` or `q=1`, where the order of an ARCH model is `p`. Two were
`random_state=` on a simulation, two were `lags=1` with `mean="constant"`, and one was a `method`
beside `lags=8`. Of the 10 reports on arch's own code, one is a test that passes `max_lags=2.5` to
check that it raises, where the advice is beside the point. The other nine are tests and the
documentation page above.

### The units of the data

arch estimates by maximum likelihood, and the optimizer works badly when the variance of the data
is far from 1. arch warns `DataScaleWarning` when the variance is outside the range it likes, and
its message asks for a value between 1 and 1000. Daily returns as decimals have a variance near
0.0001: the 5,030 daily S&P 500 returns in `arch.data.sp500` have 0.000145.

#### unscaled-returns

**Severity:** warning. **Fix:** none.

Reports a series from `pct_change`, or from `log` with `diff` or `shift`, that is given to `arch_model`, `ConstantMean`, `ZeroMean`, `ARX`, `HARX`, `LS`, `MAMean` or `ARCHInMean` with nothing that multiplies it and no `rescale`.

Example:

```python
returns = prices.pct_change().dropna()
res = arch_model(returns).fit(disp="off")
```

Use instead:

```python
returns = 100 * prices.pct_change().dropna()
res = arch_model(returns).fit(disp="off")
```

A constant mean GARCH(1,1) was fitted to the S&P 500 returns in three forms, on each release.

| Returns | arch | Warnings | Optimizer code | mu | omega |
| --- | --- | --- | --- | --- | --- |
| `100 * price.pct_change()` | 9.0.0, 8.0.0 | none | 0 | 0.0564 | 0.0175 |
| `price.pct_change()` | 9.0.0, 8.0.0 | DataScaleWarning, ConvergenceWarning | 4 | 4.02 | 0.0000029 |
| `np.log(price).diff()` | 9.0.0, 8.0.0 | DataScaleWarning, ConvergenceWarning | 4 | -2.73 | 0.0000029 |
| `price.pct_change()` | 5.0, 6.1.0, 6.2.0 | DataScaleWarning | 0 | -0.0011 | 0.0000029 |
| `price.pct_change()` with `rescale=True` | 9.0.0, 8.0.0 | none | 0 | 0.0564 | 0.0175 |

A mean of 4.02 for returns whose mean is 0.00056 is a failed fit, and on the older releases the
optimizer reported success with a mean of the wrong sign. `rescale=False` turns the warning off
and gave the same failed fit.

The rule reads how the series is built and not its values. It reports the expression in the
call, and a name or a column that every assignment in the file builds that way. A series that is
multiplied anywhere, such as `100 * prices.pct_change()` or `returns = returns * 100`, is left
alone, and so is a call that says `rescale`, in either way. Returns that come from a file or a
function are not seen.

All 23 reports on the public code were returns as decimals: each series was a `pct_change` or a
log difference with no factor.

#### rescaled-units

**Severity:** warning. **Fix:** none.

Reports `rescale=True` on `arch_model` or a mean class in a file that reads `conditional_volatility` or calls `forecast` and never reads `scale`.

Example:

```python
res = arch_model(returns, rescale=True).fit(disp="off")
variance = res.forecast(horizon=1).variance.iloc[-1, 0]
```

Use instead:

```python
res = arch_model(returns, rescale=True).fit(disp="off")
variance = res.forecast(horizon=1).variance.iloc[-1, 0] / res.scale**2
```

With `rescale=True` arch may multiply the data before it estimates, and the result reports the
factor as `scale`. On the S&P 500 returns as decimals, arch 9.0.0 and 8.0.0 chose a scale of
100. The parameters were those of the returns in percent, the last conditional volatility was
1.9926 where the data's was 0.019926, and the variance forecast one step ahead was 3.5965, which
is 10,000 times the variance in the units of the data.

When the data already has a variance near 1 the scale is 1.0 and nothing changes, and results
that have no unit, such as the standardized residuals, are right at any scale. The rule cannot
see either, so it is a warning. Of its 14 reports on the public code, 11 used a volatility or a
forecast in rescaled units, two of them with a division by 10,000 written into the code where
the square of `scale` belongs. Three had wrong advice: two fitted data that was made standard normal
first, and one only divided the residuals by the volatility. The 4 reports on arch's own code
are tests of `rescale` itself.

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

The files in `samples/pandas/`, `samples/numpy/`, `samples/scipy/`, `samples/sklearn/`,
`samples/sklego/`, `samples/mapie/`, `samples/category_encoders/`, `samples/hamilton/` and
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

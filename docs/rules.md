# Rules

Find a rule by its id on this page to see what it reports, its severity and fix, and the code to
write instead. The table gives each of the thirteen rule packs with its number of rules and
package checks in `xpd rules --format json`, and the files that the pack runs on.

| Pack | Rules | Runs on |
| --- | --- | --- |
| [pandas](#pandas) | 124 | files that import pandas |
| [NumPy](#numpy) | 59 | files that import NumPy |
| [SciPy](#scipy) | 30 | files that import SciPy |
| [scikit-learn](#scikit-learn) | 32 | files that import scikit-learn |
| [scikit-lego](#scikit-lego) | 23 | files that import scikit-lego |
| [MAPIE](#mapie) | 13 | files that import MAPIE |
| [Category Encoders](#category-encoders) | 16 | files that import Category Encoders |
| [linearmodels](#linearmodels) | 25 | files that import linearmodels |
| [arch](#arch) | 13 | files that import arch |
| [pymoo](#pymoo) | 23 | files that import pymoo |
| [plotly](#plotly) | 25 | files that import plotly, and files that make plotly the plotting backend of pandas |
| [Hamilton](#hamilton) | 1 | files that import Hamilton |
| [Python](#python) | 12 | every file |

Each rule section has the same parts:

```text
#### <rule id>                 the heading, which the url of the rule points to
**Severity:** ... **Fix:** ...  error, warning or info, and whether xpd can fix it, safe or unsafe
The rule reports ...                    the code that the rule finds, and the code it leaves alone
Example: and Use instead:      the reported code and the code to write instead
notes                          measurements, and what an unsafe fix can change
```

`xpd rules --format json` prints each section as the `doc` field of its rule.

## pandas

The pandas rules find code that is wrong on pandas 3, where copy-on-write is always on, and code
that has a shorter or faster form. Read the notes of a rule before you apply a fix that can change
the result, as the table shows.

| Fix | Result | How to apply it |
| --- | --- | --- |
| safe | the same result | `xpd check --fix` |
| unsafe | can change the result | `xpd check --fix --unsafe-fixes` |
| none, with a replacement under `Use instead:` as advice | can change the result | edit the code by hand |

Examples assume `import pandas as pd` and `import numpy as np`, and names such as `df`,
`output_columns` and `mapping` stand for values in your program. A fix that writes `pd.` or `np.`
applies only in a file that imports pandas as `pd` or NumPy as `np`, and in any other file xpd
reports the code without a fix.

xpd skips a rule that asks for a newer API than the installed pandas has. The pseudocode shows how
xpd finds the installed version.

```text
venv = $VIRTUAL_ENV, or else the nearest .venv in or above the working directory
if venv holds a folder named pandas-<version>.dist-info
  skip each rule in the table that needs a newer pandas than <version>
else
  run every rule
```

| Rule | Runs on |
| --- | --- |
| [`lambda-in-assign`](#lambda-in-assign) | pandas 3.0 and later |
| [`concat-then-category`](#concat-then-category) | pandas 3.1 and later |
| every other pandas rule | every pandas version |

### Code that is wrong or broken on pandas 3

Each rule in this group finds code that gives a wrong result, raises, or warns on pandas 3.0 and
3.1.

#### chained-assignment

**Severity:** error. **Fix:** unsafe: `df.loc[i, "a"] = v`, and `df.loc[df.index[0], "a"] = v` for a write through `.iloc` or `.iat` on a column, which writes every row when the index holds the label twice. Other chains and `update` on a column have no fix.

The rule reports `df["a"][i] = v`, `df[mask]["a"] = v`, `df.loc[mask]["a"] = v`, `df.a[mask] = v`, `df["a"].iloc[0] = v` and `df["a"].update(other)`, which write to a copy, so the frame keeps its old values. It also reports `df[col][mask] = v`, where a variable names the column.

Example:

```python
df["load"][row] = value
```

Use instead:

```python
df.loc[row, "load"] = value
```

The fix is unsafe, because xpd cannot see whether the value on the left is a frame. The table shows
which forms get a fix.

| Reported form | Fix |
| --- | --- |
| `df["a"][i] = v` and `df[col][mask] = v` | `df.loc[i, "a"] = v` and `df.loc[mask, col] = v` |
| `df[mask]["a"] = v` | `df.loc[mask, "a"] = v` |
| `df.loc[mask]["a"] = v` | `df.loc[mask, "a"] = v` |
| any other chain with a text label, such as `df.iloc[0]["a"] = v` | none |
| `df["a"].iloc[0] = v` and `df["a"].iat[0] = v` | `df.loc[df.index[0], "a"] = v` |
| `df.a[mask] = v` and `df["a"].update(other)` | none |

A mask held in a name gets the fix when the enclosing function assigns only masks to the name, such
as a comparison or a call to `isin`. The rule reports `df.a[mask] = v` only when the mask reads the
same frame, so `self.items[i > 3] = v` is left alone. It reports `update` only on a name such as
`df` or `frame`, because `state["seen"].update(extra)` is how code updates a dict of dicts.

The rule leaves alone code that more often fills a list or a dict, as the table shows.

| Left alone | Reason |
| --- | --- |
| a dict, list or set assigned to `x["a"][key]` | the code fills a nested dict |
| two plain keys, as in `rows[0]["name"] = v` | the value is more often a list or a dict |

#### inplace-on-column

**Severity:** error. **Fix:** unsafe: `df["b"] = df["b"].fillna(0)`.

The rule reports `df["b"].fillna(0, inplace=True)`, which changes a copy, so the frame keeps its old values. It also reports `df[column].fillna(0, inplace=True)`, where a variable names the column.

Example:

```python
df["load"].fillna(0, inplace=True)
```

Use instead:

```python
df["load"] = df["load"].fillna(0)
```

The fix is unsafe, because xpd cannot tell a column from a value in a dict. The fix drops comments between the arguments of the call.

#### inplace-result-assigned

**Severity:** error. **Fix:** unsafe: `df = df.dropna()`.

The rule reports `df = df.dropna(inplace=True)`, which sets `df` to None.

Example:

```python
df = df.dropna(inplace=True)
```

Use instead:

```python
df = df.dropna()
```

The fix drops comments between the arguments of the call.

#### no-inplace

**Severity:** info. **Fix:** unsafe: `df = df.dropna()`.

The rule reports any other `inplace=True`, which gives no speedup and keeps the call out of a method chain.

Example:

```python
df.dropna(inplace=True)
```

Use instead:

```python
df = df.dropna()
```

The fix applies only to a call that stands alone as a statement, on a name, an attribute or a subscript. It drops comments between the arguments of the call.

#### epoch-from-int-cast

**Severity:** error. **Fix:** unsafe: `(col - pd.Timestamp("1970-01-01")) // pd.Timedelta("1s")`, with `"1ms"` for `10**6` and `"1us"` for `10**3`. The result changes on purpose, a column with a time zone raises TypeError, because the epoch has none, a missing value gives NaN, and the fix of `asi8` and `.values` returns an Index or a Series where the old code returned a NumPy array.

The rule reports `df["d"].astype("int64") // 10**9`, `index.asi8 // 10**9` and `df["d"].values.astype("float64") / 1e9`, which are off by a factor of 1,000 now that datetimes default to microseconds. It also reports `.view(np.int64)` divided the same way.

Example:

```python
seconds = df["time"].astype("int64") // 10**9
```

Use instead:

```python
seconds = (df["time"] - pd.Timestamp("1970-01-01")) // pd.Timedelta("1s")
```

The replacement assumes dates without a time zone, and for dates with a time zone you subtract `pd.Timestamp("1970-01-01", tz="UTC")` instead. A missing timestamp stays missing.

#### compare-to-missing

**Severity:** error. **Fix:** unsafe: `df["b"].isna()`.

The rule reports `df["b"] == np.nan`, `df["k"].astype(str) == "nan"` and `df[df["b"] == None]`, which are never true. It reports `None` only inside a subscript, because `config["mode"] == None` on a dict is valid Python.

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

The rule reports `2 in df["a"]`, which looks for 2 among the index labels and not among the values. The rule reports only a value before `[` that is named like a frame, which means `df`, `frame`, or a name with one of them as a word. It leaves `key in data["k"]` alone, because such code is more often a test on a nested dict or a string.

Example:

```python
found = 2 in df["load"]
```

Use instead:

```python
found = (df["load"] == 2).any()
```

The fix is unsafe, because xpd also reports `"x" in frame_data["names"]` when `frame_data` holds a dict.

#### mask-precedence

**Severity:** error. **Fix:** unsafe: `(df["a"] > 1) & (df["b"] < 3)`. The fix closes the comparison before each `&` or `|` and opens it again after, so the result changes on purpose.

The rule reports `df["a"] > 1 & df["b"] < 3`, where Python applies `&` before the comparisons, so the code compares `df["a"]` with `1 & df["b"]`.

Example:

```python
selected = df[df["load"] > 1 & df["wind"] < 3]
```

Use instead:

```python
selected = df[(df["load"] > 1) & (df["wind"] < 3)]
```

#### write-through-values

**Severity:** error. **Fix:** none.

The rule reports `df["a"].values[0] = 9`, which raises ValueError, because the array is read-only under copy-on-write.

Example:

```python
df["load"].values[0] = value
```

Use instead:

```python
df.loc[df.index[0], "load"] = value
```

xpd gives the replacement as advice and doesn't apply it.

#### object-dtype-check

**Severity:** error. **Fix:** unsafe: `pd.api.types.is_string_dtype(df["k"])` for `dtype == object`, `is_object_dtype` and `dtype.name == "object"`, and `["object", "str"]` in `select_dtypes`, which selects the same columns on pandas 3 and raises TypeError on pandas 2. `dtypes == object`, a test with `in`, and any other spelling of object in `select_dtypes`, such as `np.dtype("O")`, have no fix.

The rule reports `df["k"].dtype == object`, `select_dtypes(include="object")`, `is_object_dtype(df["k"])`, `df["k"].dtype in ("object", "category")` and `df["k"].dtype.name == "object"`, which are False for a text column, because pandas 3 stores text in the `str` dtype. It leaves `dtype.kind == "O"` alone, which is still True for a text column.

Example:

```python
is_text = df["name"].dtype == object
```

Use instead:

```python
is_text = pd.api.types.is_string_dtype(df["name"])
```

#### datetime-unit-check

**Severity:** error. **Fix:** unsafe: `pd.api.types.is_datetime64_dtype(df["d"])` for `dtype`, `str(dtype)` and `dtype.name`, `pd.api.types.is_datetime64_any_dtype` for `is_datetime64_ns_dtype`, `select_dtypes("datetime64")`, `(isinstance(df["d"].dtype, pd.DatetimeTZDtype) and str(df["d"].dtype.tz) == "UTC")` for a dtype with a time zone, and `select_dtypes("datetimetz")`, which also selects columns of other zones. Each is True for every unit. `dtypes`, and `str(dtype)` or `dtype.name` with a time zone, have no fix.

The rule reports `df["d"].dtype == "datetime64[ns]"`, `df["d"].dtype == "datetime64[ns, UTC]"`, `select_dtypes("datetime64[ns]")` and `is_datetime64_ns_dtype(df["d"])`. Datetimes default to microseconds in pandas 3, so on pandas 3.1 each comparison was False for a column parsed from text, and `select_dtypes` selected no column.

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

The rule leaves alone the code in the table, where the old comparison can still be right.

| Left alone | Reason |
| --- | --- |
| a file that calls `as_unit("ns")` or `astype("datetime64[ns]")` | the file makes nanosecond columns itself |
| the dtype of a value other than a column taken by a label, as in `df["time"].dtype`, or a name such as `series`, `col` or `index` | the value can be a NumPy array, which keeps the unit it was made with |

A column made from a nanosecond source, such as a NumPy array of `datetime64[ns]`, keeps the nanosecond unit, and the old comparison is still True for it.

#### groupby-apply-reads-key

**Severity:** error. **Fix:** none.

The rule reports `df.groupby("k").apply(lambda g: g["k"])`, which raises KeyError.

Example:

```python
keys = df.groupby("region").apply(lambda group: group["region"].iloc[0])
```

Use instead:

```python
keys = df.groupby("region").apply(lambda group: group.name)
```

`group.name` exists only inside the function passed to `apply`, and with more than one group key it holds a tuple.

xpd gives the replacement as advice and doesn't apply it.

#### removed-in-pandas-3

**Severity:** error. **Fix:** safe: `map`, `"ME"`, `"h"`, `"min"`, a Timedelta unit such as `"min"` for `"T"`, `sep=r"\s+"`, `lineterminator=`, `sep=` for a separator passed by position, `is_monotonic_increasing`, `np.` for `pd.np.` and `pd.json_normalize`. unsafe: `ffill()` and `bfill()`, which label and order the rows of a group-by differently, `"M"` in `resample`, `dt.isocalendar().week`, `writer.close()`, `pd.Series(x).value_counts()`, the alias in a comparison with `infer_freq` or `freqstr`, and these forms, which raise on pandas 3: dropping `delim_whitespace=False`, `verbose`, `infer_datetime_format`, `mangle_dupe_cols`, `squeeze=False`, `encoding` of `to_excel` and `freq` of `pd.Timestamp`, `.squeeze("columns")` for `squeeze=True`, `(s - s.mean()).abs().mean()` for `mad()`, `to_numpy()` for `as_matrix()`, `.loc` for `.ix`, `pd.Index(data, dtype="int64")` for `pd.Int64Index(data)`, `from pandas import json_normalize`, `items()`, `ffill()` for `pad()`, `is_monotonic_increasing` for `is_lexsorted()`, `inclusive=` for `closed="left"` or `"right"`, `ValueError` added to a handler of the pytz errors, `dt.days` and `dt.normalize()` for a cast to a day unit. The other forms have no fix, and the comment above each of them in the rule file says why.

The rule reports `applymap`, `fillna(method=)`, frequencies such as `"M"`, `"H"` and `"T"`, `delim_whitespace=True`, `dt.week`, and the removed arguments and names listed below.

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

The rule also reports the code in the table, and each form raises on pandas 3.1.

| Kind | Reported code |
| --- | --- |
| Arguments of `read_csv` | `error_bad_lines`, `warn_bad_lines`, `squeeze`, `verbose`, `keep_date_col`, `infer_datetime_format`, `mangle_dupe_cols`, `prefix`, `date_parser`, and a separator passed by position. For bad lines, pass `on_bad_lines="skip"`. |
| Writing | `to_csv(line_terminator=)`, a separator passed to `to_csv` by position, `to_excel(encoding=)`, `read_excel(squeeze=)`, and `writer.save()` on a name that holds the word writer |
| Names | `pd.value_counts`, `is_monotonic`, `mad`, `as_matrix`, `ix`, `pd.np`, `pd.datetime`, `pd.Int64Index`, `pd.UInt64Index`, `pd.Float64Index`, `pd.io.json.json_normalize`, `pandas.util.testing`, `SettingWithCopyWarning`, the option `mode.use_inf_as_na`, and `pd.to_numeric(errors="ignore")` |
| Time series | `tshift`, `pad()`, `first("3D")`, `last("3D")`, `resample(loffset=, base=, kind=)`, `date_range(closed=)`, `to_datetime(infer_datetime_format=)`, `to_datetime(errors="ignore")`, `pct_change(fill_method="pad")`, `between_time(include_start=, include_end=)`, `pd.Timestamp(freq=)` and `astype("timedelta64[D]")` |
| Timedelta units | `"T"`, `"L"`, `"U"` and `"N"` in `pd.Timedelta("5T")`, `pd.Timedelta(1, unit="T")`, `pd.to_timedelta(x, unit="T")` and `resample(offset="30T")` |
| Error handlers | a handler for `pytz.exceptions.AmbiguousTimeError` or `NonExistentTimeError`, which pandas 3 no longer raises, also with a pytz zone. pandas 3 raises ValueError for a time that is repeated or missing. |
| Frequency comparisons | an old alias compared with `pd.infer_freq(index)` or `index.freqstr`, as in `pd.infer_freq(index) == "H"`, which is never equal, because pandas returns `"h"` |
| Other methods | `iteritems()`, `is_lexsorted()`, `lexsort_depth` and `groupby(axis=1)` |

The rule fixes a frequency alias by what the call takes, because `"M"`, `"Q"` and `"Y"` stay valid for periods.

| Call | What the rule does |
| --- | --- |
| a call that takes only datetimes, such as `date_range`, `timedelta_range`, `interval_range`, `to_offset`, `Grouper`, `floor`, `snap` and `rolling` | rewrites the alias |
| `resample`, `shift` and `pct_change` | rewrites `"M"`, `"Q"` and `"Y"` with an unsafe fix, and reports `"A"` without a fix, because the new alias is `"YE"` for datetimes and `"Y"` for periods |
| `asfreq` | leaves `"M"` alone, and reports `"A"` without a fix |
| `to_period`, `period_range`, `pd.Period` and `pd.PeriodIndex`, which take periods | rewrites `"H"`, `"T"`, `"S"`, `"L"`, `"U"` and `"N"`, writes `"Y"` for `"A"`, and leaves `"M"`, `"Q"` and `"Y"` alone |

The rule reports a handler for a pytz error only when its `try` block holds a pandas call, such as `tz_localize`, `.dt.`, `pd.` or `floor`. It leaves the handler alone when the handler also names ValueError or Exception, and when the block calls the `localize` of pytz, which still raises the pytz errors.

The fix for an alias compared with `infer_freq` or `freqstr` is unsafe, because the comparison can then be equal. The rule reports `"M"` only in a comparison with `infer_freq`, because the `freqstr` of a monthly PeriodIndex is still `"M"`. The `writer.close()` fix is unsafe, because xpd knows the writer only by its name. `.loc` reads every key as a label where `ix` read an integer as a position, the pytz handler then also catches a ValueError of another cause, and the fix of `squeeze=True` applies only to a call on one line. pandas 3 also removed `DataFrame.lookup`. The rule doesn't report it, because other libraries have a method of the same name.

#### reduction-level-argument

**Severity:** error. **Fix:** unsafe: `df.groupby(level=0).sum()`.

The rule reports `df.sum(level=0)` and the other reductions with `level=`, which raise TypeError. The fix applies when `level=` is the only argument. It is unsafe, because xpd could not run the old form on pandas 3 to compare the two results.

Example:

```python
totals = df["load"].sum(level="region")
```

Use instead:

```python
totals = df["load"].groupby(level="region").sum()
```

#### stack-dropna

**Severity:** error, or warning for `future_stack=False`. **Fix:** unsafe: `stack()` for `stack(future_stack=False)`, which keeps the rows that are all missing. `dropna=` has no fix, because the replacement depends on its value.

The rule reports `df.stack(dropna=...)` with either value, which raises ValueError, and `df.stack(future_stack=False)`, which is deprecated. `dropna=` has no fix, because the replacement depends on its value.

Example:

```python
long = wide.stack(dropna=True)
```

Use instead:

```python
long = wide.stack().dropna(how="all")
```

For `dropna=False`, write `stack()` alone. The new `stack` keeps the rows that are all missing, which the old default dropped.

#### frame-append

**Severity:** error. **Fix:** none.

The rule reports `df = df.append(row)` and `df.append(row, ignore_index=True)`, which raise AttributeError, because pandas 2.0 removed `DataFrame.append`.

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

The replacement assumes that `parts` is not empty. For single rows, collect the rows in a list and build the frame once.

The rule tells a frame from a list by the way the code uses the result, because `list.append` returns None and takes no keywords. The table shows what it reports.

| Code | Reported |
| --- | --- |
| an `append` whose result is assigned, returned, passed on or read | yes |
| an `append` that passes `ignore_index`, `verify_integrity` or `sort` | yes |
| an `append` on a module named like NumPy, such as `np`, `_np` or `anp` | no |
| `df.index.append(x)` and `df.columns.append(x)` | no, because `Index.append` still exists |
| `labels = labels.append(more)` on an Index held in a plain name | yes, by mistake |

xpd gives the replacement as advice and doesn't apply it.

#### str-replace-regex-default

**Severity:** error. **Fix:** unsafe: `regex=True`.

The rule reports `df["s"].str.replace(r"\d+", "")`, which replaces nothing, because `str.replace` reads its pattern as plain text since pandas 2.0. The rule reports only a pattern that can't be meant as plain text, as the table shows.

| Pattern | Reported |
| --- | --- |
| a class such as `\d`, a set in brackets, `.*` or `.+`, a `^` at the start, or two choices joined by a pipe | yes |
| `"."`, `"$"` and `"("` | no |

Example:

```python
digits = df["phone"].str.replace(r"\D", "")
```

Use instead:

```python
digits = df["phone"].str.replace(r"\D", "", regex=True)
```

The fix is unsafe, because the code can mean a set in brackets as plain text.

#### positional-key-on-labels

**Severity:** error. **Fix:** unsafe: `.iloc[0]`.

The rule reports `df["k"].value_counts()[0]`, `df.iloc[i][0]`, `df.dtypes[0]` and a frame reduction such as `df.isna().sum()[0]`, which raise KeyError, because an integer key on a Series is a label on pandas 3. It also reports a negative key, as in `mean()[-1]`.

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

The fix is unsafe, because the old form still works on labels that are whole numbers. On such labels `value_counts()[0]` is the count of the value 0. The same holds for `df.dtypes[0]` and a frame reduction such as `df.sum()[0]`, which get `.iloc[0]` too.

#### day-frequency-with-time-zone

**Severity:** warning. **Fix:** unsafe: `"24h"` for `"D"` and `"1D"` in `shift` and `pct_change`, and `pd.Timedelta(days=n)` for `pd.offsets.Day(n)`, which keeps 24 hours. Several days, `Day()` with no count, `resample` and `Grouper` have no fix.

The rule reports `ts.shift(freq="D")`, `ts.index + pd.offsets.Day(1)` and `ts.resample("D", offset="6h")` in a file that names a time zone. `"D"` was 24 hours on pandas 2 and is a calendar day on pandas 3. The table shows the result of each version for quarter-hourly data in `Europe/Copenhagen`.

| Code | pandas 2.3 | pandas 3.1 |
| --- | --- | --- |
| `ts.shift(freq="D")` | shifted by 24 hours | raised ValueError when the data held 31 March or 27 October 2024, because a shifted time fell in the daylight saving change |
| `ts.resample("D", offset="6h")` | started each bin at 06:00 | started each bin at 00:00 with only a RuntimeWarning, also on an index in UTC |

pandas 3.1 ignores `origin=` and a frequency of several days, such as `"7D"`, in the same way as `offset=`.

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

xpd can't see whether an index has a time zone, so it reports only in a file that holds `tz_localize`, `tz_convert`, `tz=` or `utc=True`. It misses a frame that got its time zone in another file. On an index without a time zone both versions give the same result. In a file that names a time zone, the rule also reports a PeriodIndex of days by mistake, unless the name of the index holds the word period. On a PeriodIndex the fix `"24h"` raises.

#### mixed-offsets-without-utc

**Severity:** warning. **Fix:** none.

The rule reports `pd.to_datetime(s, format="%Y-%m-%d %H:%M%z")` without `utc=`, which raises ValueError on pandas 3.1 when the text holds more than one offset. Local times across a daylight saving change hold two offsets, such as `+0100` and `+0200`.

| Version, on text with offsets `+0100` and `+0200` | Result |
| --- | --- |
| pandas 2.3 | an object Index and a FutureWarning |
| pandas 3.1 | ValueError |

Example:

```python
times = pd.to_datetime(df["time"], format="%Y-%m-%d %H:%M%z")
```

Use instead:

```python
times = pd.to_datetime(df["time"], format="%Y-%m-%d %H:%M%z", utc=True).dt.tz_convert("Europe/Copenhagen")
```

xpd doesn't apply the replacement, because the zone that `dt.tz_convert` needs is not in the file. On a column that holds one offset, `utc=True` alone changes the result from the offset to UTC.

#### deprecated-in-pandas-3

**Severity:** warning. **Fix:** safe: `pd.Timestamp.now("UTC")`, `pd.Timestamp.fromtimestamp(t, "UTC")`, `"h"` and `"s"` for the Timedelta units `"H"` and `"S"`, drop `copy=`. unsafe: delete the copy-on-write option line, because `pd.get_option` reads the value back, and drop `copy=` from `astype`.

The rule reports `pd.set_option("mode.copy_on_write", True)`, `copy=False`, `pd.Timestamp.utcnow()`, `pd.Timestamp.utcfromtimestamp(t)`, and the units `"H"` and `"S"` in `pd.Timedelta("1H")`, `pd.to_timedelta(x, unit="H")` and `resample(offset="1H")`. Each form warns on pandas 3.1.

Example:

```python
now = pd.Timestamp.utcnow()
```

Use instead:

```python
now = pd.Timestamp.now("UTC")
```

The rule reports `astype(copy=)` only on a column taken by a label or on a value named like a frame, because NumPy's `astype` has the same keyword. Dropping `copy=` from `astype` is unsafe, because on a NumPy array `astype(copy=False)` can return the same array. The rule deletes the copy-on-write option line at the top level of a file and in a block that holds another statement. As the only statement of a block it reports the line without a fix, because the block would be empty.

### Shorter forms, including the pandas 3.0 and 3.1 API

Each rule in this group names a shorter way to write the same code, often with an API that pandas 3.0 or 3.1 added. Most of the rules are warnings, and the rest are info.

#### sort-then-reset-index

**Severity:** info. **Fix:** unsafe: `df.sort_values("b", ignore_index=True)`. `reset_index` always gives a RangeIndex, and `ignore_index=True` keeps the old index when the method has nothing to do, as on an empty frame.

The rule reports `df.sort_values("b").reset_index(drop=True)`, also after `sort_index`, `dropna`, `drop_duplicates`, `explode` and `sample`.

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

The rule reports `df.set_index("d").resample("D").sum()`. The rule reports only a chain that ends in a named aggregation such as `sum` or `mean`, because `asfreq` raises with `on=`. A function passed to `agg` also sees other data with `on=`.

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

The rule reports `df["b"] - df["b"].shift()`. The fix is unsafe, because `diff` wraps around on a column of unsigned integers and returns booleans for a boolean column.

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

The rule reports `(df["a"] >= 10) & (df["a"] <= 20)`. The fix is unsafe, because the value can be a NumPy array, which has no `between`.

Example:

```python
inside = (df["load"] >= 10) & (df["load"] <= 20)
```

Use instead:

```python
inside = df["load"].between(10, 20)
```

The rule leaves comparisons alone on `df.index`, `df.index.hour`, `df.columns`,
`df["load"].values` and `df["load"].array`, because an index or an array has no `between`.

#### groupby-then-reset-index

**Severity:** warning. **Fix:** unsafe: `df.groupby("k", as_index=False)[["a", "b"]].sum()`.

The rule reports `df.groupby("k")[["a", "b"]].sum().reset_index()`. The fix is unsafe, because a key that is not a column comes back in another place.

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

The rule reports `g.agg({"b": "mean"}).rename(columns={"b": "b_mean"})`.

Example:

```python
summary = df.groupby("region").agg({"load": "mean"}).rename(columns={"load": "mean_load"})
```

Use instead:

```python
summary = df.groupby("region").agg(mean_load=("load", "mean"))
```

xpd gives the replacement as advice and doesn't apply it.

#### merge-indicator-anti-join

**Severity:** warning. **Fix:** none.

The rule reports `m[m["_merge"] == "left_only"]` after `merge(..., indicator=True)`.

Example:

```python
merged = left.merge(right, on="key", how="left", indicator=True)
unmatched = merged[merged["_merge"] == "left_only"]
```

Use instead:

```python
unmatched = left.merge(right, on="key", how="left_anti")
```

The anti join returns only the columns of the left frame, without the indicator and without the columns of the right frame. Use it when you need the unmatched rows of the left frame and not every merged column.

xpd gives the replacement as advice and doesn't apply it.

#### value-counts-share

**Severity:** warning. **Fix:** unsafe: `df["k"].value_counts(normalize=True)`.

The rule reports `df["k"].value_counts() / len(df["k"])`. The fix is unsafe, because it names the result `proportion` instead of `count`, and it leaves missing values out of the total.

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

The rule reports `np.where(a < 20, "low", np.where(a < 70, "mid", "high"))`.

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

`case_when` uses the first condition that matches, and it returns a Series instead of a NumPy array.

xpd gives the replacement as advice and doesn't apply it.

#### query-string

**Severity:** info. **Fix:** none.

The rule reports `df.query("a > 50 and b < 0")`, because editors, type checkers and xpd can read the column names in `pd.col("a")` and not in a string. The rule leaves a string that reads like SQL alone, such as `conn.query("SELECT ...")`.

Example:

```python
selected = df.query("load > 50 and wind < 3")
```

Use instead:

```python
selected = df.loc[(pd.col("load") > 50) & (pd.col("wind") < 3)]
```

xpd gives the replacement as advice and doesn't apply it.

#### filter-items

**Severity:** warning. **Fix:** unsafe: `df[cols]`.

The rule reports `df.filter(items=cols)`.

Example:

```python
selected = df.filter(items=output_columns)
```

Use instead:

```python
selected = df[output_columns]
```

The fix is unsafe, because `df[cols]` raises KeyError on a label that is absent, where `filter` skipped the label.

#### inline-column-list

**Severity:** info. **Fix:** none.

The rule reports a list of three or more text literals written inside the call, as in `df[["k", "a", "b"]]` and `df.groupby(["k", "s", "raw"])`, and asks for a named list passed by name. The table shows where the rule looks.

| Place | Reported |
| --- | --- |
| `df[[...]]` and `df.loc[rows, [...]]` | yes |
| an argument of `groupby`, `sort_values`, `set_index`, `select` or `drop_duplicates` | yes |
| `by=`, `subset=`, `on=`, `left_on=`, `right_on=`, `columns=`, `id_vars=`, `value_vars=` or `usecols=` | yes |
| `pd.DataFrame(data, columns=[...])` | no, because the list there defines the frame |

Example:

```python
selected = df[["load", "wind", "solar"]]
```

Use instead:

```python
output_columns = ["load", "wind", "solar"]
selected = df[output_columns]
```

xpd gives the replacement as advice and doesn't apply it.

#### concat-then-category

**Severity:** warning. **Fix:** unsafe: `pd.concat(parts, union_categories=True)`.

The rule reports `pd.concat(parts).astype("category")` on pandas 3.1 and later.

Example:

```python
combined = pd.concat(parts).astype("category")
```

Use instead:

```python
combined = pd.concat(parts, union_categories=True)
```

The fix is unsafe, because the order of the categories differs. Parts that are not categorical also stay as they are.

### Python work per row or per value

Each rule in this group finds code that calls a Python function once per row or once per value,
and names the pandas method that works on the whole column at once. Several rules don't run on
test files, because a test builds its expected value row by row on purpose.

#### apply-known-function

**Severity:** warning. **Fix:** unsafe: `df["s"].str.len()`, which raises on a column that is not text, `df["b"].isna()`, `pd.to_datetime(df["raw"])`, `df["b"].round(2)`.

The rule reports `df["s"].apply(len)`, `df["b"].apply(pd.isna)`, `df["raw"].apply(pd.to_datetime)` and `df["b"].apply(lambda v: round(v, 2))`. It reports only a column taken by a text label, because on a frame `apply` works per column.

Each fix is unsafe, and the table gives the reason.

| Fix | What can differ |
| --- | --- |
| `pd.to_datetime(df["raw"])` | a call per value parses each text on its own, and the whole column must share one format |
| `df["b"].isna()` | on a column of lists, `pd.isna` per value returned an array per cell |
| `df["b"].round(2)` | Python and NumPy round some halfway values differently |
| `df["s"].str.len()` | it raises on a column that is not text |

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

The rule reports `df["s"].str.contains("ta_")` with a pattern that has no regular expression character, where `regex=False` gives the same match faster.

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

The rule reports `df[["a", "b"]].copy()`, `df[mask].copy()` and `df.loc[...].copy()`, because under copy-on-write the selection already behaves as a copy. It leaves a single text label and a slice alone, because on a dict or a NumPy array the copy keeps a later write away from the original.

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

The rule reports `df["k"].apply(lambda key: table.loc[key, "v"])` and `lambda key: table[table["k"] == key]["v"].iloc[0]`, which read another frame once per row. `map` with a Series looks up the whole column at once.

| Form | Rows | `map` with a Series was faster by |
| --- | --- | --- |
| the `.loc` lambda | 100,000 | 50 to 64 times |
| the mask lambda | 20,000 | 777 to 848 times |

Example:

```python
df["price"] = df["item"].apply(lambda item: prices.loc[item, "price"])
```

Use instead:

```python
df["price"] = df["item"].map(prices["price"])
```

The fix is unsafe, because a key that is absent gives a missing value where the lambda raised.

#### apply-recomputes-aggregate

**Severity:** warning. **Fix:** none.

The rule reports `df["b"].apply(lambda v: v - df["b"].mean())`, which computes the mean of the whole column again for every row. The time grows with the square of the row count. On 20,000 rows the lambda took 0.34 seconds and the column form took 0.2 milliseconds. The rule reports a reduction of a column taken by a text label. It leaves `lambda v: v["scores"].mean()` alone, because the lambda reads the row's own value.

Example:

```python
centered = df["load"].apply(lambda value: value - df["load"].mean())
```

Use instead:

```python
centered = df["load"] - df["load"].mean()
```

xpd gives the replacement as advice and doesn't apply it.

#### enumerate-map-encoding

**Severity:** warning. **Fix:** unsafe: `pd.factorize(s)[0]`, and `pd.factorize(s, sort=True)[0]` for the sorted values. factorize returns a NumPy array where the lookup returned a Series, and it gives a missing value the code -1.

The rule reports `df["k"].map({v: i for i, v in enumerate(df["k"].unique())})`, a dict of codes built by hand, which was 7 to 17 times slower than `pd.factorize`.

Example:

```python
codes = df["region"].map({name: code for code, name in enumerate(df["region"].unique())})
```

Use instead:

```python
codes = pd.factorize(df["region"])[0]
```

The table compares the result of `factorize` with the result of the dict. Pass `sort=True` to `factorize` for the order of `sorted`.

| What | The dict | `pd.factorize` |
| --- | --- | --- |
| Order of the codes | the order in which the values first appear | the same |
| Type of the result | Series | NumPy array |
| Code of a missing value | a code of its own | -1 |

#### copy-in-chain

**Severity:** warning. **Fix:** unsafe: drop `.copy()`. Without the copy, `.values` and `.to_numpy()` of the result give an array that is read-only or shares memory with the source.

The rule reports `df.copy().rename(...)` and `df.rename(...).copy()`, also with `assign`, `reset_index`, `set_index`, `astype`, `fillna`, `drop(columns=)` and `pd.concat`, and it reports `drop(name, axis=1).copy()`. Under copy-on-write each of the methods returns a new frame, so the copy does the work twice. Each copy cost 4 to 10 milliseconds on 6 columns, and dropping it was 3 to 118 times faster.

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

The rule leaves a copy next to a slow method such as `sort_values` or `merge` alone, because dropping the copy made the code less than 2 times faster. It leaves `df.copy().pipe(f)` alone, because `f` may write to its argument.

#### row-loop

**Severity:** warning. **Fix:** none.

The rule reports a loop over `iterrows()`, a loop over `itertuples()` or `to_dict("records")` that only computes from the row values, and `for i in range(len(df))` with `df.iloc[i]`. It does not run on test files.

Example:

```python
total = sum(row.load * row.weight for row in df.itertuples())
```

Use instead:

```python
total = (df["load"] * df["weight"]).sum()
```

The replacement sums numeric columns without missing values. When you replace a Python reduction, check the missing values and the floating point rounding. Keep a loop when its work can't be written on columns.

The rule leaves alone a loop over `itertuples()` or `to_dict("records")` that hands the row or its values to a function, as in `orders = [Order(id=row["id"], total=row["total"]) for row in df.to_dict("records")]`, because the loop turns rows into Python objects. A cast such as `int(row["a"])` and a list method such as `out.append(...)` don't count as a function here. The rule reports `iterrows()` in every loop, because `iterrows()` builds a Series for each row. Use `itertuples()` or `to_dict("records")` when each row has to become an object.

xpd gives the replacement as advice and doesn't apply it.

#### apply-axis-1

**Severity:** warning. **Fix:** none.

The rule reports `df.apply(f, axis=1)`, and `agg` and `transform` with `axis=1` and a lambda or a `"_".join`, which call a function once per row. On 100,000 rows the `agg` and `transform` forms were 327 to 2,735 times slower than whole columns. A named method, as in `agg("sum", axis=1)`, runs on whole columns and is left alone.

Example:

```python
total = df.apply(lambda row: row["load"] + row["wind"], axis=1)
```

Use instead:

```python
total = df["load"] + df["wind"]
```

xpd gives the replacement as advice and doesn't apply it.

#### apply-text-method

**Severity:** warning. **Fix:** unsafe: `df["s"].str.startswith("A")`.

The rule reports `df["s"].map(lambda v: v.startswith("A"))`, which calls Python once per value where the `str` accessor works on the whole column. The fix is unsafe, because a column method returns a missing value where the lambda raised. The dtype can also change, as when `dt.year` gives int32 where the lambda gave int64.

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

The rule reports `df["d"].apply(lambda t: t.year)`, which calls Python once per value where the `dt` accessor works on the whole column. The fix is unsafe, because a column method returns a missing value where the lambda raised. The dtype can also change, as when `dt.year` gives int32 where the lambda gave int64.

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

The rule reports `df["a"].apply(lambda v: 1 if v > 50 else 0)` and `lambda v: min(v, 1.0)`. The rule leaves alone a lambda that reduces its argument, as in `x.isnull().all()`, because the lambda gets a whole Series per group.

Example:

```python
flag = df["load"].apply(lambda value: 1 if value > 50 else 0)
```

Use instead:

```python
flag = pd.Series(np.where(df["load"] > 50, 1, 0), index=df.index)
```

`pd.Series(..., index=df.index)` keeps the row labels. Check the name, the dtype and the missing values of the result.

xpd gives the replacement as advice and doesn't apply it.

#### apply-arithmetic

**Severity:** warning. **Fix:** unsafe: `df["b"] * 2 + 1`.

The rule reports `df["b"].apply(lambda v: v * 2 + 1)`, which calls Python once per value where the same arithmetic on the column runs once. The fix is unsafe, because a column method returns a missing value where the lambda raised. The dtype can also change, as when `dt.year` gives int32 where the lambda gave int64.

Example:

```python
scaled = df["load"].apply(lambda value: value * 2 + 1)
```

Use instead:

```python
scaled = df["load"] * 2 + 1
```

#### builtin-on-column

**Severity:** warning. **Fix:** unsafe: `df["a"].sum()`, `df["k"].nunique()`, `df["k"].nunique(dropna=False)` for `len(df["k"].unique())`, `(df["a"] > 50).sum()`, `set(df["k"].unique())`, `df["a"].tolist()`, `df["a"].to_numpy()`, and `df["k"].value_counts()` for `Counter`, which returns a Series sorted by count where `Counter` returned a dict, and leaves out the missing values.

The rule reports `sum(df["a"])`, `len(set(df["k"]))`, `sum(df["a"] > 50)`, `set(df["k"])`, `Counter(df["k"])`, `list(df["a"])`, `np.array(df["a"].tolist())`, `len(df["k"].unique())` and `set(df["k"].values)`. All but `len(df["k"].unique())` were 2.6 to 483 times slower than the column method, and `len(df["k"].unique())` took as long as `nunique(dropna=False)`. For `set` and `list` the rule reports only a value before `[` that is named like a frame, because both are also how code copies a value of a dict. Even so, the rule reports a dict of lists with such a name, as in `list(frame_data["names"])`. The rule doesn't run on test files.

Example:

```python
total = sum(df["load"])
```

Use instead:

```python
total = df["load"].sum()
```

The fix is unsafe, because the column methods skip missing values and the builtins did not.

#### list-over-column

**Severity:** warning. **Fix:** none.

The rule reports `[v.upper() for v in df["s"]]`, and a `for` loop over a column that appends to a list, which was 13 to 19 times slower than the column method. The rule reports only a value before `[` that is named like a frame, which means `df`, `frame`, or a name with one of them as a word. It leaves `x["key"]` in a `for` clause alone, because such a value is more often a list inside a dict. The rule doesn't run on test files.

Example:

```python
names = [name.upper() for name in df["name"]]
```

Use instead:

```python
names = df["name"].str.upper()
```

The result is a Series with row labels instead of a Python list. Call `tolist` only where the next step needs a list.

xpd gives the replacement as advice and doesn't apply it.

#### frame-growth-in-loop

**Severity:** warning. **Fix:** none.

The rule reports `out.loc[len(out)] = row` inside a loop, which grows the frame by one row at a time.

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

xpd gives the replacement as advice and doesn't apply it.

#### len-of-filter

**Severity:** warning. **Fix:** unsafe: `int((df["a"] > 50).sum())`.

The rule reports `len(df[df["a"] > 50])`, which builds a filtered frame only to count its rows. The frame must be a plain name, and the rule doesn't run on test files.

Example:

```python
count = len(df[df["load"] > 50])
```

Use instead:

```python
count = int((df["load"] > 50).sum())
```

#### sort-then-head

**Severity:** warning. **Fix:** unsafe: `df.nsmallest(10, "b")`, `df.nlargest(10, "b")` for `tail(10)`, `df.iloc[df["b"].argmin()]`, `df.iloc[df["b"].argmax()]` for `iloc[-1]`, `df["b"].min()`, `df["b"].max()`, `df["b"].idxmin()`, and `df["b"].min()` and `.max()` for `sorted(df["b"])[0]` and `[-1]`. A sort by more than one column, a frame that is not held in a name and a negative count of `tail` have no fix.

The rule reports `df.sort_values("b").head(10)`, `df.sort_values("b").iloc[0]`, `df["b"].sort_values().iloc[0]`, `df.sort_values("b").tail(10)` and `sorted(df["b"])[0]`, which sort every row to keep a few. It leaves a negative count alone, because `head(-3)` keeps all but the last 3 rows and `nsmallest(-3)` returns no rows.

Example:

```python
largest = df.sort_values("load", ascending=False).head(10)
```

Use instead:

```python
largest = df.nlargest(10, "load")
```

The fix is unsafe, and the table gives the reason for each form.

| Fix | What can differ |
| --- | --- |
| `nsmallest` and `nlargest` | they raise on a text column, and they order ties differently |
| `df.iloc[df["b"].argmin()]` | `argmin` raises when every value is missing, where the sort returned the first row |
| `idxmin` | it can name another row when the lowest value appears twice |
| `max()` for `sort_values().iloc[-1]` | the sort puts missing values last, so the old form returned NaN where `max()` skips them |
| `nlargest` for `tail` | it lists the rows largest first, where `tail` listed them largest last |
| `min()` and `max()` for `sorted` | they return NumPy scalars where `sorted` gave Python numbers, and `sorted` does not order a column with NaN |

#### replace-with-mapping

**Severity:** warning. **Fix:** unsafe: `s.map(lambda v: mapping.get(v, v))`. A key of NaN, text mapped to numbers, and a dict of dicts on a frame give another result, and a list in the name has no `get`.

The rule reports `df["k"].replace(mapping)` with a mapping held in a name, and asks for `map` with `mapping.get(v, v)`. The `map` form keeps the values that the mapping lacks, and it keeps the dtype.

Example:

```python
names = df["name"].replace(mapping)
```

Use instead:

```python
names = df["name"].map(lambda value: mapping.get(value, value))
```

The replacement assumes a plain dict of single values. `replace` can be faster with a short mapping, and `map` doesn't replace by regular expression.

#### concat-in-loop

**Severity:** warning. **Fix:** none.

The rule reports a frame that grows inside a loop, as in `out = pd.concat([out, part])`, because each pass copies every earlier row. The rule leaves a concat with `axis=1` alone, because on pandas 3 it shares the columns it already has.

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

The replacement assumes that `paths` is not empty and that the empty starting frame set no columns or dtypes. One `concat` at the end copies each row once.

The rule leaves alone a `pd.concat` inside a loop that builds a frame from other objects, such as `panel = pd.concat(parts_of_this_month)`, because it copies each row once.

xpd gives the replacement as advice and doesn't apply it.

#### apply-math-function

**Severity:** warning. **Fix:** unsafe: `np.sqrt(df["w"])`, `df["b"].abs()`.

The rule reports `df["w"].map(np.sqrt)`, `df["w"].apply(lambda v: np.log(v))`, `df["w"].apply(math.sqrt)` and `df["b"].apply(abs)`, which call the function once per value. With a bare function name it reports `apply` only on a column taken by a text label, because on a frame `apply` works per column. It leaves `df["w"].apply(np.sqrt)` alone, because `Series.apply` already hands a NumPy function the whole column. `map` calls the function once per value.

Example:

```python
root = df["wind"].map(np.sqrt)
```

Use instead:

```python
root = np.sqrt(df["wind"])
```

The fix is unsafe, because `math.sqrt` and `math.log` raise on a value outside their domain, where NumPy returns NaN.

#### apply-cast

**Severity:** warning. **Fix:** unsafe: `df["b"].astype(int)`.

The rule reports `df["b"].apply(int)`, `df["raw"].map(float)` and `df["b"].apply(lambda v: int(v))`, which cast one value at a time. With a bare function name it reports `apply` only on a column taken by a text label. The rule covers `int` and `float` only, because on floats with missing values `astype(str)` was slower than `apply(str)` and kept the missing values.

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

The rule reports `df["a"].apply(lambda v: v > 50)` and `df["a"].apply(lambda v: v in wanted)`, which compare one value at a time. It leaves a chain such as `0 < v < 5` alone, and a test on text such as `"ta" in v` or `v in "abc"`.

Example:

```python
high = df["load"].apply(lambda value: value > 50)
```

Use instead:

```python
high = df["load"] > 50
```

The `isin` fix is unsafe, because a dict, a range or a custom container can answer `in` differently from `isin`.

#### apply-pd-series

**Severity:** warning. **Fix:** unsafe: `pd.DataFrame(df["pair"].tolist(), index=df["pair"].index)`.

The rule reports `df["pair"].apply(pd.Series)`, which builds one Series per row to split a column of lists or dicts. It reports only a column taken by a text label, because on a frame `apply` works per column.

Example:

```python
parts = df["pair"].apply(pd.Series)
```

Use instead:

```python
parts = pd.DataFrame(df["pair"].tolist(), index=df["pair"].index)
```

The fix is unsafe, because the old form turned whole numbers into floats when a row also held floats, and the fix keeps them whole.

#### index-loop

**Severity:** warning. **Fix:** none.

The rule reports `for label in df.index` with `df.loc[label, "a"]` or `df.at[label, "a"]` in the body, also as a comprehension, which reads the frame one label at a time. It leaves a loop alone that never reads the frame by its label, and it doesn't run on test files.

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

xpd gives the replacement as advice and doesn't apply it.

#### zip-over-columns

**Severity:** warning. **Fix:** none.

The rule reports a loop or comprehension over `zip(df["a"], df["w"])`, which works on one pair of values at a time. It reports only when every argument of `zip` is a column taken by a text label, or a `.values`, `.to_numpy()` or `.tolist()`. It leaves `dict(zip(...))` alone, because it isn't a loop. The rule doesn't run on test files.

Example:

```python
out = [load * weight for load, weight in zip(df["load"], df["weight"])]
```

Use instead:

```python
out = (df["load"] * df["weight"]).tolist()
```

Keep the result as a Series when the next step takes one, and call `tolist` only where the next step needs a list.

xpd gives the replacement as advice and doesn't apply it.

### groupby, agg and transform

Replace a lambda or `apply` in a `groupby` with a built-in aggregation, which pandas runs in compiled code for all groups at once.

#### groupby-lambda

**Severity:** warning. **Fix:** unsafe: `"max"` for min, max, median, count and nunique, `"sum"`, `"mean"`, `g.quantile(0.9)` for a reduction with arguments, and `"sum"` or `"max"` on a resampler for a lambda and for `np.sum`, `np.mean`, `np.min` and `np.max`. The named method can give another dtype, and it raises on a column the lambda could reduce. A count of a condition, and `np.median`, `np.std` and `np.var` on a resampler, have no fix.

The rule reports `g["b"].agg(lambda v: v.max())`, `g.agg(np.sum)`, `pivot_table(aggfunc=np.mean)`, `g.agg(lambda v: v.quantile(0.9))`, `g.agg(lambda v: (v > 0).sum())` and a lambda or NumPy function on `resample`, which run Python once per group. The rule leaves a lambda inside a list alone, because the result column would get a new name. It leaves `np.median`, `np.std` and `np.var` alone, because they treat missing values and degrees of freedom differently from the named methods.

Example:

```python
maximum = df.groupby("region")["load"].agg(lambda group: group.max())
```

Use instead:

```python
maximum = df.groupby("region")["load"].agg("max")
```

The table shows how much faster the replacements were. The count of a condition has no fix.

| Reported code | Write instead | Faster by |
| --- | --- | --- |
| `g.agg(lambda v: v.quantile(0.9))` | `g.quantile(0.9)` | 4.8 times with 1,000 groups, 345 times with 100,000 |
| `g.agg(lambda v: (v > 0).sum())` | `(df["b"] > 0).groupby(df["k"]).sum()` | not measured |
| a lambda on a resampler | the named method | 3.4 to 82 times |

The fixes for `sum`, `mean`, `std`, `var` and `prod` are unsafe, because pandas adds up each group with compensated summation and the lambda didn't. Compensated summation keeps track of the rounding error of each addition, and on the values 1e16, 1, 1 and -1e16 the sum changes from 0 to 2.

#### groupby-lambda-window

**Severity:** warning. **Fix:** unsafe: `g["b"].shift(1)`, `g["b"].cumsum()`. The method can give another dtype or index type on an empty or all-missing group.

The rule reports `g["b"].transform(lambda v: v.shift(1))`, which runs Python once per group where the groupby method runs on all groups at once.

Example:

```python
previous = df.groupby("region")["load"].transform(lambda group: group.shift(1))
```

Use instead:

```python
previous = df.groupby("region")["load"].shift(1)
```

The fixes for `cumsum` and `cumprod` are unsafe, because the groupby methods add up a running total with compensated summation, which keeps track of the rounding error of each addition. A total over values far apart in size can therefore change. The rule leaves `shift(freq=)` alone, because it moves the index.

#### groupby-lambda-arithmetic

**Severity:** warning. **Fix:** unsafe: `df["w"] / df.groupby("k")["w"].transform("sum")`, and `(df["w"] - g.transform("min")) / (g.transform("max") - g.transform("min"))` for min-max scaling of `g = df.groupby("k")["w"]`.

The rule reports `g["w"].transform(lambda v: v / v.sum())`, `v - v.mean()`, `v / v.max()`, the z-score, min-max scaling and `v.fillna(v.mean())`. With 1,000 groups the lambdas `v - v.mean()` and `v / v.max()` were 6 times slower than `transform` with a method name, and with 100,000 groups they were 390 times slower. Min-max scaling on a group-by that does not select one column of a frame by name has no fix.

Example:

```python
share = df.groupby("region")["load"].transform(lambda group: group / group.sum())
```

Use instead:

```python
share = df["load"] / df.groupby("region")["load"].transform("sum")
```

The fix is unsafe, because the sums use compensated summation, which keeps track of the rounding error of each addition. The name of the result can also change.

#### groupby-filter-lambda

**Severity:** warning. **Fix:** unsafe: `df[df.groupby("k")["k"].transform("size") > 10]`.

The rule reports `df.groupby("k").filter(lambda g: len(g) > 10)`, which runs Python once per group to count its rows.

Example:

```python
selected = df.groupby("region").filter(lambda group: len(group) > 10)
```

Use instead:

```python
selected = df[df.groupby("region")["region"].transform("size") > 10]
```

The replacement assumes that no group key is missing. Check how each form treats a missing key before you rewrite a filter.

#### groupby-apply

**Severity:** warning. **Fix:** none.

The rule reports any `groupby(...).apply` with a lambda, which runs Python once per group. The rule leaves alone a lambda that sorts each group, or that builds a dict, a list or a joined text per group.

Example:

```python
totals = df.groupby("region")["load"].apply(lambda group: group.sum())
```

Use instead:

```python
totals = df.groupby("region")["load"].sum()
```

The named reduction can change the name of the index, the dtype and the floating point rounding, so check the result before you replace `apply`.

xpd gives the replacement as advice and doesn't apply it.

#### aggregate-then-merge-back

**Severity:** warning. **Fix:** none.

The rule reports `df.merge(df.groupby("k")["b"].sum()...)` and `df["k"].map(df.groupby("k")["b"].sum())`, where `transform` gives each row the aggregate of its group in one step. The rule reports an aggregation written inside the call on the same frame, and it misses an aggregate stored in a variable.

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

`transform` keeps the index and the order of the rows, where a merge can give the result a new index.

xpd gives the replacement as advice and doesn't apply it.

#### groupby-first-skips-missing

**Severity:** warning. **Fix:** unsafe: `first(skipna=False)` and `last(skipna=False)`, which change the result on purpose to the whole first or last row.

The rule reports `df.groupby("k").first()` and `last()`, which skip missing values per column, so one result row can mix values from different rows. The rule leaves `df.groupby("k")["b"].last()` alone, because one column can't mix rows.

Example:

```python
first = df.groupby("region").first()
```

Use instead:

```python
first = df.groupby("region").first(skipna=False)
```

`skipna=False` keeps the missing values of the first row.

### Tidy data

Each rule in this group finds a longer way to reshape a frame, or a default of a reshaping method
that changes values without a warning. A tidy frame has one row per observation and one column per
variable. The measurements used pandas 3.1.1.

#### column-loop-to-long

**Severity:** info. **Fix:** none.

The rule reports a loop over column names that selects one column at a time, renames it to a shared name and stacks the parts with `pd.concat`. The loop can be a list comprehension or a `for` statement that appends. For 100,000 rows and 20 columns, `melt` gave the same 2,000,000 rows in the same order and ran 1.4 times faster.

Example:

```python
parts = [df[["id", year]].rename(columns={year: "sales"}).assign(year=year) for year in years]
long = pd.concat(parts, ignore_index=True)
```

Use instead:

```python
long = df.melt(id_vars="id", value_vars=years, var_name="year", value_name="sales")
```

xpd gives the replacement as advice and doesn't apply it.

#### stack-then-reset-index

**Severity:** info. **Fix:** none.

The rule reports `df.set_index(keys).stack().reset_index()`, which builds a long frame whose new columns are named `level_1` and `0`. On 100,000 rows with 20 columns, `melt` gave the same values and ran 2.9 times faster. The table shows where the two results differ.

| What differs | `stack().reset_index()` | `melt` |
| --- | --- | --- |
| Names of the new columns | `level_1` and `0` | `variable` and `value`, or the names passed to `melt` |
| Order of the values | row by row | column by column |

Example:

```python
long = df.set_index("id").stack().reset_index()
```

Use instead:

```python
long = df.melt(id_vars="id", var_name="year", value_name="sales")
```

xpd gives the replacement as advice and doesn't apply it.

#### melt-default-names

**Severity:** info. **Fix:** none.

The rule reports `melt` without `var_name`, which names the column of old column names `variable`. The column of their values is named `value` unless the call passes `value_name`.

Example:

```python
long = df.melt(id_vars="id")
```

Use instead:

```python
long = df.melt(id_vars="id", var_name="year", value_name="sales")
```

xpd gives the replacement as advice and doesn't apply it.

#### pivot-table-default-mean

**Severity:** warning. **Fix:** none.

The rule reports `pivot_table` without `aggfunc`, which takes the mean of the rows that share a cell without a warning. The table shows the cell for two rows of 10 and 30 with one city and one year.

| Call | Cell |
| --- | --- |
| `pivot_table` without `aggfunc` | 20.0 |
| `pivot_table(aggfunc="sum")` | 40 |
| `pivot` | ValueError "Index contains duplicate entries, cannot reshape" |

Example:

```python
table = df.pivot_table(index="city", columns="year", values="sold")
```

Use instead:

```python
table = df.pivot_table(index="city", columns="year", values="sold", aggfunc="sum")
```

xpd gives the replacement as advice and doesn't apply it. Write `aggfunc="mean"` when you want the mean.

#### unstack-without-fill

**Severity:** warning. **Fix:** unsafe: `fill_value=0`.

The rule reports `unstack` right after `size`, `count`, `value_counts` or `nunique` without `fill_value`. A pair without rows comes back as a missing value, and the table shows the dtype of the counts.

| Call | Cell of a pair without rows | Dtype of the counts |
| --- | --- | --- |
| `unstack()` | missing value | `float64` |
| `unstack(fill_value=0)` | 0 | `int64` |

The fix is unsafe, because a mean over the cells changes when the missing values become 0. The rule leaves an `unstack` with more than one argument alone.

Example:

```python
table = df.groupby(["city", "grade"]).size().unstack()
```

Use instead:

```python
table = df.groupby(["city", "grade"]).size().unstack(fill_value=0)
```

### Index alignment and joins

Each rule in this group finds code that turns off the pairing of rows by index label without a
warning, or code that pairs by index through a longer form. pandas pairs rows by index label in an
assignment, in `concat(axis=1)` and in a join. The measurements used pandas 3.1.1.

#### assign-drops-alignment

**Severity:** warning. **Fix:** unsafe: `df["b"] = other["b"]`, `df.loc[:, "b"] = other["b"]` and `assign(b=other["b"])`, which pair the rows by index label. A label that the two indexes do not share gives a missing value.

The rule reports `df["m"] = other["m"].values`, `.to_numpy()` and `.tolist()`, also through `df.loc[:, "m"]` and as a keyword of `df.assign`. The array has no index, so pandas pairs the rows by position. With the same 4 rows in another order, all 4 values landed on the wrong row. The rule leaves a column of the same frame alone, because it has the same index.

Example:

```python
df["region"] = other["region"].values
```

Use instead:

```python
df["region"] = other["region"]
```

When the two indexes share no labels, merge the two frames on a key column instead.

#### concat-columns-unchecked

**Severity:** info. **Fix:** none.

The rule reports `pd.concat([...], axis=1)` without `join=`, which pairs rows by index label and keeps every label of every frame. The table shows two frames of 4 rows whose labels differ in one row.

| Call | Rows | Dtype of both columns |
| --- | --- | --- |
| `pd.concat([left, right], axis=1)` | 5, with a missing value | `float64` |
| `pd.concat([left, right], axis=1, join="inner")` | 3 shared rows | `int64` |

Example:

```python
both = pd.concat([left, right], axis=1)
```

Use instead:

```python
both = pd.concat([left, right], axis=1, join="inner")
```

xpd gives the replacement as advice and doesn't apply it. Write `join="outer"` when you want the missing values.

#### merge-on-both-indexes

**Severity:** info. **Fix:** unsafe: `left.join(right, how="inner")`. None for a merge with another keyword than `how`, `validate` and `sort`.

The rule reports `left.merge(right, left_index=True, right_index=True)` and the `pd.merge` form. The merge gave a frame equal to `left.join(right, how="inner")`, and the two were also equal for `how="left"`, `"right"` and `"outer"` and with `validate="1:1"`. The fix is unsafe, because `join` raises ValueError when the two frames share a column name. `merge` adds the suffixes `_x` and `_y` to such columns.

Example:

```python
both = left.merge(right, left_index=True, right_index=True)
```

Use instead:

```python
both = left.join(right, how="inner")
```

#### merge-asof-unsorted

**Severity:** warning. **Fix:** unsafe: `pd.merge_asof(left.sort_values("t"), right.sort_values("t"), on="t")` when both frames are passed by position and the key by `on=`. The sort reorders the rows of the result. A frame passed by keyword, `left_on` and `right_on`, and the indexes have no fix.

The rule reports `pd.merge_asof(...)` in a function that has no `sort_values` and no `sort_index`, and at module level in a module that has none. An unsorted left frame raised ValueError "left keys must be sorted", and an unsorted right frame raised "right keys must be sorted". The rule also reports a frame that another function sorts, by mistake.

Example:

```python
matched = pd.merge_asof(trades, quotes, on="time")
```

Use instead:

```python
matched = pd.merge_asof(trades.sort_values("time"), quotes.sort_values("time"), on="time")
```

### MultiIndex

Each rule in this group finds a MultiIndex that the code builds or reads one row at a time, and
names the method that works on whole levels.

#### multiindex-from-tuples

**Severity:** warning. **Fix:** unsafe: `pd.MultiIndex.from_arrays([df["k"], df["a"]])`, `pd.MultiIndex.from_product([keys, numbers])`, also for the comprehension with two `for` clauses.

The rule reports `pd.MultiIndex.from_tuples(zip(df["k"], df["a"]))`, `from_tuples(itertools.product(keys, numbers))` and `from_tuples([(k, n) for k in keys for n in numbers])`, which build one Python tuple per row.

Example:

```python
index = pd.MultiIndex.from_tuples(list(zip(df["region"], df["year"])), names=["region", "year"])
```

Use instead:

```python
index = pd.MultiIndex.from_arrays([df["region"], df["year"]], names=["region", "year"])
```

On 1,000,000 rows the methods that take whole levels ran faster, as the table shows.

| Reported code | Write instead | Faster by |
| --- | --- | --- |
| `from_tuples` over `zip` | `from_arrays` | 27 times |
| `from_tuples` over a comprehension | `from_product` | 118 times |

The fix is unsafe, because without `names=` the method `from_arrays` takes the level names from the Series it gets. `from_tuples` left the level names empty. The rule leaves `itertools.product(keys, repeat=2)` alone.

#### level-values-unique

**Severity:** warning. **Fix:** unsafe: `df.index.unique("k")`, `df.index.unique("k").nunique()`.

The rule reports `df.index.get_level_values("k").unique()` and `.nunique()`, which build one value per row first. On 1,000,000 rows `unique` with a level ran 22 times faster. The rule doesn't run on test files.

Example:

```python
regions = df.index.get_level_values("region").unique()
```

Use instead:

```python
regions = df.index.unique("region")
```

The fix is unsafe, because a level built from a range comes back as a RangeIndex, where the old form gave an Index. The values are the same.

#### tuple-slice-on-levels

**Severity:** info. **Fix:** safe: `pd.IndexSlice["a", :, 1]`, and `pd.IndexSlice["a":"c", :]` for a slice with bounds. None for a slice call with keywords or unpacked arguments.

The rule reports a tuple with `slice(None)` inside `.loc[...]`, on the rows or on the columns. `pd.IndexSlice["a", :, 1]` was equal to `("a", slice(None), 1)`, and `.loc` gave equal frames with the two. A slice that has bounds, as in `slice("a", "c")`, becomes `"a":"c"`, and `pd.IndexSlice` builds the same tuple.

Example:

```python
rows = df.loc[("a", slice(None), 1), :]
```

Use instead:

```python
rows = df.loc[pd.IndexSlice["a", :, 1], :]
```

### Time series

Each rule in this group finds date and time code that works one value at a time, goes through
text, or breaks across a daylight saving change. The time zone rules were run on pandas 2.3.3 and
3.1.1 with quarter-hourly data in `Europe/Copenhagen`.

#### rolling-apply

**Severity:** warning. **Fix:** safe: `rolling(20).max()` for min, max and median. unsafe: sum, mean, std and var, and `rolling(20).max()` for a NumPy function such as `np.max` with sum, mean, min, max or median, which can differ in the last bit and skip a missing value. The `raw=True` case, which reports as info, and `np.std` and `np.var`, which divide by n where the window methods divide by n - 1, have no fix.

The rule reports `rolling(20).apply(lambda w: w.max())`, `rolling(20).apply(f)` without `raw=True`, `rolling(20).agg(np.max)` and `rolling(20).apply(np.mean, raw=True)`, which call Python once per window. On 200,000 rows `rolling(60).agg(np.max)` was 1,031 times slower than `rolling(60).max()`.

Example:

```python
average = df["load"].rolling(20).apply(lambda window: window.mean())
```

Use instead:

```python
average = df["load"].rolling(20).mean()
```

The fixes for sum, mean, std and var are unsafe, because the window methods keep running totals. Values far apart in size can then give another result. For a function with no window method, the message names `raw=True` and `engine="numba"`.

#### text-time-key

**Severity:** warning. **Fix:** none.

The rule reports `df.groupby(df["d"].dt.strftime("%Y-%m"))`, which turns every date into text to build the group key.

Example:

```python
monthly = df.groupby(df["time"].dt.strftime("%Y-%m"))["load"].sum()
```

Use instead:

```python
monthly = df.groupby(df["time"].dt.to_period("M"))["load"].sum()
```

The result has a PeriodIndex instead of text labels. On a column with a time zone, `to_period` drops the zone and gives a UserWarning, so group on `pd.Grouper(key="time", freq="MS")` instead.

xpd gives the replacement as advice and doesn't apply it.

#### object-dates

**Severity:** warning. **Fix:** none.

The rule reports `df["d"].dt.date` and `dt.time`, which turn a datetime64 column into a column of Python objects. The rule leaves alone a `dt.date` that is one side of a comparison, because the replacement `dt.normalize()` keeps datetime64. A datetime64 value compared with a Python `date` raises TypeError.

Example:

```python
days = df["time"].dt.date
```

Use instead:

```python
days = df["time"].dt.normalize()
```

`normalize` returns timestamps at midnight and not Python dates, so keep `dt.date` when another library needs Python dates. For `dt.time`, `normalize` drops the time of day, so keep the time as the time since midnight, `df["time"] - df["time"].dt.normalize()`.

xpd gives the replacement as advice and doesn't apply it.

#### apply-on-dates

**Severity:** warning. **Fix:** unsafe: `df["d"] + pd.Timedelta(days=7)`.

The rule reports `df["d"].apply(lambda t: t + pd.Timedelta(days=7))`, `t.replace(day=1)` and `t.strftime(...)`, which call Python once per date. For `replace(day=1)` the fix writes `df["d"] - pd.to_timedelta(df["d"].dt.day - 1, unit="D")`, which keeps the time of day.

Example:

```python
next_week = df["time"].apply(lambda time: time + pd.Timedelta(days=7))
```

Use instead:

```python
next_week = df["time"] + pd.Timedelta(days=7)
```

The fixes are unsafe, because a missing date gives a missing result where the lambda raised.

#### strftime-date-part

**Severity:** warning. **Fix:** unsafe: `df["d"].dt.year`, `df["d"].dt.day_name()`.

The rule reports `df["d"].dt.strftime("%Y").astype(int)`, the same with `%m`, `%H` and `%j`, and `dt.strftime("%A")` and `"%B"`, which were 33 to 198 times slower than the date part on 1,000,000 rows.

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

The fixes are unsafe, because a date part is int32 where `astype(int)` gave int64, and `day_name` is English where `strftime` follows the locale.

#### apply-date-expression

**Severity:** warning. **Fix:** none.

The rule reports `df["d"].apply(lambda t: t.weekday() >= 5)` and `lambda t: t.hour * 60 + t.minute`, a lambda that compares a date part or computes with parts, which was 22 to 94 times slower than the column form. The rule has no fix, because the rewrite depends on the operator.

Example:

```python
weekend = df["time"].apply(lambda time: time.weekday() >= 5)
```

Use instead:

```python
weekend = df["time"].dt.weekday >= 5
```

xpd gives the replacement as advice and doesn't apply it.

#### strftime-round-trip

**Severity:** warning. **Fix:** unsafe: `df["d"].dt.normalize()`, `df["d"].dt.floor("h")`, and `df["d"].dt.normalize() == pd.Timestamp(day)` for the comparison, which raises TypeError on a column with a time zone.

The rule reports `pd.to_datetime(df["d"].dt.strftime("%Y-%m-%d"))` and `df["d"].dt.strftime("%Y-%m-%d") == "2023-06-01"`, which turn dates into text to drop the time and were 18 to 202 times slower than `normalize`.

Example:

```python
day = pd.to_datetime(df["time"].dt.strftime("%Y-%m-%d"))
```

Use instead:

```python
day = df["time"].dt.normalize()
```

The fix is unsafe, because on dates with a time zone the round trip through text drops the zone.

#### index-year-mask

**Severity:** warning. **Fix:** unsafe: `df.loc["2023"]`. On a PeriodIndex of years the label is one period, so the fix returns the value where the mask returned a Series of one row.

The rule reports `df[df.index.year == 2023]`, which computes the year of every row to build a mask.

Example:

```python
year = df[df.index.year == 2023]
```

Use instead:

```python
year = df.loc["2023"]
```

`df.loc["2023"]` selects by label and needs a sorted DatetimeIndex.

#### apply-date-method

**Severity:** warning. **Fix:** unsafe: `df["d"].dt.floor("h")`, `df["d"].dt.weekday`, `df["d"].dt.date`, `df["d"].dt.isocalendar().week`, `df["d"].dt.tz_localize(None)` for `replace(tzinfo=None)`, `df["d"].dt.tz_convert(zone)` for `astimezone(zone)`, and `(df["d"] - pd.Timestamp("1970-01-01")) / pd.Timedelta("1s")` for `timestamp()`, which raises TypeError on a column with a time zone. `toordinal` has no fix.

The rule reports `df["d"].apply(lambda t: t.floor("h"))`, and the same with `ceil`, `normalize`, `tz_localize`, `tz_convert`, `day_name`, `month_name`, `to_period`, `total_seconds`, `weekday`, `date`, `isocalendar`, `timestamp`, `toordinal`, `replace(tzinfo=None)` and `astimezone`, which call Python once per date. It leaves `round` alone, because `lambda v: v.round(2)` is more often a number. It also leaves `replace(tzinfo=zone)` alone, because `replace` skips the daylight saving check that `tz_localize` makes.

Example:

```python
hours = df["time"].apply(lambda time: time.floor("h"))
```

Use instead:

```python
hours = df["time"].dt.floor("h")
```

The table shows the measured speed of the column forms.

| Lambda | Column form | Rows | Speed |
| --- | --- | --- | --- |
| `t.floor("h")` | `dt.floor("h")` | 1,000,000 | 1,354 times faster |
| `t.replace(tzinfo=None)` | `dt.tz_localize(None)` | 200,000 | 2.3 ms against 746 ms |
| `t.timestamp()` | `(df["d"] - pd.Timestamp("1970-01-01")) / pd.Timedelta("1s")` | not stated | 229 times faster |

The fix is unsafe, because the dtype can change. `dt.weekday` is int32 where the lambda gave int64, and `isocalendar().week` is UInt32 where the lambda gave int64.

#### apply-parse-date

**Severity:** warning. **Fix:** unsafe: `pd.to_datetime(df["raw"], format="%Y-%m-%d")`.

The rule reports `df["raw"].apply(lambda v: datetime.strptime(v, "%Y-%m-%d"))`, and a lambda or `map` that calls `pd.to_datetime`, `pd.Timestamp` or `parser.parse` per value. On 1,000,000 rows `pd.to_datetime` with a format ran 24 times faster than `strptime`. The rule reports `strptime` only on `datetime`, because `time.strptime` returns another type.

Example:

```python
when = df["raw"].apply(lambda text: datetime.strptime(text, "%Y-%m-%d"))
```

Use instead:

```python
when = pd.to_datetime(df["raw"], format="%Y-%m-%d")
```

The fix is unsafe, because a call per value parses each text on its own, while `pd.to_datetime` needs one format for the whole column. `strptime` also raises on a missing value, where `pd.to_datetime` returns NaT.

#### timedelta-per-value

**Severity:** warning. **Fix:** unsafe: `pd.to_timedelta(df["n"], unit="D")` for `pd.Timedelta(v, unit="D")`, for a keyword such as `days=v`, and for the numbers written as text with a unit such as `" days"`. The Timedelta per value gave microseconds where `to_timedelta` gives nanoseconds, a DateOffset per value gave objects, and a number written with an exponent raised ValueError in the text. Any other text after the numbers has no fix.

The rule reports `df["n"].apply(lambda v: pd.Timedelta(days=v))`, the same with `timedelta` and `pd.DateOffset`, and `pd.to_timedelta(df["n"].astype(str) + " days")`. On 1,000,000 rows `pd.to_timedelta` ran 1,549 times faster. The rule leaves `pd.DateOffset(months=v)` alone, because a month has no fixed length.

Example:

```python
shift = df["days"].apply(lambda days: pd.Timedelta(days=days))
```

Use instead:

```python
shift = pd.to_timedelta(df["days"], unit="D")
```

`unit=` takes `"W"`, `"D"`, `"h"`, `"min"`, `"s"`, `"ms"` and `"us"`. On fractions the two forms can differ in the last digits.

#### tz-localize-without-dst

**Severity:** warning. **Fix:** none.

The rule reports `df["d"].dt.tz_localize("Europe/Copenhagen")` without `ambiguous=` or without `nonexistent=`. A zone with daylight saving has an hour each year that happens twice and an hour that never happens. `tz_localize` raises on a time in either hour, unless `ambiguous=` and `nonexistent=` say what to do. On half-hourly data for 2024 in `Europe/Copenhagen` the call raised.

| Zone | Reported |
| --- | --- |
| a zone with daylight saving, such as `"Europe/Copenhagen"` | yes |
| `"UTC"`, `"GMT"`, a zone under `Etc/`, or an offset such as `"+01:00"` | no, because they did not raise |
| a zone held in a name | no |
| a zone without daylight saving, such as `"Asia/Tokyo"` | yes, by mistake |

Example:

```python
local = df["time"].dt.tz_localize("Europe/Copenhagen")
```

Use instead:

```python
local = df["time"].dt.tz_localize("Europe/Copenhagen", ambiguous="infer", nonexistent="shift_forward")
```

The table lists the values that the two arguments take.

| Argument | Value | Result |
| --- | --- | --- |
| `ambiguous=` | `"infer"` | reads the order of the rows, and needs both copies of the repeated hour |
| `ambiguous=` | `"NaT"` | makes the time missing |
| `ambiguous=` | a boolean array | says which rows are summer time |
| `nonexistent=` | `"shift_forward"`, `"shift_backward"`, `"NaT"` or a Timedelta | moves the time out of the missing hour, or makes it missing |

xpd gives the replacement as advice and doesn't apply it.

#### date-range-then-localize

**Severity:** warning. **Fix:** unsafe: `pd.date_range(start, end, freq="h", tz=zone)`.

The rule reports `pd.date_range(start, end, freq="h").tz_localize(zone)`, which builds the steps in local clock time and then fails on the daylight saving change.

Example:

```python
hours = pd.date_range("2024-10-26", "2024-10-28", freq="h").tz_localize("Europe/Copenhagen")
```

Use instead:

```python
hours = pd.date_range("2024-10-26", "2024-10-28", freq="h", tz="Europe/Copenhagen")
```

The fix is unsafe, because the two forms differ across a daylight saving change. Hourly steps from 26 to 28 October 2024 raised when localized, and gave 50 hours with `tz=`. Without a daylight saving change in the range, the two forms gave an equal index. The fix adds an argument after the last one, so the rule leaves a call alone that ends in a trailing comma or holds a comment.

#### to-datetime-then-localize

**Severity:** warning. **Fix:** unsafe: `pd.to_datetime(df["raw"], utc=True)`.

The rule reports `pd.to_datetime(df["raw"]).dt.tz_localize("UTC")`, where `utc=True` sets the zone in the same call.

Example:

```python
when = pd.to_datetime(df["raw"]).dt.tz_localize("UTC")
```

Use instead:

```python
when = pd.to_datetime(df["raw"], utc=True)
```

The fix is unsafe, because on text that holds an offset `utc=True` converts to UTC where `tz_localize` raised TypeError. The fix adds an argument after the last one, so the rule leaves a call alone that ends in a trailing comma or holds a comment.

#### convert-then-drop-zone

**Severity:** warning. **Fix:** safe: `df["d"].dt.tz_convert(None)`.

The rule reports `df["d"].dt.tz_convert("UTC").dt.tz_localize(None)`, and the same on an index or a frame, where `tz_convert(None)` gives the same UTC times in one step.

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

The rule reports `ts.index = ts.index + pd.Timedelta(hours=6)` and `ts.index += timedelta(days=1)`, where `shift(freq=)` moves the index in one call.

Example:

```python
ts.index = ts.index + pd.Timedelta(hours=6)
```

Use instead:

```python
ts = ts.shift(freq=pd.Timedelta(hours=6))
```

The fix is unsafe, because the assignment changes the object in place and `shift` returns a new one.

#### date-range-slice

**Severity:** warning. **Fix:** unsafe: `pd.date_range(start, end, freq="15min", inclusive="left")`.

The rule reports `pd.date_range(start, end, freq="15min")[:-1]`, which drops the end with a slice where `inclusive="left"` leaves it out. The rule leaves a range with `periods=` alone.

Example:

```python
steps = pd.date_range("2024-03-30", "2024-04-01", freq="15min")[:-1]
```

Use instead:

```python
steps = pd.date_range("2024-03-30", "2024-04-01", freq="15min", inclusive="left")
```

The fix is unsafe when the end isn't a step of the range. Then the slice drops the last real step, and `inclusive="left"` keeps it. The fix adds an argument after the last one, so the rule leaves a call alone that ends in a trailing comma or holds a comment.

#### index-time-mask

**Severity:** warning. **Fix:** safe: `ts.at_time(time(12))`. unsafe: `ts.between_time("8:00", "16:00", inclusive="left")`.

The rule reports `ts[ts.index.time == time(12)]` and `ts[(ts.index.hour >= 8) & (ts.index.hour < 16)]`, which build a Python time or an hour for every row. The table shows the times on 2,000,000 rows.

| Mask | Time | Replacement | Time |
| --- | --- | --- | --- |
| `ts.index.time == time(12)` | 241 ms | `at_time` | 25 ms |
| the two hour masks | 51 ms | `between_time` | 28 ms |

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

The `at_time` fix applies to a time object, such as `time(12)`. The rule leaves a comparison with text alone, because text never equals a time. The `between_time` fix applies to whole hours written as numbers. It is unsafe when the first hour is later than the second. Then the masks select nothing, and `between_time` selects the hours across midnight.

#### reindex-to-date-range

**Severity:** warning. **Fix:** unsafe: `ts.asfreq("15min")`.

The rule reports `ts.reindex(pd.date_range(ts.index.min(), ts.index.max(), freq="15min"))`, where `asfreq` fills in the missing steps in one call.

Example:

```python
full = ts.reindex(pd.date_range(ts.index.min(), ts.index.max(), freq="15min"))
```

Use instead:

```python
full = ts.asfreq("15min")
```

The fix is unsafe, because `asfreq` keeps the name of the index and the reindex dropped it.

#### divide-by-shift

**Severity:** warning. **Fix:** unsafe: `df["load"].pct_change(96)`.

The rule reports `df["load"] / df["load"].shift(96) - 1` and `(df["load"] - df["load"].shift(96)) / df["load"].shift(96)`, which `pct_change` computes in one call.

Example:

```python
change = df["load"] / df["load"].shift(96) - 1
```

Use instead:

```python
change = df["load"].pct_change(96)
```

The fix is unsafe, because `(s - s.shift(n)) / s.shift(n)` rounds differently. The result is close to the old one and not equal bit for bit. On pandas 2 `pct_change` also filled missing values forward first.

#### date-part-key

**Severity:** warning. **Fix:** none.

The rule reports `df.groupby([df["d"].dt.year, df["d"].dt.month])`, and the same with `index.year` and `index.month`. On 2,000,000 quarter-hourly rows in `Europe/Copenhagen` the two keys took 89 ms and one monthly key took 14 ms.

Example:

```python
monthly = df.groupby([df["time"].dt.year, df["time"].dt.month])["load"].sum()
```

Use instead:

```python
monthly = df.groupby(pd.Grouper(key="time", freq="MS"))["load"].sum()
```

The result has one timestamp per month in place of a year and a month, and it also holds the months without rows.

xpd gives the replacement as advice and doesn't apply it.

#### row-window-on-uneven-times

**Severity:** warning. **Fix:** none.

The rule reports `rolling` with an integer window and `on=`. A window given as a number counts rows, also when `on=` names a time column. `rolling(3)` without `on=` gave the same numbers as with it. The table shows the sum for 2024-01-10 on six days with gaps.

| Call | Sum for 2024-01-10 | Rows in the window |
| --- | --- | --- |
| `rolling(3, on="day").sum()` | 14.0 | three rows that reach back 8 days |
| `rolling("3D", on="day").sum()` | 8.0 | the one row of the last 3 days |

The rule misses a row window on a frame with the times in its index, because xpd can't see what the index holds.

Example:

```python
weekly = df.rolling(7, on="day")["sold"].sum()
```

Use instead:

```python
weekly = df.rolling("7D", on="day")["sold"].sum()
```

xpd gives the replacement as advice and doesn't apply it. A time window starts with `min_periods=1`, where the row window gives missing values until it has 7 rows.

### Stated intent

Each rule in this group asks for an argument that states what the code expects, so that pandas
raises when the data doesn't match instead of giving a wrong result.

#### merge-without-validate

**Severity:** warning. **Fix:** none.

The rule reports `merge` without `validate=`, which duplicates rows without a warning when a key repeats. The rule leaves a merge with `how="cross"` alone, because it pairs every row on purpose.

Example:

```python
joined = left.merge(right, on="key")
```

Use instead:

```python
joined = left.merge(right, on="key", validate="many_to_one")
```

Pick the `validate` value that matches your data, because `"many_to_one"` is only the example. `"many_to_many"` checks nothing.

xpd gives the replacement as advice and doesn't apply it.

#### merge-without-on

**Severity:** warning. **Fix:** none.

The rule reports `merge` without `on=`, which joins on every shared column name.

Example:

```python
joined = left.merge(right, validate="many_to_one")
```

Use instead:

```python
joined = left.merge(right, on="key", validate="many_to_one")
```

Name the key columns that you mean to join on.

xpd gives the replacement as advice and doesn't apply it.

#### to-csv-index

**Severity:** warning. **Fix:** none.

The rule reports `df.to_csv(path)` without `index=`, which writes the index as an extra column without a name, or under its name when the index has one. The rule leaves alone a call with no argument and a call that forwards `**options`.

Example:

```python
df.to_csv(path)
```

Use instead:

```python
df.to_csv(path, index=False)
```

xpd gives the replacement as advice and doesn't apply it.

### Method chains

Each rule in this group asks for one chain of methods where the code builds a frame in separate
steps. A chain names the frame once and lists the steps in the order they run. No variable
holds a frame that is half done.

#### lambda-in-assign

**Severity:** info. **Fix:** safe: `pd.col("a") * 2`. The rule runs on pandas 3.0 and later.

The rule reports a lambda in `df.assign(name=lambda d: d["a"] * 2)`, `df.loc[lambda d: d["a"] > 2]` and `df[lambda d: d["a"] > 2]` that reads its frame only as `d["name"]`. On pandas 3.1.1, `pd.col` gave an equal frame for each kind of lambda body that the rule reports, as the table shows.

| Lambda body | Reported |
| --- | --- |
| arithmetic, a comparison, a mask operator such as `&` or `~`, a method, a reduction, or the `str` accessor | yes |
| a method that takes another column, or an earlier keyword of the same `assign` | yes |
| a column of a frame renamed earlier in the chain | yes |
| the frame used in another way, as in `d.a` or `len(d)` | no |
| a column passed to a function, as in `f(d["a"])` | no |
| another frame read by name, as in `other["x"]` | no |

Example:

```python
df = df.assign(total=lambda d: d["price"] * d["units"])
```

Use instead:

```python
df = df.assign(total=pd.col("price") * pd.col("units"))
```

#### reassign-ladder

**Severity:** info. **Fix:** none.

The rule reports the first of three or more statements in a row that each assign one name from a method of itself, as in `df = df.dropna()`. The rule counts only methods that return a frame or a Series, such as `assign`, `rename`, `dropna`, `merge` and `sort_values`, so it leaves a run of `str` methods alone. Another statement between two of the lines ends the run. The chain gave a frame equal to the three statements.

Example:

```python
df = df.dropna(subset=["price"])
df = df.rename(columns={"qty": "units"})
df = df.assign(total=pd.col("price") * pd.col("units"))
```

Use instead:

```python
df = (
    df.dropna(subset=["price"])
    .rename(columns={"qty": "units"})
    .assign(total=pd.col("price") * pd.col("units"))
)
```

xpd gives the replacement as advice and doesn't apply it.

#### nested-step-calls

**Severity:** info. **Fix:** unsafe: `df.pipe(clean).pipe(add_totals).pipe(add_shares)`.

The rule reports three functions called one inside the other on a frame, as in `add_shares(add_totals(clean(df)))`. `DataFrame.pipe(f)` calls `f(frame)`, and the two forms gave equal frames. The three functions must be plain names and not Python builtins. xpd must also know that the innermost argument is a frame, which means that the argument is one of the following:

- a call such as `pd.read_csv(...)` or `pd.DataFrame(...)`
- a variable whose nearest earlier assignment is such a call
- a parameter annotated `pd.DataFrame`

The fix is unsafe, because the chain raises AttributeError when the first or the second function returns something without `pipe`, such as a dict. [`xpd graph`](commands.md#graph) reads the pipe form as a pipeline of steps.

Example:

```python
result = add_shares(add_totals(clean(df)))
```

Use instead:

```python
result = df.pipe(clean).pipe(add_totals).pipe(add_shares)
```

### Charts

Each rule in this group asks a file to draw with plotly as the plotting backend of pandas, so that
xpd can check the figure with the rules of its [plotly](#plotly) pack. pandas draws `df.plot` with
matplotlib unless the plotting backend is set. With `pd.options.plotting.backend = "plotly"`,
`df.plot.bar(x="team", y="score")` returned a figure equal, as JSON, to
`px.bar(df, x="team", y="score")`. The measurements used pandas 3.1.1 with plotly 7.1.0 and
matplotlib installed.

#### plot-without-plotly-backend

**Severity:** info. **Fix:** unsafe: the line `pd.options.plotting.backend = "plotly"` after `import pandas as pd`. df.plot then returns a plotly Figure in place of a matplotlib Axes. Any other first import of pandas has no fix.

The rule reports a file that draws with a plot method of the accessor and sets no plotting backend. The report is on the first import of pandas, once per file. xpd reports only a file where the setting changes no other line:

- The file calls `line`, `bar`, `barh`, `area`, `scatter`, `box` or `hist` on the accessor. `kde`, `density`, `pie` and `hexbin` raised `NotImplementedError` with plotly, so a file that names one is left alone.
- Every argument of those calls is `x`, `y` or `title`. A file with another keyword, such as `figsize` or `ax`, is left alone, and so is a file that calls `df.plot(...)` itself.
- Each call stands alone as a statement or is returned. A file that assigns the result is left alone, because the result is a matplotlib Axes there.
- The file imports neither matplotlib nor seaborn.

The rule also reports nothing when one of the following modules sets the backend:

- the `__init__.py` of a package that the file is in
- a module of the project that the file imports
- a module that one of those modules imports

Example:

```python
import pandas as pd

df.plot.bar(x="team", y="score")
```

Use instead:

```python
import pandas as pd

pd.options.plotting.backend = "plotly"

df.plot.bar(x="team", y="score")
```

#### plot-kind-argument

**Severity:** info. **Fix:** safe: `df.plot.bar(x="team", y="score")`.

The rule reports `df.plot(kind="bar", ...)` for the eleven kinds that the accessor has a method for: `line`, `bar`, `barh`, `box`, `hist`, `kde`, `density`, `area`, `pie`, `scatter` and `hexbin`. The two forms were equal as JSON with the plotly backend, and xpd reads the method form when pandas draws with plotly. The rule leaves alone a kind that only another backend knows, such as `"violin"` with plotly.

Example:

```python
df.plot(kind="bar", x="team", y="score")
```

Use instead:

```python
df.plot.bar(x="team", y="score")
```

#### matplotlib-on-frame

**Severity:** info. **Fix:** none.

The rule reports `plt.plot`, `plt.scatter` and `plt.bar` on two columns of one frame, `plt.hist` on one column, and `sns.scatterplot`, `sns.histplot` and `sns.boxplot` with `data=`. The plot accessor drew the same values with the plotly backend. `plt` and `sns` must come from the import of `matplotlib.pyplot` and of `seaborn`.

| Left alone | Reason |
| --- | --- |
| `sns.barplot` and `sns.lineplot` | seaborn draws the mean of each group, and plotly draws every row |
| `plt.barh` | it takes the labels first, and with plotly `df.plot.barh` takes the values as `x` |

Example:

```python
plt.scatter(df["score"], df["cost"])
```

Use instead:

```python
pd.options.plotting.backend = "plotly"

df.plot.scatter(x="score", y="cost")
```

xpd gives the replacement as advice and doesn't apply it.

### Reading and writing files

Each rule in this group finds a call that reads or writes a file in a slower way than it needs to,
or that writes a CSV file that reads back wrong.

#### read-csv-slow-option

**Severity:** warning. **Fix:** unsafe: drop `engine="python"` when a comma comes before it. An option that only the python engine takes, such as a callable `on_bad_lines`, then raises ValueError. `low_memory=False` and `sep=None` have no fix, because the dtypes and the separator are not in the file.

The rule reports `read_csv(..., engine="python")`, `sep=None` and `low_memory=False`, which turn off the fast C reader. On 1,000,000 rows `sep=None` was 68 times slower than a named separator, measured with the reader's default threads on a 32-core machine. The rule leaves `engine="python"` alone when the call needs it, which is for `skipfooter` or for a separator of several characters other than `\s+`.

Example:

```python
df = pd.read_csv(path, engine="python")
```

Use instead:

```python
df = pd.read_csv(path)
```

The replacement assumes that the call uses no option that only the Python engine has. In place of `low_memory=False`, pass the dtypes of the columns with `dtype=`.

#### read-then-head

**Severity:** warning. **Fix:** unsafe: `nrows=1000`, `nrows=5` for `head()` and `nrows=n` for `iloc[:n]` with a whole number. `iloc[:n]` with any other count has no fix.

The rule reports `pd.read_csv(path).head(1000)` and `.iloc[:1000]`, which parse the whole file to keep the first rows. `nrows` was 45 times faster than `head`, measured with the reader's default threads on a 32-core machine.

Example:

```python
sample = pd.read_csv(path).head(1000)
```

Use instead:

```python
sample = pd.read_csv(path, nrows=1000)
```

The fix is unsafe, because pandas guesses the dtypes from the rows it reads, so a column can get another dtype.

#### read-csv-converters

**Severity:** warning. **Fix:** none.

The rule reports `read_csv(..., converters=...)`, which calls a Python function once per value and was 3.6 to 5.1 times slower than `dtype` or a column method after the read, measured with the reader's default threads on a 32-core machine.

Example:

```python
df = pd.read_csv(path, converters={"price": float, "name": str.strip})
```

Use instead:

```python
df = pd.read_csv(path, dtype={"price": float})
df["name"] = df["name"].str.strip()
```

xpd gives the replacement as advice and doesn't apply it.

#### read-json-lines-engine

**Severity:** warning. **Fix:** unsafe: `engine="pyarrow"`. The `json.loads` loop has no fix.

The rule reports `pd.read_json(path, lines=True)` without an engine, and `pd.DataFrame([json.loads(line) for line in f])`. `read_json` with the pyarrow engine was 25 times faster than without an engine, and 48 times faster than the `json.loads` loop, measured with the reader's default threads on a 32-core machine. The rule leaves a call alone that reads from a `StringIO`, because the pyarrow engine raises on a text buffer.

Example:

```python
df = pd.read_json(path, lines=True)
```

Use instead:

```python
df = pd.read_json(path, lines=True, engine="pyarrow")
```

The fix is unsafe, because pyarrow can pick another dtype for a column.

#### to-csv-append-header

**Severity:** error. **Fix:** none.

The rule reports `df.to_csv(path, mode="a")` without `header=`, which writes the header again on every call.

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

xpd gives the replacement as advice and doesn't apply it.

### Parquet storage

Each rule in this group asks a file that stores a frame to use Parquet, or to read only the
columns and rows it needs from a Parquet file. `to_parquet` and `read_parquet` returned a frame
equal to the one written. The measurements used pandas 3.1.1 with pyarrow 26.0.1 and a frame of
1,000,000 rows with the dtypes `int64`, `float64`, `str`, `datetime64[us]`, an ordered category
and `Int64`. The table gives the file size of the frame in each format.

| Format | File size |
| --- | --- |
| Parquet | 14.7 MB |
| CSV | 47.5 MB |
| pickle | 49.0 MB |

#### csv-round-trip

**Severity:** warning. **Fix:** none.

The rule reports `df.to_csv(path)` in a file that also calls `pd.read_csv(path)` on the same path, written the same way in both calls. `read_csv` guesses every dtype again, and the table shows the dtypes that changed. `to_csv` took 0.93 seconds and `to_parquet` took 0.09 seconds.

| Column written | Read back from CSV |
| --- | --- |
| `datetime64[us]` | `str` |
| ordered category | `str` |
| `Int64` | `float64` |

The rule leaves a file alone that only writes a CSV, because another program may read it. It also leaves `to_csv` with `mode=` alone, which appends to a file.

Example:

```python
df.to_csv("clean.csv", index=False)
again = pd.read_csv("clean.csv")
```

Use instead:

```python
df.to_parquet("clean.parquet")
again = pd.read_parquet("clean.parquet")
```

xpd gives the replacement as advice and doesn't apply it.

#### pickle-or-hdf-storage

**Severity:** info. **Fix:** none.

The rule reports `to_pickle`, `pd.read_pickle`, `to_hdf`, `pd.read_hdf` and `pd.HDFStore`. `read_pickle` runs the function that the file names. For example, a pickle that named `print` made `pd.read_pickle` print a line. HDF needs the PyTables package, and without it `to_hdf` raised ImportError. Parquet holds data only, and it needs pyarrow.

Example:

```python
df.to_pickle("clean.pkl")
again = pd.read_pickle("clean.pkl")
```

Use instead:

```python
df.to_parquet("clean.parquet")
again = pd.read_parquet("clean.parquet")
```

xpd gives the replacement as advice and doesn't apply it.

#### read-parquet-all-columns

**Severity:** warning. **Fix:** safe: `columns=["id", "price"]`.

The rule reports `pd.read_parquet(path)[["id", "price"]]` and `pd.read_parquet(path)["price"]`, which read every column and keep the named ones. The names must be strings. The two frames were equal, with the columns in the order of the list, and a file with a named index gave the same index both ways.

| Columns read with `columns=` | Faster by |
| --- | --- |
| 2 of 50 | 9.6 and 9.2 times in two runs, measured with the reader's default threads on a 32-core machine |
| 2 of 6 | 1.7 times |

Example:

```python
prices = pd.read_parquet(path)[["id", "price"]]
```

Use instead:

```python
prices = pd.read_parquet(path, columns=["id", "price"])
```

#### read-parquet-then-filter

**Severity:** info. **Fix:** none.

The rule reports `df = pd.read_parquet(path)` followed directly by `df = df[df["id"] >= value]`, with `>`, `>=`, `<`, `<=`, `==`, `!=` or `isin`. With `filters=`, pyarrow skips each row group that holds no matching row, and the values were equal. A row group is a block of rows that Parquet stores together. The rule has no fix, because the index can differ. A frame without a stored index came back numbered from 0, where the mask kept the old labels.

| File, filtered to the last tenth of a sorted column | Faster by |
| --- | --- |
| 10 row groups | 5.3 and 4.6 times in two runs, measured with the reader's default threads on a 32-core machine |
| 1 row group | 1.3 to 1.5 times |

Example:

```python
df = pd.read_parquet(path)
df = df[df["id"] >= 900_000]
```

Use instead:

```python
df = pd.read_parquet(path, filters=[("id", ">=", 900_000)])
```

xpd gives the replacement as advice and doesn't apply it.

#### chunked-csv-concat

**Severity:** warning. **Fix:** unsafe: `pd.read_csv(path)` without `chunksize`, when concat takes nothing but the chunks and `ignore_index=True`. pandas guesses the dtypes per chunk, so a column with mixed values can differ. A concat with other arguments, such as `keys=`, has no fix.

The rule reports `pd.concat(pd.read_csv(path, chunksize=n))`, also with `list(...)` or a comprehension around the reader. The code joins every chunk again, so the whole file is in memory anyway. The two frames were equal, and the table shows how long the chunked read took in two runs against one `read_csv` without `chunksize`.

| Threads of the reader | Time against one `read_csv` |
| --- | --- |
| the default, on a 32-core machine | 3.1 and 2.9 times as long |
| one | 1.1 and 1.0 times as long |

The rule leaves a loop alone that reduces each chunk before the concat.

Example:

```python
df = pd.concat(pd.read_csv(path, chunksize=100_000))
```

Use instead:

```python
df = pd.read_csv(path)
```

### Packages

`xpd check` warns once per run about each package in the table that the project's venv lacks,
when the venv has pandas. The warning points at the nearest `pyproject.toml`, or at the venv
when there is none. A project without a venv gets no package warning, and `xpd hook` never gives
one.

| Check | Package | Extra condition |
| --- | --- | --- |
| [`missing-pyarrow`](#missing-pyarrow) | pyarrow | none |
| [`missing-bottleneck`](#missing-bottleneck) | bottleneck | none |
| [`missing-plotly`](#missing-plotly) | plotly | the project uses matplotlib or seaborn |
| [`missing-pandas-stubs`](#missing-pandas-stubs) | pandas-stubs | the project uses mypy, pyright, basedpyright, ty or pyrefly |

A project uses a tool when the venv has it or `pyproject.toml` has a `[tool.<name>]` table for it.
The output shows the four warnings for a venv that holds pandas and matplotlib, next to a
`pyproject.toml` with a `[tool.mypy]` table.

```text
$ xpd check --format text report.py
pyproject.toml: warning [pandas/missing-pyarrow] pyarrow is not installed, so text columns are stored as Python strings. With pyarrow the .str methods ran 3 to 11 times faster on 1,000,000 rows. Add pyarrow to the project.
pyproject.toml: warning [pandas/missing-bottleneck] bottleneck is not installed. With it std ran 4 times faster and median 1.7 times faster on 10,000,000 rows. Add bottleneck to the project.
pyproject.toml: warning [pandas/missing-plotly] The project has matplotlib and no plotly, so df.plot returns a static matplotlib Axes. With plotly installed and pd.options.plotting.backend = "plotly", df.plot.bar(x="team", y="score") returned the same figure as px.bar(df, x="team", y="score"), which xpd checks with its plotly rules. Add plotly to the project.
pyproject.toml: warning [pandas/missing-pandas-stubs] The project has a type checker but not pandas-stubs, so calls to pandas are not type checked. Add pandas-stubs as a development dependency.
xpd: 4 issues
```

numexpr has no check, because it changed little in the measurements. numba has no check, because
it helps only a function passed to `rolling(...).apply`. The `rolling-apply` message names numba.

#### missing-pyarrow

**Severity:** warning. **Fix:** none.

Without pyarrow, pandas stores text columns as Python strings. With pyarrow the `.str` methods ran 3 to 11 times faster on 1,000,000 rows.

The warning appears when pyarrow is not installed.

In a project managed with uv, add the package with the command below.

```sh
uv add pyarrow
```

Add the package to the project environment, not to xpd itself.

#### missing-bottleneck

**Severity:** warning. **Fix:** none.

With bottleneck, `std` ran 4 times faster and `median` 1.7 times faster on 10,000,000 rows.

The warning appears when bottleneck is not installed.

In a project managed with uv, add the package with the command below.

```sh
uv add bottleneck
```

Add the package to the project environment, not to xpd itself.

#### missing-plotly

**Severity:** warning. **Fix:** none.

Without plotly, `df.plot` returns a static matplotlib Axes. With plotly as the plotting backend, `df.plot.bar(x="team", y="score")` returned the same figure as `px.bar(df, x="team", y="score")`, which xpd checks with its plotly rules.

The warning appears when plotly is not installed and the project uses matplotlib or seaborn.

In a project managed with uv, add the package with the command below.

```sh
uv add plotly
```

Add the package to the project environment, not to xpd itself.

#### missing-pandas-stubs

**Severity:** warning. **Fix:** none.

Without pandas-stubs, a type checker doesn't check calls to pandas.

The warning appears when pandas-stubs is not installed and the project uses mypy, pyright, basedpyright, ty or pyrefly.

In a project managed with uv, add the package as a development dependency with the command below.

```sh
uv add --dev pandas-stubs
```

Add the package to the project environment, not to xpd itself.



## NumPy

The NumPy rules find code that gives a wrong result or raises on NumPy 2.5, and code that has a
faster or more exact form. They run on files that import `numpy`, and examples assume
`import numpy as np`. Each claim in a message was measured on NumPy 2.5.5, and the table says what each claim means.

| Claim | What it means on NumPy 2.5.5 |
| --- | --- |
| a form is removed | it raises |
| a form is wrong | it gives the wrong value |
| a form is faster | the median of 7 runs on one core, at 1,000,000 values unless the rule says otherwise |

A fix that writes `np.` applies only in a file with `import numpy as np`, and a fix that writes
`assert_allclose` only in a file with `from numpy.testing import assert_allclose`. A safe fix returns a
value of the same dtype that is equal bit for bit, and the **Fix:** line of an unsafe fix names
what can differ.

xpd skips a rule, or one form of a rule, that asks for an API newer than the NumPy in the
project's virtual environment (venv). When the venv has no NumPy, or there is no venv, every rule runs.

| Rule | Runs on |
| --- | --- |
| [`char-module`](#char-module), [`log-of-det`](#log-of-det), [`unpackbits-popcount`](#unpackbits-popcount) and [`vecdot`](#vecdot) | NumPy 2.0 and later |
| some forms of [`removed-in-numpy-2`](#removed-in-numpy-2) | NumPy 1.22, 1.25 or 2.0 and later, by form |
| some forms of [`deprecated-in-numpy-2`](#deprecated-in-numpy-2) | NumPy 2.0 and later |
| every other NumPy rule | every NumPy version |

### Code that gives a wrong result or raises on NumPy 2.5

Fix every report in the group, because the reported code gives a wrong value without an error,
or raises on every input. Every fix in the group is unsafe, because it changes the result to the
one the code meant, except the fix of `positional-tolerance`, which only names the argument.

#### compare-to-nan

**Severity:** error. **Fix:** unsafe: `np.isnan(x)`, `np.isnan(np.unique(x)).any()` for `np.nan in np.unique(x)`, and `not np.isnan(np.unique(x)).any()` for `not in`. `isnan` raises on an array of text or objects, where `in` returned False.

The rule reports `x == np.nan`, `x != np.nan`, `x == float("nan")`, `x == math.nan` and `np.nan in np.unique(x)`. None of them finds NaN, because NaN equals nothing.

Example:

```python
missing = x == np.nan
```

Use instead:

```python
missing = np.isnan(x)
```

In a file that imports pandas, the pandas rule [`compare-to-missing`](#compare-to-missing) reports the code instead. The rule leaves a comparison alone that is passed to an assert function, because a test may check how a comparison with NaN behaves.

#### argument-in-wrong-slot

**Severity:** error. **Fix:** unsafe: `np.zeros((3, 4))`, `np.hstack((a, b, a))` with any keywords after the tuple, and `np.percentile` with the same arguments for a quantile call with more arguments. The linspace form has no fix, because the code does not show whether a count or a step was meant.

The rule reports `np.zeros(3, 4)`, `np.full(3, 4, 0)`, `np.reshape(a, 2, 3)`, `np.vstack(a, b)`, `np.concatenate(a, [4, 5])`, `np.random.rand((3, 4))`, `np.random.random(3, 4)`, `np.linspace(0, 1, 0.1)` and `np.quantile(x, 95)`. Each call puts an argument in the place of another parameter, so it raises.

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

The rule reports `np.percentile(x, 0.95)` and `np.percentile(x, [0.25, 0.75])`. `np.percentile` takes percents from 0 to 100, so both calls ask for values near the minimum.

Example:

```python
upper = np.percentile(x, 0.95)
```

Use instead:

```python
upper = np.quantile(x, 0.95)
```

The rule leaves a list such as `[0.5, 99.5]` alone, where one number is a percent.

#### positional-tolerance

**Severity:** warning. **Fix:** safe: `np.isclose(a, b, rtol=1e-3)`, which names the parameter that the third argument already fills. Where an absolute tolerance was meant, change it to `atol=`. A call with a splat before the third argument has no fix.

The rule reports `np.isclose(a, b, 1e-3)` and `np.allclose(a, b, 1e-3)`. The third argument sets `rtol`, and `atol` stays 1e-8.

Example:

```python
same = np.isclose(a, b, 1e-3)
```

Use instead:

```python
same = np.isclose(a, b, atol=1e-3)
```

The rule leaves a call alone that also passes `atol`, or that passes a value named like `rtol`.

#### arange-float-step

**Severity:** warning. **Fix:** none.

The rule reports `np.arange(0, 1, 0.1)`, whose length depends on rounding. For example, `np.arange(1, 1.3, 0.1)` ends with 1.3000000000000003. The [NumPy docs for arange](https://numpy.org/doc/stable/reference/generated/numpy.arange.html) recommend `np.linspace` for a step that is not whole.

Example:

```python
ticks = np.arange(0, 1, 0.1)
```

Use instead:

```python
ticks = np.linspace(0, 0.9, 10)
```

The rule leaves a step alone that is a multiple of 1/64, such as 0.5 or 0.25, because such a step is exact. xpd gives the replacement as advice and doesn't apply it.

#### randint-single-value

**Severity:** error. **Fix:** unsafe: `np.random.randint(0, 2)` and `rng.integers(0, 2)`, with the other arguments kept, so the result is a 0 or a 1 where it was always 0.

The rule reports `np.random.randint(0, 1, n)`, `np.random.randint(1)` and `rng.integers(0, 1)`. Each call always returns 0, because the upper bound is excluded.

Example:

```python
coin = rng.integers(0, 1, size=10)
```

Use instead:

```python
coin = rng.integers(0, 2, size=10)
```

#### result-is-none

**Severity:** error. **Fix:** unsafe: `x.sort()`. A result assigned to another name, or returned, has no fix.

The rule reports `x = x.sort()`, `m = np.fill_diagonal(m, 0)`, `y = rng.shuffle(y)`, `w = w.fill(0)` and `return v.sort()`. The calls change the array in place and return None, so the code gets None in place of the array.

Example:

```python
values = values.sort()
```

Use instead:

```python
values.sort()
```

The rule leaves a sort alone that has an argument other than an axis, such as `xp.sort(x)` in the array API, because such a sort returns a value.

#### result-discarded

**Severity:** error. **Fix:** unsafe: `x = np.sort(x)`.

The rule reports `np.sort(x)`, `np.append(x, 4)`, `np.clip(x, 0, 1)`, `x.astype(float)` and `m.reshape(3, 2)` standing alone as a statement. Each call returns a new array that the code drops, so the statement does nothing.

Example:

```python
np.append(lengths, 1)
```

Use instead:

```python
lengths = np.append(lengths, 1)
```

The rule leaves alone a call with `out=`, a call in a `try` block or in a `with pytest.raises` block, and a call in a benchmark method of asv (airspeed velocity). After the fix, other names bound to the old array keep its old values.

#### write-to-copy

**Severity:** error. **Fix:** unsafe: `x[0] = 9` for `x.astype(int)[0] = 9` and `x.copy()[0] = 9`, so the write reaches `x`, cast to the dtype of `x`. A mask or a list of positions, and `flatten`, have no fix.

The rule reports `x[x > 2][0] = v`, `x[[0, 1]][0] = v`, `m.flatten()[0] = v` and `x.astype(int)[0] = v`. Each one writes to a copy, so the array keeps its old values.

Example:

```python
x[x > 2][0] = -1
```

Use instead:

```python
x[np.flatnonzero(x > 2)[0]] = -1
```

The rule leaves `x[1:4][0] = v` and `m[0][mask] = v` alone, because indexing with a slice or a whole number returns a view.

#### array-truth-value

**Severity:** error. **Fix:** unsafe: `x[(x > 2) & (x < 8)]` for `and`, `|` for `or`, `~` for `not`, and `(2 < x) & (x < 8)` for a chained comparison of names and numbers. On an array of one value the old form indexed with one bool. Other operands, such as a mask held in a name, have no fix.

The rule reports `x[(x > 2) and (x < 8)]`, `x[not (x > 2)]` and `x[2 < x < 8]`. Each one raises ValueError, because `and`, `or`, `not` and a chained comparison ask for the truth value of a whole array.

Example:

```python
inside = x[(x > 2) and (x < 8)]
```

Use instead:

```python
inside = x[(x > 2) & (x < 8)]
```

#### mask-operator-precedence

**Severity:** error. **Fix:** unsafe: `(x > 2) & (x < 8)` when the four operands are names, numbers or attributes. Other operands have no fix.

The rule reports `x > 2 & x < 8`. Python applies `&` before the comparisons, so the code compares `x` with `2 & x`.

Example:

```python
inside = x[x > 2 & x < 8]
```

Use instead:

```python
inside = x[(x > 2) & (x < 8)]
```

In a file that imports pandas, the pandas rule [`mask-precedence`](#mask-precedence) reports the code instead.

#### int-mask-as-index

**Severity:** error. **Fix:** unsafe: `y[y > 15]`. The `~` form has no fix.

The rule reports `y[(y > 15).astype(int)]`, which picks element 0 and element 1 over and over. It also reports `~(y > 15).astype(int)`, which gives -1 and -2.

Example:

```python
large = y[(y > 15).astype(int)]
```

Use instead:

```python
large = y[y > 15]
```

The rule leaves alone a lookup such as `labels[(score > t).astype(int)]`, where the mask comes from another array. Such a lookup maps each value to one of two labels.

#### iterator-to-array

**Severity:** error. **Fix:** unsafe: `np.array([v * 2 for v in y])`, and `np.array(sorted({1, 2, 3}))` for a set, where sorted raises on values that do not compare.

The rule reports `np.array(v * 2 for v in y)`, `np.array(map(float, y))`, `np.array(d.values())`, `np.array({1, 2})`, `np.max(v for v in y)` and `np.sum(map(f, y))`, because NumPy doesn't read the values out of an iterator. The table shows what each call returned on NumPy 2.5.5.

| Call on a generator | Result |
| --- | --- |
| `np.array(v for v in y)` | an object array with no dimensions that holds the generator |
| `np.max(v for v in y)` | the generator itself |
| `np.any(v for v in [0, 0])` | True |
| `np.sum(v for v in y)` | TypeError "Calling np.sum(generator) is deprecated" |

Example:

```python
doubled = np.array(v * 2 for v in values)
```

Use instead:

```python
doubled = np.array([v * 2 for v in values])
```

The rule leaves `np.fromiter` alone, because it reads the values out of an iterator.

#### nan-to-int

**Severity:** warning. **Fix:** none.

The rule reports `np.where(m, x, np.nan).astype(int)`, which turns each NaN into -9223372036854775808.

Example:

```python
counts = np.where(seen, totals, np.nan).astype(int)
```

Use instead:

```python
counts = np.where(seen, totals, 0).astype(int)
```

xpd gives the replacement as advice and doesn't apply it.

#### read-uninitialized

**Severity:** error. **Fix:** unsafe: `np.zeros(n)` and `np.zeros_like(a)` with the same arguments, so the code computes on zeros.

The rule reports `np.empty(n).sum()` and `np.empty_like(x) + 1`, which compute with whatever bytes were in memory.

Example:

```python
total = np.empty(n) + offset
```

Use instead:

```python
total = np.zeros(n) + offset
```

The rule leaves `np.empty(n) * np.nan` alone, because it gives NaN everywhere.

#### array-default-argument

**Severity:** warning. **Fix:** none.

The rule reports `def f(acc=np.zeros(2))` when the function writes into `acc`, because every call shares the one default array.

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

xpd gives the replacement as advice and doesn't apply it.

#### append-flattens

**Severity:** error. **Fix:** none.

The rule reports `np.append(m, [[5, 5]])` and `np.insert(m, 0, [[5, 5]])` without `axis`. Both calls flatten the array, so the row becomes separate values in one dimension.

Example:

```python
grown = np.append(m, [[5, 5]])
```

Use instead:

```python
grown = np.append(m, [[5, 5]], axis=0)
```

xpd gives the replacement as advice and doesn't apply it.

#### compare-reductions

**Severity:** error. **Fix:** unsafe: `np.array_equal(a, b)`.

The rule reports `a.all() == b.all()` and `np.any(a) == np.any(b)`, which compare two single booleans and not the arrays.

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

The rule reports `len(np.where(z > 0))`, `len(np.nonzero(z))` and `if np.where(z > 9):`, because `np.where` with one argument returns a tuple with one array per dimension. The length of the tuple is the number of dimensions, and the tuple is always true.

Example:

```python
matches = len(np.where(z > 0))
```

Use instead:

```python
matches = np.count_nonzero(z > 0)
```

### Code that raises or warns on NumPy 2.5

Replace each name, method, module or keyword that NumPy removed or deprecated with the form that
the rule's message names, because the old form raises or warns on NumPy 2.5.

#### removed-in-numpy-2

**Severity:** error. **Fix:** safe: the builtin for `np.int`, `np.float`, `np.complex`, `np.object`, `np.str` and `np.unicode`, `np.float64` for `np.float_` and the other scalar aliases, `np.nan` and `np.inf` for `np.NaN`, `np.Inf` and their kin, `np.prod`, `np.cumprod`, `np.any`, `np.all`, `np.round` and `np.vstack` for the renamed functions, `np.exceptions.AxisError` and the other warnings and errors, `np.trapezoid`, `np.sort(a, axis=0)`, `a.item()`, `np.pad` for `np.lib.pad` and the other functions that `np.lib` re-exported, a shape by position for `newshape=`, `method=` for `interpolation=` and `minlength=0`. unsafe: `np.exceptions.RankWarning` and `np.exceptions.DTypePromotionError`, because no NumPy release has both the old and the new place, `"S"` for the dtype code `"a"`, because a `dtype=` keyword of another library can give the text another meaning, deleting `fix_imports=`, `bias=` and `ddof=`, because a call in the deleted value no longer runs, `np.ptp(a)`, `a.tobytes()` and `a[i, j] = v` for `a.itemset`, because the pattern cannot tell an array from another object with a method of that name, `np.isin` for `np.in1d`, `density=` for `normed=`, `np.asarray` for `np.array(x, copy=False)`, `np.frombuffer` for `np.fromstring`, deleting `style=`, and the module path for `numpy.linalg.linalg`, `numpy.fft.helper`, `numpy.testing.utils` and the private modules of `numpy.lib`, imported or read by attribute. Also unsafe, because the old form raises on NumPy 2.5: `(-np.inf)` and `(-0.0)` where `np.NINF` and `np.NZERO` are part of a larger operand, `(x + 1).item()` for `np.asscalar(x + 1)`, `from numpy.exceptions import` for an import of the moved errors and warnings, `from numpy.lib.array_utils import byte_bounds`, `a.flat[4] = v` for `a.itemset(4, v)`, `newshape=` moved by position before the other keywords, and `copy=None` for the other `np.array(copy=False)` calls. The other forms have no fix.

The rule reports the names, methods, modules and keywords that NumPy 1.24 through 2.5 removed, and its message says what to write instead. The rule leaves alone code under a NumPy version check, a `hasattr(np, ...)` check, or a `try` that catches ImportError or AttributeError. Such code exists to support old NumPy.

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

**Severity:** warning. **Fix:** safe: `np.trunc` for `np.fix`, `np.ma.round` for `np.ma.round_`, `out=` for an output passed by position to `np.maximum` and `np.minimum`, and a datetime64 string without its `Z`. unsafe: `np.rec` and `np.char` for `np.core.records` and `np.core.defchararray`, which export fewer names, `np.<name>` for `np.core.<name>` and for `np.core.numeric.<name>` and the other public submodules, `from numpy import` for a `from numpy.core` import of public names, `randint(low, high + 1)` for `random_integers(low, high)`, with or without a size and with brackets around a `high` such as `n * 2`, `randint(1, high + 1)` for `random_integers(high)`, `axes=range(-len(s), 0)` added where `s=` has no `axes=`, which reads `s` twice, `x = x.reshape(shape)` for `x.shape = shape`, and `x = x.view(dtype)` for `x.dtype = dtype`. The other forms have no fix, and `x.strides = v` has none because `as_strided` does not check the bounds that the assignment checked.

The rule reports the forms in the table, and each of them warns on NumPy 2.5.

| Kind | Reported form |
| --- | --- |
| Modules and names | `numpy.core`, `np.typename`, `np.char.array`, `np.char.asarray` and `np.char.chararray` |
| Testing helpers | `np.testing.assert_warns` and `suppress_warnings` |
| Dates and times | a timedelta64 without a unit, and a time zone in a datetime64 string |
| Calls and attributes | `fftn` and its kin with `s=` and no `axes=`, and setting `shape`, `dtype` or `strides` on a name bound from an `np.` call in the same function |

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

Write one expression on whole arrays in place of code that calls Python once per value or copies
the whole array once per pass, because the whole array form ran up to 530 times faster. The
message of each rule names the whole array form. The table shows the speed that the message
reports, as medians of 7 runs on one core with NumPy 2.5.5, on float64 and int64.

| Rule | Write instead | Faster by |
| --- | --- | --- |
| [`element-loop`](#element-loop) | one expression on whole arrays | 70 to 530 times on 1,000,000 values |
| [`growth-in-loop`](#growth-in-loop) | a list, and one `np.concatenate` after the loop | 130 to 170 times for 1,000 passes over 1,000,000 values |
| [`per-value-comprehension`](#per-value-comprehension) | the expression on `np.asarray(...)` | 60 to 470 times on an array of 1,000,000 values, and 2 to 24 times on a list |
| [`builtin-on-array`](#builtin-on-array) | the method, such as `.sum(axis=0)` | 37 to 47 times on 1,000,000 values |
| [`np-vectorize`](#np-vectorize) | code on whole arrays, such as `np.where` or `np.select` | not measured |

#### element-loop

**Severity:** warning. **Fix:** none.

The rule reports a `for` loop over `range`, `np.arange` or `np.ndindex`, and a `while i < n` loop with `i += 1`, when the loop's only statement writes one element of a NumPy array from arithmetic on the loop position. It also reports a loop over `np.nditer` or `np.ndenumerate` without `external_loop`. The rule leaves a loop alone whose body calls another function, reads the array it writes, or writes a slice.

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

The rule reports `out = np.append(out, x)`, `out = np.concatenate((out, x))` and the same with `vstack`, `hstack`, `column_stack`, `dstack` and `stack` inside a loop that doesn't reset `out` or read it elsewhere. Each call copies every earlier value.

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

**Severity:** warning. **Fix:** unsafe: the element with the loop variable replaced by `np.asarray(a)`, as in `np.asarray(a) * 2 + 1`, or by `np.arange(n)` for a range, and `np.sqrt(np.asarray(a))` for `map`. Python ints do not wrap where int64 does, a division by zero gives inf where Python raised, and `map` returned an iterator. An element with `if`, `and`, `or`, `not`, `min`, `max` or a math function, and a loop over a call, have no fix.

The rule reports `np.array([x * 2 + 1 for x in a])`, `np.array([math.sqrt(x) for x in a])`, `np.array([i * 0.5 for i in range(n)])` and `map(np.sqrt, a)`, where the element is arithmetic or a NumPy or math function on the loop variable.

Example:

```python
roots = np.array([math.sqrt(x) for x in values])
```

Use instead:

```python
roots = np.sqrt(np.asarray(values))
```

#### builtin-on-array

**Severity:** warning. **Fix:** unsafe: `np.abs(a).sum(axis=0)`, `np.abs(a).max(axis=0)`, `np.sort(np.abs(a))` for `sorted`, which gives an array, and `np.abs(a).tolist()` for `list`, which gives Python numbers.

The rule reports `sum`, `min`, `max`, `sorted` and `list` around a NumPy call, as in `sum(np.abs(a))`, or around a comparison with a NumPy call, as in `sum(np.abs(a) > 1)`.

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

The rule reports `np.vectorize`, `np.frompyfunc` and `np.apply_along_axis`, whether called or used as a decorator.

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

Build, copy and store each array with the NumPy function that the rule names, because it ran 1.6
to 880 times faster on 1,000,000 values. The table shows the speed that the message of each rule
reports.

| Rule | Write instead | Faster by |
| --- | --- | --- |
| [`array-construction`](#array-construction) | `np.arange`, `np.zeros` or `np.full` | 150 times for `np.arange` |
| [`needless-copy`](#needless-copy) | `.ravel()` | 1.6 to 1.8 times |
| [`file-roundtrip`](#file-roundtrip) | `np.save` and `np.load` | 27 to 880 times to read, and 420 to 510 times to write |

#### array-construction

**Severity:** warning. **Fix:** safe: `np.zeros(n, dtype=np.float32)`. unsafe: `np.full(n, 3, dtype=np.float64)`, which gives a zero dimensional array where the product of an empty shape gave a scalar, `np.arange(n)`, `np.full(n, 0)`, `np.arange(n, dtype=float)`.

The rule reports `np.array(range(n))`, `np.array([0] * n)`, `np.ones(n) * 3`, `np.zeros(n).astype(np.float32)` and `np.arange(n).astype(float)`. Each one builds the values twice, or builds a Python int for every value.

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

The rule reports `.flatten()` on the result of a NumPy call that returns a new array, `x.copy().astype(t)` and `np.array(x).copy()`. Each one copies an array that is already a new copy.

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

The rule reports `np.loadtxt(f)` and `np.genfromtxt(f)` in a file that also calls `np.savetxt(f, ...)`, and `np.fromfile(f)` in a file that also calls `x.tofile(f)`, with the same file argument. `np.save` and `np.load` keep the dtype and the shape.

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

### Math on whole arrays with a faster or more exact function

Call the NumPy function that each rule names, because it does the same work without a full sort,
a temporary array or a step that loses digits. Every rule in the group is a warning. The table
shows the speed that the message of each rule reports, measured with NumPy 2.5.5 on one core,
with BLAS and LAPACK, the linear algebra libraries that NumPy calls, on one thread. The other rules in the group give a more exact result and have no
measured speed.

| Rule | Reported form | Size |
| --- | --- | --- |
| [`sort-then-select`](#sort-then-select) | 39 times slower than `np.min` | 1,000,000 values |
| [`count-nonzero`](#count-nonzero) | 1.7 times slower than `np.count_nonzero` | 1,000,000 values |
| [`where-shortcut`](#where-shortcut) | 17 times slower than `np.asarray` | 1,000,000 values |
| [`reduce-last`](#reduce-last) | 8 times slower than `np.sum` | 1,000,000 values |
| [`power-half`](#power-half) | 2 times slower than `np.sqrt` | 1,000,000 values |
| [`sum-of-products`](#sum-of-products) | 2.4 times slower than `np.vdot` | 1,000,000 values |
| [`norm-by-hand`](#norm-by-hand) | 3.9 times slower than `np.linalg.norm` | 1,000,000 values |
| [`vecdot`](#vecdot) | 2 to 3.5 times slower than `np.vecdot` | 1,000,000 values |
| [`inv-then-multiply`](#inv-then-multiply) | 3.8 times slower than `np.linalg.solve` for a vector | a 1,000 by 1,000 matrix |
| [`eig-on-symmetric`](#eig-on-symmetric) | 3.6 times slower than `np.linalg.eigh` | a 1,000 by 1,000 matrix |
| [`matmul-diagonal`](#matmul-diagonal) | 34 times slower than `np.einsum` | 1,000 by 1,000 matrices |
| [`einsum-optimize`](#einsum-optimize) | 339 times slower than `optimize=True` | three 100 by 100 matrices |

#### sort-then-select

**Severity:** warning. **Fix:** unsafe: `np.min(x)`, `np.max(x)`, `np.sort(np.partition(x, 5)[:5])`, `np.sort(np.partition(x, -5)[-5:])`, `np.sort(x)`, `np.argmin(x)`, `np.argmax(x)`. The positions of the k smallest or largest and the rank by a double argsort have no fix.

The rule reports `np.sort(x)[0]`, `np.sort(x)[-1]`, `np.sort(x)[:5]`, `np.sort(x)[-5:]`, `np.sort(x)[::-1][:5]`, `x[np.argsort(x)]`, `np.argsort(x)[0]`, `np.argsort(x)[-1]`, `np.argsort(x)[:5]`, `np.argsort(x)[-5:]` and `np.argsort(np.argsort(x))`, with a literal count under 100.

Example:

```python
top = np.sort(scores)[-5:]
```

Use instead:

```python
top = np.sort(np.partition(scores, -5)[-5:])
```

#### count-nonzero

**Severity:** warning. **Fix:** safe: `np.any(x > 2)`. unsafe: `np.count_nonzero(x > 0)`, also for `len(x[x > 0])`, which gave a Python int.

The rule reports `np.sum(x > 0)`, `len(np.where(c)[0])`, `len(np.nonzero(c)[0])`, `len(np.flatnonzero(c))`, `len(x[x > 0])` and `np.sum(x > 2) > 0`.

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

The rule reports `np.where(x > 0, True, False)`, `np.where(x > 0, 1, 0)`, `np.where(x > 0, x, 0)`, `np.where(x < 0, 0, x)` and `np.arange(len(x))[x > 1]`.

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

The rule reports `np.exp(x) - 1`, `1 - np.exp(x)`, `np.log(1 + x)`, `np.log(x + 1)` and `np.log(1 - p)`, but not on a number literal.

Example:

```python
log_survival = np.log(1 - p)
```

Use instead:

```python
log_survival = np.log1p(-p)
```

#### logaddexp

**Severity:** warning. **Fix:** unsafe: `np.logaddexp(a, b)`, `np.logaddexp(0, x)`, `np.max(x) + np.log(np.sum(np.exp(x - np.max(x))))` and `np.exp(x - np.max(x)) / np.sum(np.exp(x - np.max(x)))`. The last bits change, an array of only -inf gives NaN, and an empty array raises. A softmax summed along an axis, and an operand that calls a function, have no fix.

The rule reports `np.log(np.exp(a) + np.exp(b))`, `np.log(1 + np.exp(x))`, `np.log1p(np.exp(x))`, `np.log(np.sum(np.exp(x)))` and `np.exp(x) / np.sum(np.exp(x))`.

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

The rule reports `np.log(np.prod(x))`.

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

The rule reports `np.cumsum(x)[-1]` and `np.cumprod(x)[-1]`.

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

The rule reports `np.power(x, 0.5)`.

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

The rule reports `np.sum(a * b)`, `np.sum(x ** 2)`, `np.sum(np.square(x))` and `np.mean((a - b) ** 2)`, but not when an operand is a number, a comparison, a conjugate or a new axis.

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

The rule reports `np.sqrt(np.sum(x ** 2))`, `np.sqrt(np.sum(x * x))`, `np.sqrt(np.sum(np.square(x)))`, `np.sqrt((x ** 2).sum())` and `np.sqrt((x * x).sum())`.

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

The rule reports `np.sum(x ** 2, axis=-1)` and `np.sum(np.square(x), axis=-1)`. It also reports `np.sum(a * b, axis=-1)` and `np.sum(a.conj() * b, axis=-1)` where `a` and `b` are names or subscripts. `np.vecdot` multiplies and sums in one pass without the temporary array, and ran 2 to 3.5 times faster on 1,000,000 values.

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

The rule reports `np.linalg.inv(a) @ b`, `np.linalg.inv(a).dot(b)`, `np.dot(np.linalg.inv(a), b)` and `np.linalg.inv(a) @ b @ c`, as in the normal equations.

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

The rule reports `np.log(np.linalg.det(a))` and `np.log(np.abs(np.linalg.det(a)))`.

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

The rule reports `np.linalg.eig` and `np.linalg.eigvals` on a matrix the code made symmetric: `x @ x.T`, `x.T @ x`, `np.dot(x, x.T)`, `np.dot(x.T, x)`, `a + a.T`, `np.cov(...)` and `np.corrcoef(...)`.

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

The rule reports `np.diag(a @ b)`, `np.diag(np.dot(a, b))`, `np.diagonal(a @ b)`, `np.diag(a @ b.T)`, `np.trace(a @ b)` and `np.trace(np.dot(a, b))`.

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

The rule reports `np.einsum` with three or more 2-D operands and a 2-D result, such as `"ij,jk,kl->il"`, and no `optimize=`.

Example:

```python
chain = np.einsum("ij,jk,kl->il", a, b, c)
```

Use instead:

```python
chain = np.einsum("ij,jk,kl->il", a, b, c, optimize=True)
```

### The API that NumPy's docs prefer on NumPy 2.5

Write the form that NumPy's docs prefer in place of each old form that the rules report, although
the old form still runs without a warning on NumPy 2.5.5. A rule reports an old form when NumPy's
docs call it legacy, not recommended or kept for backwards compatibility, or when the newer form
ran at least 1.5 times faster. A rule that states only a preference of the docs is info, and the
rest are warnings, as the table shows. xpd skips the rules that name an API added in NumPy 2.0 on
an older NumPy.

| Severity | Rules |
| --- | --- |
| warning | [`legacy-random`](#legacy-random), [`rng-in-loop`](#rng-in-loop), [`clock-seed`](#clock-seed) and [`unpackbits-popcount`](#unpackbits-popcount) |
| info | the other 10 rules of the group |

#### legacy-random

**Severity:** warning. **Fix:** none. A Generator draws a different stream from the same seed, and `np.random.seed` elsewhere would no longer reach the rewritten call.

The rule reports `np.random.seed`, `np.random.rand`, `np.random.randint`, `np.random.randn`, `np.random.choice`, every other distribution function in `np.random`, `np.random.get_state`, `np.random.RandomState(...)` outside files that import scikit-learn, and imports of the same names from `numpy.random`. The message names the Generator method, such as `integers` for `randint`. When the method ran at least 1.5 times faster, the message also gives the speedup in the table.

| Generator method | Faster by |
| --- | --- |
| `random` | 2 times |
| `integers` | 2.5 times |
| `exponential` | 2.4 times |
| `choice(..., replace=False)` | 280 times |

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

The rule reports `np.random.default_rng()` without a seed inside a `for` or `while` body or a comprehension. Each call gathers entropy, the random bits for a seed, from the operating system, which takes about 5 microseconds. The rule leaves a loop alone that seeds each pass from its loop variable.

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

The rule reports a seed taken from `time.time()`, `time.time_ns()`, `time.perf_counter()`, `datetime.now()` and similar clocks. Two runs that start in the same tick of the clock get the same numbers. NumPy recommends a seed from the operating system, and a log of the seed when a run must be repeatable.

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

**Severity:** info. **Fix:** unsafe: `np.polynomial.Polynomial.fit(x, y, deg).convert().coef[::-1]` for `polyfit`, and the `numpy.polynomial` function on `coef[::-1]` for `poly1d`, `polyval`, `roots`, `poly`, `polyder` and `polyint`, reversed again where coefficients come out. A poly1d argument raises, whole number coefficients become floats, roots come out sorted, and `poly1d` becomes a `Polynomial` with other methods. `polyadd`, `polysub`, `polymul`, `polydiv` and calls with other arguments have no fix.

The rule reports `np.polyfit`, `np.poly1d`, `np.polyval`, `np.roots`, `np.poly`, `np.polyder`, `np.polyint`, `np.polyadd`, `np.polysub`, `np.polymul` and `np.polydiv`.

Example:

```python
slope, intercept = np.polyfit(x, y, 1)
```

Use instead:

```python
intercept, slope = np.polynomial.Polynomial.fit(x, y, 1).convert().coef
```

#### almost-equal-assert

**Severity:** info. **Fix:** unsafe: `np.testing.assert_allclose(a, b, rtol=0, atol=1.5e-7)`, with `1.5e-6` for `assert_array_almost_equal` and `1.5e-<decimal>` for a literal decimal. A difference of exactly `atol` failed before and passes after, dicts and ragged lists are no longer compared item by item, and complex values are compared by magnitude. A bare name that the file imports from `numpy.testing` gets `assert_allclose(a, b, rtol=0, atol=1.5e-7)` in a file that also imports `assert_allclose` from `numpy.testing`. `assert_approx_equal(a, b)` gets `assert_allclose(a, b, rtol=1e-6)`, which can fail where a difference up to ten times `rtol` passed. Any other bare name, an `err_msg` argument, a computed decimal and `significant=` have no fix.

The rule reports `assert_almost_equal`, `assert_array_almost_equal` and `assert_approx_equal`, which NumPy lists under "Asserts (not recommended)".

Example:

```python
np.testing.assert_array_almost_equal(result, expected, decimal=4)
```

Use instead:

```python
np.testing.assert_allclose(result, expected, rtol=0, atol=1.5e-4)
```

#### assert-on-arrays

**Severity:** info. **Fix:** unsafe: `np.testing.assert_allclose(a, b, rtol=1e-05, atol=1e-08, equal_nan=False)`, with the `rtol` and `atol` that the call passes, and `np.testing.assert_array_equal(a, b)`, each with `err_msg=` for a message written as a string. assert_allclose fails on shapes that `np.allclose` broadcasts, assert_array_equal treats NaN in the same place as equal, and both still run under `python -O`. A message that is not a string, `equal_nan` and a splat have no fix.

The rule reports `assert np.allclose(a, b)`, `assert np.array_equal(a, b)`, `assert np.all(a == b)` and `assert (a == b).all()`. A failed assert shows only False, where the testing functions show the elements that differ.

Example:

```python
assert np.allclose(weights, expected)
```

Use instead:

```python
np.testing.assert_allclose(weights, expected, rtol=1e-05, atol=1e-08)
```

#### legacy-testing-helper

**Severity:** info. **Fix:** unsafe: `assert x` and `assert x, msg`, with each part in parentheses when the call is on more than one line. The statement is skipped under `python -O`, and its message is built only on failure. `assert_raises` has no fix.

The rule reports `numpy.testing.assert_`, which NumPy's testing guidelines call the legacy form of the assert statement. It also reports `numpy.testing.assert_raises` and `assert_raises_regex`, where the guidelines prefer `pytest.raises`. The rule reports a bare `assert_raises` only in a file that imports it from `numpy.testing`, because SciPy imports `pytest.raises` under that name.

Example:

```python
assert_(res.success, res.message)
```

Use instead:

```python
assert res.success, res.message
```

#### char-module

**Severity:** info. **Fix:** safe: `np.strings.upper` and every other name that is the same function in both modules. Unsafe: the comparisons, such as `np.strings.equal`, which do not strip trailing whitespace, and `np.strings.multiply`, which raises a different error on bad input. Unsafe also: `partition` and `rpartition`, because `np.strings` returns a tuple of three arrays.

The rule reports a function of `np.char` when `np.strings`, added in NumPy 2.0, has a function of the same name. NumPy's docs call `np.char` legacy.

Example:

```python
names = np.char.upper(names)
```

Use instead:

```python
names = np.strings.upper(names)
```

#### dot-on-matrices

**Severity:** info. **Fix:** unsafe: `a @ b`, with parentheses around an operand such as `a + 1`, and around the product where it is part of a larger operand, as in `(a @ b).T`. `@` raises on a scalar and on two Python lists, and on arrays of more than two dimensions it works on stacks of matrices, where `np.dot` does not.

The rule reports `np.dot(a, b)`, because the docs of `np.dot` prefer `matmul` or `a @ b` for two 2-D arrays.

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

The rule reports `kind="mergesort"` in `np.sort`, `np.argsort` and the `sort` and `argsort` methods. The docs keep `"mergesort"` only for backwards compatibility, and both names select the same stable sort.

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

The rule reports `np.set_printoptions(**saved)`, which restores options that the code changed earlier. The docs ask for `np.printoptions` as a context manager, which also restores the options when an error is raised.

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

The rule reports `as_strided(...)`, because a wrong shape or stride reads memory outside the array. Its docs ask for extreme care and point to `sliding_window_view`, `broadcast_to` and `reshape`.

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

The rule reports `import numpy as <name>` with a name other than `np` or `xp`, and `from numpy import *`, which replaces Python's `sum`, `min`, `max`, `round`, `abs`, `all` and `any`.

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

The rule reports `np.unpackbits(...).sum()`, `np.sum(np.unpackbits(...))` and `np.count_nonzero(np.unpackbits(...))`. `np.bitwise_count`, added in NumPy 2.0, counts the set bits of each value, and ran 37 times faster on 1,000,000 uint64 values.

Example:

```python
ones = np.unpackbits(masks.view(np.uint8)).sum()
```

Use instead:

```python
ones = np.bitwise_count(masks).sum()
```

## SciPy

The SciPy rules report code that raises or warns on SciPy 1.18.1, code that loses digits, and
SciPy calls made once per value, in each file that imports `scipy`. The examples assume
`import numpy as np`.

When a message uses a word from the table, the word means the result of a check on SciPy 1.18.1
with NumPy 2.5.5.

| Word in the message | What was measured |
| --- | --- |
| removed, deprecated | The old form raises under `python -W error`. |
| inexact | The old form lost at least 3 decimal digits against mpmath at 50 digits, and the form to write lost fewer than 1. |
| faster | The range over repeated runs on one core, at 1,000,000 values unless the rule says otherwise. The faster form was not slower at 1,000 values. |
| legacy, preferred | SciPy's own source calls the old form legacy or prefers the new one. Such rules have severity info and claim nothing about speed or accuracy. |

A rule reports a name only when the dotted name starts with `scipy`, or when the file's own
imports bind it from `scipy`. A form made only of NumPy names, such as `np.linalg.inv(a) @ b`, belongs to the
[NumPy](#numpy) rules. A rule that asks for an API newer than the project's SciPy is skipped.

| Fix | What it promises |
| --- | --- |
| safe | The same object, or a value of the same type that is equal bit for bit. xpd offers no safe fix where the file binds the name again or the call spreads `*args` or `**kwargs`. |
| unsafe | The message names what can differ. |

### Names that are gone or that warn

Replace each name that SciPy removed or deprecated with the name the rule gives, because a removed name raises on import or on the call.

#### removed-in-scipy

**Severity:** error. **Fix:** safe: `trapezoid`, `cumulative_trapezoid` and `simpson` for `trapz`, `cumtrapz` and `simps`, in the module the code already names or in an import as `trapezoid as trapz`, `scipy.signal.windows.<name>` for the 19 windows that `scipy.signal` held, such as `hann`, and `np.<name>` for the 438 names that were the NumPy object on the `scipy` namespace, such as `scipy.pi`. On SciPy 1.13.1, 1.12.0 and 1.3.3 each new form is the same object as the old one or gives a result with an equal pickle. Unsafe, because the results differ on a SciPy that runs both forms: `pinv` for `pinv2`, whose third positional argument was another tolerance, `median_abs_deviation` for `median_absolute_deviation`, which had another default scale, `gibrat` for `gilbrat`, which is another class, `stats.mstats.find_repeats`, which returns a plain tuple, `impulse`, `lsim` and `step` for `impulse2`, `lsim2` and `step2`, `betainc` and `betaincinv` for `btdtr` and `btdtri`, each also in an import as `new as old`, `np.kron`, `np.tri`, `np.tril` and `np.triu` for the `scipy.linalg` names, `np.<name>` for `sqrt`, `log` and the six other `numpy.lib.scimath` functions on the `scipy` namespace, and `scipy.datasets` for `face`, `ascent` and `electrocardiogram` on `scipy.misc`, which raises ImportError without the pooch package. Unsafe, because no SciPy that runs here holds the old form: `eye` for `speye`, `stats.mstats.kendalltau_seasonal`, `expm` for `expm2` and `expm3`, `stats.chi2.sf` for `stats.chisqprob`, `scipy.special.betainc` and `erfc` for `scipy.stats.betai` and `erfc`, `np.linalg.slogdet`, `sparse.csgraph.connected_components` for `sparse.cs_graph_components`, `scipy.special` for `comb`, `factorial` and the other names on `scipy.misc` or the `scipy` namespace, six windows such as `taylor`, and 46 NumPy names such as `np.round`. The other forms have no fix, because the name has no replacement that takes the same arguments, the message names two, or the new module is one that the code does not name, as in `from scipy import misc` or an import from the old module.

The rule reports SciPy names that raise on SciPy 1.18.1. Its message says what to write
instead, or says that SciPy has no public replacement.

| Old names | On SciPy 1.18.1 |
| --- | --- |
| Removed functions for integration, windows, wavelets, Legendre polynomials, spherical Bessel functions, statistics and linear algebra | The name is gone. |
| `interp2d` | A call raises `NotImplementedError`. |
| Everything in `scipy.misc` | The module is empty and warns when imported. |
| NumPy functions and constants on the `scipy` namespace, such as `scipy.pi` | The name is gone. |

The rule leaves alone code under a SciPy version check, a `hasattr` check, or a `try` that
catches `ImportError` or `AttributeError`. Such code exists to support old SciPy.

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

**Severity:** warning. **Fix:** safe: the public namespace for a name taken from one of the 107 private modules that SciPy deprecated, such as `scipy.sparse.csr_matrix` for `scipy.sparse.csr.csr_matrix` and `from scipy.ndimage import gaussian_filter` for `from scipy.ndimage.filters import gaussian_filter`, which give the same object, and `tri.find_simplex(xi)` for `scipy.spatial.tsearch(tri, xi)`, which is what `tsearch` returns. Unsafe: `vq` for `scipy.cluster.vq.py_vq`, and `vq as py_vq` in an import, because `vq` returns the codes as int32 where `py_vq` returned int64, and float32 distances for float32 input. The other forms have no fix, because the public module lacks the name or holds another object, an import of a whole private module would change every later use, or the replacement takes other arguments or is another package.

The rule reports names that warn with `DeprecationWarning` on SciPy 1.18.1, and its message
names the replacement that SciPy's warning names. The names are:

- every name taken from a private module, such as `scipy.stats.stats`, `scipy.linalg.basic`,
  `scipy.ndimage.filters` or `scipy.sparse.linalg.isolve`, and an import of such a module
- `scipy.odr`, which SciPy 1.19 removes
- calls of `minkowski_distance`, `minkowski_distance_p`, `distance_matrix`, `tsearch`,
  `lagrange`, `pade`, `approximate_taylor_polynomial` and `py_vq`

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

### Arguments, defaults and legacy functions

Change each argument that SciPy removed, deprecated or ignores, and move each legacy function to the interface that SciPy's documentation names as its replacement.

#### removed-keyword

**Severity:** error. **Fix:** safe: `rtol=` for `tol=` on the sparse.linalg solvers, `restart=` for gmres `restrt=`, `assume_a='pos'` for linalg.solve `sym_pos=True`, `subset_by_index=` for eigh and eigvalsh `eigvals=`, `workers=` for `n_jobs=` on the cKDTree query methods, and `scale=1.0` for iqr `scale='raw'`, and it deletes kendalltau `initial_lexsort=` and fmin_l_bfgs_b `iprint=` and `disp=`. On the last SciPy that takes the old form, from 1.8.1 to 1.17.1, each new form gives a result with an equal pickle, and fmin_l_bfgs_b prints nothing either way. Unsafe: it deletes simpson `even=`, because `even='avg'` gives another result, and it writes `workers=` for `n_jobs=` on KDTree and `scale=1.0` for median_abs_deviation `scale='raw'`, because neither ever took the old form. The other forms have no fix, because the call already passes the new keyword, or the message names two rewrites, or the value the new form needs is not in the file.

The rule reports keywords and argument values that SciPy removed and that raise on SciPy 1.18.1.

| Function | Removed form | Fix |
| --- | --- | --- |
| `cg`, `gmres` and the other iterative solvers of `scipy.sparse.linalg` | `tol=`, `atol='legacy'`, a fourth positional argument | `rtol=` for `tol=` |
| `gmres` | `restrt=` | `restart=` |
| `linalg.solve` | `sym_pos=` | `assume_a='pos'` for `sym_pos=True` |
| `linalg.eigh`, `linalg.eigvalsh` | `eigvals=`, `turbo=` | `subset_by_index=` for `eigvals=` |
| `linalg.pinv`, `linalg.pinvh` | `cond=`, `rcond=` | none |
| `integrate.simpson` | `even=` | deletes it |
| `integrate.cumulative_trapezoid` | an `initial` other than 0 | none |
| the query methods of `KDTree` and `cKDTree` | `n_jobs=` | `workers=` |
| `stats.kendalltau` | `initial_lexsort=` | deletes it |
| `stats.iqr`, `stats.median_abs_deviation` | `scale='raw'` | `scale=1.0` |
| `optimize.fmin_l_bfgs_b` | `iprint=`, `disp=` | deletes them |
| `spatial.distance.cdist`, `spatial.distance.pdist` | the metric `'wminkowski'` | none |

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

**Severity:** warning. **Fix:** safe: deletes lombscargle `precenter=False`, which is what the default does. Unsafe: deletes qr `lwork=`, because an `lwork` below the number of rows raised ValueError. The other forms have no fix, because `precenter=True` needs `y` centred too, the positional forms need the names of the arguments after them, and `variant=` and `method='highs'` change the result or its options.

The rule reports arguments that warn on SciPy 1.18.1 and that SciPy will remove. The forms whose
message names `variant=` are skipped on SciPy older than 1.17.

| Function | Argument that warns |
| --- | --- |
| `signal.lombscargle` | `precenter=`, and a fourth or fifth positional argument |
| `linalg.qr` | `lwork=`, by keyword or by position |
| `stats.anderson_ksamp` | `midrank=`, by keyword or by position |
| `optimize.linprog` | `method='simplex'`, `'interior-point'` or `'revised simplex'` |

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

**Severity:** warning. **Fix:** safe: `callback_type='legacy'` for gmres with a callback, and `spmatrix=True` for hb_read and mmread. Each is what SciPy 1.18.1 does after it warns. Unsafe: `mode='grid-constant'` for zoom with `grid_mode=True` and no `mode=`, because the grid mode pads in another way and the values near the edges change. The other forms have no fix, because the result of anderson and anderson_ksamp changes type, and an explicit zoom `mode=` is outside the match.

The rule reports calls that rely on a default that warns on SciPy 1.18.1. A form whose message
names an argument added in SciPy 1.17 or 1.18 is skipped on an older SciPy.

| Call | Default that warns | Fix |
| --- | --- | --- |
| `gmres` with `callback=` | no `callback_type=` | `callback_type='legacy'` |
| `stats.anderson` | no `method=` | none |
| `stats.anderson_ksamp` | no `variant=` | none |
| `io.hb_read`, `io.mmread` | no `spmatrix=` | `spmatrix=True` |
| `ndimage.zoom` with `grid_mode=True` | mode `'constant'`, the default, or `'wrap'` | `mode='grid-constant'` when the call passes no `mode=` |

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

**Severity:** warning, and error with `'trust-constr'`, where the key raises TypeError. **Fix:** none.

The rule reports a key in the `options=` dict of `optimize.minimize` that the chosen `method=`
doesn't take but another method takes. Examples are `disp` and `iprint` for L-BFGS-B, `maxiter`
for TNC and `maxfun` for BFGS. On SciPy 1.18.1, minimize warns OptimizeWarning and ignores such a
key, and with `'trust-constr'` it raises TypeError. xpd took the keys of each method from a run of
minimize with each pair of method and key.

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

The rule reports an argument of `optimize.minimize` that the chosen method ignores. On SciPy
1.18.1, minimize warns RuntimeWarning and goes on without the argument.

| Argument | Methods that ignore it |
| --- | --- |
| `jac=` | Nelder-Mead, Powell, COBYLA, COBYQA |
| `hess=`, `hessp=` | each method that uses no Hessian |
| `constraints=` | each method that can't handle constraints |
| `bounds=` | CG, BFGS, Newton-CG, and the trust-region methods other than trust-constr |

Example:

```python
res = minimize(f, x0, method="BFGS", jac=grad, bounds=[(0, 1)] * n)
```

Use instead:

```python
res = minimize(f, x0, method="L-BFGS-B", jac=grad, bounds=[(0, 1)] * n)
```

#### rng-keyword

**Severity:** info. **Fix:** unsafe: `rng=` for `random_state=` and `seed=`. The same integer gives a different stream under `rng=`, and `qmc.Sobol` raises for a `RandomState` instance passed as `rng=`.

The rule reports `random_state=` and `seed=` passed by keyword to the functions and classes that
SciPy 1.15 moved to `rng=`. SciPy's docs say new code should pass `rng`. Examples are
`sparse.random`, `svds`, `bootstrap`, `permutation_test`, `kmeans2`, `differential_evolution`,
`basinhopping`, `dual_annealing`, `directed_hausdorff`, `Rotation.random` and the qmc engines.
The old keywords run without a warning on SciPy 1.18.1.

The rule is skipped on SciPy older than 1.15. It leaves alone a call under a SciPy version check,
because `rng=` raises on an older SciPy. xpd reads only the installed SciPy and can't see which
SciPy versions a library supports. The message therefore says to keep the old keyword when the
code must also run on SciPy older than 1.15.

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

The rule reports the functions that `scipy.optimize` lists as legacy and not recommended for new
scripts. Its message names the call that replaces each one:

| Legacy function | Write instead |
| --- | --- |
| `fmin`, `fmin_powell`, `fmin_cg`, `fmin_bfgs`, `fmin_ncg`, `fmin_l_bfgs_b`, `fmin_tnc`, `fmin_cobyla`, `fmin_slsqp` | `minimize(fun, x0, method=...)` with `'Nelder-Mead'`, `'Powell'`, `'CG'`, `'BFGS'`, `'Newton-CG'`, `'L-BFGS-B'`, `'TNC'`, `'COBYLA'` or `'SLSQP'` |
| `fminbound` | `minimize_scalar(fun, bounds=(a, b), method='bounded')` |
| `brent`, `golden` | `minimize_scalar(fun, method='brent')` or `method='golden'` |
| `leastsq` | `least_squares(fun, x0)` |
| `fsolve` | `root(fun, x0)` |
| `broyden1`, `broyden2`, `anderson`, `excitingmixing`, `linearmixing`, `diagbroyden` | `root(fun, x0, method=...)` with the same name |
| `newton_krylov` | `root(fun, x0, method='krylov')` |

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

The rule reports `integrate.odeint`, because SciPy's docs ask new code to use `solve_ivp`.
Neither solver was faster on every problem measured, so the message makes no claim about speed.
The message also tells you what changes when you switch:

- `solve_ivp` raises ValueError when `t_eval` repeats a time, which `odeint` accepts.
- The values differ in the last digits.
- Keep the default method of `solve_ivp` for a large system, because `method='LSODA'` stores a
  full Jacobian and raised MemoryError at 1,000,000 equations.

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

The rule reports the interpolation classes and functions that SciPy marks as legacy, and its
message names the replacement that SciPy recommends:

| Legacy name | Write instead |
| --- | --- |
| `interp1d` | `make_interp_spline(x, y, k=1)` for linear interpolation, with `axis=-1` when `y` has more than one dimension |
| `Rbf` | `RBFInterpolator`, which takes the points as one (n, d) array |
| `UnivariateSpline`, `splrep` | `make_splrep` |
| `InterpolatedUnivariateSpline` | `make_interp_spline` |
| `LSQUnivariateSpline` | `make_lsq_spline` |
| `splprep` | `make_splprep` |
| `splev` | a `BSpline` that you call, with `nu=n` where `splev` took `der=n` |
| `splint`, `splder`, `splantider` | the `integrate`, `derivative` and `antiderivative` methods of a `BSpline` |
| `spalde` | the derivative of a `BSpline` |
| `sproot` | `PPoly.from_spline(spl).roots()` |

The message for `interp1d` says what each function does outside the range of the data, as the
table shows. A form whose replacement was added in SciPy 1.7 or 1.15 is skipped on an older SciPy.

| Function | Outside the range of the data |
| --- | --- |
| `interp1d` | raises |
| `make_interp_spline` | extrapolates |
| `numpy.interp` | repeats the end value |

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

The rule reports `signal.stft`, `istft`, `spectrogram` and `check_COLA`, which SciPy marks as
legacy in favor of the `ShortTimeFFT` class. It also reports `signal.sosfreqz`, a legacy alias
of `freqz_sos`, but not on SciPy older than 1.15.

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

The rule reports names that SciPy keeps for old code:

| Name | What SciPy says | Write instead |
| --- | --- | --- |
| `ndimage.sum` | kept for backwards compatibility | `ndimage.sum_labels` |
| `stats.ranksums` | legacy | `stats.mannwhitneyu`, which gives the same p-value and the U statistic in place of z |
| `sparse.bmat` | returns a sparse matrix | `sparse.block_array`, which returns a sparse array |

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

Use the SciPy function that computes the quantity directly, because the two-step form loses digits or overflows where the direct function doesn't.

#### log-of-special

**Severity:** warning. **Fix:** unsafe: `special.log_ndtr(x)`, `special.gammaln(x)`, `special.gammaln(n + 1)`, `special.betaln(a, b)`, `special.log_expit(x)`, `special.log_softmax(x)`. `log_expit` needs SciPy 1.8 and `log_softmax` SciPy 1.5. A function imported by name is fixed only in a file that imports the log form from `scipy.special`.

The rule reports `np.log` of a `scipy.special` function that has a log form of its own, under
any import of `scipy.special`. The table shows where the old form overflowed or lost digits.

| Old form | Where the old form failed | Write instead |
| --- | --- | --- |
| `np.log(special.ndtr(x))` | -inf below -37.75 | `special.log_ndtr(x)` |
| `np.log(special.gamma(x))` | inf at 200 | `special.gammaln(x)` |
| `np.log(special.factorial(n))` | inf at 200 | `special.gammaln(n + 1)` |
| `np.log(special.beta(a, b))` | -inf at a = b = 1e6 | `special.betaln(a, b)` |
| `np.log(special.expit(x))` | 3 or more digits lost above 8 and below -710 | `special.log_expit(x)` |
| `np.log(special.softmax(x))` | -inf for an entry more than 745 below the largest | `special.log_softmax(x)` |

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

The rule reports `np.log` of the `pdf`, `pmf`, `cdf` or `sf` of a `scipy.stats` distribution,
written as `stats.norm`, imported by name as `norm`, or frozen in the same expression as
`stats.norm(0, 2)`. Each method has its own list of distributions, and each distribution on a
list was measured in a tail. `np.log(stats.binom.pmf(k, n, p))` is not reported, because `binom.logpmf` lost up to 3.6
digits more near the mode.

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

The rule reports `1 - dist.cdf(...)` and `1 - dist.sf(...)` for a `scipy.stats` distribution
from a measured list. For `stats.norm`, `1 - cdf` lost 3 or more digits above 4.25 and gave 0
above 8.3. `lognorm`, `f`, `poisson`, `binom`, `nbinom` and `geom` are not on the list, because
their `sf` lost digits too.

Example:

```python
pvalue = 2 * (1 - stats.norm.cdf(np.abs(z)))
```

Use instead:

```python
pvalue = 2 * stats.norm.sf(np.abs(z))
```

#### special-complement

**Severity:** warning. **Fix:** unsafe: `special.erfc(x)`, `special.erf(x)`, `special.gammaincc(a, x)`, `special.gammainc(a, x)`, `special.betaincc(a, b, x)`, `special.erfc(-x)`, `special.ndtr(-x)`, `special.expit(-x)`. `betaincc` needs SciPy 1.11. `erf`, `erfc`, `gammainc`, `gammaincc` and `betainc` imported by name are fixed only in a file that imports the new name from `scipy.special`.

The rule reports 1 plus or minus a `scipy.special` function, where SciPy has a function that
keeps the digits of a small result. The old form lost 3 or more digits in the following ranges:

| Old form | Digits lost for | Write instead |
| --- | --- | --- |
| `1 - special.erf(x)` | x from 2.75 to 26.5 | `special.erfc(x)` |
| `1 - special.erfc(x)` | x below 6e-5 | `special.erf(x)` |
| `1 - special.gammainc(a, x)`, `1 - special.gammaincc(a, x)` | the tails, where the result falls below about 1e-4 | `special.gammaincc(a, x)`, `special.gammainc(a, x)` |
| `1 - special.betainc(a, b, x)` | x from 0.32 to 0.95, with a = 2 and b = 30 | `special.betaincc(a, b, x)` |
| `1 + special.erf(x)` | x below -2.75 | `special.erfc(-x)` |
| `1 - special.ndtr(x)` | x above 4.25 | `special.ndtr(-x)` |
| `1 - special.expit(x)` | x above 8 | `special.expit(-x)` |

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

The rule reports `dist.ppf(1 - p, ...)` and `dist.isf(1 - p, ...)` for a continuous
`scipy.stats` distribution from a measured list, and `special.ndtri(1 - p)`. The old form lost 3
or more digits for every p below 5e-7. A probability written as a number, such as `1 - 0.05`,
is left alone unless it is below 5e-7.

Example:

```python
crit = stats.norm.ppf(1 - alpha / 2)
```

Use instead:

```python
crit = stats.norm.isf(alpha / 2)
```

#### comb-by-factorials

**Severity:** warning. **Fix:** unsafe: `special.comb(n, k)`. `factorial` imported by name is fixed only in a file that imports `comb` from `scipy.special`.

The rule reports `special.factorial(n) / (special.factorial(k) * special.factorial(n - k))`, and
the same with the factors in another order or divided one at a time. `factorial` overflows to inf
above 170, so at n = 200 and k = 3 the ratio gave nan. `comb` lost no digits at the same n and k.

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

The rule reports `np.log(linalg.det(a))`, `np.log(np.abs(linalg.det(a)))` and
`np.log(abs(linalg.det(a)))`, where `linalg` is `scipy.linalg` or `det` was imported from it. On two
covariance matrices the old form gave -inf and inf, where `slogdet` lost less than one digit, as
the table shows. `np.log(np.linalg.det(a))` belongs to the NumPy rule
[`log-of-det`](#log-of-det).

| Covariance matrix | `np.log(linalg.det(a))` | Digits that `slogdet` lost |
| --- | --- | --- |
| 120 features at scale 1e-3 | -inf | 0.54 |
| 80 features at scale 1e4 | inf | 0.27 |

Example:

```python
logdet = np.log(linalg.det(cov))
```

Use instead:

```python
logdet = np.linalg.slogdet(cov)[1]
```

### SciPy calls made once per value or once per pair

Replace each SciPy call that Python makes once per value, once per pair of points or once per
pass of a loop with one call on whole arrays, which does the same work in compiled code. Each
speedup is a median of 7 runs on one core with SciPy 1.18.1, on float64 and float32, at 1,000,000
values unless the rule names another size.

#### distance-per-pair

**Severity:** warning. **Fix:** none.

The rule reports a metric function of `scipy.spatial.distance`, such as `euclidean`, `cityblock`
or `cosine`, called inside a comprehension, a generator or a loop body. On 1,000 by 1,000 points,
`cdist` ran 960 to 2,100 times faster.

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

The rule reports a metric function of `scipy.spatial.distance` passed to `cdist` or `pdist` as
the metric, by position or as `metric=`. The metric's name as a string runs the same metric in C, and
on 1,000 by 1,000 points it ran 1,100 to 2,100 times faster.

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

The rule reports a comprehension or generator whose element is one SciPy call with the loop name
as an argument. A filter with `if`, a second `for`, and an element that does more than the call
are left alone.

| Element of the comprehension | One call on the array was faster by |
| --- | --- |
| a `scipy.special` ufunc | 7 to 23 times |
| a `scipy.stats` distribution method, such as `norm.cdf` | 92 to 730 times |
| a distribution frozen from the loop name, such as `norm(m, 2.0).pdf(x)` | 16 to 17 times |
| `stats.scoreatpercentile(x, p)` over 1,000 percentiles, which sorts the data on every call | 2.1 to 3.2 times |

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

The rule reports `tree.query(q)` inside a comprehension or a `for` loop, where `q` is the loop
name or a row indexed by it. `tree` must come from a `KDTree` or `cKDTree` call in the same
function. For 1,000,000 points on a tree of 10,000 points, one query of all points ran 16 to 17
times faster.

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

The rule reports `scipy.sparse.linalg.spsolve(A, b)` and `scipy.linalg.solve(A, b)` inside a
comprehension or a loop that doesn't assign `A` or take it as its loop name. Each call factors
`A` again, and factoring once was faster:

| Matrix | Right sides | Factoring once was faster by |
| --- | --- | --- |
| sparse, 10,000 rows | 100 | 17 to 20 times |
| dense, 1,000 by 1,000 | 20 | 18 to 23 times |

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

**Severity:** warning. **Fix:** unsafe: `spla.spsolve(A, b)` when `inv` is reached through its module. The last bits change, and a `b` of shape `(n, 1)` gives a 1-D result. `inv` imported by name is fixed only in a file that imports `spsolve` from `scipy.sparse.linalg`.

The rule reports `scipy.sparse.linalg.inv(A) @ b` and `inv(A).dot(b)`, because the inverse of
most sparse matrices is dense. On a tridiagonal matrix of 1,000 rows, whose inverse holds 1,000,000
values, `spsolve` ran 260 to 270 times faster.

Example:

```python
x = spla.inv(A) @ b
```

Use instead:

```python
x = spla.spsolve(A, b)
```

### Sparse matrices

Build a sparse matrix in a format made for writes, and use the sparse array classes in place of the matrix classes.

#### empty-csr-write

**Severity:** warning. **Fix:** none.

The rule reports a write into a `csr_matrix`, `csc_matrix`, `csr_array` or `csc_array` built from
a shape alone, as in `A = csr_matrix((n, m))` followed by `A[i, j] = v`, `A[i, j] += v` or a slice
write, in a function that doesn't assign `A` again. Each write adds a stored value and copies the
whole matrix. On SciPy 1.18.1 a write emits `SparseEfficiencyWarning`, which raises under
`python -W error`. Moreover, 1,000 writes ran 30 times slower than the same writes into a
`lil_matrix`.

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

The rule reports a call to a sparse matrix class or builder whose SciPy docstring points to the
sparse array API:

| Old call | What the docstring says |
| --- | --- |
| `csr_matrix`, `csc_matrix`, `coo_matrix`, `lil_matrix`, `dok_matrix`, `bsr_matrix`, `dia_matrix` | the sparse matrix interface is expected to be deprecated |
| `eye`, `identity`, `diags`, `spdiags`, `rand`, `random`, `bmat` | it encourages `eye_array`, `diags_array`, `dia_array`, `random_array` and `block_array` |

A function that also names a sparse array class, and a branch taken on a condition that mentions
a matrix, are left alone, because such code builds the matrix on purpose.

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

Use the class or module that SciPy's documentation prefers, because the older name stays only for old code.

#### ckdtree

**Severity:** info. **Fix:** unsafe: `KDTree` for `cKDTree`, with the same arguments. The default `leafsize` differs, 16 against 10, so a query can return another one of two equally near points, and the tree's node classes differ. `cKDTree` imported by name is fixed only in a file that imports `KDTree` from `scipy.spatial`. Skipped on SciPy older than 1.6.

The rule reports a call to `cKDTree`, whose docstring says it is functionally identical to
`KDTree` and kept only for backward compatibility.

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

The rule reports an import of `scipy.fftpack`, whose module docstring marks it as legacy and
says new code should use `scipy.fft`. `scipy.fft` was no faster in the measurements, so the rule
claims only the preference.

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

The scikit-learn rules report code that gives a leaked or wrong score, code that raises or warns
on scikit-learn 1.9.1, and code that does the same work twice, in each file that imports
`sklearn`. Every number about the scikit-learn rules was measured on scikit-learn 1.9.1. The examples assume
that the scikit-learn names they use are imported, and names such as `X_train`, `y_test` and
`model` stand for values from your program.

Several rules read the names of variables, such as `X_test` and `y_pred`, because xpd reads the
code and never the values it holds. A name that misleads therefore gives a wrong report.

A rule document whose advice needs a newer release of scikit-learn carries `since`. xpd reads the
installed version from the project's venv and skips the document on an older release. When the
venv has no scikit-learn, or there is no venv, every document runs.

### The proof each rule needs

Each rule document ships only with a saved proof that its old form is wrong. A proof is one of
the following kinds:

| Proof | What runs | When it passes |
| --- | --- | --- |
| score | The old and the new form, over 30 seeds on data with no signal, where the honest score is 0.5. | The old form's mean is more than three standard errors above the honest score, and the new form's mean is within 0.02 of it. |
| interpreter | The old form under `python -W error`. | The old form raises an exception or a warning, and the new form runs clean. |
| speed | Both forms on one thread, as the median of 7 runs, at two data sizes. | The new form is at least 1.5 times faster at both sizes. |

A document is `info` when no proof passed for it. Such a document either changes a score in a
way the proofs can't show, or changes nothing that can be measured. For example, a ranking metric
given class labels scores 0.5 on data with no signal in both forms, and a missing seed changes
nothing that can be measured.

The rules were then judged on 1,414 held-back files from 268 public repositories, which were set
aside before any rule was written. The judging had the following parts:

1. Two readers who did not write the rules labelled the same 478 reports, each without the
   other, and marked every report whose advice was wrong.
2. Recall was measured by planting 2,749 old forms in copies of the held-back files and counting
   the plants that a rule reported.
3. Every fix was run before and after, on its held-back sites and on cases built for each form,
   and the fitted model and its predictions were compared bit for bit.

| Rule | Severity | Proof | Held-back reports | Wrong advice | Recall | Fix |
| --- | --- | --- | --- | --- | --- | --- |
| [`fit-on-test`](#fit-on-test) | error, info | score: 0.997 against 0.525 AUC | 20 | 0 and 0 of 20 | 103 of 130 | unsafe |
| [`fit-before-split`](#fit-before-split) | error | score: 0.771 against 0.497 AUC | 24 | 0 and 0 of 24 | 91 of 162 | none |
| [`cross-val-on-fitted-data`](#cross-val-on-fitted-data) | error | score: 0.764 against 0.518 AUC | 10 | 0 and 0 of 10 | 72 of 78 | none |
| [`tune-on-test`](#tune-on-test) | error | score: 0.597 against 0.507 accuracy | 1 | 0 and 0 of 1 | 15 of 50 | none |
| [`stateful-function-transformer`](#stateful-function-transformer) | info | none of the three | 0 | no reports | 24 of 30 | none |
| [`groups-ignored`](#groups-ignored) | error | score: 0.849 against 0.480 accuracy | 0 | no reports | 56 of 57 | unsafe |
| [`missing-groups`](#missing-groups) | error | interpreter: ValueError | 0 | no reports | 48 of 50 | none |
| [`leave-one-out-score`](#leave-one-out-score) | error | interpreter: UndefinedMetricWarning | 0 | no reports | 14 of 33 | none |
| [`auc-from-labels`](#auc-from-labels) | error, info | score: log loss 18.00 against 0.70 | 20 | 2 and 2 of 20 | 55 of 72 | unsafe |
| [`metric-arguments-swapped`](#metric-arguments-swapped) | error, info | score: precision 0.471 against 0.294 | 43 | 0 and 0 of 30 | 66 of 100 | unsafe |
| [`score-on-training-rows`](#score-on-training-rows) | error | score: 1.000 against 0.510 accuracy | 0 | no reports | 10 of 27 | none |
| [`unknown-scoring-name`](#unknown-scoring-name) | error | interpreter: InvalidParameterError, ValueError | 4 | 0 and 0 of 4 | 16 of 32 | none |
| [`removed-in-sklearn`](#removed-in-sklearn) | error | interpreter: ModuleNotFoundError, ImportError, TypeError, InvalidParameterError | 321 | 1 and 0 of 58 | 517 of 600 | unsafe |
| [`deprecated-in-sklearn`](#deprecated-in-sklearn) | warning | interpreter: FutureWarning | 101 | 3 and 0 of 57 | 274 of 308 | safe, unsafe |
| [`removed-metric-api`](#removed-metric-api) | error | interpreter: ImportError, TypeError | 13 | 0 and 0 of 13 | 74 of 74 | unsafe |
| [`deprecated-metric-api`](#deprecated-metric-api) | warning | interpreter: FutureWarning | 0 | no reports | 48 of 48 | safe |
| [`fit-params-keyword`](#fit-params-keyword) | error | interpreter: TypeError | 4 | 0 and 0 of 4 | 32 of 32 | unsafe |
| [`column-transformer-removed`](#column-transformer-removed) | error | interpreter: TypeError | 3 | 0 and 0 of 3 | 30 of 30 | unsafe |
| [`metadata-routing`](#metadata-routing) | error | interpreter: RuntimeError, ValueError, TypeError, UnsetMetadataPassedError | 0 | no reports | 34 of 34 | unsafe |
| [`pipeline-fit-params`](#pipeline-fit-params) | error | interpreter: ValueError | 0 | no reports | 23 of 30 | none |
| [`pipeline-step-form`](#pipeline-step-form) | error | interpreter: ValueError, TypeError | 0 | no reports | 29 of 35 | unsafe |
| [`transformed-target-pair`](#transformed-target-pair) | error | interpreter: ValueError | 0 | no reports | 31 of 36 | unsafe |
| [`pandas-output-sparse`](#pandas-output-sparse) | error | interpreter: ValueError | 0 | no reports | 29 of 31 | unsafe |
| [`predict-per-row`](#predict-per-row) | warning | speed: 2.77 times | 2 | 0 and 2 of 2 | 28 of 32 | none |
| [`score-after-predict`](#score-after-predict) | warning | speed: 1.62 times | 17 | 0 and 0 of 17 | 31 of 32 | unsafe |
| [`predict-and-predict-proba`](#predict-and-predict-proba) | warning | speed: 1.83 times | 22 | 0 and 0 of 22 | 30 of 30 | none |
| [`cross-val-score-per-metric`](#cross-val-score-per-metric) | warning | speed: 1.60 times | 14 | 0 and 0 of 14 | 30 of 30 | none |
| [`vectorizer-fit-then-transform`](#vectorizer-fit-then-transform) | warning | speed: 1.63 times | 12 | 0 and 4 of 12 | 19 of 25 | unsafe |
| [`refit-per-n-estimators`](#refit-per-n-estimators) | warning | speed: 1.64 times | 0 | no reports | 30 of 30 | none |
| [`tsne-exact`](#tsne-exact) | info | none of the three | 0 | no reports | 21 of 30 | unsafe |
| [`random-state-without-shuffle`](#random-state-without-shuffle) | error | interpreter: ValueError | 9 | 0 and 0 of 9 | 54 of 54 | none |
| [`missing-random-state`](#missing-random-state) | info | none of the three | 527 | 1 and 1 of 52 | 79 of 80 | none |

| Column | What it gives |
| --- | --- |
| Proof | A score proof gives the old form's mean against the new form's mean. A speed proof gives the lowest ratio over the classes the rule reports and both sizes. |
| Wrong advice | The reports that reader A and reader B called wrong, out of the reports they labelled. A rule with more reports than labels was sampled. A rule with no report on the held-back files has no number. |
| Recall | The plants that the rule reported, out of the plants made for it. |

The rules were narrowed after the readers' labels came in. For a narrowed rule, wrong advice
counts the labelled reports that the rule still makes and that are still wrong, so the number is
not a clean measurement. Recall was measured on the rules before that narrowing.

Recall on code in other projects is likely lower than the table says. Most plants were inserted
into a file in shapes the judges chose, and such shapes are closer to the documented examples than
the code in other projects is. For the metric and pipeline rules, recall was 190 of 280 on plants turned from a right
form already in the file, against 311 of 357 on inserted plants.

### Leakage

Each rule in the group reports a form that lets the test rows, or every row, shape what the
model learns, so the score on the test rows is too high. The score proofs use data whose target
is drawn apart from every feature, where an honest AUC is 0.5, and give the mean over 30 seeds.

A step that doesn't read the target moved the test AUC by at most 0.006 when it was fitted before
the split, so the rules leave it alone. Such steps are a scaler, an imputer, PCA, a binner, a
one-hot encoder and a text vectorizer.

#### fit-on-test

**Severity:** error for `fit` with a target and for `partial_fit`, and info for `fit_transform` and for `fit` without a target. **Fix:** unsafe: `scaler.transform(X_test)` for `scaler.fit_transform(X_test)`, and `selector.transform(X_test)` for `selector.fit_transform(X_test, y_test)` on a named encoder or selector. A fit on test rows can be meant, and transform raises on a step that was never fitted. The other forms have no fix.

The rule reports `model.fit(X_test, y_test)` and `model.partial_fit(X_test, y_test)`, which train
on the test rows. It also reports `scaler.fit_transform(X_test)` and `scaler.fit(X_test)`, which
refit a step on them.

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

The rule reads the name of the first argument, and for `fit_transform` also the name of the step:

| Call | Name of the first argument | Name of the step |
| --- | --- | --- |
| `fit` | holds `test` | not read |
| `partial_fit` | holds `test` or `holdout`, or is a validation name such as `X_val` | not read |
| `fit_transform` | holds `test` or `holdout`, or is a validation name such as `X_val` | such as `scaler`, `imputer`, `encoder`, `ohe` or `tfidf` |

The rule reads the step's name, because code that refits `PolynomialFeatures` or a step for a plot
does it on purpose.

A refitted step can't move a score on data with no signal, so the forms without a target are
info. With the target, only some steps leaked, so `fit_transform` with a target is info as well.

| Form | Data | Result |
| --- | --- | --- |
| `LogisticRegression` fitted on 60 test rows and scored on them | no signal | 0.997 AUC, against 0.525 when fitted on the training rows |
| five `partial_fit` calls on the test rows | no signal | 0.924 AUC, against 0.518 |
| category_encoders `TargetEncoder` refitted on the test rows with the target | no signal | 0.992 AUC |
| `SelectKBest` refitted on the test rows with the target | no signal | 0.438 AUC |
| scikit-learn `TargetEncoder` refitted on the test rows with the target | no signal | 0.499 AUC |
| `StandardScaler` refitted on batches of 5 test rows | signal | 0.048 AUC lost |
| `OrdinalEncoder` refitted on 30 test rows | signal | 0.058 AUC lost |

The rule leaves alone `PermutationImportance` and `permutation_importance`, a fit on train and
test rows joined, a `LabelBinarizer` or `MultiLabelBinarizer`, and a `LabelEncoder` whose result
goes straight into another call, such as a plot.

The fix scales or codes the test rows with the statistics of the training rows. On the cases built
for it, the fix changed the test matrix by up to 0.60 and some predicted labels, and a test level
that the step hasn't seen now raises. At 4 of 17 held-back sites the fixed code raised, because the scaler
was built again on the line before, or one `LabelEncoder` was last fitted on another column.

The rule as the readers saw it gave wrong advice in 3 and 3 of 23 labelled reports, and the
narrowed rule no longer makes the reports. Recall was 41 of 45 for the error forms and 62 of 85
for the info forms. The rule misses a step whose name it doesn't read, such as
`rs = RobustScaler()` or `imp`, and a test name with no separator, such as `Xtest`. It also
misses held-out rows named `X_dev`, `X_te` or `X_cv`.

#### fit-before-split

**Severity:** error. **Fix:** none.

The rule reports a step that learns from the target and is fitted on every row before
`train_test_split`, such as `selector.fit_transform(X, y)`, `encoder.fit(X, y).transform(X)`,
`fit_resample`, `resample(..., replace=True)`, a target mean per group from
`groupby(...)[target].transform("mean")`, and columns picked by `df.corr()[target]`.

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

xpd gives the replacement as advice and doesn't apply it.

The rule reports the fit when a later statement of the same block passes its result to
`train_test_split`. On data with no signal, the steps gave the following mean test AUC:

| Step fitted on every row | Before the split, against after | Reported |
| --- | --- | --- |
| `SelectKBest` | 0.771 against 0.497 | yes |
| `SelectFromModel` | 0.684 against 0.519 | yes |
| `RFE` | 0.750 against 0.495 | yes |
| `RFECV` | 0.739 against 0.506 | yes |
| `SequentialFeatureSelector` | 0.551 against 0.477 | yes |
| category_encoders `TargetEncoder` | 0.880 against 0.495 | yes |
| `TargetEncoder().fit(X, y).transform(X)` | 0.889 against 0.509 | yes |
| a target mean per group | 0.887 against 0.497 | yes |
| 10 columns picked by `df.corr()` | 0.771 against 0.497 | yes |
| `SMOTE` | 0.815, against 0.478 to 0.493 for the samplers after the split | yes |
| `RandomOverSampler` | 0.737 | yes |
| `ADASYN` | 0.819 | yes |
| `EditedNearestNeighbours` | 0.678 | yes |
| `resample(replace=True)` | 0.970 | yes |
| `RandomUnderSampler` | 0.512 against 0.491 | no, it only drops rows |
| `NearMiss` | 0.448 | no, it only drops rows |
| `TomekLinks` | 0.516 | no, it only drops rows |
| scikit-learn `TargetEncoder.fit_transform`, which cross-fits | 0.522 against 0.504 | no |

The rule leaves alone a label encoder on the target, when the result is named `y`, `target` or
`labels`, or the encoder's name holds `label`. The target mean and the `df.corr()` forms need the
same column to become the target later in the block, as in `y = df["target"]`.

The rule as the readers saw it gave wrong advice in 2 and 0 of 26 labelled reports. Recall was 91
of 162. The rule found none of 24 columns picked by correlation in other shapes, such as
`corr.index[1:11]` read back as `df[top_features]`. It also misses a target mean written with
`.map` or `.mean()`, `resample` imported under another name, and `SelectFromModel` fitted in one
statement and applied in the next.

#### cross-val-on-fitted-data

**Severity:** error. **Fix:** none.

The rule reports a step that learns from the target, or `fit_resample`, run on every row before
`cross_val_score`, another cross-validation function, or a search fitted on the result.

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

xpd gives the replacement as advice and doesn't apply it.

The rule covers `cross_val_score`, `cross_validate`, `cross_val_predict`, `learning_curve`,
`validation_curve` and `permutation_test_score` when the fitted result is the second argument. It
also covers a `GridSearchCV`, `RandomizedSearchCV`, `HalvingGridSearchCV` or
`HalvingRandomSearchCV` that is assigned in the same file and fitted on the result.

| Step fitted on every row | Cross-validated AUC | With the step inside the folds |
| --- | --- | --- |
| `SelectKBest`, then `cross_val_score` | 0.764 | 0.518 for a Pipeline |
| `SMOTE`, then `cross_val_score` | 0.838 | 0.510 |
| `SelectKBest`, then a search, as `best_score_` | 0.770 | 0.522 |
| `SMOTE`, then a search, as `best_score_` | 0.854 | 0.521 |
| `RandomUnderSampler`, left alone | 0.519 | 0.511 |
| `NearMiss`, left alone | 0.458 | 0.508 |
| `TomekLinks`, left alone | 0.530 | 0.508 |

The three under-samplers didn't pass the proof, so the rule leaves them alone. The rule as the
readers saw it gave wrong advice in 1 and 0 of 11 labelled reports.

#### tune-on-test

**Severity:** error. **Fix:** none.

The rule reports a loop that builds one model per setting, scores each model on the test rows and
keeps the best score.

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

xpd gives the replacement as advice and doesn't apply it.

The best of 30 test accuracies for `n_neighbors` from 1 to 30 was 0.597, where a `GridSearchCV`
on the training rows gave 0.507. The rule needs all of the following in the loop:

1. The loop variable is a keyword of the call that built the scored model.
2. A `score`, `predict` or `predict_proba` runs on a name with `test` in it.
3. The loop picks a setting. A pick is a `>` comparison that reads or sets a name with `best` or
   `max` in it, or whose branch reads the setting. A later `max`, `argmax` or `idxmax` of the
   list that the loop appends to is a pick too.

A keyword named like `random_state`, `seed`, `method` or `model` is left alone, because such a
loop compares or averages.

The rule as the readers saw it gave wrong advice in 2 and 2 of 3 labelled reports, on loops that
printed whether the training score was far above the test score. The narrowed rule no longer
makes the reports. Recall was 15 of 50. The rule found none of 20 thresholds picked by the test
score. It also misses scores kept in a dict and picked with `max(results, key=results.get)`, a
loop over `ParameterGrid` that builds the model with `**params`, and a list comprehension picked
with `np.argmax`.

#### stateful-function-transformer

**Severity:** info. **Fix:** none.

The rule reports `FunctionTransformer(lambda X: (X - X.mean(axis=0)) / X.std(axis=0))`, a function
that takes statistics of the rows it gets. At predict time it then scales by the test rows.

Example:

```python
pipe = make_pipeline(FunctionTransformer(lambda X: (X - X.mean(axis=0)) / X.std(axis=0)), model)
```

Use instead:

```python
pipe = make_pipeline(StandardScaler(), model)
```

xpd gives the replacement as advice and doesn't apply it.

The rule is info, because on data with no signal it doesn't move a score. Predicted in batches of
5 rows it scored 0.484 against 0.494 for `StandardScaler`. On data with signal, over 30 seeds,
`LogisticRegression` scored lower behind the function:

| Batch of test rows | Behind the function | Behind `StandardScaler` |
| --- | --- | --- |
| 5 | 0.773 | 0.847 |
| 20 | 0.824 | 0.847 |

A reduction along axis 1 works per row and is left alone. Recall was 24 of 30, and the rule found
none of 6 functions defined with `def` and passed by name.

### Splits and groups

Each rule in the group reports a split of the rows that leaks or gives no score.

#### groups-ignored

**Severity:** error. **Fix:** unsafe: `cv=GroupKFold(5)` for `cv=KFold(5)`, `cv=GroupKFold()` when cv is missing, `cv=GroupKFold(5)` for `cv=5`, and `cv=GroupKFold(n_splits=5)` in a search built inside the fit call. The folds change on purpose. Other splitters, a splitter or a search held in a name, and a call to split have no fix.

The rule reports `groups=` given to a cross-validation function or a search whose `cv` ignores
it. It also reports the groups given to `split` on a splitter that ignores them, such as
`KFold(5).split(X, y, g)`.

Example:

```python
scores = cross_val_score(model, X, y, groups=patient_id, cv=5)
```

Use instead:

```python
scores = cross_val_score(model, X, y, groups=patient_id, cv=GroupKFold(n_splits=5))
```

xpd applies the replacement with `--fix --unsafe-fixes`.

scikit-learn warns "The groups parameter is ignored by KFold" and splits the rows as if no groups
were given. How much the score leaks depends on the row order:

| Data | Model | `cv=5` with `groups=` | `GroupKFold(5)` |
| --- | --- | --- | --- |
| 60 groups of 8 rows in random order, one label per group drawn apart from the features | `RandomForestClassifier` | 0.849 accuracy | 0.480 |
| the same groups with the rows of each group next to each other, and folds that are not shuffled | `RandomForestClassifier` | 0.497 | not given |
| 40 groups of 5 near copies | 1-nearest-neighbour | 0.998 | 0.487 |

A `cv` ignores groups when it is missing, a whole number, or one of `KFold`, `StratifiedKFold`,
`ShuffleSplit`, `StratifiedShuffleSplit`, `RepeatedKFold`, `RepeatedStratifiedKFold`,
`TimeSeriesSplit`, `LeaveOneOut`, `LeavePOut` and `PredefinedSplit`, given in the call or assigned
to a name earlier. The rule reports such a `cv` in the following places:

- as `cv` of `cross_val_score`, `cross_validate`, `cross_val_predict`, `learning_curve`,
  `validation_curve` and `permutation_test_score` with `groups=`
- in a `GridSearchCV`, `RandomizedSearchCV`, `HalvingGridSearchCV` or `HalvingRandomSearchCV` built
  in the same block as a `fit` that gets `groups=`
- in its own `split(X, y, groups)`, called directly or through a name

A `cv` held in a name that xpd can't trace to one of the listed splitters is left alone. Recall was 56
of 57.

#### missing-groups

**Severity:** error. **Fix:** none.

The rule reports a group splitter that gets no groups, such as
`cross_val_score(model, X, y, cv=GroupKFold(5))` or `GroupKFold(5).split(X, y)`, which raises
ValueError.

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

xpd gives the replacement as advice and doesn't apply it.

On scikit-learn 1.9.1 each form raises "The 'groups' parameter should not be None", and
`StratifiedGroupKFold` raises that it found 1 group. The rule covers `GroupKFold`,
`StratifiedGroupKFold`, `GroupShuffleSplit`, `LeaveOneGroupOut` and `LeavePGroupsOut` in the
following places:

- as `cv` of `cross_val_score`, `cross_validate`, `cross_val_predict`, `learning_curve`,
  `validation_curve` and `permutation_test_score`
- in their own `split(X, y)`, called directly or through a name
- in a `GridSearchCV`, `RandomizedSearchCV`, `HalvingGridSearchCV`, `HalvingRandomSearchCV` or
  `RFECV` whose `fit` gets no `groups=`

A call with `params=` or `**` is left alone, because it can carry the groups.
`SequentialFeatureSelector` and `LogisticRegressionCV` take groups only through metadata routing,
so the rule doesn't read them. Recall was 48 of 50.

#### leave-one-out-score

**Severity:** error. **Fix:** none.

The rule reports `cv=LeaveOneOut()` with a score that needs more than one row. Such a score is R2
or ROC AUC given as `scoring`, or the R2 that a regressor scores by default.

Example:

```python
scores = cross_val_score(Ridge(), X, y, cv=LeaveOneOut())
```

Use instead:

```python
r2 = r2_score(y, cross_val_predict(Ridge(), X, y, cv=LeaveOneOut()))
```

xpd gives the replacement as advice and doesn't apply it.

With one row per fold, each fold warns and scores nan, and a search over nan scores picks its first
candidate. Under `python -W error` each warning is an `UndefinedMetricWarning` that raises.

| Scoring | Warning on a fold of one row |
| --- | --- |
| R2 | "R^2 score is not well-defined with less than two samples" |
| ROC AUC | "Only one class is present in y_true" |

The rule covers `cross_val_score`, `cross_validate`, `GridSearchCV`, `RandomizedSearchCV`,
`validation_curve`, `learning_curve`, `permutation_test_score` and `RFECV`. It knows a regressor
only by its class name in the same call. `explained_variance` and `average_precision` score 1 or 0
on a fold of one row and give no nan, so they are left alone. `cv=len(X)`, which is leave one out
under another name, is left alone by design.

Recall was 14 of 33, and the plants included the two scorings that the rule left out after the
recall run. The rule misses a splitter kept in a name, as in `loo = LeaveOneOut()` and `cv=loo`,
and `LeaveOneOut` imported under another name. It also misses a regressor passed by a variable
with no `scoring`.

### Metrics and scoring

Each rule in the group reports a metric given the wrong values, a score on the wrong rows, or a
scorer name that doesn't exist.

#### auc-from-labels

**Severity:** error for `log_loss` and `brier_score_loss` given labels, for the whole `predict_proba` matrix where one score per row is needed, for `decision_function` given to a loss, and for `roc_auc_score` with `multi_class` given labels, and info for ranking metrics given labels, column 0 of `predict_proba`, and `make_scorer(roc_auc_score)` without `response_method`. **Fix:** unsafe: `log_loss(y_test, model.predict_proba(X_test))` for labels or `decision_function` given to a loss, `model.predict_proba(X_test)[:, 1]` for labels given to `brier_score_loss` and for the whole matrix given to `roc_curve`, `precision_recall_curve` or `det_curve`, `model.predict_proba(X_test)` for labels given to `roc_auc_score` with `multi_class`, and column 1 for column 0 when no `pos_label` is passed. Each fix changes the score on purpose, and the `multi_class` fix raises on a target with two classes. The other forms have no fix.

The rule reports a metric that needs one probability or score per row and gets class labels or a
whole matrix, such as `log_loss(y_test, model.predict(X_test))`,
`roc_auc_score(y_test, model.predict_proba(X_test))` and
`roc_auc_score(y_test, model.predict(X_test))`.

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

The rule was measured with `LogisticRegression` over 30 seeds:

| Form | Data | Old form | Right form |
| --- | --- | --- | --- |
| `log_loss` given the labels | no signal | 17.998 | 0.699, where the honest loss is ln 2, 0.693 |
| `brier_score_loss` given the labels | no signal | 0.499 | 0.253, where the honest loss is 0.25 |
| a ranking metric given the labels | no signal | 0.495 | 0.498 |
| AUC from the labels | signal | 0.735 | 0.822 |
| average precision from the labels | signal | 0.677 | 0.827 |
| AUC from column 0 of `predict_proba` | signal | 0.178 | not given |
| `make_scorer(roc_auc_score)` without `response_method`, in `cross_val_score` | signal | 0.739 | 0.824 |

Ranking metrics given labels are info, because on noise the labels and the probabilities scored
about the same. The following forms raise ValueError:

- the whole `predict_proba` matrix given to `roc_auc_score` without `multi_class`, or to
  `roc_curve`, `precision_recall_curve` or `det_curve`
- `roc_auc_score(multi_class="ovr")` given labels on three classes
- `decision_function` given to `log_loss` or `brier_score_loss`

The rule also reports `y_pred = model.predict(X_test)` when a later line passes `y_pred` to the
metric. The rule leaves alone a `predict` on a Booster that `xgb.train` or `lgb.train` returned,
and a `predict` whose argument is named like predictions or probabilities, as on a calibrator. It
also leaves alone files that import keras, tensorflow or torch, because `predict` returns
probabilities there.

The fix changes the loss on purpose, from 12.0 to 0.63 on the case built for it. On three classes
the old form raised and the fixed form runs. On an `SVC` without probabilities, the fixed form
raises AttributeError.

The readers called 2 and 2 of the 20 held-back reports wrong advice. Recall was 55 of 72. The rule
found none of 13 plants that keep column 0 or the whole `predict_proba` matrix in a variable
before the metric. It also misses a `predict` variable whose argument is wrapped, as in
`model.predict(np.array(X_test.values))`.

#### metric-arguments-swapped

**Severity:** error for `precision_score`, `classification_report` and `precision_recall_fscore_support`, and for scores passed first to a ranking metric or a loss, and info for the other metrics that change with the order. **Fix:** unsafe: `precision_score(y_test, y_pred)`, and the labels first for scores passed first, as in `roc_auc_score(y_test, scores)`. A name can mislead.

The rule reports a metric whose predictions come first, such as `precision_score(y_pred, y_test)`
and `roc_auc_score(y_score, y_test)`.

Example:

```python
precision = precision_score(y_pred, y_test)
```

Use instead:

```python
precision = precision_score(y_test, y_pred)
```

The rule reports when the first name holds `pred` or `hat`, and the second holds `true`, `test`,
`val`, `actual`, `target` or `label`, or is `y`. With the arguments swapped, the precision a metric
reports is the recall. On noise where 30 percent of the rows are positive,
`LogisticRegression(class_weight="balanced")` gave the following numbers:

| Metric | Swapped | Right order | Severity |
| --- | --- | --- | --- |
| precision, and `classification_report` for class 1 | 0.471 | 0.294 | error |
| recall | 0.294 | 0.471 | info |
| balanced accuracy | 0.496 | 0.495 | info |
| R2 | -186.06 | -0.022 | info |
| R2, on data with signal | -0.069 | 0.486 | info |

The metrics other than precision are info, because their swapped numbers on noise are not above
the right value. Scores passed first to `roc_auc_score`, `average_precision_score`, `roc_curve`,
`precision_recall_curve`, `log_loss` or `brier_score_loss` raise ValueError, because the true
values then look continuous. `accuracy_score`, `f1_score`, `jaccard_score`, `matthews_corrcoef`,
`cohen_kappa_score` and the error metrics such as `mean_squared_error` return the same number in
either order, so they are left alone.

The fix changes the number on purpose. Precision becomes the old recall, and a confusion matrix is
transposed. Of 43 held-back reports, 30 were labelled, and neither reader called one wrong.
Recall was 66 of 100. The rule found none of 9 swapped keyword values, such as
`confusion_matrix(y_true=y_pred, y_pred=y_true)`, and 2 of 7 `log_loss` calls with the scores
first. It also misses a column slice first, such as `roc_auc_score(y_prob[:, 1], y_test)`, and
true values named with `train` or `Y_`.

#### score-on-training-rows

**Severity:** error. **Fix:** none.

The rule reports `model.score(X_train, y_train)` and
`accuracy_score(y_train, model.predict(X_train))` when nothing around them scores held-out rows.

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

xpd gives the replacement as advice and doesn't apply it.

The rule reads the names of the rows. On noise, each model scored higher on its training rows:

| Model | Accuracy on the training rows | On the test rows |
| --- | --- | --- |
| `DecisionTreeClassifier` with default settings | 1.000 | 0.510 |
| 1-nearest-neighbour | 1.000 | 0.495 |
| `LogisticRegression` | 0.545 | 0.501 |

The rule stays silent when code nearby scores a name with `test`, `val` or `holdout` in it, or
calls cross-validation or a search. Nearby means the function around the score, the functions next
to it, or the file for code outside functions.

Recall was 10 of 27. The rule found none of 9 metrics given a `predict` variable of the training
rows, as in `y_pred = knn.predict(X_train)` and `accuracy_score(y_train, y_pred)`. It also misses
rows whose names don't say train, as in `model.score(X, y)` after `model.fit(X, y)`.

#### unknown-scoring-name

**Severity:** error. **Fix:** none.

The rule reports `scoring="mean_squared_error"` and any other string that `get_scorer` refuses. It
reads `scoring` in a search, `cross_val_score`, `cross_validate` and the other scikit-learn
functions that take it, and the argument of `get_scorer` itself.

Example:

```python
scores = cross_val_score(model, X, y, scoring="mean_squared_error")
```

Use instead:

```python
scores = cross_val_score(model, X, y, scoring="neg_mean_squared_error")
```

xpd gives the replacement as advice and doesn't apply it.

The rule checks each string against `sklearn.metrics.get_scorer_names()` on scikit-learn 1.9.1.
`scoring="mean_squared_error"`, `scoring="log_loss"` and `scoring="auc"` raise
InvalidParameterError, and `"roc_auc_score"` in a list given to `cross_validate` raises
ValueError. A string with `{` or `%` in it, or an f-string, is left alone, because code fills it in
later. Recall was 16 of 32. The rule found none of 16 scoring strings or dicts of scorer names kept
in a variable.

### Removed and deprecated forms

Each rule in the group reports a form that raises or warns on scikit-learn 1.9.1. Each form was
run under `python -W error`, where the old form raised and the new form ran clean. The messages
take each replacement from the 1.9.1 source or from the error text, such as the docstring that
says `sparse` was renamed `sparse_output` in 1.2.

A fix of a removed form turns code that raises into code that runs, so no such fix can be bit
equal or safe. A fix that drops a keyword deletes the comments inside the argument
list, and it joins a call written over several lines onto one line.

#### removed-in-sklearn

**Severity:** error. **Fix:** safe: `from joblib import` for the names that `sklearn.externals.joblib` took from joblib, the public module for a private module path such as `sklearn.linear_model.logistic`, `sklearn.feature_extraction.text` for `stop_words`, the renamed keywords `sparse_output=`, `estimator=`, `max_iter=`, `store_cv_results=`, `ensure_all_finite=`, `metric=` and `alphas=`, dropping `n_jobs` from `KMeans`, `behaviour="new"` and `"deprecated"`, `algorithm="SAMME"`, `iid=False`, `multi_class="auto"`, `min_impurity_split=None`, and `normalize=False` outside the Lars and orthogonal matching pursuit models, new values for refused ones, such as `max_features="sqrt"` or `1.0` for `"auto"`, `loss="log_loss"`, `criterion="squared_error"` and `"absolute_error"`, `loss="squared_error"`, `C=np.inf` for `penalty="none"` and `min_samples_split=2` for `1`, `max_iter=` with `tol=None` for `n_iter=` on the SGD models, and `alphas=` for `n_alphas=` beside `alphas=None`. unsafe, because no older release ran both forms with the same result: `sklearn.model_selection` for `sklearn.cross_validation`, `sklearn.grid_search` and `sklearn.learning_curve`, `import joblib` and `import joblib as` for the module `sklearn.externals.joblib`, a private module path that scikit-learn 0.19.2 to 0.23.2 did not have, or that imports `VALID_METRICS` from a neighbors module or `all_estimators` from `sklearn.utils.testing`, `get_feature_names_out()`, which returns an array, `n_splits=` for `n_folds=` and `n_iter=`, which the `model_selection` splitters never took, dropping `n_jobs` from `MiniBatchKMeans`, which never took it, dropping the other values of `behaviour`, `algorithm` and `iid`, `multi_class="multinomial"`, `normalize=False` on the Lars and orthogonal matching pursuit models, `OneVsRestClassifier(LogisticRegression())` for `multi_class="ovr"`, and `CalibratedClassifierCV(FrozenEstimator(model))` for `cv="prefit"`. A fix that rewrites the whole call applies only when the keyword is on the first line of the call. The other forms have no fix.

The rule reports imports of removed modules and names, such as `sklearn.cross_validation`,
`sklearn.externals.joblib` and `six`, `load_boston`, `preprocessing.Imputer`, private module paths
and `sklearn.feature_extraction.stop_words`. It also reports the following forms:

- `get_feature_names()`
- keywords that the signature no longer has
- values that parameter validation refuses
- positional arguments past the keyword-only marker
- splitters given the data the way `sklearn.cross_validation` took it

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

The proof ran 49 forms on scikit-learn 1.9.1. Each old form raised ModuleNotFoundError,
ImportError, AttributeError, TypeError for an unexpected keyword or too many positional arguments,
or InvalidParameterError, and its new form ran clean. A grid with
`max_features: ["auto", "sqrt"]` raised through the FitFailedWarning that `GridSearchCV` gives. A
backslash line break inside a call is not counted as a positional argument. `SCORERS`, the
`plot_*` functions, `jaccard_similarity_score`, `squared=` and `fit_params=` are left to
[`removed-metric-api`](#removed-metric-api) and [`fit-params-keyword`](#fit-params-keyword).

Every fixed form ran on the case built for it, and each renamed keyword and refused value gave the
same model as a reference written from the 1.9.1 docstrings. Some fixes change what the code
means:

| Fix | What changes |
| --- | --- |
| `get_feature_names_out()` | It returns an array where `get_feature_names()` returned a list, so `print` shows `array([...])`. At 1 held-back site the fix broke a project class that defined its own `get_feature_names`. |
| dropping `multi_class` | Every multiclass fit uses the multinomial loss. With liblinear on three or more classes the fixed call raises ValueError. |
| `OneVsRestClassifier(...)` for `multi_class="ovr"` | The model is a `OneVsRestClassifier`, so `coef_` moves to its `estimators_`, and a grid key such as `logisticregression__C` needs `estimator__` in front. |
| dropping `algorithm="SAMME.R"` | A SAMME.R model becomes a SAMME model. |
| `OneHotEncoder(sparse_output=False, dtype=np.int)` | At 2 held-back sites the call also held `np.int`, which is gone from NumPy. The same run with `--fix --unsafe-fixes` rewrote it to `int`, and the fixed call ran. |

Of 321 held-back reports, 58 were labelled, and reader A called 1 wrong. Recall was 517 of 600.
Of the misses, 36 were `SCORERS`, the `plot_*` functions and `jaccard_similarity_score`, which
`removed-metric-api` reports. The rule found none of 18 refused values in a parameter grid for
keywords other than `max_features`, and none of 16 refused values held in a variable. It also
misses `sklearn.feature_extraction.stop_words` imported as a module.

#### deprecated-in-sklearn

**Severity:** warning. **Fix:** safe: drop `probability=False` from `SVC` and `NuSVC`, drop `penalty="elasticnet"` beside an `l1_ratio`, drop `n_jobs=` from `LogisticRegression`, `criterion="squared_error"` for `"friedman_mse"` on regression trees and forests, drop `criterion=` from GradientBoosting, `metric=` for MDS `dissimilarity=`, `metric_mds=` for MDS `metric=True` or `False`, `alphas=` for `n_alphas=`, `alphas=` for `n_alphas=` beside `alphas=None`, `alphas=100` for `alphas=None`, `C=np.inf` for `penalty=None` on `LogisticRegression`, and `init="random"` for MDS without `init`. Unsafe: drop `penalty="l2"`, `l1_ratio=1` for `penalty="l1"`, and `l1_ratio=0` for `l1_ratio=None` on `LogisticRegression`. Also unsafe: `CalibratedClassifierCV(SVC(), ensemble=False)` for `probability=True`, whose probabilities and predictions differ, `l1_ratios=(0,)` or `(1,)` for `penalty="l2"` or `"l1"` on `LogisticRegressionCV`, and `l1_ratios=(0,), scoring="accuracy", use_legacy_attributes=True` on a `LogisticRegressionCV` that sets none of the three. There `l1_ratio_`, `scores_`, `coefs_paths_` and `n_iter_` change form. A passive aggressive model without `C` or `loss` becomes `SGDClassifier` or `SGDRegressor` with `penalty=None`, `learning_rate="pa1"` and `eta0=1.0`, which fits the same model as another class. The other forms have no fix.

The rule reports forms that warn FutureWarning on scikit-learn 1.9.1. Its message gives the
release that removes each form or changes its default:

| Form | Removed or changed in | Write instead |
| --- | --- | --- |
| `SVC(probability=True)`, `NuSVC(probability=True)` | 1.11 | `CalibratedClassifierCV(SVC(), ensemble=False)` |
| `probability=False` on `SVC` and `NuSVC` | 1.11 | nothing, because it is the default |
| `probA_`, `probB_` | 1.11 | the calibrators of `CalibratedClassifierCV(ensemble=False)` |
| `LogisticRegression(penalty=...)` | 1.10 | `l1_ratio`, 0 for L2, 1 for L1 and a value between for elasticnet, and `C=np.inf` for no penalty |
| `LogisticRegression(l1_ratio=None)` | 1.10 | `l1_ratio=0` |
| `LogisticRegression(n_jobs=...)` | 1.10 | nothing, because it has had no effect since 1.8 |
| `LogisticRegressionCV(penalty=...)` | 1.10 | `l1_ratios`, `(0,)` for L2 and `(1,)` for L1 |
| `LogisticRegressionCV` without `l1_ratios`, `scoring` and `use_legacy_attributes` | 1.10 and 1.11 | all three set |
| `criterion="friedman_mse"` on regression trees and forests | 1.11 | `criterion="squared_error"` |
| `criterion=` on GradientBoosting | 1.11 | nothing, because it has no effect |
| `PassiveAggressiveClassifier`, `PassiveAggressiveRegressor` | 1.10 | `SGDClassifier(loss="hinge", penalty=None, learning_rate="pa1", eta0=1.0)`, or `SGDRegressor` with `loss="epsilon_insensitive"` |
| `TargetEncoder(shuffle=, random_state=)` | 1.11 | a splitter as `cv`, such as `cv=KFold(5, shuffle=True, random_state=0)` |
| `MDS(dissimilarity=)` | 1.10 | `metric=` |
| `MDS(metric=True)` or `metric=False` | 1.10 | `metric_mds=` |
| `MDS` without `init` | 1.10 | `init="random"` to keep the current result |
| `HDBSCAN` without `copy` | 1.10 | `copy=False` to keep the current behavior, or `copy=True` |
| `n_alphas=` on the path functions | 1.11 | `alphas=` |
| `alphas=None` | 1.11 | `alphas=100` |
| a negative `power_t` | 1.10 | a value of 0 or more |
| `sklearn.utils.extmath.stable_cumsum` | 1.10 | `np.cumulative_sum` |

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

The proof ran 30 forms on scikit-learn 1.9.1. Under `-W error` each old form raised its
FutureWarning, and its new form ran clean. `LogisticRegression(n_jobs=None)` gives no warning and is
left alone.

Every safe fix gave a bit equal fitted model and bit equal predictions in every run. A fix that
drops a keyword changes the stored constructor parameter, which is not counted as a change of the
model.

| Safe fix | Runs |
| --- | --- |
| dropping `probability=False` | 16 |
| dropping `penalty="elasticnet"` | 16 |
| dropping `n_jobs` | 38 |
| `criterion="squared_error"` for `"friedman_mse"` | 16 |
| dropping the GradientBoosting `criterion` | 6 |
| the two MDS renames | 4 and 6 |
| `alphas=` for `n_alphas=` | 8 |
| `alphas=100` for `alphas=None` | 6 |

The three unsafe fixes were bit equal in 48 runs, 32 runs and 8 runs. They differ only with
`C=np.inf`, which 1.9.1 turns into no penalty only when `penalty` is left at its default:

| Unsafe fix | With `C=np.inf` |
| --- | --- |
| `l1_ratio=1` for `penalty="l1"`, with liblinear | The fixed form raises where the old form ran. |
| dropping `penalty="l2"`, with liblinear | The fixed form raises ValueError where the old form didn't finish in 60 s. |
| `l1_ratio=0` for `l1_ratio=None`, with saga | The fixed form runs where the old form raised TypeError. |

Of 101 held-back reports, 57 were labelled, and reader A called 3 wrong. Recall was 274 of 308. Of
the misses, 12 were `sklearn.metrics.cluster.entropy`, which
[`deprecated-metric-api`](#deprecated-metric-api) reports. The rule also misses `HDBSCAN`,
`stable_cumsum` and the PassiveAggressive classes reached through a module or another name, and a
deprecated value in a parameter grid or a variable.

#### removed-metric-api

**Severity:** error. **Fix:** safe: `root_mean_squared_error(...)` for `squared=False`, where the file imports that name or calls it through `metrics.`, dropping `squared=True` and `eps="auto"`, `response_method=` for `needs_proba=True` or `needs_threshold=True`, `name=` for `estimator_name=`, `y_score=` for `y_pred=`, and `curve_kwargs={...}` for matplotlib keywords on `RocCurveDisplay.from_estimator`, `from_predictions` and `plot`. unsafe: `metrics.RocCurveDisplay.from_estimator` and the matching Display class for `metrics.plot_*`, because each is another function and `plot_confusion_matrix` named its third argument `y_true`, dropping another value of `eps=`, which clipped the probabilities at another bound, and `curve_kwargs={...}` on `from_cv_results`, which never took matplotlib keywords. `SCORERS`, the imports of the `plot_*` functions, `jaccard_similarity_score`, `make_scorer` with `squared=` and `label=` have no fix.

The rule reports metric APIs that scikit-learn removed:

- `SCORERS`, `plot_confusion_matrix` and the other `plot_*` functions, and
  `jaccard_similarity_score`
- `squared=` on `mean_squared_error` and `mean_squared_log_error`, and `eps=` on `log_loss`
- `squared=`, `needs_proba=` and `needs_threshold=` passed to `make_scorer`
- the `RocCurveDisplay` keywords that 1.7 replaced

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

xpd applies the replacement with `--fix --unsafe-fixes`.

Every form raises on scikit-learn 1.9.1. `SCORERS`, the `plot_*` functions and
`jaccard_similarity_score` raise ImportError, and the keywords raise TypeError. `make_scorer`
passes `squared=`, `needs_proba=` and `needs_threshold=` on to the metric, so the error comes when
the scorer runs. A search with the default `error_score` then scores every fold as nan.

The rule reports `squared=` and `eps=` only on the scikit-learn functions, imported from
`sklearn.metrics` or called as `metrics.<name>`, because torchmetrics has a `mean_squared_error`
with `squared=`. Old module paths such as `sklearn.metrics.ranking` are left to
[`removed-in-sklearn`](#removed-in-sklearn).

The four `RocCurveDisplay` forms raised TypeError in the first run, with matplotlib 3.11.2 added
to scikit-learn 1.9.1. The saved rerun had no matplotlib, so the saved results hold no passing
proof for them. Their fixes are unsafe, because a form that raises leaves nothing to compare bit for bit.
Neither reader called one of the 13 held-back reports wrong. Recall was 74 of 74.

#### deprecated-metric-api

**Severity:** warning. **Fix:** safe: `log_loss(y_test, y_proba=proba)`, `PrecisionRecallDisplay(precision=precision, recall=recall, name="model")`, `y_score=` for `y_pred=` on `PrecisionRecallDisplay.from_predictions` and `DetCurveDisplay.from_predictions`, and `curve_kwargs={"color": "red"}` for `color="red"` on `PrecisionRecallDisplay`, which already used those keywords as `curve_kwargs`. The other forms have no fix.

The rule reports metric APIs that warn on scikit-learn 1.9.1. Its message gives the release that
removes each form, from the warning text:

| Form | Removed in | Write instead |
| --- | --- | --- |
| `y_pred=` on `log_loss` and `d2_log_loss_score` | 1.11 | `y_proba=` |
| `estimator_name=` on `PrecisionRecallDisplay` | 1.10 | `name=` |
| `y_pred=` on `PrecisionRecallDisplay.from_predictions` and `DetCurveDisplay.from_predictions` | 1.10 | `y_score=` |
| matplotlib keywords passed straight to `PrecisionRecallDisplay` | 1.11 | `curve_kwargs={...}` |
| `sklearn.metrics.cluster.entropy` | 1.10 | `scipy.stats.entropy` of the class counts |

Example:

```python
loss = log_loss(y_test, y_pred=proba)
```

Use instead:

```python
loss = log_loss(y_test, y_proba=proba)
```

The four display forms passed in the first run with matplotlib 3.11.2, and the saved rerun had no
matplotlib. The two fixes rename a keyword and gave bit equal results, the `log_loss` rename in 12
of 12 runs and the `PrecisionRecallDisplay` rename in 2 of 2 with the same plot label.
`scipy.stats.entropy` of the class counts differs from `entropy` in the last bits, so that form
has no fix. Recall was 48 of 48.

#### fit-params-keyword

**Severity:** error. **Fix:** unsafe: `params=` for `fit_params=` on the cross-validation functions, because the old form raises. The search form has no fix.

The rule reports `fit_params=` on `cross_val_score`, `cross_validate`, `cross_val_predict`,
`learning_curve`, `validation_curve`, `permutation_test_score` and the four search classes.

Example:

```python
scores = cross_val_score(model, X, y, fit_params={"sample_weight": w})
```

Use instead:

```python
scores = cross_val_score(model, X, y, params={"sample_weight": w})
```

xpd applies the replacement with `--fix --unsafe-fixes`.

The keyword raises TypeError on scikit-learn 1.9.1. The documents for `cross_val_score`,
`cross_validate` and `cross_val_predict` carry `since: "1.4"`, the release that added `params`,
and the documents for the other three functions carry `since: "1.6"`. On `GridSearchCV`,
`RandomizedSearchCV`, `HalvingGridSearchCV` and `HalvingRandomSearchCV` the message says to drop
the keyword and pass the values to `fit`. The rename is an unsafe fix, because the old form raises
and leaves nothing to compare. Neither reader called one of the 4 held-back reports wrong. Recall was
32 of 32.

#### column-transformer-removed

**Severity:** error. **Fix:** unsafe: drop `force_int_remainder_cols`, because the old form raises.

The rule reports `force_int_remainder_cols=` on `ColumnTransformer` and `make_column_transformer`.

Example:

```python
ColumnTransformer(transformers, remainder="passthrough", force_int_remainder_cols=False)
```

Use instead:

```python
ColumnTransformer(transformers, remainder="passthrough")
```

xpd applies the replacement with `--fix --unsafe-fixes`.

The keyword raises TypeError on scikit-learn 1.9.1, with either value. The remainder columns now
come back in the form the other columns were given in. Neither reader called one of the 3
held-back reports wrong. Recall was 30 of 30.

### Pipeline and metadata routing

Each rule in the group reports a form that raises at construction or at fit on scikit-learn
1.9.1. No held-back file uses metadata routing, a misnamed step, `TransformedTargetRegressor` in a
wrong mix, or pandas output on a sparse encoder. The rules in the group therefore made no report
there and have no number for wrong advice.

#### metadata-routing

**Severity:** error. **Fix:** unsafe: `params={"groups": groups}` for `groups=` when routing is on, because the old form raises. The other forms have no fix.

The rule reports forms that raise because metadata routing is off, such as `set_fit_request` and
`Pipeline(transform_input=...)`. It also reports forms that raise because routing is on, such as
`groups=`, `step__sample_weight=` and `params=` with no request in the file.

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

xpd applies the replacement with `--fix --unsafe-fixes`.

The rule counts a file as routed when any call in it passes `enable_metadata_routing` with a value
other than `False`. A function or a script that never calls a `set_*_request` method has asked
for nothing.

| Routing | Form | Raises |
| --- | --- | --- |
| off | `set_fit_request` and the other request setters | RuntimeError |
| off | `Pipeline(transform_input=...)` | ValueError at fit |
| on | `groups=` on a cross-validation function | ValueError |
| on | a `step__sample_weight` keyword on `fit` | TypeError |
| on | `params=` when no estimator asked for the metadata | UnsetMetadataPassedError |

On scikit-learn 1.9.1 a weighted `cross_validate` with routing on also needs
`set_score_request`, or it raises for the scorer. Recall was 34 of 34.

#### pipeline-fit-params

**Severity:** error. **Fix:** none.

The rule reports `pipe.fit(X, y, sample_weight=w)` on a Pipeline built in the same block, with
metadata routing off.

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

xpd gives the replacement as advice and doesn't apply it.

With routing off, `Pipeline.fit` raises "ValueError: Pipeline.fit does not accept the
sample_weight parameter". The rule reads the Pipeline from an assignment earlier in the same
block, or from the call itself. A file that turns routing on is left alone, because there a plain
name is the right form.

Recall was 23 of 30. The rule found none of 6 `sample_weight=` keywords given to the `fit` of a
search over a Pipeline, which raises the same ValueError. It also misses a Pipeline built outside
a loop and fitted inside it.

#### pipeline-step-form

**Severity:** error. **Fix:** unsafe: `make_pipeline(a, b)` for `make_pipeline([a, b])`, because the old form raises at fit. The other forms have no fix.

The rule reports step names with `__`, `make_pipeline` given one list, `Pipeline` given bare
estimators, and an imbalanced-learn sampler before another step in the Pipeline of scikit-learn.

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

xpd applies the replacement with `--fix --unsafe-fixes`.

| Form | Error |
| --- | --- |
| a step name with two underscores in a row, in the list given to `Pipeline`, `ColumnTransformer` or `FeatureUnion` | ValueError at fit |
| `make_pipeline([a, b])` | TypeError at fit |
| `Pipeline([a, b])` | "TypeError: cannot unpack non-iterable" |
| a sampler such as `SMOTE` before another step | "TypeError: All intermediate steps should be transformers" |

The first three forms are reported only when the name comes from scikit-learn, imported from
`sklearn.pipeline` or `sklearn.compose` or called through the two modules. Other libraries, such as
xfeat, have a Pipeline that takes a plain list. The rule as the readers saw it made 4 reports on
the held-back files, on such Pipelines, and both readers called all 4 wrong. Files that import
from `imblearn.pipeline` are left alone, and so is a sampler as the last step, which fits.

Recall was 29 of 35, measured before the rule was narrowed to the Pipeline of scikit-learn. The
rule found 4 of 10 plants of `Pipeline` given bare estimators. It misses them given by `steps=`,
by names that hold the estimators, or as a single step.

#### transformed-target-pair

**Severity:** error. **Fix:** unsafe: `inverse_func=np.expm1` beside `func=np.log1p`, because the old form raises at fit. The other forms have no fix.

The rule reports `TransformedTargetRegressor` with `func` and no `inverse_func`, with
`inverse_func` and no `func`, or with both a `transformer` and functions.

Example:

```python
model = TransformedTargetRegressor(regressor=Ridge(), func=np.log1p)
```

Use instead:

```python
model = TransformedTargetRegressor(regressor=Ridge(), func=np.log1p, inverse_func=np.expm1)
```

xpd applies the replacement with `--fix --unsafe-fixes`.

Each of the three mixes raises ValueError at fit. Recall was 31 of 36, and the rule found none of 5
plants that import the class under another name, as in `TransformedTargetRegressor as TTR`.

#### pandas-output-sparse

**Severity:** error. **Fix:** unsafe: `OneHotEncoder(sparse_output=False)` in a file that sets pandas output for every transformer, because the old form raises. The `set_output` form has no fix.

The rule reports pandas output on a OneHotEncoder that keeps its default `sparse_output=True`,
set with `set_output` or with `set_config(transform_output="pandas")`.

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

xpd applies the replacement with `--fix --unsafe-fixes`.

Each form raises "ValueError: Pandas output does not support sparse data" on scikit-learn 1.9.1.
The rule reports `set_output(transform="pandas")` on a OneHotEncoder, or on a Pipeline,
ColumnTransformer or FeatureUnion that holds one, in the same expression or assigned in an
earlier statement of the block. With `set_config` or `config_context` it reports every
OneHotEncoder in the file. A OneHotEncoder with `sparse_output=False` is left alone. Recall was 29
of 31.

### Work done twice

Each rule in the group reports work that scikit-learn already did, and the new form does it once.
The numbers are speed proofs on scikit-learn 1.9.1, on one thread, as the median of 7 runs of each
form at two data sizes. A rule reports only the classes where both sizes passed 1.5 times.

#### predict-per-row

**Severity:** warning. **Fix:** none.

The rule reports `model.predict([row])`, `model.predict_proba(X[i].reshape(1, -1))`,
`model.predict(df.iloc[[i]])` and the same calls of `decision_function` and `predict_log_proba` in
a loop, a comprehension or a lambda over the rows.

Example:

```python
labels = [model.predict([row])[0] for row in X_test]
```

Use instead:

```python
labels = model.predict(X_test)
```

xpd gives the replacement as advice and doesn't apply it.

The rule reports a call whose rows come from the loop variable, either the row itself or the data
indexed by it. Each call checks the input and sets up the model again, so one call on all rows was
faster:

| Model | At 500 rows | At 5,000 rows |
| --- | --- | --- |
| `SVC` | 2.77 times | 2.79 times |
| `RandomForestClassifier` | 216 times | 356 times |
| `LogisticRegression.predict_proba` | 104 times | 816 times |

The labels were bit equal. The probabilities of `LogisticRegression` differed by up to 3.3e-16,
so the rule has no fix. A loop that writes into the data it predicts from, as recursive
forecasting does, is left alone.

Reader B called both held-back reports wrong advice, and reader A called neither wrong. Recall was
28 of 32. The 4 misses were `DataFrame.apply` with the row's values in a list, as in
`X_test.apply(lambda row: model.predict([row.values])[0], axis=1)`.

#### score-after-predict

**Severity:** warning. **Fix:** unsafe: `f1_score(y_test, y_pred)`, and `accuracy_score(y_test, y_pred)` or `r2_score(y_test, y_pred)` for `model.score(X_test, y_test)` when the file binds the name to a classifier or to a regressor. Code between the two lines can change the predictions. A name bound to both has no fix.

The rule reports `model.score(X_test, y_test)` and `f1_score(y_test, model.predict(X_test))` after
`y_pred = model.predict(X_test)`. It reports them only when the file builds `model` as a forest, a
boosting or bagging model, k-nearest neighbours, an SVM, an MLP or `GaussianNB`.

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

The rule reads the class from the line that builds the model, because `score` costs the most
where `predict` does. Over the 13 reported classes that were timed, at 1,000 and 10,000 test rows,
the score call took 1.62 to 879 times as long as the metric alone, and gave the same number.
`GaussianNB` had the smallest gain, 1.62 times at 1,000 rows. The decision trees fell below 1.5
times at 1,000 rows in one of two runs:

| Class | First run | Saved rerun |
| --- | --- | --- |
| `DecisionTreeClassifier` | 1.45 times | 1.54 times |
| `DecisionTreeRegressor` | 1.52 times | 1.66 times |

The rule leaves both decision trees alone, as it does `LogisticRegression` and `LinearRegression`.
A name that the file also binds to another class is left alone.

The fix on the metric form is unsafe, because code between the two lines can change the saved
predictions. It gave the same score in 12 of 12 runs on both held-back shapes, and the score
changed only when the model was fitted again between the two lines. Neither reader called one of
the 17 held-back reports wrong. Recall was 31 of 32.

#### predict-and-predict-proba

**Severity:** warning. **Fix:** none.

The rule reports `model.predict(X_test)` next to `model.predict_proba(X_test)` when the file builds
`model` as a `RandomForestClassifier`, `ExtraTreesClassifier` or `BaggingClassifier`.

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

xpd gives the replacement as advice and doesn't apply it.

The rule covers the classes whose `predict` takes the argmax of `predict_proba`. Deriving the
labels from one `predict_proba` was 1.83 to 2.00 times faster for the three classes at 1,000 and
10,000 rows, with bit equal labels and probabilities. The rule leaves other classes
alone, because they decide the label from a raw score, where a tie at 0.5 can give another label. Neither reader called one
of the 22 held-back reports wrong. Recall was 30 of 30.

#### cross-val-score-per-metric

**Severity:** warning. **Fix:** none.

The rule reports `cross_val_score(model, X, y, scoring="accuracy")` followed by
`cross_val_score(model, X, y, scoring="precision")` on the same model, data and `cv`.

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

xpd gives the replacement as advice and doesn't apply it.

The rule reports the first call when a later call in the same block passes the same model, data
and `cv` with another `scoring`. With `LogisticRegression`, two scorings in one `cross_validate`
were 1.60 times faster at 2,000 rows and 1.84 times faster at 20,000 rows, and the scores were bit
equal. Neither reader called one of the 14 held-back reports wrong. Recall was 30 of 30.

#### vectorizer-fit-then-transform

**Severity:** warning. **Fix:** unsafe: `vectorizer.fit_transform(docs)`. The form in two statements has no fix.

The rule reports `vectorizer.fit(docs).transform(docs)`, and `vectorizer.fit(docs)` followed by
`vectorizer.transform(docs)`, on a `CountVectorizer` or `TfidfVectorizer`.

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

`fit` of the two vectorizers runs `fit_transform` and drops the matrix, so `fit_transform` was
faster:

| Vectorizer | At 2,000 documents | At 20,000 documents |
| --- | --- | --- |
| `CountVectorizer` | 1.63 times | 2.06 times |
| `TfidfVectorizer` | 1.82 times | 2.03 times |

The fix is unsafe, because `fit_transform` leaves the column indices of each row unsorted. The
fitted vectorizer is bit equal, but the `TfidfVectorizer` values differ by up to 1.1e-16. A
`MultinomialNB` fitted on the matrix gave probabilities that differ by up to 3e-15.

Reader B called 4 of the 12 held-back reports wrong advice, and reader A called none wrong. Recall
was 19 of 25. The 6 misses were a fitted vectorizer assigned back to its own name, as in
`cv = cv.fit(corpus)` followed by `cv.transform(corpus)`.

#### refit-per-n-estimators

**Severity:** warning. **Fix:** none.

The rule reports a `RandomForest`, `ExtraTrees`, `GradientBoosting` or `Bagging` model or an
`IsolationForest` built with `n_estimators=n` in a loop over `n` and fitted there.

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

xpd gives the replacement as advice and doesn't apply it.

The new form relies on `warm_start=True`, which keeps the trees already grown and draws every seed
from `random_state` in the same order. The grown models were bit equal to models fitted from
scratch, and warm start was faster:

| Trees | Classes | Warm start was faster by |
| --- | --- | --- |
| `n_estimators` 10, 20, 50 and 100, at 1,000 and 5,000 rows | `RandomForestClassifier`, `RandomForestRegressor`, `ExtraTreesClassifier`, `GradientBoostingClassifier`, `BaggingClassifier`, `IsolationForest` | 1.64 to 2.75 times |
| 10 to 100 in steps of 10 | `RandomForestClassifier` | 6.82 and 3.59 times |

The rule leaves the following loops alone:

- a loop whose model is passed to a cross-validation function or to `clone`, which clones it, so
  warm start can't help
- a loop with another loop inside it, where another setting changes between fits
- a loop that reads a clock, because it times each fit

The rule as the readers saw it made 1 report on the held-back files, on a loop that timed each
fit, and both readers called it wrong. Recall was 30 of 30.

#### tsne-exact

**Severity:** info. **Fix:** unsafe: drop `method="exact"`, which gives another embedding.

The rule reports `TSNE(method="exact")`.

Example:

```python
embedding = TSNE(method="exact", random_state=0).fit_transform(X)
```

Use instead:

```python
embedding = TSNE(random_state=0).fit_transform(X)
```

xpd applies the replacement with `--fix --unsafe-fixes`.

The rule is info, because the default Barnes-Hut method approximates the forces between distant
points. Its embedding therefore differs from the exact one, so no speed proof applies. Over 20 seeds on
`make_classification` data, with the default number of threads, Barnes-Hut gave a trustworthiness
with 5 neighbours within 0.001 of the exact method:

| Rows | Barnes-Hut was faster, as a median | Slowest seed |
| --- | --- | --- |
| 300 | 4.5 times | 4.1 times |
| 1,000 | 16.0 times | 11.4 times |

Recall was 21 of 30. The rule misses `TSNE` imported under another name and a method held in a
variable.

### Reproducible results

Pass `random_state` wherever scikit-learn shuffles or draws, so two runs give the same split and the same score.

#### random-state-without-shuffle

**Severity:** error. **Fix:** none.

The rule reports `KFold(n_splits=5, random_state=42)` without `shuffle=True`, which raises
ValueError.

Example:

```python
folds = KFold(n_splits=5, random_state=42)
```

Use instead:

```python
folds = KFold(n_splits=5, shuffle=True, random_state=42)
```

xpd gives the replacement as advice and doesn't apply it.

The rule covers `KFold`, `StratifiedKFold` and `StratifiedGroupKFold`, and `GroupKFold` from
scikit-learn 1.6, which added its `shuffle`. Each raises "Setting a random_state has no effect
since shuffle is False" on scikit-learn 1.9.1. A file that imports the removed
`sklearn.cross_validation` is left alone unless the call passes `n_splits`, because the old classes
took other arguments. `removed-in-sklearn` reports the import in such a file.

The rule as the readers saw it gave wrong advice in 2 and 2 of 11 labelled reports, on such files,
and the narrowed rule no longer makes the reports. Recall was 54 of 54.

#### missing-random-state

**Severity:** info. **Fix:** none.

The rule reports `train_test_split(X, y)`, `RandomForestClassifier()` and `KFold(shuffle=True)`
without `random_state`.

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

Any fixed number makes the run repeatable, and 0 is only the example.

xpd gives the replacement as advice and doesn't apply it.

The rule is info, because a seed doesn't move the expected score. A seed can still change which
model wins a comparison, as the worse of two models whose true accuracy differs a little won more
often without a seed:

| Split | Worse model won, of 300 trials |
| --- | --- |
| each model on its own split, without a seed | 25.0 percent |
| both models on one shared split | 14.3 percent |

The classes that the rule names gave another output in five fresh runs on scikit-learn 1.9.1:

| Kind | Classes |
| --- | --- |
| ensembles | the forests, extra trees, bagging, gradient boosting, `AdaBoostRegressor`, `IsolationForest`, `RandomTreesEmbedding` |
| trees and other models | the decision trees, the MLPs, the SGD models |
| clustering | `KMeans`, `MiniBatchKMeans`, `SpectralClustering`, `AffinityPropagation` |
| decomposition and embedding | `TruncatedSVD`, `NMF`, `LatentDirichletAllocation`, `FastICA`, `MiniBatchDictionaryLearning`, `MDS`, `SpectralEmbedding`, `Nystroem`, `RBFSampler`, the random projections |
| splitters | `ShuffleSplit`, `StratifiedShuffleSplit`, `RepeatedKFold`, `RepeatedStratifiedKFold`, and `KFold`, `StratifiedKFold`, `GroupKFold` and `StratifiedGroupKFold` with `shuffle=True` |
| classes with a setting | `TSNE` with `init="random"`, `SVC(probability=True)`, a HistGradientBoosting model with `early_stopping=True`, a Gaussian mixture with more than one component |

A HistGradientBoosting model without `random_state` is reported with a message that names 10,000
rows, because above that size it stops early on a validation part drawn at random.
`LogisticRegression`, `PCA`, `Perceptron`, `AdaBoostClassifier` and `TSNE` with its default `init`
gave the same output and are not listed.

The rule leaves alone a file that calls `np.random.seed`, or a seed helper such as `set_seed` or
`seed_everything` that the file doesn't define and doesn't import from TensorFlow, PyTorch or
`random`. It also leaves alone a call with `shuffle=False` or `**`, a `train_test_split` whose
result is thrown away, and a constructor that only `isinstance` reads. Of 527 held-back reports,
52 were labelled, and each reader called 1 wrong. Recall was 79 of 80.

## scikit-lego

The scikit-lego rules report arguments that give a wrong number without an error, steps that read
the wrong rows or can't be fitted, and code that raises or warns on scikit-lego 0.9.10, in each
file that imports `sklego`. Every number about the scikit-lego rules was measured on scikit-lego 0.9.10 with
scikit-learn 1.9.1, NumPy 2.5.5 and pandas 3.1.1. The examples assume that the names they use are
imported, and names such as `X_train`, `y_test` and `model` stand for values from your program.

A rule document whose advice needs a newer release of scikit-lego carries `since`. xpd reads the
installed version from the project's venv and skips the document on an older release. When the
venv has no scikit-lego, or there is no venv, every document runs.

The pack has no package checks. cvxpy, umap-learn and formulaic are extras that only some
estimators need, and a package check can't see which estimators a project uses. When an extra is
missing, scikit-lego raises an ImportError that names it.

### The proof each rule needs

A rule is an `error` only when code was run that shows the wrong number or the exception, and
its message quotes what was measured. A proof is one of the following kinds:

| Proof | What runs |
| --- | --- |
| number | The old and the new form on the same data, and a comparison of what they return. |
| interpreter | The old form, with its exception or its warning under `python -W error`, and the new form, which runs clean. |
| speed | The old form, timed at two settings. |

A rule is a `warning` when the proof passed and a pattern can't see whether the code wants the old
form. A rule is `info` when no proof of the listed kinds applies.

The rules were then run over scikit-lego's own source, docs scripts and tests at the 0.9.10 tag,
134 Python files, and every report was read. Of the 46 reports there, 42 are on a form that the
rule describes, most of them in tests that build a deprecated or superseded class or check that a
wrong call raises. The other 4 are on code that is right as it stands:

- the package's own export of `PandasTypeSelector`
- two tests of `add_lags` with a positive lag
- one test that passes `threshold=0.1` to `GMMOutlierDetector` only to build an object

After that reading, `debug-pipeline-default-log` was narrowed to leave a file alone when it
imports `logging` or uses pytest's `caplog`, which removed 5 reports. `unseeded-random-estimator`
was narrowed to leave test files alone, which removed 7.

| Rule | Severity | Proof | Reports on scikit-lego's files | On code that is right | Fix |
| --- | --- | --- | --- | --- | --- |
| [`thresholder-score`](#thresholder-score) | error | number: 0.887 at three thresholds against 0.740, 0.887 and 0.847 | 0 | 0 | none |
| [`estimator-transformer-proba`](#estimator-transformer-proba) | error | number: shape (800, 1) for 400 rows | 0 | 0 | none |
| [`column-capper-fraction`](#column-capper-fraction) | error | number: 9,910 of 10,000 values changed against 1,000 | 0 | 0 | none |
| [`dict-mapper-not-int`](#dict-mapper-not-int) | error | number: 0.5 and 1.7 became 0 and 1. Interpreter: ValueError | 0 | 0 | none |
| [`regression-outlier-column-name`](#regression-outlier-column-name) | error | number: coefficients 0, 0 and 1, and 11 rows flagged against 10 | 2 | 0 | none |
| [`pca-outlier-after-scaler`](#pca-outlier-after-scaler) | error | number: 320 rows flagged against 20 | 0 | 0 | none |
| [`outlier-threshold`](#outlier-threshold) | error | interpreter: ValueError. Number: 380 of 400 rows flagged against 20 | 1 | 1 | none |
| [`grouped-shrinkage-arguments`](#grouped-shrinkage-arguments) | error | number: probabilities bit equal with and without the keyword. Interpreter: TypeError | 0 | 0 | unsafe |
| [`add-lags-positive`](#add-lags-positive) | warning | number: accuracy 0.996 against 0.474 on noise | 2 | 2 | none |
| [`decay-on-shuffled-rows`](#decay-on-shuffled-rows) | warning | number: slope 1.621 against 3.028 | 0 | 0 | none |
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

Each rule in the group reports an argument that gives another number than the code asks for,
without an error, or that raises where the argument is read.

#### thresholder-score

**Severity:** error. **Fix:** none.

The rule reports `GridSearchCV`, `RandomizedSearchCV`, `cross_val_score`, `cross_validate` and the
other search and validation functions when they hold a `Thresholder(...)` and no `scoring=`. It
also reports `.score(...)` on a `Thresholder`. `Thresholder.score` returns the score of the inner
model, so it doesn't change with `threshold`.

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

xpd gives the replacement as advice and doesn't apply it.

On 400 rows with 20 percent positives, from `make_classification(n_samples=400, weights=[0.8],
random_state=0)` and `train_test_split(random_state=0)`, the grid search on the training rows gave
its mean test score, and `score` and `predict` ran on the test rows:

| Threshold | Grid search without `scoring` | With `scoring="accuracy"` | `Thresholder.score` | Accuracy of `predict` |
| --- | --- | --- | --- | --- |
| 0.1 | 0.887 | 0.740 | 0.850 | 0.710 |
| 0.5 | 0.887 | 0.887 | 0.850 | 0.850 |
| 0.9 | 0.887 | 0.847 | 0.850 | 0.840 |

scikit-learn's `FixedThresholdClassifier` made the same predictions as `Thresholder` at each
threshold, and its `score` gave 0.710, 0.850 and 0.840.

#### estimator-transformer-proba

**Severity:** error. **Fix:** none.

The rule reports `EstimatorTransformer(..., predict_func="predict_proba")` and
`"predict_log_proba"`. When the target has one dimension, the transformer reshapes the output to
one column, so the probabilities of the two classes stack into twice as many rows.

Example:

```python
features = EstimatorTransformer(LogisticRegression(), predict_func="predict_proba")
```

Use instead:

```python
features = EstimatorTransformer(LogisticRegression(), predict_func="decision_function")
stack = StackingClassifier([("lr", LogisticRegression())], stack_method="predict_proba")
```

xpd gives the replacement as advice and doesn't apply it.

A binary model on 400 rows came back from `transform` with the shape (800, 1). Inside a
`FeatureUnion`, and inside a pipeline before a model, `fit` raised ValueError on the row counts.
`decision_function` gave (400, 1), and `StackingClassifier(stack_method="predict_proba").transform`
gave one row per input row.

#### column-capper-fraction

**Severity:** error. **Fix:** none.

The rule reports `ColumnCapper(quantile_range=(0.05, 0.95))` and other ranges whose upper number is
1 or less. `ColumnCapper` reads the range in percent, as its default `(5.0, 95.0)` shows, so
fractions cap every value at a percentile below 1.

Example:

```python
capper = ColumnCapper(quantile_range=(0.05, 0.95))
```

Use instead:

```python
capper = ColumnCapper(quantile_range=(5, 95))
```

xpd gives the replacement as advice and doesn't apply it.

| `quantile_range` | Values changed, of 10,000 normal values | Values after capping |
| --- | --- | --- |
| `(0.05, 0.95)` | 9,910 | 92 distinct values from -3.272 to -2.379 |
| `(5, 95)` | 1,000 | from -1.628 to 1.640 |

#### dict-mapper-not-int

**Severity:** error. **Fix:** none.

The rule reports `DictMapper` with a dict literal that maps to a float with a fraction or to text,
and a `default` that is such a float, text or NaN. `DictMapper` casts every mapped value and the
default to int.

Example:

```python
mapper = DictMapper({"low": 0.5, "high": 1.7}, default=-1)
```

Use instead:

```python
mapper = DictMapper({"low": 0, "high": 1}, default=-1)
weights = frame["level"].map({"low": 0.5, "high": 1.7}).fillna(-1)
```

xpd gives the replacement as advice and doesn't apply it.

| Mapping | Result | Reported |
| --- | --- | --- |
| `{"a": 0.5, "b": 1.7}` with `default=-1` | 0, 1 and -1, with no warning | yes |
| a text value | ValueError "invalid literal for int() with base 10" | yes |
| `default=np.nan` | ValueError "cannot convert float NaN to integer" | yes |
| a float with no fraction, such as `1.0` | the same value | no |
| `{"a": "1"}` with `default=0`, a string that holds a whole number | 1 | no |

#### regression-outlier-column-name

**Severity:** error. **Fix:** none.

The rule reports `RegressionOutlierDetector(model, column="y")`, where the column is a name. The
detector finds the target of a frame by name, and then drops the column whose position equals
`column` from the features. A name equals no position, so the target stays among the features and
the inner model predicts it from itself.

Example:

```python
detector = RegressionOutlierDetector(LinearRegression(), column="y").fit(frame)
```

Use instead:

```python
position = frame.columns.get_loc("y")
detector = RegressionOutlierDetector(LinearRegression(), column=position).fit(frame.to_numpy())
```

xpd gives the replacement as advice and doesn't apply it.

On 500 rows with 10 planted outliers, the two forms gave the following fits:

| Form | Features | Coefficients | Spread of the residuals | Rows flagged |
| --- | --- | --- | --- | --- |
| `column="y"` on the frame | 3 | 0, 0 and 1 | 9.50e-16 | 11 |
| `column=2` on `frame.to_numpy()` | 2 | not given | 3.25 | the 10 planted rows and no other |

A position on a frame is wrong in another way, which a pattern can't see. The detector compares the
frame's column names with the number and finds no match, so the first column becomes the target.
scikit-lego's own test of the name form passes because the planted rows have the largest rounding
errors.

#### pca-outlier-after-scaler

**Severity:** error. **Fix:** none.

The rule reports `PCAOutlierDetection` and `UMAPOutlierDetection` without `variant="absolute"` in a
pipeline that holds `StandardScaler`, `RobustScaler` or `PowerTransformer`. The default
`variant="relative"` divides the reconstruction error of a row by the sum of the row's values,
which a centring step puts near zero.

Example:

```python
detector = make_pipeline(StandardScaler(), PCAOutlierDetection(n_components=1, threshold=0.05))
```

Use instead:

```python
detector = make_pipeline(StandardScaler(), PCAOutlierDetection(n_components=1, threshold=2.0, variant="absolute"))
```

xpd gives the replacement as advice and doesn't apply it, because the threshold has other
units.

| Data, 1,000 rows near a line with 20 planted outliers | Rows flagged by the relative variant, with `threshold=0.1` | By `variant="absolute"`, with `threshold=2.0` |
| --- | --- | --- |
| raw | the 20 planted rows | the 20 planted rows |
| after `StandardScaler` | 320, of which 19 were planted | the 20 planted rows |

After the scaler, 510 rows had a negative ratio. A scaler built with `with_mean=False`,
`with_centering=False` or `standardize=False` doesn't centre, and the rule leaves it alone.

#### outlier-threshold

**Severity:** error. **Fix:** none.

The rule reports `PCAOutlierDetection` and `UMAPOutlierDetection` built without a `threshold`. It
also reports `GMMOutlierDetector` and `BayesianGMMOutlierDetector` whose `threshold` is below 0.5
or above 1, with `method="quantile"` written out or left as the default.

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

xpd gives the replacement as advice and doesn't apply it.

Without a threshold, `fit` of the PCA and UMAP detectors raised ValueError "The `threshold` value
cannot be `None`." The mixture detectors keep the share `threshold` of the training rows as
inliers:

| `threshold` | Training rows flagged, of 400 |
| --- | --- |
| 0.05 | 380 |
| 0.4 | 240 |
| 0.5 | 200 |
| 0.95 | 20 |
| 0.99, the default | 4 |
| 2 | ValueError |

`threshold=2` ran with `method="stddev"`, which counts standard deviations. A call whose `method`
is anything but the text `"quantile"`, such as a variable, is left alone, because it may be
`"stddev"`. A detector inside a search is left alone, because the search may set the threshold.

#### grouped-shrinkage-arguments

**Severity:** error. **Fix:** unsafe: `shrinkage_kwargs={"alpha": 0.1}` for `alpha=0.1`, and the same for `min_n_obs=`. The other forms have no fix.

The rule reports spellings of shrinkage that fail on the grouped and hierarchical meta estimators:

- any extra keyword on `GroupedClassifier`, which drops it
- `shrinkage="constant"` or `"min_n_obs"` without `shrinkage_kwargs`
- a bare `alpha=` or `min_n_obs=` keyword

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
`shrinkage="relative"` its probabilities were bit equal to the probabilities without the keyword. They
differed by up to 0.29 from `HierarchicalClassifier(shrinkage="relative")`, which does shrink.

| Form | Error |
| --- | --- |
| `shrinkage="constant"` without a setting | TypeError "constant_shrinkage() missing 1 required positional argument: 'alpha'" in `fit` |
| a bare `alpha=` keyword | TypeError "got an unexpected keyword argument 'alpha'" |

`GroupedPredictor` took the bare keyword until scikit-lego 0.9.2, so the document about its bare
keyword carries `since: "0.9.2"`.

### Rows and fits

Each rule in the group reports a step fitted on rows it shouldn't see in that order, or a meta
estimator built around a model it can't fit.

#### add-lags-positive

**Severity:** warning. **Fix:** none.

The rule reports `add_lags` with a positive lag, as a number, in a list or from `range`.
`add_lags` shifts by the negative of the lag, so lag 1 puts the value of the next row on each row.
pandas `shift(1)` goes the other way.

Example:

```python
features = add_lags(frame, "sales", [1, 2])
```

Use instead:

```python
features = add_lags(frame, "sales", [-1, -2])
```

xpd gives the replacement as advice and doesn't apply it.

On a frame with sales 10 to 50, the two calls gave the following columns:

| Call | Values of the new column |
| --- | --- |
| `add_lags(frame, "sales", 1)` | 20, 30, 40, 50 |
| `add_lags(frame, "sales", -1)` | 10, 20, 30, 40, from the second row on |

On a series of pure noise, a model given the column from lag 1 predicted the sign of the next row
with accuracy 0.996. A model given the column from lag -1 predicted the sign of its own row with
accuracy 0.474. A positive lag is right when the new column is the target of a forecast, so the
rule is a warning.

A `range` is reported when its numbers are written out and it holds a positive lag, which means it
starts at 1 or more, or counts up to a stop of 2 or more. `range(0, -3, -1)` holds 0, -1 and -2
and is left alone.

#### decay-on-shuffled-rows

**Severity:** warning. **Fix:** none.

The rule reports `train_test_split` without `shuffle=False` in a file that builds a
`DecayEstimator`. The estimator weights each row by its position, the last row most, and never
reads a date. It is therefore right only on rows in time order.

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

xpd gives the replacement as advice and doesn't apply it.

On 2,000 rows whose slope changed from 1 to 3 at row 1,000, the model was fitted on the first
1,500 rows and scored on the last 500:

| Order of the 1,500 training rows | Slope | Mean absolute error |
| --- | --- | --- |
| in time order | 3.028 | 0.078 |
| shuffled, as the mean of 30 shuffles | 1.621 | 0.648 |

A pattern can't see which rows the estimator is fitted on, so the rule is a warning.

#### outlier-remover-before-model

**Severity:** error. **Fix:** none.

The rule reports `OutlierRemover` in a pipeline that also holds a step named like a regressor or a
classifier. The remover drops rows of X while the pipeline is fitted, and a scikit-learn pipeline
can't drop the same rows of y.

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

xpd gives the replacement as advice and doesn't apply it.

`fit` raised ValueError "Found input variables with inconsistent numbers of samples: [328, 400]".
A pipeline that ended in `KMeans`, which reads no y, fitted on the 328 kept rows, and the rule
leaves it alone.

#### decay-without-sample-weight

**Severity:** error. **Fix:** none.

The rule reports `DecayEstimator` around a `Pipeline`, and around a scikit-learn model whose `fit`
takes no `sample_weight`, such as `KNeighborsRegressor`, `GaussianProcessRegressor` and
`LinearDiscriminantAnalysis`. `DecayEstimator` passes its weights as
`fit(X, y, sample_weight=...)`.

Example:

```python
model = DecayEstimator(make_pipeline(StandardScaler(), Ridge()))
```

Use instead:

```python
model = make_pipeline(StandardScaler(), DecayEstimator(Ridge()))
```

xpd gives the replacement as advice and doesn't apply it.

The pipeline raised ValueError "Pipeline.fit does not accept the sample_weight parameter", and
`KNeighborsRegressor` raised TypeError "Model KNeighborsRegressor.fit() does not have
'sample_weight'". With the `DecayEstimator` around the last step, the pipeline fitted.

The list of models has 22 names, among them `PassiveAggressiveClassifier`,
`PassiveAggressiveRegressor`, `CCA` and `PLSCanonical`. It holds every classifier and regressor of
`sklearn.utils.all_estimators()` on scikit-learn 1.9.1 that takes no argument to build and whose
`fit` raised that TypeError inside a `DecayEstimator`.

#### zero-inflated-roles

**Severity:** error. **Fix:** none.

The rule reports `ZeroInflatedRegressor` with a class named like a regressor as the classifier, or
a class named like a classifier as the regressor. The classifier comes first and decides whether a
value is zero, and the regressor comes second.

Example:

```python
model = ZeroInflatedRegressor(LinearRegression(), LogisticRegression())
```

Use instead:

```python
model = ZeroInflatedRegressor(classifier=LogisticRegression(), regressor=LinearRegression())
```

xpd gives the replacement as advice and doesn't apply it.

`fit` raised ValueError "`classifier` has to be a classifier", and with a classifier in second
place it raised "`regressor` has to be a regressor". The rule reads the class names, and it knows
that `LogisticRegression` is a classifier.

### Scorers, datasets and helpers

Call each scikit-lego scorer, dataset loader and pipeline helper with the arguments it reads, because the forms below raise or return something other than the code expects.

#### scorer-called-as-metric

**Severity:** error. **Fix:** unsafe: `p_percent_score("sex")` for `make_scorer(p_percent_score("sex"))`. The other forms have no fix.

The rule reports `make_scorer` around `p_percent_score`, `equal_opportunity_score`,
`correlation_score` or `subset_score`. It also reports `p_percent_score(y_true, y_pred)` and
`equal_opportunity_score(y_true, y_pred)`. Each function takes the column to look at and returns a
scorer with the signature `(estimator, X, y)`.

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

`make_scorer` calls the scorer with `(y_true, y_pred)`, which the scorer can't take:

| Form | Result |
| --- | --- |
| `make_scorer` around each of the four functions, called | IndexError for three of them, and TypeError for `equal_opportunity_score` |
| the same scorer inside `GridSearchCV` | a best score of NaN, because scikit-learn caught the error |
| `p_percent_score(y, y_pred)` | a function, with no error |
| `p_percent_score("sex")` given to `scoring=` | a score |

#### dataset-frame-and-xy

**Severity:** error. **Fix:** none.

The rule reports `load_penguins`, `load_arrests`, `load_chicken`, `load_abalone`, `load_heroes` and
`load_hearts` called with both `return_X_y=True` and `as_frame=True`. The loaders test `as_frame`
first and return the whole frame, with the target in it.

Example:

```python
X, y = load_penguins(return_X_y=True, as_frame=True)
```

Use instead:

```python
frame = load_penguins(as_frame=True)
X, y = frame.drop(columns="species"), frame["species"]
```

xpd gives the replacement as advice and doesn't apply it.

Unpacking raised ValueError "too many values to unpack (expected 2)" for all six loaders.
scikit-learn's own loaders return a pair of frames for the same two keywords. `fetch_creditcard`
passes both keywords to `fetch_openml` and is left alone.

#### log-step-extra-bare

**Severity:** error. **Fix:** none.

The rule reports `@log_step_extra` without arguments, as a bare decorator or with empty
parentheses. `log_step` works both ways, but `log_step_extra` takes the log functions as its
arguments. As a bare decorator it takes the decorated function as a log function.

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

xpd gives the replacement as advice and doesn't apply it.

| Form | Result |
| --- | --- |
| `@log_step_extra` | Calling the function returned another function, not a frame, and raised nothing. |
| `@log_step_extra()` | ValueError "Supply at least one log_function for log_step_extra" |

#### selector-without-feature-names

**Severity:** error. **Fix:** none.

The rule reports `set_output(...)` and `get_feature_names_out()` on a pipeline that holds
`ColumnSelector`, `ColumnDropper` or `TypeSelector`, called on the pipeline itself or on a name the
file assigns it to. The three classes have `get_feature_names` and neither of the two scikit-learn
methods.

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

xpd gives the replacement as advice and doesn't apply it.

For each of the three classes, `set_output` raised ValueError "Unable to configure output for
ColumnSelector(...) because `set_output` is not available.", and `get_feature_names_out` raised
AttributeError "Estimator columnselector does not provide get_feature_names_out". The
`ColumnTransformer` form did both, and it names the columns `keep__age` and `keep__fare`.

#### debug-pipeline-default-log

**Severity:** info. **Fix:** none.

The rule reports `log_callback="default"` on `DebugPipeline` and `make_debug_pipeline` in a file
that neither imports `logging` nor uses pytest's `caplog`. The default callback logs each step at
level INFO, which Python hides until logging is set up.

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

xpd gives the replacement as advice and doesn't apply it.

Without the setup, fitting the pipeline printed 0 characters. After
`logging.basicConfig(level=logging.INFO)` it logged one line for each step, such as
`[StandardScaler()] shape=(400, 6) time=0s`. Logging may be set up in another module, so the rule
is info.

### Removed, deprecated and superseded forms

Each rule in the group reports a form that raises or warns on scikit-lego 0.9.10, and each removed
form raised when it was run. The deprecated and superseded forms were run under `python -W error`,
where the old form raised its warning and the new form ran clean. The messages take each
replacement from the text of the warning or from the release that made the change. A fix of a
removed form turns code that raises into code that runs, so no such fix is safe.

#### removed-in-sklego

**Severity:** error. **Fix:** safe: `as_frame=` for `give_pandas=` on the dataset loaders, which gave equal values with either keyword on scikit-lego 0.6.3. unsafe: `DemographicParityClassifier` for `FairClassifier`, `GroupedPredictor` for `GroupedEstimator`, and `sklego.preprocessing` for the import of `OutlierRemover` from `sklego.meta` or `sklego.meta.outlier_remover`, because on 0.7.4 each old name is another class than the new one, and `decay_kwargs={"decay_rate": 0.99}` for `decay=0.99`, which no release took beside `decay=` and which gives the same weights only to rounding. The other forms have no fix.

The rule reports forms that scikit-lego removed. An import inside a `try` block that catches the
error is left alone.

| Old form | Removed in | Write instead |
| --- | --- | --- |
| an import of `PatsyTransformer` or its module | 0.8.0 | `FormulaicTransformer`, with no fix |
| an import of `FairClassifier` or its module | 0.8.0 | `DemographicParityClassifier` |
| an import of `GroupedEstimator` or its module | 0.8.0 | `GroupedPredictor` |
| `from sklego.meta import OutlierRemover` | 0.8.0 | `from sklego.preprocessing import OutlierRemover` |
| `give_pandas=` on the dataset loaders | 0.6.6 | `as_frame=` |
| `decay=` on `DecayEstimator` | 0.7.0 | `decay_kwargs={"decay_rate": ...}` |

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
`PatsyTransformer` has no fix, because `FormulaicTransformer` reads the formula with formulaic and
not with patsy. The weights that `decay=0.97` gave in scikit-lego 0.6 are equal to the weights of
`decay_kwargs={"decay_rate": 0.97}` to rounding, and not bit for bit. Each document carries `since`
with the release that removed its form.

#### deprecated-in-sklego

**Severity:** warning. **Fix:** safe: `ClusterFoldValidation` for `KlusterFoldValidation`, `TypeSelector` for `PandasTypeSelector`, `date_series=` for `date_serie=`, `n_features_in_` for `dim_`, `num_fit_cols_`, `n_columns_` and `num_cols_`, `n_fitted_levels_` for `n_levels_`, `coef_` for `coefs_`, and `penalty=None` for `penalty="none"`. `allowed_strategies`, `allowed_methods` and `allowed_effects` have no fix.

The rule reports the names, keywords and fitted attributes that raise a DeprecationWarning:

| Old form | Write instead |
| --- | --- |
| `KlusterFoldValidation` | `ClusterFoldValidation` |
| `PandasTypeSelector` | `TypeSelector` |
| `TimeGapSplit(date_serie=)` | `date_series=` |
| the attributes `dim_`, `num_fit_cols_`, `n_columns_` and `num_cols_` | `n_features_in_` |
| `n_levels_` | `n_fitted_levels_` |
| `coefs_` | `coef_` |
| `penalty="none"` on the fairness classifiers | `penalty=None` |
| the properties `allowed_strategies`, `allowed_methods` and `allowed_effects` | no fix |

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

Every fix was run before and after on the pack's sample file, and the folds, the selected columns,
the attribute values, the coefficients and the predictions were equal.

An old attribute name is reported only on an object that the file builds from a scikit-lego class
with that property. The object is a call chain that starts at the class, as in
`ColumnCapper().fit(X).n_columns_`, or a name that the file assigns such a chain to:

| Attribute | Classes |
| --- | --- |
| `dim_` | `RandomRegressor`, `RandomAdder`, `DictMapper`, `OutlierRemover` |
| `num_fit_cols_` | the two mixture naive Bayes classifiers |
| `n_columns_` | `ColumnCapper` |
| `num_cols_` | `IntervalEncoder` |
| `n_levels_` | `HierarchicalRegressor`, `HierarchicalClassifier` |

`coefs_` is reported only in a file that names `ProbWeightRegression` or `DeadZoneRegressor` and no
MLP, because scikit-learn's MLP models have their own `coefs_`. The same name on any other object,
such as `cfg.dim_` on your own class, is left alone, because the rename would raise AttributeError
there. An assignment to the name, and the definition of a function or class of the name, are the
file's own and are left alone. Each document carries `since` with the release that began to warn.

#### superseded-estimator

**Severity:** warning for the classes that warn, and info for `IdentityTransformer()`. **Fix:** unsafe: `FunctionTransformer()` for `IdentityTransformer()`, in a file that imports `FunctionTransformer` from `sklearn.preprocessing`. The step is then another class, whose parameters and repr differ. The other classes have no fix.

The rule reports the classes that scikit-lego tells users to replace since 0.9.8, where building
one raises a UserWarning that names the replacement. It also reports `IdentityTransformer()`
without arguments.

| Class | Replacement |
| --- | --- |
| `QuantileRegression`, `LADRegression` | scikit-learn's `QuantileRegressor` |
| `RepeatingBasisFunction` | scikit-learn's `SplineTransformer(extrapolation="periodic")` |
| `DemographicParityClassifier`, `EqualOpportunityClassifier` | fairlearn's `ThresholdOptimizer` around a `LogisticRegression` |
| `IdentityTransformer()` | `FunctionTransformer()` |

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

xpd gives the other replacements as advice and doesn't apply them, because each one is another
model with other parameter names. On 500 rows `QuantileRegression(quantile=0.8)` and
`QuantileRegressor(quantile=0.8, alpha=0)` gave the same coefficients to three decimals. The spline
is another basis than the radial one, so its columns are other numbers. `IdentityTransformer()` and
`FunctionTransformer()` both returned their input itself.

### Speed and repeatable runs

Keep `GroupTimeSeriesSplit` to few folds, and give each estimator that draws random numbers a seed, so a run finishes and repeats.

#### group-time-series-split-folds

**Severity:** warning. **Fix:** none.

The rule reports `GroupTimeSeriesSplit` with 6 or more splits. The class tries every combination of
split points over the groups to find the most even folds, so each extra fold multiplies the time.

Example:

```python
folds = GroupTimeSeriesSplit(n_splits=6)
```

Use instead:

```python
folds = GroupTimeSeriesSplit(n_splits=5)
```

xpd gives the replacement as advice and doesn't apply it.

| Data | `split` at 4 splits | At 5 splits | At 6 splits |
| --- | --- | --- | --- |
| 3,000 rows in 60 groups | 0.09 s | 1.08 s | 10.48 s |
| 3,000 rows in 40 groups, with the default number of threads | 0.015 s | 0.10 s | 1.07 s |

The class warns itself above 5 splits when there are more than 30 groups. With a few groups the search is short, so
the rule is a warning.

#### unseeded-random-estimator

**Severity:** info. **Fix:** none.

The rule reports `RandomRegressor` and `RandomAdder` built without `random_state`, outside test
files. Each call then draws new numbers.

Example:

```python
baseline = RandomRegressor(strategy="normal")
```

Use instead:

```python
baseline = RandomRegressor(strategy="normal", random_state=0)
```

xpd gives the replacement as advice and doesn't apply it.

Two `predict` calls of one fitted `RandomRegressor()` differed, and so did two `transform` calls of
one `RandomAdder()` on its training rows. Both pairs were equal with `random_state=0`.

### Habits the pack does not report

The pack has no rule for the following habits:

- `MaximumRelevanceMinimumRedundancy` with the default `kind="auto"` scores a whole-number
  regression target as classes. On one such target it picked the features 0, 1 and 3, against 0, 1
  and 4 with `kind="regression"`. A pattern can't see the target, and a rule on every call without
  `kind` would report the library's own tests.
- `RegressionOutlierDetector` with a column position on a frame, and `TimeGapSplit`, `RandomAdder`
  and `OutlierRemover` given an input type they don't take, depend on what `fit` is given, which a
  pattern can't see.
- The extras cvxpy, umap-learn and formulaic have no rule, because a package check can't see which
  estimators a project uses.

## MAPIE

The MAPIE rules report the MAPIE 0.x API, which raises on MAPIE 1, code that breaks the coverage
that a conformal method promises, arguments that MAPIE refuses, and code that runs the model twice,
in each file that imports `mapie`. [MAPIE](https://mapie.readthedocs.io/) builds prediction
intervals and prediction sets around a scikit-learn model with conformal methods, which promise
that the intervals or sets cover a chosen share of new rows. Every number about the MAPIE rules was
measured on MAPIE 1.6.0 with scikit-learn 1.9.1. The examples assume that the MAPIE and
scikit-learn names they use are imported, and names such as `X_train`, `X_conf`, `y_test` and
`model` stand for values from your program.

MAPIE 1.0 changed the API:

| MAPIE 0.x | MAPIE 1 |
| --- | --- |
| `MapieRegressor`, `MapieClassifier` | one class per method |
| `alpha` | `confidence_level` on the constructor, which is `1 - alpha` |
| `fit` | `fit` and `conformalize` |

A rule document about the change carries `since`. xpd reads the installed version from the
project's venv and skips the document on an older release, so a project that still has MAPIE 0.9
gets no report about the 0.x API. When the venv has no MAPIE, or there is no venv, every document
runs.

Some rules read the names of variables, such as `X_train`, because xpd reads the code and never
the values it holds. A name that misleads therefore gives a wrong report. The other rules tie a call to an object by the
variable the file assigns it to, and to rows by the variable given to an earlier call.

### The proof each rule needs

Each rule document ships only with a proof that was run. A proof is one of the following kinds:

| Proof | What it does |
| --- | --- |
| coverage | Runs the old and the new form over 30 seeds with `confidence_level=0.9`, and gives the mean share of test rows that the intervals or sets cover. |
| interpreter | Runs the old form on MAPIE 1.6.0, under `python -W error` for a warning. It passes when the old form raises and the new form runs clean. |
| speed | Times both forms on one thread, as the median of 5 or 7 runs, at two data sizes, and compares the arrays they return. |
| rerun | Runs the old form twice in fresh objects. It passes when the two results differ and the new form gives the same result twice. |

A document is `error` only on a coverage proof or an interpreter proof, and its message quotes
what was measured. A document that can also match code written that way on purpose is `warning`, such as a level
below 0.5 in a plot of coverage against the level.

The rules were then run over MAPIE's own code at the 1.6.0 tag, and every report was read. The
code was its 162 Python files, which hold the library, its tests and its examples, and the Python
code of its 21 documentation pages and notebooks. A rule that reported correct code there was
narrowed, and the table gives what is left.

| Rule | Severity | Proof | Reports on MAPIE's own code | Wrong advice | Fix |
| --- | --- | --- | --- | --- | --- |
| [`removed-in-mapie`](#removed-in-mapie) | error | interpreter: ImportError, ModuleNotFoundError, TypeError, ValueError, AttributeError | 1 | 0 | unsafe |
| [`deprecated-in-mapie`](#deprecated-in-mapie) | warning | interpreter: FutureWarning, and intervals equal with and without `gamma` | 2 | 0 | safe |
| [`alpha-as-confidence-level`](#alpha-as-confidence-level) | warning, error | coverage: 0.108 against 0.907, and interpreter: ValueError | 0 | no reports | unsafe |
| [`calibrate-on-training-rows`](#calibrate-on-training-rows) | error | coverage: 0.468 against 0.907 | 6 | 1 | none |
| [`coverage-on-seen-rows`](#coverage-on-seen-rows) | error, warning | coverage: 1.000 against 0.913 | 0 | no reports | none |
| [`shuffled-time-series-split`](#shuffled-time-series-split) | warning | coverage: 0.921 against 0.536 | 0 | no reports | unsafe |
| [`prefit-estimator-not-fitted`](#prefit-estimator-not-fitted) | error | interpreter: NotFittedError, AttributeError, ValueError | 0 | no reports | none |
| [`quantile-estimator-loss`](#quantile-estimator-loss) | error | interpreter: ValueError | 0 | no reports | unsafe |
| [`predict-function-returns-labels`](#predict-function-returns-labels) | error | interpreter: IndexError | 0 | no reports | unsafe |
| [`risk-control-method`](#risk-control-method) | error, warning | interpreter: ValueError, UserWarning | 0 | no reports | none |
| [`predict-and-predict-interval`](#predict-and-predict-interval) | warning | speed: 1.97 times | 3 | 3 | none |
| [`refit-per-confidence-level`](#refit-per-confidence-level) | warning | speed: 2.87 times | 1 | 1 | none |
| [`unseeded-resampling`](#unseeded-resampling) | info | rerun: the result differs | 36 | 0 | none |

A coverage proof gives the old form's mean against the new form's mean. A speed proof gives the
lowest ratio over the forms the rule reports and both sizes. Wrong advice counts the reports where
following the message would not fix a problem in the code.

All 5 reports with wrong advice are in MAPIE's tests:

- Three tests call `predict` beside `predict_interval` or `predict_set` to assert that the two
  agree.
- One test builds one object per level to compare it with the list form.
- One test conformalizes on the rows of a `DummyRegressor` that predicts a constant, which learns
  nothing from them.

Four more reports of `calibrate-on-training-rows` are tests that fit and conformalize on the same
rows to check the mechanics and don't measure coverage. The sixth is an example in MAPIE's
gallery that conformalizes on its training rows. No rule reported the library's own source, and
the examples, documentation pages and notebooks got 9 reports, none with wrong advice.

Recall was not measured, because no MAPIE code from other projects was held back. The table
therefore says how often a rule is wrong when it reports, and not how often it misses. The pack has no rule for
`mapie.exchangeability_testing`, `mapie.experimental` and the FWER procedures. Their misuse is in
the data, such as rows shuffled before an exchangeability test, and a pattern can't see it.

### The 0.x API

Each rule in the group reports a form that raises or warns on MAPIE 1.6.0. Each form was run
there, and the messages take each replacement from the v1 release notes or from the error text. A
fix of a removed form turns code that raises into code that runs, so it can't be compared with the
old form and is never safe.

#### removed-in-mapie

**Severity:** error. **Fix:** safe: `numpy.typing` for `mapie._typing`, because on MAPIE 0.9.2 `ArrayLike` and `NDArray` of `mapie._typing` are the objects of `numpy.typing`. unsafe: `mapie.metrics.regression`, `mapie.metrics.classification` or `mapie.metrics.calibration` for `mapie.metrics` when every imported name is in that module, `confidence_level=1 - alpha` for `alpha=` on a constructor, on `TimeSeriesRegressor.predict`, on `regression_mwi_score` and on `coverage_width_based` when the value is a number or a name, `fit_params={"sample_weight": w}` for `sample_weight=w`, `update` for `partial_fit`, and one stacked array for the two bounds given to `regression_coverage_score` and `regression_mean_width_score`. A list of numbers given as `alpha=` on a constructor or on `TimeSeriesRegressor.predict` becomes `confidence_level=` with 1 minus each number, `fit_conformalize` replaces `fit` on a cross or jackknife object, `method=` becomes `conformity_score=` on a classifier when its value is `"lac"`, `"aps"`, `"raps"` or `"top_k"`, and `method=` is taken out of `TopLabelCalibrator`, which drops the comments between the arguments. The removed classes and modules, `alpha=` on `predict_interval`, `predict_set` and `predict`, `cv=` and the other values of `method` have no fix, because the rewrite reaches other calls or the message names more than one. No release ran an unsafe fix's old and new form side by side, except `sample_weight=` and `fit_params=` on `TimeSeriesRegressor.fit` in 1.4.0, which ignored `sample_weight=` and gave other values, and `partial_fit` and `update` in 1.1.0, which are two methods.

The rule reports the MAPIE 0.x API that raises on MAPIE 1.6.0:

| Old form | Where the rule reports it |
| --- | --- |
| `MapieRegressor`, `MapieClassifier`, `MapieQuantileRegressor`, `MapieTimeSeriesRegressor`, `MapieCalibrator`, `MapieMultiLabelClassifier`, `PrecisionRecallController`, `MondrianCP`, `ConformityScore` | an import |
| modules that are gone, such as `mapie.quantile_regression`, `mapie.control_risk` and `mapie._typing` | an import |
| metric functions from `mapie.metrics` itself | an import |
| `alpha=` | the MAPIE 1 constructors, `predict_interval`, `predict_set`, `predict` of an object that the file builds from a MAPIE 1 class, and the metrics |
| `fit` | a `CrossConformalRegressor`, `CrossConformalClassifier` or `JackknifeAfterBootstrapRegressor` |
| `cv=` | the split classes |
| `cv="prefit"` or `cv="split"` | the cross classes |
| `method=` | the classifiers and `TopLabelCalibrator` |
| `method="naive"` | any class |
| `sample_weight=` | `fit` or `fit_conformalize` |
| `partial_fit` | a `TimeSeriesRegressor` |
| the lower and upper bounds as two arguments | a metric |

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

The rule leaves alone an import inside a `try` block that catches the error, code behind a version
check, and code inside `pytest.raises`. Each of its documents carries the release that removed the
form:

| Form | Removed in |
| --- | --- |
| most forms | 1.0 |
| `partial_fit` | 1.2 |
| `PrecisionRecallController` | 1.3 |
| `sample_weight=` on `TimeSeriesRegressor.fit` | 1.4.1 |

The forms around `partial_fit` and `sample_weight` changed in the following releases:

| Form | Worked on | Failed on |
| --- | --- | --- |
| `TimeSeriesRegressor.partial_fit` | 1.1.0, in the wheel | 1.2.0, where it is gone |
| `sample_weight=` on `TimeSeriesRegressor.fit` | up to 1.4.0 | 1.4.1 and later, with TypeError |
| `fit_params={"sample_weight": w}` | 1.4.0 and later | 1.0.1 to 1.3.0 |
| `sample_weight=` on `fit` of the split classes | not given | 1.0.1 already |

The fix `confidence_level=1 - alpha` is written only for a number or a name. `alpha=[0.05, 0.2]`
was common in 0.x code, and `1 - [0.05, 0.2]` raises TypeError, so a list of numbers gets
`confidence_level=[1 - 0.05, 1 - 0.2]`. Another expression is reported without a fix, because the
file does not show whether it is one number or a list. `confidence_level=[0.95, 0.8]` gave
intervals of shape (3, 2, 2) for 3 rows.

`predict(X, alpha=0.1)` raised TypeError on each of the split, cross, jackknife and quantile
classes. Their level is set in the constructor, so the `predict` document has no fix. `fit` raised
AttributeError on `CrossConformalRegressor`, `CrossConformalClassifier` and
`JackknifeAfterBootstrapRegressor`, on 1.0.1 and on 1.6.0. The rule reports it on a name that the
file assigns from one of the three and from nothing else. `CrossConformalizedQuantileRegressor` has
a `fit` on 1.6.0 and is left alone.

The rule reports an import and doesn't rename the class, because the constructors differ.
`MapieRegressor(model, cv="prefit")` becomes `SplitConformalRegressor(model, prefit=True)`, and
`alpha` moves from `predict` to the constructor. Two fixes write `np.` and apply only in a file
with `import numpy as np`. On the same bounds, the fixed calls of `regression_coverage_score`,
`regression_mean_width_score` and `coverage_width_based` on MAPIE 1.6.0 returned the value that
the old calls returned on MAPIE 0.9.2, bit for bit, and `regression_mwi_score` returned it to 15
digits.

#### deprecated-in-mapie

**Severity:** warning. **Fix:** safe: `BinaryRisk` for `BinaryClassificationRisk`, and `aggregate_point_predictions=` for `aggregate_predictions=` and for `ensemble=` on a `JackknifeAfterBootstrapRegressor`. `update` has no fix, because the call of `adapt_conformal_inference` is a new statement.

The rule reports forms that MAPIE renamed or stopped reading:

| Old form | Since | Write instead |
| --- | --- | --- |
| `BinaryClassificationRisk` | 1.4 | `BinaryRisk` |
| `aggregate_predictions=` on `CrossConformalRegressor.predict` and `predict_interval` | 1.4.1 | `aggregate_point_predictions=` |
| `ensemble=` on the same methods of `JackknifeAfterBootstrapRegressor` | 1.4.1 | `aggregate_point_predictions=` |
| `gamma=`, `confidence_level=` and `optimize_beta=` on `TimeSeriesRegressor.update`, which reads none of them | 1.3 | `update` without them, and `adapt_conformal_inference` for `gamma` |

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

The fixes are safe. A controller built with `BinaryRisk` picked the same thresholds and gave the
same predictions as one built with `BinaryClassificationRisk`, and the intervals with the new
keywords were bit equal to the ones with the old keywords. A file that imports both risk classes
ends with `BinaryRisk` twice in its import.

`ensemble=` is reported only on an object that the file assigns from
`JackknifeAfterBootstrapRegressor` and from no other class, because `TimeSeriesRegressor.predict`
still takes it. With `method="aci"`, the intervals after
`update(X, y, confidence_level=0.9, gamma=0.5)` were bit equal to the ones after
`update(X, y, confidence_level=0.9)` on MAPIE 1.3.0, 1.4.0 and 1.6.0. They differed on MAPIE
1.2.0, which needs `confidence_level` in `update`, so the `update` document carries `since: "1.3"`.

### Coverage that does not hold

Each rule in the group reports code that breaks the coverage a conformal method promises. The
promise is that the intervals cover a share of new rows, and it needs the following conditions:

- conformity scores from rows the model did not learn from
- a level that is the share to cover
- test rows that the method has not seen

Each number below is the mean share of test rows covered over the seeds 0 to 29, with a
`RandomForestRegressor` of 50 trees unless another model is named. The regression data are 1,000
rows of 5 normal columns, with a target of the first column, the sine of the second and normal
noise, split into 60, 20 and 20 percent by `train_conformalize_test_split`.

#### alpha-as-confidence-level

**Severity:** warning for a level below 0.5 and for a variable named `alpha` or `delta`, and error for a percent. **Fix:** unsafe, because the level changes to the one the code meant: `confidence_level=1 - 0.1` for `confidence_level=0.1`, 1 minus each number of a list, `1 - alpha` for the level of `regression_mwi_score` and `coverage_width_based`, and `confidence_level=0.90` for `confidence_level=90`. A variable named `alpha` or `delta` and any other percent have no fix, because the file does not show whether the variable holds a list or which share the number means.

The rule reports a `confidence_level` that holds the share to miss, such as
`confidence_level=0.1`, `confidence_level=[0.05, 0.1]` and `confidence_level=alpha`. It also
reports the same values given to `regression_mwi_score` and `coverage_width_based` in the place of
the level, and a percent such as `confidence_level=90`.

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

| `confidence_level` | Share of test rows covered | Width of the intervals |
| --- | --- | --- |
| 0.1 | 0.108 | 0.292 |
| 0.9 | 0.907 | 3.609 |

MAPIE runs both levels without a warning. The rule reads a literal below 0.5 on the constructors
of the conformal classes and on `TimeSeriesRegressor.predict`, and it leaves alone 0.5 itself and a
list with a level of 0.5 or more. A risk controller is left alone, because its level is
`1 - delta` and a low one is a choice.

A percent raises "Invalid confidence_level or alpha. Allowed values are between 0 and 1", but only
from `predict_interval` or `predict_set`, after the model is fitted. A risk controller raises
"delta must be in (0, 1]".

#### calibrate-on-training-rows

**Severity:** error. **Fix:** none. The rows held out from the fit are not in the file.

The rule reports `conformalize` on the training rows, known by the name of its first argument or
because an earlier `fit` got the same variable. It also reports `calibrate` of a risk controller on
the training rows, and `fit` of a `TopLabelCalibrator` or `VennAbersCalibrator` with
`cv="prefit"` on the training rows.

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

xpd gives the replacement as advice and doesn't apply it.

| Form | Old form | Right form |
| --- | --- | --- |
| a regressor conformalized on its training rows | 0.468 coverage | 0.907 with held-out rows |
| a `RandomForestClassifier` conformalized on its training rows | 0.467 coverage | 0.906 |
| a model fitted on rows that include the conformalization rows | 0.473 coverage | not given |
| a `GridSearchCV` fitted on the conformalization rows | 0.679 coverage | 0.902 |
| a `BinaryClassificationController` with a target precision of 0.85, calibrated on the training rows | precision 0.638 on test rows, below the target in 30 of 30 runs | with held-out rows, a threshold in 1 of 30 runs and none below the target |
| a `TopLabelCalibrator` with `cv="prefit"` fitted on the training rows | top-label calibration error 0.178 | 0.085 with held-out rows, and 0.124 without calibration |
| a `VennAbersCalibrator` fitted on the training rows | top-label calibration error 0.128 | 0.061 |

The rule reads the name for `train`, `trn`, `tr`, `fit` and `training`, and leaves alone a name
with `conf`, `calib`, `cal`, `val`, `hold`, `test` or `res` in it. The form without a name needs an
earlier `fit` with a target in the same block or the same chain. A scaler fitted on the
conformalization rows is therefore left alone, and so is a class that calls `fit` and
`conformalize` on itself. A `fit` is left out when a statement between it and the `conformalize`
assigns to the variable, as in `X, y = load("conformalize")`, because the name then holds other
rows.

A model picked from six settings by its score on the conformalization rows covered 0.901, against
0.904 for a model picked by cross-validation on the training rows, so the rule doesn't report it.

#### coverage-on-seen-rows

**Severity:** error for rows given to `fit` or `fit_conformalize`, and warning for rows given to `conformalize`. **Fix:** none. The rows held out from the fit are not in the file.

The rule reports `predict_interval` and `predict_set` on the variable that an earlier statement gave
to `fit`, `fit_conformalize` or `conformalize` of the same object, where the result goes into a
MAPIE coverage metric.

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

xpd gives the replacement as advice and doesn't apply it.

| Object | Rows given to | Coverage of the same rows | Coverage of the test rows |
| --- | --- | --- | --- |
| `CrossConformalRegressor` | `fit_conformalize` | 1.000 | 0.913 |
| `SplitConformalRegressor` | `fit` | 1.000 | 0.907 |
| any | `conformalize` | 0.905, between 0.900 and 0.905 in each of the 30 runs | not given |

`conformalize` picks the quantile that covers its rows, so the 0.905, with a standard error of
0.000, shows nothing about new rows. The `conformalize` document is therefore a warning.

The rule reports only where the result reaches a coverage metric. The result reaches one when the
call, or the second name the call is assigned to, is an argument of `regression_coverage_score`,
`classification_coverage_score`, the size-stratified coverage (SSC) functions,
`regression_mwi_score` or `hsic`. A prediction for seen
rows that is only plotted is left alone, and so is one for a variable that a statement in between
assigns other rows to.

#### shuffled-time-series-split

**Severity:** warning. **Fix:** unsafe: `shuffle=False` is added at the end of the arguments, and the comments between them are dropped. The parts then hold other rows. A split with `stratify` has no fix, because scikit-learn refuses `stratify` with `shuffle=False`.

The rule reports `train_test_split` and `train_conformalize_test_split` without `shuffle=False`,
where the first result goes into `fit` of a `TimeSeriesRegressor`.

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

On a random walk of 900 rows, the intervals covered the following shares of the test rows:

| Model | Test rows | Coverage |
| --- | --- | --- |
| `TimeSeriesRegressor` | a shuffled test part | 0.921 |
| `TimeSeriesRegressor` | a test part after the training rows | 0.536 |
| `SplitConformalRegressor` on a shuffled split | its own test rows | 0.891 |
| `SplitConformalRegressor` on a shuffled split | the 300 rows that came after them | 0.144 |

The rule can't see that the rows are in time order, so it is a warning.

### Arguments that raise

Each rule in the group reports a form that raises when the object is used on MAPIE 1.6.0. Code
inside `pytest.raises` or `pytest.warns` is left alone.

#### prefit-estimator-not-fitted

**Severity:** error. **Fix:** none. Each message names two rewrites, and the code does not show which one is meant.

The rule reports `SplitConformalRegressor` and `SplitConformalClassifier` with an estimator built
in the argument list and no `prefit=False`, and `fit` on such an object without `prefit=False`. It
also reports `ConformalizedQuantileRegressor(prefit=True)` with one estimator.

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

xpd gives the replacement as advice and doesn't apply it.

`prefit` is `True` by default on the split classes, which says the estimator is already fitted.
With an estimator built in the argument list, each call raised:

| Call | Error |
| --- | --- |
| `conformalize`, with a regressor | NotFittedError |
| `conformalize`, with a classifier | "Invalid classifier. Fitted classifier does not contain 'classes_' attribute" |
| `fit` | "The fit method must be skipped when the prefit parameter is set to True" |

The rule reads a class name or `make_pipeline` as a new estimator, so `LinearRegression().fit(X, y)`
and a model loaded from a file are left alone. A wrapper around a fitted model is fitted too.
`SplitConformalRegressor(FrozenEstimator(fitted))` ran, so `FrozenEstimator` is left alone. A class
that is given a `.fit(...)` chain is left alone too. `fit` is reported on an object that the file assigns
from a split class without `prefit=False` and from nothing else.

`ConformalizedQuantileRegressor` has `prefit=False` as its default. With `prefit=True` it needs
three fitted estimators, for the lower quantile, the upper quantile and the median. A
`QuantileRegressor` or a `HistGradientBoostingRegressor` alone raised "Estimator for prefit must
be an iterable object". A `GradientBoostingRegressor` alone, which can be iterated over its trees,
raised AttributeError when it was not fitted and "You need to have provided 3 different
estimators" when it was. The rule reads only a class built in the argument list, with or without a `fit`
chained to it, as one estimator. A call of a function may return the list of three, and
`ConformalizedQuantileRegressor(fit_three(X, y), prefit=True)` ran.

The conditional classes `ConditionalSplitConformalRegressor` and
`ConditionalSplitConformalClassifier` raised the same errors, and the rule reads them by their
`estimator` keyword.

#### quantile-estimator-loss

**Severity:** error. **Fix:** unsafe, because the old fit raises: `loss="quantile"` in place of the loss of `GradientBoostingRegressor` and `HistGradientBoostingRegressor`, at the end of the arguments, which drops the comments between them. `LGBMRegressor`, which needs an alpha, a call with a positional argument and another model have no fix.

The rule reports `ConformalizedQuantileRegressor` and `CrossConformalizedQuantileRegressor` with an
estimator built in the argument list that has no quantile loss. Such an estimator is
`GradientBoostingRegressor` or `HistGradientBoostingRegressor` without `loss="quantile"`,
`LGBMRegressor` without `objective="quantile"`, or a scikit-learn regressor that has none, such as
`RandomForestRegressor`.

Example:

```python
mapie = ConformalizedQuantileRegressor(GradientBoostingRegressor(random_state=0), confidence_level=0.9)
```

Use instead:

```python
mapie = ConformalizedQuantileRegressor(GradientBoostingRegressor(loss="quantile", random_state=0), confidence_level=0.9)
```

| Estimator | Result |
| --- | --- |
| a boosting model without the quantile loss | "You need to set the loss/objective argument of your base model to `quantile`" |
| a model of another class | "The base model is not supported" |
| `GradientBoostingRegressor(loss="quantile")`, `HistGradientBoostingRegressor(loss="quantile")` | runs, because MAPIE sets the quantile itself |
| `LGBMRegressor(objective="quantile")` | "The matching parameter `alpha_name` for estimator does not exist", until it also gets an `alpha` |

The rule reads only an estimator built in the argument list. A `loss` or `objective` that isn't
written as text, such as `loss=loss`, may hold `"quantile"` and is left alone.

#### predict-function-returns-labels

**Severity:** error. **Fix:** unsafe: `model.predict_proba`.

The rule reports `BinaryClassificationController` with `model.predict` as its `predict_function`.

Example:

```python
controller = BinaryClassificationController(model.predict, "precision", target_level=0.8)
```

Use instead:

```python
controller = BinaryClassificationController(model.predict_proba, "precision", target_level=0.8)
```

With `model.predict`, `calibrate` raised IndexError, "Error when calling the predict_function",
because the controller reads the probability of the positive class from the second column.
`MultiLabelClassificationController` ran with `predict`, so the rule leaves it alone.

#### risk-control-method

**Severity:** error for a method the risk does not take and for a missing `confidence_level`, and warning for an argument that `"crc"` ignores. **Fix:** none. A missing `confidence_level` is a value that the file does not hold, and each other message names two rewrites.

The rule reports `MultiLabelClassificationController` and `SemanticSegmentationController` with a
method that the risk doesn't take, or without a `confidence_level` that the method needs, or with
an argument that the method ignores. Each error comes from `calibrate`:

| Arguments | Result | Severity |
| --- | --- | --- |
| `risk="precision"` with `method="crc"` or `"rcps"` | "Invalid method 'crc' for risk 'precision'. Valid methods are: ['ltt']" | error |
| `method="ltt"` with recall | "Invalid method 'ltt' for risk 'recall'. Valid methods are: ['crc', 'rcps']" | error |
| `method="rcps"` or `risk="precision"`, with no `confidence_level` | "Invalid confidence_level. confidence_level cannot be `None` when using method 'rcps'", and the same for `'ltt'` | error |
| `method="crc"` beside a `confidence_level` or an `rcps_bound` | a UserWarning that the argument is ignored | warning |

Example:

```python
controller = MultiLabelClassificationController(model.predict_proba, risk="precision", method="crc")
```

Use instead:

```python
controller = MultiLabelClassificationController(model.predict_proba, risk="precision", confidence_level=0.9)
```

xpd gives the replacement as advice and doesn't apply it.

### Work done twice

Take the points and the intervals from one call, and pass every confidence level to one estimator, because each extra call repeats the same work.

#### predict-and-predict-interval

**Severity:** warning. **Fix:** none. The rewrite needs a new name and changes two statements.

The rule reports `predict` on the same object and the same rows as a `predict_interval` or
`predict_set` in the same block.

Example:

```python
y_pred = mapie.predict(X_test)
_, y_pis = mapie.predict_interval(X_test)
```

Use instead:

```python
y_pred, y_pis = mapie.predict_interval(X_test)
```

xpd gives the replacement as advice and doesn't apply it.

`predict_interval` and `predict_set` return the point predictions as their first value. With a
forest of 50 trees on one thread, both forms returned the same arrays, bit for bit, and the pair
of calls took longer than the single call:

| Calls | At 2,000 rows | At 20,000 rows |
| --- | --- | --- |
| `predict` and `predict_interval`, against `predict_interval` alone | 1.99 times | 1.99 times |
| `predict` and `predict_set`, against `predict_set` alone | 1.98 times | 1.97 times |

The two calls must be statements of the same block and name the same object and the same rows.

#### refit-per-confidence-level

**Severity:** warning. **Fix:** none. The rewrite replaces the loop around the call.

The rule reports a conformal object built inside a loop or a comprehension with the loop variable
as its `confidence_level`.

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

xpd gives the replacement as advice and doesn't apply it.

`confidence_level` takes a list, and the intervals then have one column per level. With three
levels, one seeded object per level took longer than one object for the list, and the intervals
were bit equal:

| Class | At 1,000 rows | At 5,000 rows |
| --- | --- | --- |
| `CrossConformalRegressor` | 2.87 times | 2.95 times |
| `SplitConformalRegressor` | 2.99 times | 3.01 times |

Without `random_state` a cross class draws other folds for each object, so the intervals per level
then differ from the list form. `ConformalizedQuantileRegressor` takes one level only, and the rule
leaves it alone.

### Reproducible results

Pass `random_state` to each MAPIE resampler, so two runs give the same intervals.

#### unseeded-resampling

**Severity:** info. **Fix:** none. The seed is not in the file.

The rule reports objects that draw random numbers and get no `random_state`. They are
`CrossConformalRegressor`, `CrossConformalClassifier` and `CrossConformalizedQuantileRegressor` with
a number of folds, `JackknifeAfterBootstrapRegressor`, `Subsample`, `BlockBootstrap`,
`train_conformalize_test_split`, and `SplitConformalClassifier` with the `"aps"` or `"raps"` score.

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

Any fixed number makes the run repeatable, and 0 is only the example.

xpd gives the replacement as advice and doesn't apply it.

The rule is info, because a seed doesn't move the expected coverage. Each form it names gave
another result in two fresh runs and the same result with `random_state`.

The cross classes shuffle their folds when `cv` is a number, which is the default. They gave the
same intervals with `cv=KFold(5)` and with `cv=-1`, so a splitter and a negative number are left
alone. `SplitConformalClassifier` gave the same sets twice with the `"lac"` and `"top_k"` scores.
It is reported only with `"aps"` and `"raps"`, which draw a random number for each row.

The rule leaves alone:

- a file that calls `np.random.seed`, after which the same objects gave the same result twice
- a call with `shuffle=False` or `**`
- a `JackknifeAfterBootstrapRegressor` whose `resampling` is a `Subsample`, which is reported
  itself
- a call whose result is thrown away

A splitter that shuffles draws its own seed, and `random_state` of the cross class doesn't reach
it. `CrossConformalRegressor(model, cv=KFold(5, shuffle=True), random_state=0)` gave other
intervals in two runs, and the same intervals with `KFold(5, shuffle=True, random_state=0)`. The
rule doesn't report such a splitter, because the scikit-learn rule
[`missing-random-state`](#missing-random-state) reports the estimator and a `KFold` with
`shuffle=True` in the same file.

## Category Encoders

The Category Encoders rules report code that leaks the target into the test rows, arguments that
give wrong codes without an error, code that raises on category_encoders 2.11.2, and forms that a
release removed or deprecated, in each file that imports `category_encoders`. Every number about the
Category Encoders rules was measured on category_encoders 2.11.2 with scikit-learn 1.9.1, pandas 3.1.1 and NumPy
2.5.5 on Python 3.13. The examples assume `import category_encoders as ce` and that the
scikit-learn names they use are imported. Names such as `X_train`, `y_test` and `encoder` stand for
values from your program.

`OneHotEncoder`, `OrdinalEncoder` and `TargetEncoder` exist in scikit-learn as well. A rule about
one of the three reports a call only with proof that the class comes from category_encoders. The
proof is the module in front of the name, as in `ce.OneHotEncoder`, or an import of the name from
`category_encoders` in the same file. A file that imports the same name from both libraries gives
no proof, so the rules about the three classes stay silent there. The other 20 encoder names
belong to category_encoders alone and need no proof.

A rule document whose advice needs a newer release carries `since`. xpd reads the installed
version from the project's venv and skips the document on an older release. When the venv has no
category_encoders, or there is no venv, every document runs.

The scikit-learn pack already reports a target encoder that is fitted before the split, fitted
before cross-validation, or refitted on the test rows, through
[`fit-before-split`](#fit-before-split),
[`cross-val-on-fitted-data`](#cross-val-on-fitted-data) and [`fit-on-test`](#fit-on-test). A file
that uses category_encoders with a split nearly always imports scikit-learn, so the Category
Encoders pack doesn't repeat the three rules. It adds the one form they leave alone, in
[`encode-before-split`](#encode-before-split).

### The proof each rule needs

Each `error` and `warning` document ships only with a proof that was run, and the message quotes
what the proof measured. A proof is one of the following kinds:

| Proof | What runs | When it passes |
| --- | --- | --- |
| score | The old and the new form, over 30 seeds on data whose target is drawn apart from every feature, where the honest AUC is 0.5. | The old form's mean is more than three standard errors above 0.5, and the new form's mean is within 0.02 of it. |
| result | The old form, with the codes it gives printed next to the codes of the new form. | The old form gives a code that is wrong for every input of that shape, such as -1.0 for every row. |
| interpreter | The old form under `python -W error`. | The old form raises an exception or a warning, and the new form runs clean. |
| speed | Both forms on one thread, as the median of 7 runs, at 10,000 and 100,000 rows. | The new form is at least 1.5 times faster at both sizes. |

category_encoders 2.11.2 itself sets a pandas option that pandas 3.1 deprecates, so the
interpreter proofs ignored the pandas warning about the option.

A document is `info` when no proof passed for it, or when its advice depends on something the code
doesn't show, such as the dtype of a column. `sklearn-ships-encoder` passed its speed proof and is
still `info`, because the two libraries return different things and the swap is a choice.

The rules were run over two sets of code:

- the library's own code at its latest release, which is the 86 Python files of the repository
  and 35 more blocks taken from its docstrings, its `.rst` pages and its README
- 55 files from public repositories that import `category_encoders`

After the runs, the rules below were narrowed, and the table gives the reports that are left:

- `invalid-handle-option` reported three scikit-learn `OneHotEncoder(handle_unknown="ignore")`
  calls in files that import the name from both libraries. A second import now removes the proof.
- `hashing-default-components` reported `HashingEncoder().hashing_trick(df, N=3)`, which passes
  the number of columns itself.
- `zero-regularization` reported `CountTargetEncoder(smoothing=0)` inside `assertRaises`. Every
  rule about a form that raises now leaves alone code that expects the error.
- `sklearn-ships-encoder` reported 6 encoders handed to `PolynomialWrapper` or `NestedCVWrapper`,
  which take only the encoders of category_encoders.

| Rule | Severity | Proof | Reports on the library | Reports on 55 public files | Fix |
| --- | --- | --- | --- | --- | --- |
| [`transform-with-test-target`](#transform-with-test-target) | error, info | score: 0.691 against 0.517 AUC | 4 | 0 | unsafe |
| [`encode-before-split`](#encode-before-split) | error | score: 0.655 against 0.495 AUC | 0 | 0 | none |
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
| [`encoder-noise-without-seed`](#encoder-noise-without-seed) | info | result: two runs differ by up to 0.15 | 4 | 0 | none |

A score proof gives the old form's mean against the new form's mean. The speed proof gives the
lowest ratio over the three classes and both sizes.

None of the 83 reports on the library names a line where the rule's claim is false. The advice
still doesn't apply to most of them, because the library uses or tests its own classes:

- All 61 reports of `sklearn-ships-encoder` are the library's tests, examples and documentation
  building its own `OneHotEncoder`, `OrdinalEncoder` or `TargetEncoder`.
- The 4 reports of `transform-with-test-target` are tests that pass a target to `transform` on
  purpose, to check what the encoder does with it.
- The 10 reports of `hashing-default-components` are calls that do run with 8 columns, of which 4
  are in tests, 4 in benchmark examples and 2 in the documentation.
- The report of `cols-left-to-dtype` is an example that hands a `TargetEncoder()` two text
  columns, which it encodes.

The 18 reports on the 55 public files all name a category_encoders class. Recall was not
measured. Most rules have no report on the public files, so nothing here says how often the old
forms occur.

### Leakage

Each rule in the group reports a target encoder that sees the labels of the test rows, so the
score on the test rows is too high. A target encoder codes a category by the labels of its rows,
so the code of a test row then holds part of its own label. The score proofs fit a random forest of 50 trees on
600 rows with one text column of 100 levels and a target drawn apart from it, and give the mean
test AUC over 30 seeds.

#### transform-with-test-target

**Severity:** error when the encoder is a `LeaveOneOutEncoder`, and info for the other supervised encoders. **Fix:** unsafe: `encoder.transform(X_test)`.

The rule reports `encoder.transform(X_test, y_test)`. The supervised encoders of category_encoders
take the target in `transform`, which no scikit-learn transformer does, and the target there is
meant for the training rows.

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

| Encoder | Test AUC with `transform(X_test, y_test)` | With `transform(X_test)` |
| --- | --- | --- |
| `LeaveOneOutEncoder` | 0.691, 11.4 standard errors above 0.5 | 0.517 |
| `CatBoostEncoder` | 0.499, 0.1 standard errors from 0.5 | 0.516 |
| `TargetEncoder`, `WOEEncoder`, `JamesSteinEncoder`, `MEstimateEncoder`, `QuantileEncoder`, `CountTargetEncoder` | the same as without the test labels | the same |

With the target, `LeaveOneOutEncoder` takes each row's own label out of the mean of its category,
so the code of a test row moves with its label. The other supervised encoders read the target in
`transform` only to add the noise of `sigma` or `randomized=True`. The form is still not meant for
test rows, so it is info for them.

The rule reads the name of the first argument, which must hold `test` or `holdout`, or be a
validation name such as `X_val`. It needs the encoder assigned from a supervised class in the same
function, or in the module for code outside a function.

The fix is unsafe. It changes the codes of the test rows of a `LeaveOneOutEncoder` or a
`CatBoostEncoder`, and it takes the noise off the test rows of an encoder with `randomized=True`.
On a `TargetEncoder` and a `WOEEncoder` without noise, the fixed line gave an equal frame.

#### encode-before-split

**Severity:** error. **Fix:** none.

The rule reports `TargetEncoder(...).fit_transform(X, y)` on every row, when the file imports
`TargetEncoder` from `category_encoders` and the result is later split by `train_test_split` or
passed to `cross_val_score` or another cross-validation function.

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

xpd gives the replacement as advice and doesn't apply it.

| Old form, on data with no signal | Test AUC | Right form |
| --- | --- | --- |
| `TargetEncoder` fitted on every row before the split | 0.655, 19.8 standard errors above 0.5 | 0.495 with the split first |
| `TargetEncoder` fitted on every row before `cross_val_score` | 0.650 | 0.502 for a Pipeline that holds the encoder and the model |
| `LeaveOneOutEncoder`, `WOEEncoder`, `JamesSteinEncoder`, `MEstimateEncoder` or `CountTargetEncoder` fitted on every row before the split, and `CatBoostEncoder` fitted with `fit(X, y)` and then `transform(X)` | 0.654 to 0.688 | not given |
| `CatBoostEncoder(...).fit_transform(X, y)` on every row before the split | 0.490, because it codes each row from the labels of the rows before it | not given |

[`fit-before-split`](#fit-before-split) and
[`cross-val-on-fitted-data`](#cross-val-on-fitted-data) of the scikit-learn pack report the form
for every step but one, a bare `TargetEncoder(...)` built in the same expression. They take it for
the scikit-learn class, which cross-fits in `fit_transform` and so never codes a row from its own
label. The
category_encoders class doesn't, so `encode-before-split` reports the form when the import shows
which class it is. The scikit-learn pack reports every other form, such as `ce.TargetEncoder(...)`,
an encoder kept in a name, or another encoder class.

### Codes that are wrong without an error

Each rule in the group reports a form that runs and gives another number than the code asks for.

#### ordinal-mapping-pairs

**Severity:** error. **Fix:** unsafe: the list of pairs written as a dict.

The rule reports an `OrdinalEncoder` mapping whose `"mapping"` entry is a list of
`(category, code)` pairs. The list of pairs was the form before category_encoders 2.0, and old
answers and notebooks still show it.

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

category_encoders 2.11.2 takes the list of pairs without an error and finds no category in it.
With `[("a", 1), ("b", 2), ("c", 3)]` every row of a column that holds a, b and c was coded -1.0.
The fix changes every code of the column, from -1.0 to the code the pair names, so it is unsafe.
The rule reports the `"mapping"` entry of a dict that also has a `"col"` entry.

#### ordinal-mapping-unknown

**Severity:** info. **Fix:** none.

The rule reports `OrdinalEncoder(mapping=...)` without `handle_unknown`. A category that the
mapping doesn't name is then coded -1, with no error and no warning.

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

xpd gives the replacement as advice and doesn't apply it.

| `{"a": 1, "b": 2}` on the column a, b, c | Result |
| --- | --- |
| without `handle_unknown` | 1.0, 2.0, -1.0 |
| with `handle_unknown="error"` | `ValueError: Unexpected categories found in column c` at fit |

A key with a typing error, or a category that appears later, reaches the model as -1, which is
below every code that the mapping gives. Whether the mapping names every category can't be read from the code, so
the rule is info. A call that states `handle_unknown`, with any value, is left alone.

#### zero-regularization

**Severity:** error. **Fix:** none.

The rule reports `WOEEncoder(regularization=0)` and `CountTargetEncoder(smoothing=0)`.

Example:

```python
encoder = ce.WOEEncoder(cols=["city"], regularization=0)
```

Use instead:

```python
encoder = ce.WOEEncoder(cols=["city"], regularization=0.5)
```

xpd gives the replacement as advice and doesn't apply it.

`WOEEncoder` adds `regularization`, 1.0 by default, to the counts of both classes before it takes
the logarithm of their ratio. With 0, a category that holds one class alone divides by zero.

| Form | Result |
| --- | --- |
| `WOEEncoder(regularization=0)` on a category of five rows of class 1 | `inf`, with no error and no warning |
| `WOEEncoder()` on the same category | 1.204 |
| `WOEEncoder(regularization=0)` on a column where every category holds both classes | finite codes |
| `CountTargetEncoder(smoothing=0)` | `ValueError: smoothing must be strictly bigger than 0.` at fit |
| `TargetEncoder(smoothing=0)` or `MEstimateEncoder(m=0)`, left alone | the plain mean of each category |

The `CountTargetEncoder` document carries `since: "2.11"`, the release that added the class.

#### hashing-default-components

**Severity:** warning. **Fix:** none.

The rule reports `HashingEncoder(...)` without `n_components`. The encoder hashes every value into
`n_components` columns, 8 by default, and all encoded columns share them.

Example:

```python
encoder = ce.HashingEncoder(cols=["city", "shop"])
```

Use instead:

```python
encoder = ce.HashingEncoder(cols=["city", "shop"], n_components=1024)
```

xpd gives the replacement as advice and doesn't apply it.

| `n_components` | Distinct encoded rows for a column of 100 distinct values |
| --- | --- |
| 8, the default | 8 |
| 64 | 50 |
| 1024 | 94 |

Two columns encoded by one `HashingEncoder()` gave 8 output columns in all, and the rows `(x, y)`
and `(y, x)` got the same code. The encoder therefore also loses which column a value came from.
The rule has no fix, because the right number depends on how many distinct values the columns
hold. A call that passes the number by position, a call inside `isinstance`, and
`HashingEncoder().hashing_trick(...)`, which takes the number as `N`, are left alone.

Other advice about the class was measured and dropped:

- With the default number of threads, `max_process=1` was 23 times faster than the default at
  1,000 rows and 2.2 times slower at 100,000 rows, so no setting is right for both.
- `FeatureHasher` of scikit-learn was 23 times faster at 10,000 rows and 5.4 times faster at
  100,000 rows with the default number of threads, but it returns a sparse matrix and takes no
  column names, so it can't take the place of the encoder.

#### cols-left-to-dtype

**Severity:** info. **Fix:** none.

The rule reports an encoder built without `cols` inside a `ColumnTransformer` or
`make_column_transformer`. Without `cols`, an encoder encodes the columns of the object, category
and string dtypes and passes every other column through.

Example:

```python
columns = make_column_transformer((ce.BinaryEncoder(), ["zip_code"]), remainder="passthrough")
```

Use instead:

```python
columns = make_column_transformer((ce.BinaryEncoder(cols="all"), ["zip_code"]), remainder="passthrough")
```

xpd gives the replacement as advice and doesn't apply it.

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

#### nan-into-pipeline

**Severity:** info. **Fix:** unsafe: `handle_unknown="value"` for `CountEncoder(handle_unknown=None)`, which codes an unseen category as 0 in place of NaN. `"return_nan"` has no fix, because the message names two rewrites, `"value"` or an imputer after the encoder.

The rule reports an encoder with `handle_unknown="return_nan"` or `handle_missing="return_nan"`
inside a `Pipeline` or `make_pipeline` call, and a `CountEncoder(handle_unknown=None)` there.

Example:

```python
model = make_pipeline(ce.OneHotEncoder(cols=["city"], handle_unknown="return_nan"), LogisticRegression())
```

Use instead:

```python
model = make_pipeline(ce.OneHotEncoder(cols=["city"], handle_unknown="value"), LogisticRegression())
```

xpd gives the replacement for `"return_nan"` as advice and doesn't apply it.

`return_nan` codes a category that fit did not see as NaN, in every output column of the row, and
in a Pipeline the NaN goes to the next step. With a `OneHotEncoder` or a `TargetEncoder` in front
of a `LogisticRegression`, `predict` on a row with an unseen city raised
`ValueError: Input X contains NaN`. An `OrdinalEncoder` with `handle_missing="return_nan"` raised
the same on a row with a missing city. The fit passed, because the training rows had no unseen
category. The error first shows on new data. With `"value"`, the default, the same `predict`
ran.

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

Each rule in the group reports a form that raises on category_encoders 2.11.2. Code inside
`assertRaises`, `pytest.raises` or a `try` block with an `except` clause expects the error and is
left alone.

#### supervised-encoder-without-target

**Severity:** error. **Fix:** none.

The rule reports `fit(X)` and `fit_transform(X)` without the target on a supervised encoder.

Example:

```python
X_encoded = ce.TargetEncoder(cols=["city"]).fit_transform(X)
```

Use instead:

```python
X_encoded = ce.TargetEncoder(cols=["city"]).fit_transform(X, y)
```

xpd gives the replacement as advice and doesn't apply it.

The supervised encoders are `CatBoostEncoder`, `CountTargetEncoder`, `GLMMEncoder`,
`JamesSteinEncoder`, `LeaveOneOutEncoder`, `MEstimateEncoder`, `QuantileEncoder`,
`SummaryEncoder`, `TargetEncoder` and `WOEEncoder`.

| Call | Error |
| --- | --- |
| `fit_transform(X)`, on each of the ten | `TypeError: fit_transform() missing argument: y` |
| `fit(X)`, on nine of them | `ValueError: Supervised encoders need a target for the fitting. The target cannot be None` |
| `fit(X)`, on `SummaryEncoder`, whose `fit` has no default for `y` | `TypeError` |

A scikit-learn transformer takes `fit(X)`, so the form is easy to carry over from an
`OrdinalEncoder` or a `OneHotEncoder`. The rule reports the call on an encoder built in the same
expression, and on a name that is assigned from a supervised class in the same function and from
nothing else there.

#### invalid-handle-option

**Severity:** error. **Fix:** unsafe: `"value"`, the default, for each string the class doesn't take, and on `HashingEncoder` the keyword is dropped. Since 2.10 the call raised at fit, and before 2.10 the string did not act as the default, so the codes can change.

The rule reports a `handle_unknown` or `handle_missing` string that the encoder doesn't take, such
as `handle_unknown="ignore"`, which belongs to the scikit-learn `OneHotEncoder`.

Example:

```python
encoder = ce.OneHotEncoder(cols=["city"], handle_unknown="ignore")
```

Use instead:

```python
encoder = ce.OneHotEncoder(cols=["city"], handle_unknown="value")
```

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
allowed since 2.11 and is left alone. `CountEncoder` checks nothing and is left alone too.

#### named-cols-after-array-step

**Severity:** error. **Fix:** none.

The rule reports an encoder with names in `cols` that comes after an imputer, a scaler or a
`ColumnTransformer` in a pipeline. Such steps hand the next step a NumPy array, and the names are
gone.

Example:

```python
model = make_pipeline(SimpleImputer(strategy="most_frequent"), ce.OneHotEncoder(cols=["city"]))
```

Use instead:

```python
imputer = SimpleImputer(strategy="most_frequent").set_output(transform="pandas")
model = make_pipeline(imputer, ce.OneHotEncoder(cols=["city"]))
```

xpd gives the replacement as advice and doesn't apply it.

The old form raised `ValueError: X does not contain the columns listed in cols: ['city'] The
encoder received a non-DataFrame input at fit, where named columns cannot be recovered
positionally` at fit. With `set_output(transform="pandas")` on the imputer the same
pipeline ran, and so did the old form with the column given by position, as in `cols=[0]`.

The rule knows that the following steps give an array: `SimpleImputer`, `KNNImputer`, `IterativeImputer`,
`StandardScaler`, `MinMaxScaler`, `MaxAbsScaler`, `RobustScaler`, `Normalizer`,
`PowerTransformer`, `QuantileTransformer`, `KBinsDiscretizer`, `PolynomialFeatures`,
`ColumnTransformer` and `make_column_transformer`. It reads `make_pipeline(...)` and
`Pipeline([...])` calls whose steps are written in the call. It stays silent in a file that
calls `set_output` or sets `transform_output`, anywhere.

#### wrapper-fit-returns-none

**Severity:** error. **Fix:** none.

The rule reports a method call on the result of `fit` of a `PolynomialWrapper` or a
`NestedCVWrapper`, and a name assigned from that `fit`. Their `fit` has no return statement, so it
returns `None` where every scikit-learn estimator returns itself.

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

xpd gives the replacement as advice and doesn't apply it.

The old form raised `AttributeError: 'NoneType' object has no attribute 'transform'` on both
wrappers. `fit_transform` returns the encoded frame on both and is left alone.

### Removed and deprecated forms

Each rule in the group reports a form that a release of category_encoders removed or deprecated.
The release of each removal was found by reading the constructor of every class in the wheels of
1.3.0, 2.2.2, 2.5.1, 2.6.4, 2.7.0, 2.8.0, 2.8.1, 2.9.0, 2.10.0 and 2.11.2, and each exception was
raised on 2.11.2. Code inside `assertRaises`, `pytest.raises` or a `try` block with an `except`
clause expects the error or handles both releases, and is left alone.

#### removed-in-category-encoders

**Severity:** error. **Fix:** safe: `list(...)` around `feature_names_out_` and `get_feature_names_out()` before `index` and `count`, which gave the same values on 2.6.4 and on 2.6.0, where the names were a list. unsafe: the two renamed names in `category_encoders.utils`, because the old mixin is another class and `get_categorical_cols` also names category and string columns, `handle_missing="value"` for `impute_missing=True` in a call without `handle_missing`, which no release took beside `impute_missing`, and dropping `randomized=` from a `LeaveOneOutEncoder` that has `sigma` or says `randomized=False`, because on 1.3.0 `sigma` alone added no noise. The other forms have no fix, and `remove`, `append`, `extend`, `insert` and `pop` have none because on a copy of the names they would change nothing that the code reads afterwards.

The rule reports forms that a release of category_encoders removed and that raise on 2.11.2.

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

#### deprecated-in-category-encoders

**Severity:** warning for `get_feature_names`, and info for `max_sample`. **Fix:** safe.

The rule reports `encoder.get_feature_names()` and `HashingEncoder(max_sample=...)`.

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
`deprecated-in-category-encoders` therefore runs only in a file that doesn't import sklearn. It
needs the receiver built from a class whose name ends in `Encoder` or `Wrapper`, in the same expression or in an
assignment of the same function.

The changelog of 2.6.4 says of `max_sample` that "it has no use anymore". `HashingEncoder` still
takes the keyword and raises no warning, so the document is info. With `max_sample=10` and
without it the encoded frame was equal, with one process and with two, so the fix that drops the
keyword is safe.

### Encoders that scikit-learn ships

Use scikit-learn's own encoder where it ships the same one, because its `OneHotEncoder` ran `fit_transform` 2.7 to 4.5 times faster.

#### sklearn-ships-encoder

**Severity:** info. **Fix:** none.

The rule reports a category_encoders `OneHotEncoder`, `OrdinalEncoder` or `TargetEncoder` that
uses nothing the scikit-learn class of the same name lacks.

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

xpd gives the replacement as advice and doesn't apply it.

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
[`cols-left-to-dtype`](#cols-left-to-dtype).

A Pipeline with the scikit-learn `TargetEncoder` was also tested against one with the
category_encoders class, and it didn't score better on the test rows. On 2,000 rows with a target
that depends on a column of 300 levels, the category_encoders `TargetEncoder` scored 0.655 AUC and
the scikit-learn class 0.616, over 30 seeds with a random forest. The rule therefore claims the
speed and nothing about the score.

### Reproducible results

Pass `random_state` to an encoder that adds noise, so two runs give the same codes.

#### encoder-noise-without-seed

**Severity:** info. **Fix:** none.

The rule reports an encoder that adds random noise without `random_state`, and a `NestedCVWrapper`
that shuffles its folds without it.

Example:

```python
encoder = ce.LeaveOneOutEncoder(cols=["city"], sigma=0.05)
```

Use instead:

```python
encoder = ce.LeaveOneOutEncoder(cols=["city"], sigma=0.05, random_state=0)
```

xpd gives the replacement as advice and doesn't apply it.

Two `fit_transform` calls on the same 400 rows gave codes that differed by the following largest
gaps:

| Form | Largest gap between two runs |
| --- | --- |
| `WOEEncoder(randomized=True)` | 0.15 |
| `JamesSteinEncoder(randomized=True)` | 0.13 |
| `MEstimateEncoder(randomized=True)` | 0.15 |
| `LeaveOneOutEncoder(sigma=0.05)` | 0.15 |
| `CatBoostEncoder(sigma=0.05)` | 0.15 |
| `NestedCVWrapper(TargetEncoder())` | 0.10 |

With `random_state=0` the two calls were equal in each row of the table, and a
`NestedCVWrapper` with `shuffle=False` was equal too. A seed does not move the expected score,
so the rule is info, as [`missing-random-state`](#missing-random-state) of the scikit-learn
pack is. A `NestedCVWrapper` given a splitter as `cv` is left alone, because the splitter
brings its own seed. A file that calls `np.random.seed` is left alone, because the encoders then
draw from the global NumPy seed.

### What the pack leaves out

The pack has no rule for the following habits, which were looked at:

- Habits that a pattern can't see in the code:
  - A target of more than two classes given to `TargetEncoder` as whole numbers is coded as the
    mean of the class numbers, which means nothing. `PolynomialWrapper` is the answer, but a
    pattern cannot see how many classes a target has. `WOEEncoder` raises `ValueError: The
    target column y must be binary` in the same case.
  - `X` and `y` with indexes that do not match raise `ValueError` at fit, and a pattern cannot
    see an index.
  - `GLMMEncoder` needs statsmodels, which category_encoders stopped installing in 2.11, and
    raises `ImportError` without it. A package check runs for every project that has
    category_encoders and cannot ask whether a file uses that class, so the pack has none.
- Advice that the measurements didn't support:
  - `encoder.fit(X_train, y_train)` followed by `encoder.transform(X_train)` gives the training
    rows of a `LeaveOneOutEncoder` the plain mean of their category. On 600 rows whose 100
    levels each carry their own share of positives, with a random forest over 30 seeds, the form
    scored 0.637 AUC on the test rows against 0.580 with the target passed. The form the
    documentation asks for was the worse one here, so no rule reports the other.
  - On the same data `LeaveOneOutEncoder()` without `sigma` scored 0.580 against 0.559 with
    `sigma=0.05` in a forest, and both scored 0.636 in a logistic regression, so there is no
    proof that the noise helps.
  - With the default number of threads, one encoder per column in a loop was 1.15 to 1.25 times
    slower than one encoder with `cols` for `OrdinalEncoder` and `TargetEncoder`, and 3.5 to 5.4
    times slower for `OneHotEncoder`.

## linearmodels

The linearmodels rules report formulas and arguments that fit another model than the code reads
as, covariance options that give other standard errors than the code asks for, code that raises on
linearmodels 7.1, statsmodels names read on linearmodels results, and forms that a release removed
or deprecated. They run in each file that imports `linearmodels`.
[linearmodels](https://bashtage.github.io/linearmodels/) fits panel models, instrument models,
systems of equations and factor models of asset prices.

Every number about the linearmodels rules was measured on linearmodels 7.1 with pandas 3.1.1,
NumPy 2.5.5, SciPy 1.18.1, statsmodels 0.15.0 and formulaic 1.2.2 on Python 3.13, and a number for another
release names it. The data sets `wage_panel`, `mroz` and `french` are the ones in
`linearmodels.datasets`. The examples assume that the linearmodels names they use are imported and
that `sm` is `statsmodels.api`. Names such as `data`, `y`, `X` and `instruments` stand for values
from your program.

Most of the pack reports code written from memory of statsmodels, because linearmodels looks like
statsmodels and differs from it in the following small ways:

| What | statsmodels | linearmodels |
| --- | --- | --- |
| intercept in a formula | added | not added |
| covariance names | `"nonrobust"`, `"HC1"`, `"cluster"` | `"unadjusted"`, `"robust"`, `"clustered"` |
| result names | `bse`, `tvalues`, `resid` | `std_errors`, `tstats`, `resids` |
| `IV2SLS` arguments | `(endog, exog, instrument=None)`, in `statsmodels.sandbox.regression.gmm` | `(dependent, exog, endog, instruments)` |
| `summary` | a method | a property |

`OLS`, `IV2SLS`, `IVGMM` and `SUR` exist in statsmodels as well. A rule about one of the four
reports a call only with proof that the class comes from linearmodels. The proof is the module in
front of the name, as in `linearmodels.IV2SLS` or `lm.IV2SLS` after `import linearmodels as lm`, or
an import of the name from `linearmodels` in the same file. A file that imports the same name from
both libraries gives no proof, so the rules about the four classes stay silent there. The other 15
model classes of `linearmodels.__all__` have names of their own and need no proof.

A rule about `fit` or about a result reads only an object it can follow to a linearmodels class.
Such an object is a call on the class itself, such as `PanelOLS(y, X).fit(...)`, or a name that
its function, or the module outside any function, assigns from such calls and from nothing else.
The rule leaves alone a name that is also assigned from anything else, that a loop, a `with`
statement or a lambda parameter binds, or that comes in as an argument. A statsmodels model in the
same file is therefore not reported. The 950 public files hold 585 `cov_type=` strings with a
statsmodels name, such as `"HC3"` and `"cluster"`, and no rule reports one of them.

A rule reads an argument only when it is written out. A `cov_type`, a formula or a flag that is a
name, an f-string or a value unpacked from a dictionary is left alone.

A rule document whose advice needs a newer release carries `since`. xpd reads the installed version
from the project's venv and skips the document on an older release. When the venv has no
linearmodels, or there is no venv, every document runs. Each `since` was checked by installing
from PyPI the release before it and the release at it, and running the form on both:

| `since` | Releases run |
| --- | --- |
| 4.15 | 4.14 and 4.15 |
| 4.18 | 4.17 and 4.18 |
| 5.0 | 4.31 and 5.0 |
| 7.1 | 7.0 and 7.1 |

### The proof each rule needs

Each `error` and `warning` document ships only with a proof that was run, and the message quotes
what the proof measured. A proof is one of the following kinds:

| Proof | What runs | When it passes |
| --- | --- | --- |
| result | The old and the new form on the same data, with what each returns printed. | The old form gives another number than the code reads as, with no error and no warning, such as a coefficient of 0.2184 where 0.0338 is the one with an intercept. |
| interpreter | The old form under `python -W error`. | The old form raises an exception or a warning, and the new form runs clean. Old releases warn about their own dependencies, so on the old releases only a warning from linearmodels counted. |
| simulation | At least 1,000 draws of data whose true coefficient is known. | It counts how often the 95% interval covers the coefficient, or how often a true null is rejected at 5%. |
| speed | Both forms on one thread, as the median of 5 fits, at two sizes. | The new form is at least 1.5 times faster at both. |

A document is `info` when its advice depends on something the code doesn't show, such as the dtype
of a column or how the errors are related. A form that was run on 7.1 and on 7.0 gave the same
result on both unless the text says otherwise.

The rules were run over two sets of code:

- the library's own code at 7.1, 258 files in all:
  - the 229 Python files of the repository, of which 165 are tests
  - the code cells of its 12 notebooks
  - the code blocks of 9 `.rst` and `.md` pages
  - the doctest lines of 8 modules
- 950 files from 623 public repositories that mention `linearmodels`, found through the GitHub
  code search, at most 4 from one repository, with notebooks turned into their code cells

After the runs, the rules below were narrowed, and the table gives the reports that are left:

- `time-before-entity-index` reported 2 frames that no panel model reads, and one frame indexed by
  date and hour on purpose. The frame must now reach a panel model, and `hour` is a time.
- `statsmodels-result-name` reported `res.tvalues` behind `hasattr(res, "tstats")`. Every rule
  about a form that raises now leaves code under a `hasattr` check alone.
- `hand-written-hausman` reported a test of OLS against instruments, which the method of 7.1
  doesn't do. Its message now names `wu_hausman()` as well.
- `absorb-not-categorical` matched its own name in a comment. It now looks for `category`.

| Rule | Severity | Proof | Reports on the library | Reports on 950 public files | Fix |
| --- | --- | --- | --- | --- | --- |
| [`formula-without-constant`](#formula-without-constant) | warning | result: 0.2184 against 0.0338 | 3 | 13 | none |
| [`effects-outside-panel-ols`](#effects-outside-panel-ols) | error | result: equal coefficients with and without the term | 0 | 0 | none |
| [`effects-keyword-with-formula`](#effects-keyword-with-formula) | error | interpreter: TypeError | 0 | 3 | none |
| [`formula-power-term`](#formula-power-term) | error | result: 2 columns where 3 were written | 1 | 1 | none |
| [`first-difference-constant`](#first-difference-constant) | error | interpreter: ValueError | 0 | 0 | none |
| [`weights-column-name`](#weights-column-name) | error | interpreter: TypeError | 0 | 0 | none |
| [`panel-ols-without-effects`](#panel-ols-without-effects) | warning | result: 0.0349 against 0.0698 | 42 | 37 | none |
| [`too-many-effects`](#too-many-effects) | error | interpreter: ValueError | 0 | 0 | none |
| [`time-before-entity-index`](#time-before-entity-index) | warning | result: 0.1622 against 0.2483 | 1 | 1 | unsafe |
| [`use-lsdv`](#use-lsdv) | warning | speed: 17 times at the lowest | 2 | 2 | none |
| [`clustered-without-clusters`](#clustered-without-clusters) | error | result: 0.0413 against 0.0494. Interpreter: TypeError | 4 | 31 | none |
| [`cov-option-without-cov-type`](#cov-option-without-cov-type) | error | interpreter: ValueError, TypeError. Result: 0.2037 against 0.3002 | 0 | 0 | none |
| [`unknown-cov-type`](#unknown-cov-type) | error | interpreter: UnknownOptionError | 2 | 0 | none |
| [`kernel-default-bandwidth`](#kernel-default-bandwidth) | warning | result: 0.8022 on 7.0 against 0.3794 on 7.1 | 22 | 1 | none |
| [`panel-default-covariance`](#panel-default-covariance) | info | simulation: 17.3% against 5.7% rejections | 325 | 372 | none |
| [`iv-missing-arguments`](#iv-missing-arguments) | error | interpreter: TypeError | 0 | 2 | none |
| [`iv-arguments-swapped`](#iv-arguments-swapped) | warning | result: 0.2021 against 0.0664 | 0 | 0 | none |
| [`iv-without-exog`](#iv-without-exog) | warning | simulation: 2.002 against 0.990 for a true 1.0 | 4 | 2 | none |
| [`manual-two-stage`](#manual-two-stage) | warning | simulation: 99.6% against 95.7% coverage | 0 | 0 | none |
| [`absorb-not-categorical`](#absorb-not-categorical) | info | result: 3.397 against 1.022 for a true 1.0 | 0 | 5 | none |
| [`statsmodels-result-name`](#statsmodels-result-name) | error | interpreter: TypeError, AttributeError | 0 | 15 | unsafe |
| [`invalid-compare-precision`](#invalid-compare-precision) | error | interpreter: ValueError | 0 | 0 | none |
| [`hand-written-hausman`](#hand-written-hausman) | info | none: the method runs on 7.1 and is absent on 7.0 | 4 | 25 | none |
| [`removed-in-linearmodels`](#removed-in-linearmodels) | error | interpreter: ModuleNotFoundError, AttributeError | 0 | 0 | unsafe |
| [`deprecated-in-linearmodels`](#deprecated-in-linearmodels) | warning | interpreter: FutureWarning. Result: equal order | 12 | 0 | safe |

A result proof gives the old form's number against the new form's number.

Of the 422 reports on the library, 399 are in its tests, which call the forms on purpose. They are
308 fits with the default covariance, 42 `PanelOLS` models without effects that are compared with
`PooledOLS`, 22 kernel estimators with the default bandwidth, and 11 uses of the deprecated names
inside the tests of the deprecations. One report names a line where the advice is wrong. The line is on a
documentation page that shows `set_index(['year','firm'])` for the `PanelOLS` that pandas had in
2017, which took the time first. One report is a defect of the library's own README, whose first
example writes `exper + exper ** 2` and fits a model without the squared term.

Of the 510 reports on the 950 public files, 108 are errors or warnings, and each was read. None names
a line where the rule's claim is false. The other 402 are `info`. Of them, 20 of the 372
`panel-default-covariance` reports, 14 `hand-written-hausman` reports and all 5
`absorb-not-categorical` reports were read, and each is a fit or a model of the kind the rule
describes. `unknown-cov-type` reports none of the 585 `cov_type=` strings with a statsmodels name,
and the 20 of them that were read are calls on statsmodels models.

Recall was not measured, so nothing here says how often a rule misses the form it describes.

### Formulas

Each rule in the group reports a formula that fits another model than it reads as. linearmodels
parses formulas with formulaic, and every model has a `from_formula`. The rules read a formula only
when it is a string in the `from_formula` call, as the first argument, the `formula` keyword, or a
value of the dictionary that a system model takes.

#### formula-without-constant

**Severity:** warning. **Fix:** none.

The rule reports a formula with no `1` term in `from_formula` of `PooledOLS`, `RandomEffects`,
`BetweenOLS`, `FamaMacBeth`, `PanelOLS` and the system models. It also reports such a formula in
`from_formula` of `IV2SLS`, `IVLIML`, `IVGMM`, `IVGMMCUE`, `IVJIVE` and `OLS` when the formula has
an instrument block in brackets. linearmodels adds no intercept to such formulas, where statsmodels
and R add one. `"y ~ x"` is therefore a regression through the origin in linearmodels and a
regression with an intercept in statsmodels.

Example:

```python
model = PooledOLS.from_formula("lwage ~ exper", data)
iv = IV2SLS.from_formula("lwage ~ exper + [educ ~ fatheduc + motheduc]", data)
```

Use instead:

```python
model = PooledOLS.from_formula("lwage ~ 1 + exper", data)
iv = IV2SLS.from_formula("lwage ~ 1 + exper + [educ ~ fatheduc + motheduc]", data)
```

On 7.1 and on 7.0, the models gave the following coefficient of `exper` on the wage panel, and of
`educ` for `IV2SLS` on the mroz data:

| Model | Without `1` | With `1` |
| --- | --- | --- |
| `PooledOLS` | 0.2184 | 0.0338, with an intercept of 1.4289 |
| `RandomEffects` | 0.1119 | 0.0589 |
| `BetweenOLS` | 0.2364 | -0.0229 |
| `IV2SLS` with an instrument block | 0.0769 | 0.0664 |

The instrument models for one equation add the intercept themselves when the formula has no block
in brackets. On 7.1, 7.0 and 5.0, `IV2SLS`, `IVLIML`, `IVGMM` and `IVGMMCUE` returned `Intercept`
and `exper` for `"lwage ~ exper"`, so the rule leaves the form alone on the four classes.

The rule stays silent when:

- the formula says `- 1` or `0 +`, which states that no intercept is meant
- the formula names a column that looks like a constant, such as `const`
- the formula has a `C()` term, whose dummies span the intercept
- a `PanelOLS` formula has effects, or the call has `other_effects`, which absorb the intercept

A model without an intercept can be meant, so the rule is a warning.

#### effects-outside-panel-ols

**Severity:** error. **Fix:** none.

The rule reports `EntityEffects`, `FixedEffects` or `TimeEffects` in a formula given to
`PooledOLS`, `RandomEffects`, `BetweenOLS`, `FirstDifferenceOLS` or `FamaMacBeth`. Every panel
model takes the three terms out of the formula, and only `PanelOLS` then uses them.

Example:

```python
model = RandomEffects.from_formula("lwage ~ 1 + hours + EntityEffects", data)
```

Use instead:

```python
model = PanelOLS.from_formula("lwage ~ 1 + hours + EntityEffects", data)
```

On 7.1 and 7.0, each of the five classes returned the same coefficients for `"lwage ~ 1 + hours"` with and without each of the three terms, and raised no error and no warning. The reader of the formula sees fixed effects that the model does not have.

#### effects-keyword-with-formula

**Severity:** error. **Fix:** unsafe: the term `EntityEffects` or `TimeEffects` is added to a formula written as a string, in place of `entity_effects=True` or `time_effects=True`. The call raised before the fix. A formula that is not a string, a keyword that is not `True`, and a call with both keywords have no fix.

The rule reports `entity_effects=` or `time_effects=` given to `PanelOLS.from_formula`. The
constructor takes the two keywords, but `from_formula` reads the effects from the formula.

Example:

```python
model = PanelOLS.from_formula("lwage ~ 1 + exper", data, entity_effects=True)
```

Use instead:

```python
model = PanelOLS.from_formula("lwage ~ 1 + exper + EntityEffects", data)
```

7.1 and 7.0 raised `TypeError: PanelOLS.from_formula() got an unexpected keyword argument 'entity_effects'`. Three of the 950 public files have the form.

#### formula-power-term

**Severity:** error. **Fix:** unsafe: `I(exper**2)` for `exper**2`, which changes the model on purpose.

The rule reports a term that is a name, `**` and a number, such as `exper**2`, in a formula of any
model. A formula reads `**` as a power of interactions, so `exper**2` is `exper`. The squared values
need `I(exper**2)`.

Example:

```python
model = IV2SLS.from_formula("lwage ~ 1 + exper + exper**2", data)
```

Use instead:

```python
model = IV2SLS.from_formula("lwage ~ 1 + exper + I(exper**2)", data)
```

On 7.1 and 7.0 the first formula gave a model with the columns `Intercept` and `exper` on `IV2SLS` and on `PooledOLS`, with no error, and the second gave three columns. The README of linearmodels itself has `exper + exper ** 2` in its first example, which fits a model with the columns `Intercept`, `exper` and `educ`. `(a + b)**2`, which is the use the operator is for, is left alone.

#### first-difference-constant

**Severity:** error. **Fix:** unsafe: the `1` term is taken out of the formula, and `add_constant(X)` becomes `X`. The call raised before the fix. A formula whose only term is the `1` has no fix.

The rule reports a `1` term in a formula of `FirstDifferenceOLS`, and `add_constant(...)` as its
`exog`. A first difference removes everything that is constant over time, so the model refuses a
constant.

Example:

```python
model = FirstDifferenceOLS.from_formula("lwage ~ 1 + hours", data)
model = FirstDifferenceOLS(y, sm.add_constant(X))
```

Use instead:

```python
model = FirstDifferenceOLS.from_formula("lwage ~ hours", data)
model = FirstDifferenceOLS(y, X)
```

Both forms raised `ValueError: Constants are not allowed in first difference regressions.` on 7.1 and 7.0.

#### weights-column-name

**Severity:** error. **Fix:** unsafe: `weights=data["hours"]` for `weights="hours"` in `from_formula` when the data is a name. The call raised before the fix. A model built from arrays has no fix, because the frame is not in the call.

The rule reports `weights="name"` on a model or on its `from_formula`. statsmodels and R take the
weights of a formula model as the name of a column, but linearmodels takes the values.

Example:

```python
model = PooledOLS.from_formula("lwage ~ 1 + exper", data, weights="hours")
```

Use instead:

```python
model = PooledOLS.from_formula("lwage ~ 1 + exper", data, weights=data["hours"])
```

7.1 raised `TypeError: weights must be array_like ... Got str` on `PooledOLS` and on `IV2SLS`, and 7.0 raised `AttributeError: 'str' object has no attribute 'ndim'`.

### Panel models

Each rule in the group reports a panel model that reads its index or its effects in another way
than the code means. A panel model takes a frame whose index has two levels, the entity first and
the time second. `PanelOLS` is the only class with effects, and it has none until it is told to.

#### panel-ols-without-effects

**Severity:** warning. **Fix:** none.

The rule reports `PanelOLS(...)` with none of `entity_effects`, `time_effects` and
`other_effects`, and `PanelOLS.from_formula` with a formula that has no effects term. The name of
the class reads like a fixed effects estimator, but without one of the three it fits pooled OLS.

Example:

```python
fixed = PanelOLS(y, X).fit()
```

Use instead:

```python
fixed = PanelOLS(y, X, entity_effects=True).fit()
pooled = PooledOLS(y, X).fit()
```

On the wage panel, on 7.1 and 7.0, the three models gave the following coefficients:

| Model | Coefficient of `exper` |
| --- | --- |
| `PanelOLS(y, X)` | 0.0349 |
| `PooledOLS(y, X)` | 0.0349 |
| `PanelOLS(y, X, entity_effects=True)` | 0.0698 |

The rule reports 37 calls in 25 of the 950 public files. Among them are results named `fe_model` and
`mod_fe`, and a function whose docstring starts "One- and two-way fixed effects estimator for panel
data" and whose body is `PanelOLS(y, exogenous).fit()`. A call that passes any of the three
keywords is left alone, also with the value `False`, and so is a call that unpacks its keywords. A
formula with a `C()` term may hold the effects as dummies and is left alone too.

#### too-many-effects

**Severity:** error. **Fix:** none.

The rule reports `PanelOLS` with `entity_effects=True`, `time_effects=True` and an
`other_effects` that is written as data. It also reports a formula with both effect terms next to
such an `other_effects`. The model takes two effects at most.

Example:

```python
model = PanelOLS(y, X, entity_effects=True, time_effects=True, other_effects=data.occupation)
```

Use instead:

```python
effects = data.reset_index()[["nr", "year", "occupation"]].astype("category")
model = AbsorbingLS(y.reset_index(drop=True), X.reset_index(drop=True), absorb=effects)
```

7.1 and 7.0 raised `ValueError: At most two effects supported.` for both forms. `AbsorbingLS` with
the three columns as categories ran on 7.1.

With `other_effects=None` the call runs, so the rule counts `other_effects` only when its value is
a subscript, an attribute or a call, such as `data[["occupation"]]`. A name that its function
assigns from such a value and from nothing else counts too. A parameter, a name that holds `None`,
a value read from `self` or from a settings object, and a `get` call are left alone.

#### time-before-entity-index

**Severity:** warning. **Fix:** unsafe: the two names in the other order.

The rule reports `set_index([time, entity])` when the frame reaches a panel model. The first name
must be a usual name for a time, such as `"year"` or `"date"`, and the second must not be. linearmodels
doesn't check which level is which.

Example:

```python
data = wage.set_index(["year", "nr"])
model = PanelOLS(data.lwage, data[["hours", "married"]], entity_effects=True)
```

Use instead:

```python
data = wage.set_index(["nr", "year"])
model = PanelOLS(data.lwage, data[["hours", "married"]], entity_effects=True)
```

With the year first, `PanelOLS` read the wage panel as 8 entities over 545 periods, with no error
and no warning:

| Index | Option | Coefficient of `married` |
| --- | --- | --- |
| `["year", "nr"]` | `entity_effects=True` | 0.1622 |
| `["nr", "year"]` | `time_effects=True` | 0.1622 |
| `["nr", "year"]` | `entity_effects=True` | 0.2483 |

The rule knows the order only by the names. A two level index is also made for other uses, so the
frame must reach a panel model. It reaches one when the `set_index` call is an argument of the
model, or when its result is assigned to a name that a panel model in the same function reads. The first form of the rule read
every `set_index` in a file that imports linearmodels, and 2 of its 4 reports on the public files were
a table of portfolios by month that no panel model reads.

The rule also stays silent when:

- another statement in the function can change the index of the same name, such as `swaplevel`, a
  second `set_index`, a merge or an assignment to `.index`
- the same chain goes on with such a call, as in `set_index(["year", "nr"]).swaplevel()`
- a later statement assigns the name again

A list with a comment inside is reported without the fix, which would delete the comment.

#### use-lsdv

**Severity:** warning. **Fix:** unsafe: `use_lsdv=True` is left out, and the estimates are equal but not bit equal.

The rule reports `use_lsdv=True` in `fit` of a `PanelOLS`. The option estimates each effect as a
dummy column, where the default removes the effects by subtracting group means and gives the same
estimates.

Example:

```python
result = PanelOLS(y, X, entity_effects=True).fit(use_lsdv=True)
```

Use instead:

```python
result = PanelOLS(y, X, entity_effects=True).fit()
```

On one thread, as the median of 5 fits with entity effects, 3 regressors and 10 periods, the two
forms took the following times:

| Entities | LSDV | Default | LSDV was slower on 7.1 by | On 7.0 by |
| --- | --- | --- | --- | --- |
| 500 | 0.298 s | 0.017 s | 17 times | 18 times |
| 3,000 | 77.1 s | 0.051 s | 1,508 times | 1,382 times |

The coefficients differed by at most 2.2e-16 and the standard errors by at most 1.7e-18, so the
estimates are equal but not bit equal, and the fix is unsafe. `use_lsmr=True` and
`low_memory=True` ran within 10% of the default and are not reported.

### Covariance

Each rule in the group reports a covariance choice that gives other standard errors than the code
asks for, or that raises. `fit` takes `cov_type` and, next to it as keywords, the options of the
chosen covariance. Each model has its own short list of names, and none of them is a statsmodels
name.

#### clustered-without-clusters

**Severity:** error. **Fix:** none, because the code does not show whether to cluster by entity or by time, and the groups for `clusters=` are not in the file.

The rule reports `fit(cov_type="clustered")` with none of `cluster_entity`, `cluster_time` and
`clusters`. Without them every observation is a cluster of its own, which is the
heteroskedasticity robust covariance under another name.

Example:

```python
result = PanelOLS(y, X, entity_effects=True).fit(cov_type="clustered")
```

Use instead:

```python
result = PanelOLS(y, X, entity_effects=True).fit(cov_type="clustered", cluster_entity=True)
```

On 7.1 and 7.0, `PanelOLS` with entity effects on the wage panel gave the following first standard errors:

| `fit` | First standard error |
| --- | --- |
| `cov_type="clustered"` | 0.041301 |
| `cov_type="robust"` | 0.041301 |
| `cov_type="clustered", cluster_entity=True` | 0.049385 |

`PooledOLS`, `RandomEffects` and `FirstDifferenceOLS` gave equal pairs in the same way. So did
`OLS`, `IV2SLS`, `IVLIML`, `AbsorbingLS`, `SUR` and `IV3SLS`, which take the groups as `clusters=`,
such as 0.427716 both ways on `IV2SLS`. `IVGMM`, `IVGMMCUE` and `IVJIVE` raise `TypeError` instead,
and the rule's message for them says so.

The rule reports 31 calls in 9 of the 950 public files, each a regression table whose standard errors
are said to be clustered and are not. `BetweenOLS` is left out, because it refuses `cluster_entity`
and its rows are the entities already.

#### cov-option-without-cov-type

**Severity:** error. **Fix:** unsafe: `cov_type="clustered"` or `cov_type="kernel"` is added to a call that names no `cov_type`. The call raised before the fix, except on `FamaMacBeth` and on the factor models of 7.0, where the fix changes the standard errors on purpose. A call that names another `cov_type` has no fix, because the code doesn't show which covariance is meant.

The rule reports `cluster_entity`, `cluster_time` or `clusters` in a `fit` call whose `cov_type` is
absent or is a name other than `"clustered"`. It also reports `bandwidth` or `kernel` in a call
whose `cov_type` is absent or is not a kernel covariance.

Example:

```python
result = PanelOLS(y, X, entity_effects=True).fit(cluster_entity=True)
fama = FamaMacBeth(y, X).fit(bandwidth=3)
```

Use instead:

```python
result = PanelOLS(y, X, entity_effects=True).fit(cov_type="clustered", cluster_entity=True)
fama = FamaMacBeth(y, X).fit(cov_type="kernel", bandwidth=3)
```

7.1 raised `ValueError: The keyword argument 'cluster_entity' is not accepted with
cov_type='unadjusted'` on `PanelOLS`. Each of the 15 classes other than `FamaMacBeth` and the
factor models was run with each of the five keywords, against no `cov_type` and against each name
the rule reads, and every call raised:

| Release | Calls | Errors |
| --- | --- | --- |
| 7.1 | 930 | `ValueError` or `UnknownOptionError` |
| 7.0 | 868 | `ValueError`, `TypeError` or `KeyError` |

`FamaMacBeth.fit` names `bandwidth` and `kernel` in its signature and reads them only with
`cov_type="kernel"`. On 7.1 and 7.0 the first standard error on the wage panel was:

| `FamaMacBeth(y, X).fit(...)` | First standard error |
| --- | --- |
| no option | 0.203664 |
| `bandwidth=3` | 0.203664 |
| `kernel="parzen"` | 0.203664 |
| `cov_type="kernel", bandwidth=3` | 0.300158 |

The factor models ignored the two keywords on 7.0, where `TradedFactorModel.fit(bandwidth=3)`
returned the standard error of the default, and they raise `ValueError` on 7.1.

#### unknown-cov-type

**Severity:** error. **Fix:** unsafe: on OLS, IV2SLS, IVLIML and AbsorbingLS, `"robust"` for `"HC0"`, `"robust", debiased=True` for `"HC1"` and `"unadjusted", debiased=True` for `"nonrobust"`, in a call that doesn't pass `debiased`. The call raised before the fix. Any other name has no fix, because the code doesn't show which name of the model is meant.

The rule reports a `cov_type` string that the model doesn't take, such as `"HC1"`, `"HAC"`,
`"cluster"` or `"nonrobust"` of statsmodels, `"clustered"` on `FamaMacBeth` or `IVSystemGMM`,
`"kernel"` on `BetweenOLS`, and `"unadjusted"` on a factor model.

Example:

```python
result = PanelOLS(y, X, entity_effects=True).fit(cov_type="HC1")
iv = IV2SLS(y, exog, endog, instruments).fit(cov_type="nonrobust")
```

Use instead:

```python
result = PanelOLS(y, X, entity_effects=True).fit(cov_type="robust")
iv = IV2SLS(y, exog, endog, instruments).fit(cov_type="unadjusted", debiased=True)
```

7.1 raises `UnknownOptionError`, which is both a `ValueError` and a `KeyError`, with the list of
names the model takes, and 7.0 raised `ValueError` or `KeyError`. The lists were read from the
error for each of the 19 model classes, and each name on a list was then run on its class:

| Models | Names |
| --- | --- |
| `PanelOLS`, `PooledOLS`, `RandomEffects`, `FirstDifferenceOLS` | `unadjusted`, `homoskedastic`, `conventional`, `robust`, `heteroskedastic`, `clustered`, `kernel`, `dk`, `driscoll-kraay`, `ac`, `autocorrelated` |
| `BetweenOLS` | `unadjusted`, `homoskedastic`, `conventional`, `robust`, `heteroskedastic`, `clustered` |
| `FamaMacBeth` | `unadjusted`, `homoskedastic`, `robust`, `heteroskedastic`, `kernel` |
| `OLS`, `IV2SLS`, `IVLIML`, `AbsorbingLS` | `unadjusted`, `homoskedastic`, `homo`, `robust`, `heteroskedastic`, `hccm`, `kernel`, `clustered`, `one-way`, and the four class names such as `KernelCovariance` |
| `IVGMM`, `IVGMMCUE`, `IVJIVE` | `unadjusted`, `homoskedastic`, `robust`, `heteroskedastic`, `kernel`, `clustered` |
| `SUR`, `IV3SLS` | `unadjusted`, `homoskedastic`, `robust`, `heteroskedastic`, `kernel`, `hac`, `clustered` |
| `IVSystemGMM` | `unadjusted`, `homoskedastic`, `robust`, `heteroskedastic`, `kernel`, `hac` |
| `TradedFactorModel`, `LinearFactorModel`, `LinearFactorModelGMM` | `robust`, `heteroskedastic`, `kernel` |

Since 7.1 the names are not case sensitive, so the rule takes a known name in any case.

On `OLS` with the mroz data, the standard errors of linearmodels were equal to the ones of statsmodels
to 1e-13 for each pair below, and the message for the instrument models gives the same pairs:

| linearmodels | statsmodels |
| --- | --- |
| `"robust"` | `"HC0"` |
| `"robust"` with `debiased=True` | `"HC1"` |
| `"unadjusted"` with `debiased=True` | `"nonrobust"` |
| `"clustered"` with `clusters=` and `debiased=True` | `"cluster"` with the same groups |

The panel models default to `debiased=True`, and their default standard errors were equal to the ones
of statsmodels.

#### kernel-default-bandwidth

**Severity:** warning. **Fix:** unsafe: `optimal_bw=True` is added. On 7.1 it gave estimates and standard errors bit equal to the default, without the warning, and on 7.0 it changes the bandwidth from `nobs - 2` to the one selected from the data.

The rule reports `weight_type="kernel"` on `IVGMM`, `IVGMMCUE` and `IVSystemGMM`, and
`cov_type="kernel"` in `fit` of `IVGMM` and `IVGMMCUE`, with neither `bandwidth` nor `optimal_bw`.
7.1 changed the default bandwidth from `nobs - 2` to one selected from the data, so the same code
gives other numbers on 7.0 and on 7.1.

Example:

```python
result = IVGMM(y, exog, endog, instruments, weight_type="kernel").fit()
```

Use instead:

```python
result = IVGMM(y, exog, endog, instruments, weight_type="kernel", optimal_bw=True).fit()
```

On the mroz data, the two releases gave the following numbers:

| Form | On 7.0 | On 7.1 |
| --- | --- | --- |
| `IVGMM(..., weight_type="kernel")`, J statistic and first standard error | 0.8022 and 0.43964 | 0.3794 and 0.43137, with `DefaultBandwidthWarning` |
| the same with `optimal_bw=True` | 0.3794 and 0.43137 | 0.3794 and 0.43137 |
| the same with `bandwidth=4` | 0.43117 | 0.43117 |
| `IVGMM(...).fit(cov_type="kernel")`, first standard error | 0.35929 | 0.4658 |
| the same with `bandwidth=4` | 0.46795 | 0.46795 |

`IVSystemGMM.fit(cov_type="kernel")` and `IV2SLS.fit(cov_type="kernel")` gave the same numbers on
both releases with no warning and are not reported.

#### panel-default-covariance

**Severity:** info. **Fix:** none.

The rule reports `fit` with no `cov_type` on `PanelOLS`, `PooledOLS`, `RandomEffects` and
`FirstDifferenceOLS`. The default is `"unadjusted"`, the covariance for errors that are
homoskedastic and unrelated over time, but the errors of one entity are usually related.

Example:

```python
result = PanelOLS(y, X, entity_effects=True).fit()
```

Use instead:

```python
result = PanelOLS(y, X, entity_effects=True).fit(cov_type="clustered", cluster_entity=True)
```

In two simulations with a true slope of 0, `PanelOLS` with entity effects gave p below 0.05 in
the following shares of the draws:

| Simulation | Default | `cov_type="robust"` | `cov_type="clustered"`, `cluster_entity=True` |
| --- | --- | --- | --- |
| 1,000 draws of 200 entities and 8 periods, with an AR(1) regressor and an AR(1) error within each entity, both with coefficient 0.8 | 17.3% | 16.4% | 5.7% |
| 300 entities and 3 periods, with errors that are heteroskedastic and unrelated over time | 22.4% | 7.1% | 5.0% |

How far the default is off depends on the data, so the rule is info. It is the rule with the most
reports, 372 in 142 of the 950 public files. `BetweenOLS` has one row for each entity and
`FamaMacBeth` has its own covariance, so both are left alone. A call that states `cov_type`, also
as `"unadjusted"`, is left alone, because the choice is written out.

### Instruments and absorbed effects

Each rule in the group reports an instrument model or an absorbed effect that is set up in another
way than the code means. `IV2SLS`, `IVLIML`, `IVGMM`, `IVGMMCUE` and `IVJIVE` take four arrays in
the order dependent, exog, endog, instruments. None of the four has a default, and none of the
models adds a constant.

#### iv-missing-arguments

**Severity:** error. **Fix:** none.

The rule reports an instrument model called with two or three arguments, and the keyword
`instrument=`. statsmodels has an `IV2SLS` of its own, in `statsmodels.sandbox.regression.gmm`,
with the signature `(endog, exog, instrument=None)`, where `endog` is the dependent variable.

Example:

```python
model = IV2SLS(y, X, Z)
```

Use instead:

```python
model = IV2SLS(y, exog, endog, instruments)
```

7.1 and 7.0 raised `TypeError: IV2SLS.__init__() missing 1 required positional argument: 'instruments'`, and `TypeError: ... got an unexpected keyword argument 'instrument'. Did you mean 'instruments'?` for the keyword. Two of the 950 public files call `IV2SLS(Y, X, Z)` on the class they import from linearmodels. A call that names one of the four as a keyword, or that unpacks its arguments, is left alone.

#### iv-arguments-swapped

**Severity:** warning. **Fix:** unsafe: the second and third arguments change places. The fix trusts the names, and it changes the estimates on purpose.

The rule reports an instrument model whose second argument has `endog` in its name and whose third
has `exog`. linearmodels takes exog and then endog, and it doesn't check the roles.

Example:

```python
model = IV2SLS(dependent, endog, exog, instruments)
```

Use instead:

```python
model = IV2SLS(dependent, exog, endog, instruments)
```

On the mroz data, `IV2SLS(lwage, educ, [const, exper], [fatheduc, motheduc])` ran without an error and gave `educ` 0.2021, where the right order gave 0.0664. The rule knows the roles only from the names, and reports no other case.

#### iv-without-exog

**Severity:** warning. **Fix:** none.

The rule reports `None` written as the `exog` of an instrument model that has an endogenous
regressor. The model then has no intercept.

Example:

```python
model = IV2SLS(y, None, x, z)
```

Use instead:

```python
model = IV2SLS(y, sm.add_constant(controls), x, z)
```

Over 2,000 draws of 500 rows from `y = 2 + x + error` with one instrument, the two forms gave the
following mean estimates:

| `exog` | Mean estimate of the coefficient on `x`, which is 1 |
| --- | --- |
| `None` | 2.002 |
| a constant column | 0.990 |

On the mroz data the coefficient on `educ` was 0.0928 without `exog` and 0.0664 with a constant and
`exper`. Data with mean zero needs no intercept, so the rule is a warning.

#### manual-two-stage

**Severity:** warning. **Fix:** none.

The rule reports a model whose `exog` holds `.fitted_values` of another fit. The model is `OLS` or
a panel class, or `IV2SLS`, `IVLIML` or `IVGMM` with `None` for `endog` and for `instruments`,
which is least squares. Such a model is two stage least squares by hand. The coefficient is right,
but the standard errors come from residuals that use the fitted values in place of the regressor.

Example:

```python
first = OLS(data.educ, data[["const", "exper", "fatheduc", "motheduc"]]).fit()
second = OLS(data.lwage, pd.concat([data[["const", "exper"]], first.fitted_values], axis=1)).fit()
```

Use instead:

```python
result = IV2SLS(data.lwage, data[["const", "exper"]], data.educ, data[["fatheduc", "motheduc"]]).fit()
```

| Measurement | By hand | `IV2SLS` |
| --- | --- | --- |
| coefficient of `educ` on the mroz data | 0.0664 | 0.0664 |
| unadjusted standard error on the mroz data | 0.0327 | 0.0311 |
| coverage of the 95% interval, over 2,000 draws of 500 rows with one instrument | 99.6% | 95.7% |

In the draws, the standard error by hand was 1.37 times the other on average. Which way the error
goes depends on the data. The rule sees only the form where the fitted values are written in the
call, and it misses a second stage that reads them from a column stored earlier. Neither set of
code has the form the rule reads.

#### absorb-not-categorical

**Severity:** info. **Fix:** none.

The rule reports `AbsorbingLS(..., absorb=...)` in a file that never says `category`,
`Categorical` or `cut(`. `absorb` reads the dtype of each column. A categorical column is absorbed
as one effect for each category, and any other column as one continuous regressor.

Example:

```python
model = AbsorbingLS(data.y, data[["x"]], absorb=data[["firm"]])
```

Use instead:

```python
model = AbsorbingLS(data.y, data[["x"]], absorb=data[["firm"]].astype("category"))
```

With 200 firms, a firm effect in the regressor and a true coefficient of 1.0, no form raised an
error or a warning:

| `absorb` | Coefficient |
| --- | --- |
| the integer firm id | 3.397 |
| none | 3.398 |
| the firm id as a category | 1.022 |

A text column raises `TypeError` on 7.1, and 7.0 took it like the integers. One of the 4 public files
that the rule reports turns its columns into text with `astype(str)` right before the call. The
code doesn't show a dtype, so the rule is info. Data that is categorical when it is loaded is
reported without need.

### Results

Read a linearmodels result by its own attribute names, pass `compare` a precision that it takes, and use the built-in Hausman test in place of one written by hand.

#### statsmodels-result-name

**Severity:** error. **Fix:** unsafe: `summary` for `summary()`, `std_errors` for `bse`, `tstats` for `tvalues`, `resids` for `resid` and `fitted_values` for `fittedvalues`. On the results of the panel models, OLS, IV2SLS, IVLIML, IVGMM, IVGMMCUE, IVJIVE and AbsorbingLS, also `resid_ss` for `ssr`, `s2` for `mse_resid`, `f_statistic.stat` for `fvalue` and `f_statistic.pval` for `f_pvalue`. The other names, and those four on the system and factor models, have no fix.

The rule reports a statsmodels result name read on a linearmodels result, such as `summary()`
called, `bse`, `tvalues`, `resid`, `fittedvalues`, `aic`, `bic`, `llf`, `fvalue`, `f_pvalue`,
`ssr`, `mse_resid`, `cov_params`, `t_test`, `f_test`, `get_robustcov_results`, `summary2`, `save`
and 36 more.

Example:

```python
result = PanelOLS(y, X, entity_effects=True).fit()
print(result.summary())
errors = result.bse
```

Use instead:

```python
result = PanelOLS(y, X, entity_effects=True).fit()
print(result.summary)
errors = result.std_errors
```

`summary` is a property, and on 7.1 and 7.0 `summary()` raised `TypeError: 'Summary' object is not callable` on the result of each of the 19 model classes. The other names are the 53 public names of a statsmodels 0.15.0 OLS result that none of the 19 results has, found with `dir()` and `hasattr`, and each raises `AttributeError`. The results of the factor models have no residuals under either name, so `resid` and `fittedvalues` are not reported on them.

| statsmodels | linearmodels |
| --- | --- |
| `summary()` | `summary` |
| `bse` | `std_errors` |
| `tvalues` | `tstats` |
| `resid` | `resids` |
| `fittedvalues` | `fitted_values` |
| `cov_params()` | `cov` |
| `fvalue`, `f_pvalue` | `f_statistic.stat`, `f_statistic.pval` |
| `ssr` | `resid_ss` |
| `mse_resid` | `s2` |
| `llf` | `loglik`, on the panel results |
| `t_test`, `f_test` | `wald_test`, on the panel and instrument results |
| `get_robustcov_results` | a new `fit(cov_type=...)` |

On `OLS` with the mroz data, each pair of the table from `bse` to `mse_resid` was equal to 1e-11 for an unadjusted fit with `debiased=True`, and `loglik` of `PooledOLS` was equal to `llf`. The rule reports 15 reads in 6 of the 950 public files. `.summary()` is called in 171 of the 950 files, and the rule reports it in the 3 where the result can be followed to a linearmodels model. Code under a `hasattr` check is a compatibility path and is left alone.

#### invalid-compare-precision

**Severity:** error. **Fix:** unsafe: `"std_errors"` for `"se"`, `"stderr"`, `"std_err"` and `"bse"`, `"tstats"` for `"t"`, `"tstat"` and `"t-stats"`, and `"pvalues"` for `"p"` and `"pvalue"`. The call raised before the fix. Any other string has no fix.

The rule reports `compare(..., precision=...)` of `linearmodels.panel` or `linearmodels.iv` with a
string other than `"tstats"`, `"std_errors"`, `"std-errors"` and `"pvalues"`.

Example:

```python
table = compare({"pooled": pooled, "fixed": fixed}, precision="se")
```

Use instead:

```python
table = compare({"pooled": pooled, "fixed": fixed}, precision="std_errors")
```

On 7.1 and 7.0 both functions took the four strings in any case, and raised `ValueError: Unknown precision value. Must be one of 'tstats', 'std_errors' or 'pvalues'.` for `"se"`, `"stderr"`, `"t"`, `"p"`, `"tstat"`, `"std_err"`, `"pvalue"`, `"t-stats"` and `"bse"`. The four strings were also read from the source. The rule reads `compare` only when the file imports it from linearmodels or calls it through the module.

#### hand-written-hausman

**Severity:** info. **Fix:** none. Needs linearmodels 7.1.

The rule reports a function whose name starts with `hausman`. It leaves alone `hausman_taylor`,
which is the Hausman and Taylor estimator, and names such as `hausman_table` or `hausman_data`,
which show results or prepare data. 7.1 added the Hausman test of random effects against fixed
effects as a method of the random effects result, and the instrument results have had
`wu_hausman()` for longer.

Example:

```python
def hausman(fe, re):
    b = fe.params - re.params
    v = fe.cov - re.cov
    return float(b @ np.linalg.inv(v) @ b)
```

Use instead:

```python
test = random_effects_result.hausman(fixed_effects_result)
```

On 7.1 the method returned a `HausmanTestResults` with the fields `statistic` and `estimates`, and 7.0 has no such method. The rule reports 25 functions in 25 of the 950 public files. It cannot see what a function tests, so it is info, and the message names both tests.

### Removed and deprecated forms

Each rule in the group reports a form that a release removed or deprecated. Each form was run on
the release before and the release at the version the rule names, both installed from PyPI.

#### removed-in-linearmodels

**Severity:** error. **Fix:** unsafe: `wald_test` for `test_linear_constraint`, because on 4.14 `test_linear_constraint` is a method of its own that warns and then calls `wald_test`. The import has no fix.

The rule reports an import of `linearmodels.utility`, which 4.18 split into the modules of
`linearmodels.shared`. It also reports `test_linear_constraint` on a result of `OLS`, `IV2SLS`,
`IVLIML` or `IVGMM`, which 4.12 renamed `wald_test` and 4.15 removed.

Example:

```python
from linearmodels.utility import MissingValueWarning, WaldTestStatistic

test = result.test_linear_constraint(restriction, value)
```

Use instead:

```python
from linearmodels.shared.exceptions import MissingValueWarning
from linearmodels.shared.hypotheses import WaldTestStatistic

test = result.wald_test(restriction, value)
```

| Form | Ran on | Raised on |
| --- | --- | --- |
| `from linearmodels.utility import ...` | 4.17 | 4.18 and 7.1, with `ModuleNotFoundError: No module named 'linearmodels.utility'` |
| `test_linear_constraint` | 4.14, where it returned a `WaldTestStatistic` | 4.15, with `AttributeError` |

`wald_test` ran on 4.14 and on 4.15. The 22 public names of the old module were looked up in the
7.1 source:

| Names of `linearmodels.utility` | Module on 7.1 |
| --- | --- |
| `MissingValueWarning`, `MemoryWarning`, `InferenceUnavailableWarning`, `SingletonWarning`, `missing_warning` | `linearmodels.shared.exceptions` |
| `AttrDict`, `ensure_unique_column`, `panel_to_frame` | `linearmodels.shared.utility` |
| `WaldTestStatistic`, `InvalidTestStatistic`, `InapplicableTestStatistic`, `InvalidTestWarning`, `quadratic_form_test` | `linearmodels.shared.hypotheses` |
| `has_constant`, `inv_sqrth` | `linearmodels.shared.linalg` |
| `pval_format`, `param_table`, `format_wide` | `linearmodels.shared.io` |
| `get_string`, `get_float`, `get_bool`, `get_array_like` | `linearmodels.shared.typed_getters` |

Code inside a try block with an except clause is a compatibility path and is left alone.

#### deprecated-in-linearmodels

**Severity:** warning. **Fix:** safe: the import of `linearmodels.__future__.ordering` is deleted when it is a statement of the module, and `absorb_options=` with `method="lsmr"` for `lsmr_options=` in a call that names neither `method` nor `absorb_options`, when the value is a dictionary written with at least one key. The two calls gave equal results on 7.1. unsafe: the same fix for any other value. With `lsmr_options=None` or an empty dictionary the old call kept the default method, which may pick HDFE. The other forms have no fix.

The rule reports `from linearmodels.__future__ import ordering`, the `lsmr_options` keyword of
`AbsorbingLS.fit`, and `linearmodels.DROP_MISSING`. 7.1 gives `FutureWarning` for each, and 8.0
removes them.

Example:

```python
from linearmodels.__future__ import ordering

result = model.fit(method="lsmr", lsmr_options={"atol": 1e-10})
```

Use instead:

```python
result = model.fit(method="lsmr", absorb_options={"atol": 1e-10})
```

The import turned off the sorting of formula variables in 4.31, and 5.0 made the unsorted order the
default. With `"y ~ 1 + z + x + w"`, the formula gave the following order of the variables:

| Release | Order without the import | With the import |
| --- | --- | --- |
| 4.31 | `Intercept, w, x, z` | `Intercept, z, x, w` |
| 5.0, 7.0 and 7.1 | `Intercept, z, x, w` | `Intercept, z, x, w` |

The fix deletes a line that does nothing from 5.0 on, and the cases of
`samples/linearmodels/migration.py` returned equal values before and after it. Because of `since`,
xpd skips the document on a project that uses 4.31. Inside a function or a block the import has no fix, because
it may be the only statement there.

| Form | Result |
| --- | --- |
| `absorb_options` | ran on 4.18, and raised `TypeError` on 4.17 |
| `lsmr_options` | gave `FutureWarning` on 4.31, 5.0, 7.0 and 7.1 |
| reading `DROP_MISSING` | ran clean on 7.0, and gave `FutureWarning: linearmodels.DROP_MISSING has no effect` on 7.1 |

The new form needs `method="lsmr"` next to the options, because the default method may pick an
algorithm that doesn't take them. On 7.1 `fit` turns the default method into `"lsmr"` when
`lsmr_options` is a dictionary that is not empty, and the fixed call gave bit equal estimates.

### What the pack leaves out

The pack has no rule for the following forms:

- Forms that a pattern can't tell apart from right code:
  - Inputs whose labels don't match, which raise `ValueError` on 7.1 where 7.0 paired them by
    position. A pattern can't see an index.
  - A frame without a two level index, a time level of text, and regressors that the effects
    absorb. Each raises with a clear message, and none shows in the code.
  - The changes of 7.1 to weighted models, to `FamaMacBeth` p-values and to the Hausman and
    specification tests. The code is the same on both releases, so a rule would report every use.
- Forms that were tried and gave no reason for a rule:
  - `check_rank=False`, because on columns that are exactly collinear 7.1 raised
    `LinAlgError: Singular matrix`. Where it was tried, the flag didn't give wrong numbers in
    silence.
  - Patsy functions that formulaic lacks. `te()` raised, and `cr()` and `cc()` raised a rank error
    next to an intercept. `Q()`, `center()`, `standardize()`, `bs()` and
    `C(x, Treatment(reference=...))` all ran. No use of the three was found in the public files.
  - xxhash, which `AbsorbingLS` uses for its cache when it is installed, as a package check. A fit
    with two absorbed columns took 0.134 s with it and 0.134 s without on 100,000 rows, and
    1.130 s against 1.123 s on 1,000,000, with the default number of threads.
- `SUR`, `IV3SLS`, `IVSystemGMM` and the factor models, which have no rule of their own. The
  formula, covariance and result rules read them.

## arch

The arch rules find code that raises on arch 9.0, options and arguments that arch refuses or
never reads, returns in units that break the fit, and random draws without a seed. They run on
files that import [arch](https://bashtage.github.io/arch/), a library for ARCH and GARCH
volatility models, unit root and cointegration tests, bootstraps and multiple comparison tests.

| Group | What its rules report |
| --- | --- |
| Removed API | API that arch 8.0 and 9.0 removed |
| Options and arguments | a string or a number that arch refuses, and an argument that arch accepts and never reads |
| The units of the data | returns as decimals, and results that are read in rescaled units |
| Forecasts and fits | forecasts that raise or that compute the same rows many times, and fits that print in a loop |
| Bootstraps, tests and covariance estimators | draws without a seed, results read before `compute`, and a bandwidth that changed its meaning in 9.0 |

Every number in the arch section was measured on arch 9.0.0 with NumPy 2.5.5, pandas 3.1.1, SciPy
1.18.1 and statsmodels 0.15.0, unless another release is named. The examples assume that the
arch names they use are imported, and names such as `returns`, `prices`, `losses` and `x` stand
for values from your program.

xpd skips a rule document about a newer arch than the one in the project's venv. arch 8.0
removed the `RandomState` paths of the bootstraps and the distributions. arch 9.0 removed
`arch.__future__` and the `type` keyword of `hedgehog_plot`, and it made some attributes read
only. It also started to check every argument. A rule document about such a change carries
`since`, and xpd reads the installed version from the venv as it does for pandas.

```text
on(check a file that imports arch)
  if the venv has no arch, or there is no venv
    run every document
  else if the document's since is above the installed version
    skip the document           # a project on arch 7.2 gets no report about what 8.0 removed
  else
    run the document
```

Each rule ties its report to a name that the file binds to arch, because the rules match the
shape of the code and do not know the type of a variable. `samples/arch/names.py` shows each
case.

| Code | Read by the rules |
| --- | --- |
| a call by the name of an arch class or function, such as `arch_model(...)` or `IIDBootstrap(...)` | yes |
| a short name that another library may have, such as `SPA`, `MCS` or `Normal` | only when the file imports it from arch |
| a method such as `fit`, `forecast` or `conf_int` on a variable | only when the file assigns the variable from an arch name and from nothing else |
| the same variable bound as a parameter, a loop variable, an `as` target or one name of a tuple assignment | no |
| an option or a value written in the call, such as `vol="gjr"` | yes |
| a variable in place of the option or the value | no |

### The proof each rule needs

Every rule document has a proof that was run, and the table gives the five kinds of proof.

| Proof | How it was run | What it shows |
| --- | --- | --- |
| interpreter | The old form ran on arch 9.0.0. A document with `since` also ran on the release before the one it names. | The old form raises and the new form runs. On the release before, the old form ran or only warned. |
| same result | A fit or a test ran with and without the argument. | Both gave the same parameters or the same statistic, so arch does not read the argument. |
| numbers | Both forms ran on the daily S&P 500 returns that ship with arch. | The two numbers. |
| speed | Both forms were timed as the median of 3 runs. | The two times, and whether the two forms return the same values. |
| rerun | The old form ran twice in fresh objects. | The two results differ, and the new form gives the same result twice. |

A document is `error` only on an interpreter proof, and its message names the exception. A
document that can also match right code, such as `rescale=True` on data that needs no
rescaling, is a `warning`.

The rules were then run over two sets of real code, and a rule that reported correct code was
narrowed or dropped.

| Set | Files | What the files are |
| --- | --- | --- |
| arch's own code at the 9.0.0 tag | 202 | 69 library files, 104 test files, the code blocks of 16 documentation pages, 9 notebooks and 4 scripts |
| public code | 1,303 from 861 repositories | 795 Python files and 508 notebooks that GitHub code search found for the import lines of arch, of which 1,268 import arch |

Every report was read except the 291 reports of `arch-unseeded-draws`, which a script checked
by looking in each file for a later `seed` call or `state` assignment on the same variable. The
table gives the reports that were left after the rules were narrowed.

| Rule | Severity | Proof | Reports on arch's own code | Reports on public code | Wrong advice | Fix |
| --- | --- | --- | --- | --- | --- | --- |
| [`removed-in-arch`](#removed-in-arch) | error | interpreter: ModuleNotFoundError, ImportError, TypeError, AttributeError, ValueError | 1 | 34 | 0 | unsafe |
| [`arch-unknown-option`](#arch-unknown-option) | error | interpreter: ValueError | 0 | 13 | 0 | unsafe |
| [`arch-ignored-argument`](#arch-ignored-argument) | warning | same result, and rerun for the two seeds | 10 | 21 | 1 | none |
| [`arch-rejected-number`](#arch-rejected-number) | error | interpreter: TypeError, ValueError | 1, over 189 files | not run | 1 | unsafe |
| [`unscaled-returns`](#unscaled-returns) | warning | numbers: mu 4.02 against 0.056, with optimizer code 4 | 0 | 21 | 0 | none |
| [`rescaled-units`](#rescaled-units) | warning | numbers: a variance forecast 10,000 times the data's | 4 | 14 | 7 | none |
| [`analytic-forecast-horizon`](#analytic-forecast-horizon) | error | interpreter: ValueError | 0 | 0 | no reports | none |
| [`forecast-per-origin`](#forecast-per-origin) | warning | speed: 0.106 seconds against 0.0005 | 1 | 0 | 0 | none |
| [`reindex-is-default`](#reindex-is-default) | info | same result: equal frames with and without the argument | 20 | 61 | 0 | safe |
| [`fit-output-in-loop`](#fit-output-in-loop) | info | numbers: 240 lines for 20 fits against 0 | 0 | 15 | 0 | unsafe |
| [`arch-unseeded-draws`](#arch-unseeded-draws) | info | rerun: the result differs | 109 | 182 | 1 | none |
| [`results-before-compute`](#results-before-compute) | error | interpreter: RuntimeError | 2 | 0 | 2 | none |
| [`opt-bandwidth-as-bandwidth`](#opt-bandwidth-as-bandwidth) | warning | numbers: 0.5245 against 0.5158 | 0 | 0 | no reports | unsafe |

Wrong advice counts the reports where following the message would not fix a real problem in that
code, and each rule's section says which reports they are. Before the rules were narrowed, the
first run made 508 reports.

| Code | Reports | Wrong advice |
| --- | --- | --- |
| arch's library source | 1 | 1, a placeholder bootstrap that a base class builds and never draws from |
| arch's tests, notebooks and documentation | 146 | 7 |
| public code | 361 | 3 |

`arch-rejected-number` was added after a review, when the public code was no longer on disk. It
ran over the 189 Python files of arch's repository and made one report, in a test that checks
the error it reports.

A rule that reported `forecast(start=...)` on a result whose `fit` had no `last_obs` was
written and dropped. The parameters of such a fit have seen the rows that are forecast, and the
forecasts did change, but most of the reports were on right code.

| Measurement on 1,500 S&P 500 returns, forecasts from row 1,000 on | Value |
| --- | --- |
| mean difference from the forecasts of a fit that stopped at row 1,000 | 13% |
| QLIKE loss with the fit on all rows | 0.142 |
| QLIKE loss with the fit that stopped at row 1,000 | 0.181 |
| reports on public code | 14, of which 7 start at the last row and forecast the future, and 4 start at row 0 to draw the fitted path |
| reports on arch's own code | 11, all examples and tests that forecast the fitted rows on purpose |

Recall was not measured, because no set of real arch code was held back. The table says how
often a rule is wrong when it reports, and not how often it misses. A rule that follows a
variable is silent when the file binds the same name to something else anywhere, also in
another function. For example, a file that uses `res` for an arch result and for another object
gets no report on `res`.

### Removed API

`removed-in-arch` reports code that raises on arch 9.0.0. Each form was run on the release
before the removal and on the release that made it, both installed from PyPI, because the
release notes of 8.0 do not list what 8.0 removed. No fix of a removed form is safe, because
the old form raises and its result cannot be compared with the result of the new form.

#### removed-in-arch

**Severity:** error. **Fix:** safe, because arch 7.2.0 ran the old and the new form and gave equal values or the same object: `seed=` replaces `random_state=` on a distribution or on one of the five bootstraps that 7.2.0 has, and names the one positional argument of a distribution, `bs.state` replaces `bs.get_state()`, `bs.state = s` replaces the statement `bs.set_state(s)`, and `bs.generator` replaces `bs.random_state` on those bootstraps. The import of `cov_nw` from `arch.utility` moves to `arch.utility.cov`, and the other names of the same import stay in a line of their own. Also safe: `plot_type=` replaces `type=` on `hedgehog_plot` of a result that the file gets from an arch model, which drew the same figure with either keyword on 8.0.0. unsafe: the same bootstrap fixes on a `LocalBlockBootstrap`, which came in 9.0 and never took the old forms, and the deleted `arch.__future__` import. Deleting `import arch.__future__.reindexing` also stops that line from binding `arch`. `seed()` on an object, a distribution with more than one argument, `hedgehog_plot` of an object the file does not tie to arch, the read only attributes and the `nested` and `studentize` pair have no fix.

The rule reports a form that arch 8.0 or 9.0 removed. The table after the example lists every form, with
what it did on the release before the removal and what it does now.

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

| Form | Before | Removed in | Now |
| --- | --- | --- | --- |
| `random_state=` on `IIDBootstrap`, `IndependentSamplesBootstrap`, `StationaryBootstrap`, `CircularBlockBootstrap`, `MovingBlockBootstrap` or `LocalBlockBootstrap` | 7.2.0: FutureWarning | 8.0.0 | TypeError, because `random_state` is read as data |
| `random_state=` or a positional argument on `Normal`, `StudentsT`, `SkewStudent` or `GeneralizedError` | 7.2.0: FutureWarning | 8.0.0 | TypeError |
| `bs.seed(1)`, `bs.get_state()`, `bs.set_state(s)` and `bs.random_state` on a bootstrap | 7.2.0: FutureWarning | 8.0.0 | AttributeError |
| `seed` on `SPA`, `StepM`, `MCS` or `RealityCheck` | 7.2.0: FutureWarning | 8.0.0 | AttributeError |
| `from arch.utility import cov_nw` | 7.2.0: runs | 8.0.0 | ImportError |
| `from arch.__future__ import reindexing` | 8.0.0: runs and does nothing | 9.0.0 | ModuleNotFoundError |
| `res.hedgehog_plot(type="mean")` | 8.0.0: FutureWarning | 9.0.0 | TypeError |
| an assignment to `block_size` of a block bootstrap, or to `bootstrap` of a multiple comparison test | 8.0.0: runs | 9.0.0 | AttributeError |
| `nested=True` with `studentize=False` on `SPA`, `StepM` or `RealityCheck` | 8.0.0: runs | 9.0.0 | ValueError |

The bootstraps take their data as keyword arguments, so 8.0.0 and 9.0.0 read `random_state` as
one more series, and they raise because a `RandomState` is not an array. `seed` takes the same
`RandomState`, a `Generator` or an integer. It exists since arch 5.0, so on 4.19
`IIDBootstrap(x, seed=0)` raised TypeError. `IIDBootstrap(x, seed=np.random.RandomState(0))` drew
the same samples on 6.1.0, 7.2.0, 8.0.0 and 9.0.0. The `state` property reads and sets the
state of the generator, and `generator` gives the generator itself.

Other objects also have the names `seed`, `get_state`, `set_state`, `random_state`, `block_size`
and `bootstrap`, such as a NumPy `RandomState`. The rule reports them only on a variable that the
file assigns from one of the arch classes and from nothing else. It reports `hedgehog_plot` on
any object, because no other library that was looked at has a method with the name. It fixes the
call only on a result that the file gets from `fit` or `fix` of an arch model.

The rule leaves alone an import inside a `try` block that catches the error, code behind a
version check, and code inside `pytest.raises`. The pack has no rule for deprecated API, because
the source of arch 9.0.0 raises no `DeprecationWarning` and no `FutureWarning`.

### Options and arguments

The rules in the group report an argument that arch 9.0 refuses or never reads. arch 9.0 checks
every argument when a function is called, and it still ignores an argument that does not apply
to the model that was asked for.

| Rule | What it reports | What happens |
| --- | --- | --- |
| `arch-unknown-option` | a string that is not one of the option's values | ValueError |
| `arch-ignored-argument` | an argument that arch accepts and never reads | the code runs and estimates another model or test than the one it describes |
| `arch-rejected-number` | a number that arch refuses | TypeError or ValueError |

#### arch-unknown-option

**Severity:** error. **Fix:** unsafe, because the old call raises: `trend="n"` for `trend="nc"` in the classes that take `"n"`. Every other option has no fix, because the call does not show which accepted option was meant.

The rule reports a string that is not one of the option's values, for each option in the table after the
example. arch raises ValueError on such a string.

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

The lists in the rule are the ones that arch 9.0.0 uses, and none is typed by hand. arch checks
each option against a table of accepted spellings, without regard to case. A script read the
tables in three steps. First, it wrapped arch's check, called each API once and kept the table
that the check was handed. Second, it ran each API with every spelling of its table, in lower
and in upper case. Third, it ran each API with 28 other strings, and each of the 28 raised
ValueError.

The tables hold more than the signatures show. For example, `vol="constant"` builds a model with
a constant variance, and `conf_int` also takes `"var"`, `"cov"`, `"debiased"` and
`"bias-corrected"`. Moreover, `method` of the unit root tests also takes `"t"` and `"tstat"`, and
`bootstrap="moving_block"` is the same as `"moving block"`.

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

The documents have no `since`, so xpd also reports on a project with an older arch. Before 9.0
some of the options took any string and fell back to one value without a warning.

| Call | Releases | Result | Value with a right option |
| --- | --- | --- | --- |
| `fit(cov_type="mle")` on 1,500 daily S&P 500 returns | 4.19, 5.0, 6.1.0, 6.2.0, 8.0.0 | the standard errors of `"classic"`, 0.00843 for omega | `"robust"`: 0.01329 |
| `PhillipsPerron(y, test_type="z")` | 4.19, 5.0, 6.1.0, 6.2.0, 8.0.0 | the statistic of `"tau"`, -1.136 | `"rho"`: -2.0998 |
| `MCS(losses, 0.1, method="range")` | 5.0, 6.1.0, 6.2.0, 8.0.0 | the p-values of `"max"` | `"R"`: other p-values |

Releases before 9.0 took some options in one case only. For example, `ADF(y, method="AIC")`
raised on 8.0.0 and runs on 9.0.0. The rule follows 9.0.0 and does not report it, and it does not
report a value that only 9.0 has, such as `method="maic"` or `align="horizon"`, either.

The public code had 13 reports, and each was a value that arch refuses:

- `trend="nc"`, which is the old statsmodels name
- `vol="AVGARCH"` and `disp="on"`
- `method="stationary"` on `MCS`
- `ZivotAndrews(y, method="c")`, where the trend was written as the method

#### arch-ignored-argument

**Severity:** warning. **Fix:** none. Each message names two rewrites, and the call does not show which one is meant, and the `rng` form needs a seed that the file does not hold.

The rule reports an argument that arch accepts and never reads for the model or the test that the call
asks for.

| Argument | Reported when the call has |
| --- | --- |
| `lags=` on `arch_model` | `mean` `"constant"`, `"zero"` or `"ls"`, or no `mean` |
| `x` on `arch_model` | `mean` `"constant"`, `"zero"`, `"ar"` or `"har"`, or no `mean` |
| `o=`, `q=` and `power=` | `vol="arch"` or `vol="harch"` |
| `power=` | `vol` `"egarch"`, `"aparch"`, `"fiaparch"` or `"ngarch"` |
| `o=` | `vol="figarch"` |
| `method=` and `max_lags=` on `ADF`, `DFGLS` or `ZivotAndrews` | a number for `lags` |
| `random_state=` on `forecast` | `method="simulation"` |
| `rng=` on `forecast` | `method="bootstrap"` |

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

A reported argument does not change the fit. Each form was fitted with and without the argument
on 1,500 daily S&P 500 returns, on arch 9.0.0 and on 8.0.0, and the table gives whether the
parameters of the two fits were equal. `"fiaparch"` and `"ngarch"` are new in 9.0 and were run
there only.

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
`"aic"` chose 0, and the statistic of `ADF` moved from -1.2674 to -1.0838.

`forecast` reads `random_state` only for `method="bootstrap"` and `rng` only for
`method="simulation"`. The table gives the variance five steps ahead from two calls with the
same seed.

| Method | Seed | First call | Second call |
| --- | --- | --- | --- |
| `"simulation"` | `random_state=np.random.RandomState(0)` | 2.586 | 2.598 |
| `"simulation"` | a seeded `rng` | 2.571 | 2.571 |
| `"bootstrap"` | a seeded `rng` | 2.588 | 2.470 |
| `"bootstrap"` | `random_state=np.random.RandomState(0)` | 2.595 | 2.595 |

The page on volatility forecasts in arch's own documentation passes `random_state` to a
simulation. A call that passes both seeds is left alone.

The rule leaves alone a value that changes nothing in a GARCH model either, such as `o=0`, `q=0`
or `power=2.0`. It also leaves alone `lags=0`, `x=None`, and a `mean` or a `vol` that is a
variable. A call with three or more positional arguments is left alone, because its `mean` may
be positional.

The public code had 21 reports, and none had wrong advice.

| Reports | Code |
| --- | --- |
| 12 | `x=` or a positional `x` with a constant mean. Seven are copies of one function that fits `arch_model(resid, x=exog, mean="Constant", ...)` as if the regressors were in the model. |
| 4 | `vol="ARCH"` with `q=3` or `q=1`, where the order of an ARCH model is `p` |
| 2 | `random_state=` on a simulation |
| 2 | `lags=1` with `mean="constant"` |
| 1 | a `method` beside `lags=8` |

Of the 10 reports on arch's own code, one is a test that passes `max_lags=2.5` to check that it
raises, and its advice is wrong. The other nine are in tests and in the
documentation page on volatility forecasts.

#### arch-rejected-number

**Severity:** error. **Fix:** unsafe, because the old call raises: a float with nothing but zeros after the point loses the point, as in `p=1` for `p=1.0`, `size=` of two digits becomes the fraction, as in `size=0.95` for `size=95`, and `o=1` is added after `vol="ngarch"` when the call has no `o`. A float such as `2.5`, any other `size`, and an `o=0` that is written have no fix.

The rule reports a number written in the call that arch refuses: a float where arch counts something, a `size` of `conf_int` of 1 or more, and `vol="ngarch"` with `o` left at 0.

Example:

```python
test = ADF(prices, lags=5.0)
model = arch_model(returns, vol="ngarch")
interval = IIDBootstrap(returns, seed=0).conf_int(np.mean, reps=1000, size=95)
```

Use instead:

```python
test = ADF(prices, lags=5)
model = arch_model(returns, vol="ngarch", o=1)
interval = IIDBootstrap(returns, seed=0).conf_int(np.mean, reps=1000, size=0.95)
```

Each form was run on arch 9.0.0 and on 8.0.0.

| Form | arch 9.0.0 | arch 8.0.0 |
| --- | --- | --- |
| `p=1.0`, `o=1.0` or `q=1.0` in `GARCH`, `EGARCH` or `APARCH`, and in `arch_model` with no `vol` or with `vol` `"garch"`, `"egarch"`, `"aparch"`, `"fiaparch"` or `"ngarch"` | TypeError: p must be an integer >= 0; got 1.0 (float) | the classes ran, and `arch_model(p=1.0)` raised AssertionError |
| `p=1.0` in `ARCH` or `FIGARCH` | TypeError: p must be an integer >= 1; got 1.0 (float) | ran |
| `lags=5.0` in `ADF`, `DFGLS`, `KPSS`, `PhillipsPerron`, `ZivotAndrews`, `VarianceRatio` or `ARX` | TypeError: lags must be an integer >= 0 or None; got 5.0 (float) | ran, apart from `PhillipsPerron` |
| `max_lags=5.0` in `ADF`, `DFGLS` or `ZivotAndrews` | TypeError: max_lags must be an integer >= 0 or None | TypeError: expected a sequence of integers or a single integer |
| `reps=10.0` in `conf_int`, `var`, `cov`, `apply`, `bootstrap`, `SPA`, `StepM` or `MCS` | TypeError: reps must be an integer >= 1; got 10.0 (float) | TypeError: 'float' object cannot be interpreted as an integer |
| `forecast(horizon=2.0)` and `hedgehog_plot(horizon=2.0)` | TypeError: horizon must be an integer >= 1; got 2.0 (float) | ValueError: horizon must be an integer >= 1. |
| `forecast(..., method="simulation", simulations=100.0)` | TypeError: simulations must be an integer >= 1 | TypeError: 'float' object cannot be interpreted as an integer |
| `conf_int(size=95)`, `size=90.0`, `size=1` | ValueError: size must be a finite float in (0, 1); got 95 (int) | ValueError: size must be strictly between 0 and 1 |
| `arch_model(vol="ngarch")` with no `o` or with `o=0` | ValueError: o must be at least 1 when vol='ngarch' | ValueError: Unknown model type in vol, for every `o` |

The documents of the first three rows and of the last carry `since: "9.0"`, because 8.0.0 took
the float in the classes and has no `"ngarch"`. `arch_model(vol="ngarch", o=1)` ran on 9.0.0,
and `o=2` raised because `o` may not be above `p`.

`arch_model` does not check an order that the process does not have, so the rule reads
`arch_model` only with the `vol` values of the first row. For example, `o=1.0` and `q=1.0` ran
with `vol="arch"` and `"harch"`, `o=1.0` ran with `"figarch"`, and all three ran with
`"constant"`.

| Keyword | Where the rule reads it | Why |
| --- | --- | --- |
| `horizon`, `simulations` | only on a result that the file gets from `arch_model` | other libraries have a `forecast` with a horizon |
| `reps`, `size` | by the name of the method | no other library that was looked at has the keywords on `conf_int`, `var`, `cov`, `apply` or `bootstrap` |

A number that is a variable is not read, and neither is a positional argument. The rule was
added after a review and was not run over the public code, which was no longer on disk. Over the
189 Python files of arch's repository it made one report, on `conf_int(mean_fn, 20, size=1.5)`
in a test that checks the error.

### The units of the data

Give arch returns in percent, and divide a result by `scale` when you let arch rescale. arch
estimates by maximum likelihood, and the optimizer works badly when the variance of the data is
far from 1. arch warns `DataScaleWarning` when the variance is outside the range it accepts, and
its message asks for a value between 1 and 1000. Daily returns as decimals have a variance near
0.0001, and the 5,030 daily S&P 500 returns in `arch.data.sp500` have 0.000145.

#### unscaled-returns

**Severity:** warning. **Fix:** none. The message names two rewrites, and the call does not show which one is meant.

The rule reports a series from `pct_change`, or from `log` with `diff` or `shift`, that is given to `arch_model`, `ConstantMean`, `ZeroMean`, `ARX`, `HARX`, `LS`, `MAMean` or `ARCHInMean` with nothing that multiplies it and no `rescale`.

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

The table gives a constant mean GARCH(1,1) fitted to the S&P 500 returns in four forms.

| Returns | arch | Warnings | Optimizer code | mu | omega |
| --- | --- | --- | --- | --- | --- |
| `100 * price.pct_change()` | 9.0.0, 8.0.0 | none | 0 | 0.0564 | 0.0175 |
| `price.pct_change()` | 9.0.0, 8.0.0 | DataScaleWarning, ConvergenceWarning | 4 | 4.02 | 0.0000029 |
| `np.log(price).diff()` | 9.0.0, 8.0.0 | DataScaleWarning, ConvergenceWarning | 4 | -2.73 | 0.0000029 |
| `price.pct_change()` | 5.0, 6.1.0, 6.2.0 | DataScaleWarning | 0 | -0.0011 | 0.0000029 |
| `price.pct_change()` with `rescale=True` | 9.0.0, 8.0.0 | none | 0 | 0.0564 | 0.0175 |

A mean of 4.02 for returns whose sample mean is 0.00021 is a failed fit, and on the older releases the
optimizer reported success with a mean of the wrong sign. `rescale=False` turns the warning off
and gave the same failed fit.

The rule reads how the series is built and not its values.

```text
report(series given to the model)
  if the series is written in the call, or every assignment in the file builds it
     as pct_change, the difference of a log, or the log of a price over its lag
     followed by nothing but dropna, fillna, copy, squeeze, to_numpy, iloc, loc or values
    report
leave alone
  a series that is multiplied anywhere       # 100 * prices.pct_change(), returns = returns * 100
  returns divided by their standard deviation, which have a variance of 1
  returns that a helper takes as an argument
  a call that says rescale, either way
not seen
  returns that come from a file or a function
```

All 21 reports on the public code were returns as decimals, and each series was a `pct_change`
or a log difference with no factor.

#### rescaled-units

**Severity:** warning. **Fix:** none. The division goes where the code reads each result, which the match does not reach.

The rule reports `rescale=True` on `arch_model` or a mean class in a file that reads `conditional_volatility` or calls `forecast` and never reads `scale`.

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
100, and the parameters matched a fit of the returns in percent.

| Result | Rescaled units | Units of the data |
| --- | --- | --- |
| last conditional volatility | 1.9926 | 0.019926 |
| variance forecast one step ahead | 3.5965 | 0.00035965, which is 3.5965 divided by `scale` squared |

The rule is a warning, because it cannot see two cases where nothing is wrong. When the data
already has a variance near 1, the scale is 1.0 and nothing changes. Results that have no unit,
such as the standardized residuals, are right at any scale.

Of its 14 reports on the public code, 11 used a volatility or a forecast in rescaled units. Two
of the 11 divided by 10,000 in the code where the square of `scale` belongs. The other three
had wrong advice. Two fitted data that was made standard normal first, and one only divided the
residuals by the volatility. The 4 reports on arch's own code are tests of `rescale` itself, and
their advice is wrong as well.

### Forecasts and fits

The rules in the group report forecasts that raise or repeat work, and fits that print. The
table gives the defaults of the two calls.

| Call | Default | Effect |
| --- | --- | --- |
| `forecast` | `method="analytic"`, `horizon=1`, `reindex=False` | forecasts from the last row unless `start` names an earlier one |
| `fit` | `disp="final"`, `update_freq=1` | prints the steps of the optimizer unless `disp="off"` |

#### analytic-forecast-horizon

**Severity:** error. **Fix:** none. The message names two methods, and the call does not show which one is meant.

The rule reports `forecast` with a `horizon` above 1 and the analytic method on a result of `arch_model` with `vol` `"egarch"`, `"aparch"` or `"fiaparch"`, or with a `power` that is not 2 and `vol` `"garch"`, `"figarch"` or no `vol`. Reports `hedgehog_plot` on such a result as well, unless the call says `horizon=1` or another method.

Example:

```python
res = arch_model(returns, vol="EGARCH", p=1, o=1, q=1).fit(disp="off")
forecasts = res.forecast(horizon=5)
```

Use instead:

```python
res = arch_model(returns, vol="EGARCH", p=1, o=1, q=1).fit(disp="off")
forecasts = res.forecast(horizon=5, method="simulation")
```

arch has no analytic formula for a forecast beyond one step when the volatility model is not a
model of the squared returns. `forecast(horizon=5)` was run for each `vol` on arch 9.0.0.

| Model | `forecast(horizon=5)` | `forecast(horizon=1)` |
| --- | --- | --- |
| `vol="egarch"`, `"aparch"`, `"fiaparch"` | ValueError: Analytic forecasts not available for horizon > 1 | runs |
| `vol="garch"` or `"figarch"` with `power=1.0` | ValueError: Analytic forecasts not available for horizon > 1 when power != 2 | runs |
| `vol="garch"`, also with `o=1`, `"arch"`, `"figarch"`, `"harch"`, `"ngarch"` | runs | runs |

The rule follows the model through the file, as in the call tree below. Each of the two names
must be assigned from nothing else.

```text
am = arch_model(returns, vol="EGARCH")     # such an arch_model call
  res = am.fit(...)                        # or am.fix(...)
    res.forecast(horizon=5)                # reported
```

A `horizon` or a `method` that is a variable is not read, and `power` is read only where
`arch_model` reads it. A model whose `volatility` the file assigns, as in
`am.volatility = GARCH(1, 0, 1)`, is left alone. A model that is built from the classes, such as
`ConstantMean(y, volatility=EGARCH())`, is not seen.

`hedgehog_plot` forecasts 10 steps with `method="analytic"` unless the call says otherwise. On
9.0.0 and on 8.0.0 it raised the same ValueError for `vol="egarch"`, and it ran with `horizon=1`
and with `method="simulation"`. The rule reads it only when every argument is a keyword, and
neither set of code had a report.

#### forecast-per-origin

**Severity:** warning. **Fix:** none. The rewrite replaces the loop around the call.

The rule reports `forecast` with `start` set to the variable of a `for` loop or a comprehension over `range(...)`, when the loop calls neither `fit` nor `fix`.

Example:

```python
rows = []
for origin in range(first, len(returns)):
    rows.append(res.forecast(horizon=10, start=origin).variance.iloc[0])
```

Use instead:

```python
variance = res.forecast(horizon=10, start=first).variance
```

`forecast(start=i)` returns one row for every origin, the row a forecast starts from, from `i`
to the end of the data. The loop computes the same rows many times over and keeps one. Both forms ran on the 5,030 S&P 500
returns with `horizon=10` and gave equal values.

| arch | Loop over the last 200 origins | One call |
| --- | --- | --- |
| 9.0.0 | 0.106 seconds | 0.0005 seconds |
| 8.0.0 | 0.329 seconds | 0.0056 seconds |

The rule leaves alone a loop that fits inside, because the loop is then a rolling estimate where
each origin has its own parameters. It also leaves alone a loop over a few chosen origins, such
as a tuple. A call whose object or other arguments hold the loop variable is left alone too, as
in `fits[i].forecast(start=i)`, because each origin has its own result there. The one report is
a test in arch's own code that steps through the origins of a hedgehog plot.

#### reindex-is-default

**Severity:** info. **Fix:** safe: the argument is taken out. When it is the first argument or follows a comment, the fix writes the arguments again without it and drops the comments between them.

The rule reports `reindex=False` given to `forecast` of an arch model or result.

Example:

```python
res = arch_model(returns).fit(disp="off")
forecasts = res.forecast(horizon=5, reindex=False)
```

Use instead:

```python
res = arch_model(returns).fit(disp="off")
forecasts = res.forecast(horizon=5)
```

Before arch 6.2, `forecast` returned a frame as long as the data and warned that the default
would change. Code from before 6.2 passes `reindex=False` to get the short frame without the
warning, and the document carries `since: "6.2"`.

| arch | `forecast()` on 1,500 rows | `forecast(reindex=False)` |
| --- | --- | --- |
| 6.1.0 | 1,500 rows and a FutureWarning | the short frame, without the warning |
| 6.2.0, 8.0.0, 9.0.0 | 1 row and no warning | the same frame as `forecast()`, for the last row and with `start` |

The fix is safe, because the two frames were equal on 6.2.0, 8.0.0 and 9.0.0. `reindex=True`
still returns the long frame and is left alone.

The import `from arch.__future__ import reindexing` comes from the same habit and is reported by
[`removed-in-arch`](#removed-in-arch). The rule made 61 reports on the public code, and 20 on
arch's own tests, notebooks and documentation.

#### fit-output-in-loop

**Severity:** info. **Fix:** unsafe: `disp="off"` is added at the end of the arguments, and comments between them are dropped. The fit returns the same result and prints nothing, so a script or a log that reads the printed lines gets none.

The rule reports `fit` without `disp` on an arch model inside a loop or a comprehension.

Example:

```python
for end in range(1000, len(returns)):
    res = arch_model(returns.iloc[:end]).fit()
```

Use instead:

```python
for end in range(1000, len(returns)):
    res = arch_model(returns.iloc[:end]).fit(disp="off")
```

`fit` has `disp="final"` and `update_freq=1`, so it prints a line for every iteration and a
summary at the end. The rule is info, because the output costs no time.

| Twenty fits of a GARCH(1,1) in a loop | Lines printed | Time |
| --- | --- | --- |
| `fit()` | 240 | 0.102 seconds |
| `fit(disp="off")` | 0 | 0.101 seconds |

`update_freq=0` alone still prints the summary. All 15 reports on the public code were fits in a
loop over windows, assets or model orders.

### Bootstraps, tests and covariance estimators

The rules in the group report random draws without a seed, test results read before
`compute`, and a bandwidth whose meaning changed in arch 9.0.

#### arch-unseeded-draws

**Severity:** info. **Fix:** none. The seed is not in the file.

The rule reports a call that draws random numbers without a seed.

| Call | Reported without |
| --- | --- |
| `IIDBootstrap`, `IndependentSamplesBootstrap`, `StationaryBootstrap`, `CircularBlockBootstrap`, `MovingBlockBootstrap`, `LocalBlockBootstrap` | `seed` |
| `SPA`, `StepM`, `MCS`, `RealityCheck` | `seed` |
| `forecast` with `method="simulation"` | `rng`, unless the file builds a distribution with a seed |
| `forecast` with `method="bootstrap"` | `random_state` |
| `simulate` on an arch model | a distribution with a seed anywhere in the file |

Example:

```python
np.random.seed(0)
bs = StationaryBootstrap(10, returns)
interval = bs.conf_int(np.mean, 1000)
```

Use instead:

```python
bs = StationaryBootstrap(10, returns, seed=0)
interval = bs.conf_int(np.mean, 1000)
```

arch draws from generators of its own, which start from a new seed in every process. Each form
was run twice without a seed and twice with one, on arch 9.0.0 and on 8.0.0. The two runs
without a seed differed on both releases, and the table gives the numbers of 9.0.0.

| Form | Two runs without a seed | Two runs after `np.random.seed(0)` | Two runs with a seed | Seeded by |
| --- | --- | --- | --- | --- |
| `IIDBootstrap(x).conf_int(np.mean, 200)`, lower bound | -0.15601, -0.15855 | -0.1328, -0.15246 | -0.15667 twice | `seed=0` |
| `StationaryBootstrap(10, x)`, the same | -0.13476, -0.13188 | not run | -0.14428 twice | `seed=0` |
| `CircularBlockBootstrap(10, x)`, the same | -0.18054, -0.14953 | not run | -0.15445 twice | `seed=0` |
| `MovingBlockBootstrap(10, x)`, the same | -0.14193, -0.14765 | not run | -0.16665 twice | `seed=0` |
| `SPA(...)`, lower p-value | 0.7233, 0.6800 | not run | 0.6700 twice | `seed=0` |
| `MCS(...)`, first p-value | 0.250, 0.283 | not run | 0.297 twice | `seed=0` |
| `forecast(horizon=5, method="simulation")`, last step | 2.671, 2.601 | 2.676, 2.534 | 2.571 twice | `Normal(seed=0)` as the model's distribution, or a seeded `rng` |
| `forecast(horizon=5, method="bootstrap")`, last step | 2.587, 2.578 | 2.468, 2.737 | 2.595 twice | `random_state=np.random.RandomState(0)` |
| `arch_model(None).simulate(params, 100)`, last value | -1.24, 0.07 | -1.27, 0.30 | 0.097 twice | `Normal(seed=0)` as the model's distribution |

arch does not draw from NumPy's global generator, so `np.random.seed(0)` did not make any two
runs equal. Of the 291 reports on the two sets of code, 49 are in a file that calls
`np.random.seed`. One
report has wrong advice, on a placeholder bootstrap that a base class in arch's own source
builds and never draws from.

`seed` is a keyword of the bootstraps, the tests and the distributions since arch 5.0, so every
document carries `since: "5.0"`. The rule is info, because a seed does not move what the method
estimates.

The rule leaves alone a bootstrap whose `state` or `generator` the file assigns. On arch 9.0.0,
8.0.0 and 5.0, `bs.generator = np.random.default_rng(0)` gave the same interval as `seed=0`. The
two documents that read the model's distribution leave alone a file that builds a distribution
with `seed=` anywhere, because a pattern cannot see which model gets it. `SPA`, `StepM`, `MCS`
and `RealityCheck` are read only when the file imports them from `arch.bootstrap`.

#### results-before-compute

**Severity:** error. **Fix:** none. `compute` must run once before the first read, in a statement that the match does not reach.

The rule reports `pvalues`, `critical_values`, `better_models`, `superior_models`, `included` and `excluded` read from an `SPA`, `StepM`, `MCS` or `RealityCheck` in a file that never calls `compute` on that object.

Example:

```python
mcs = MCS(losses, size=0.1, seed=0)
kept = mcs.included
```

Use instead:

```python
mcs = MCS(losses, size=0.1, seed=0)
mcs.compute()
kept = mcs.included
```

The tests run their bootstrap in `compute` and not when they are created. On a new object each
read raised RuntimeError, on arch 9.0.0 and on 8.0.0.

| Class | Read | Message |
| --- | --- | --- |
| `SPA` | `pvalues`, `critical_values()`, `better_models()` | compute must be called before pvalues are available |
| `StepM` | `superior_models` | compute must be called before accessing superior_models |
| `MCS` | `pvalues`, `included`, `excluded` | Must call compute before accessing results |

The rule reports a result read straight from the constructor call. It also reports a result read
from a variable that the file assigns from one of the four classes and from nothing else, when no
line of the file calls `compute` on that variable. It does not follow the order of the lines, so
a read before a later `compute` is not reported.

The rule leaves alone a variable that the file also reads without a dot after it, such as an
argument of a call, an item of a tuple or a list, or the right side of an assignment. Such a
variable can reach `compute` under another name, as in `for test in (spa, mcs): test.compute()`.
Both reports, on arch's own code, have wrong advice, because they are rows of a table in arch's
tests that pass a bad `pvalue` to `critical_values` to check the ValueError that comes first.

#### opt-bandwidth-as-bandwidth

**Severity:** warning. **Fix:** unsafe: `bandwidth=est.bandwidth` for `bandwidth=est.opt_bandwidth`. The fixed call repeats the first estimate, so the number changes, 0.5158 to 0.5245 in the Bartlett example.

The rule reports `bandwidth=` set to the `opt_bandwidth` of an estimator, on `Bartlett`, `Gallant`, `NeweyWest`, `Parzen`, `ParzenCauchy`, `ParzenGeometric`, `ParzenRiesz`, `TukeyHamming`, `TukeyHanning` and `TukeyParzen`.

Example:

```python
first = Bartlett(returns)
again = Bartlett(returns, bandwidth=first.opt_bandwidth)
```

Use instead:

```python
first = Bartlett(returns)
again = Bartlett(returns, bandwidth=first.bandwidth)
```

A kernel covariance estimator reports the bandwidth it used as `bandwidth` and the optimal
bandwidth of Newey and West as `opt_bandwidth`. Before arch 9.0 the two were one number. Since
9.0 a kernel that is 0 beyond a finite lag uses `opt_bandwidth` minus 1, and the release notes
say that `Bartlett(x, bandwidth=est.opt_bandwidth)` is not the same as `Bartlett(x)`.

| Estimator on 1,500 S&P 500 returns | `bandwidth` | `opt_bandwidth` | Long run variance | With `bandwidth=opt_bandwidth` |
| --- | --- | --- | --- | --- |
| `Bartlett`, arch 9.0.0 | 10.943 | 11.943 | 0.52445 | 0.51582 |
| `Parzen`, arch 9.0.0 | 20.467 | 21.467 | 0.48023 | 0.47335 |
| `QuadraticSpectral`, arch 9.0.0 | 7.658 | 7.658 | 0.55401 | 0.55401 |
| `Bartlett`, arch 8.0.0 | 19.221 | 19.221 | 0.45172 | 0.45172 |

A script built every estimator class of `arch.covariance.kernel` on 9.0.0 and compared the two
attributes. The two attributes differ for the ten classes the rule names, and the rule leaves
alone `Andrews`, `QuadraticSpectral` and `ZeroLag`, where they are equal. The document carries
`since: "9.0"`, and neither set of code had a report. The automatic bandwidth itself also changed in 9.0,
from 19.221 to 10.943 for `Bartlett`, so an estimate without `bandwidth` differs between the
two releases.

### What the pack leaves out

The pack has no rule for the habits in the table, and the last column gives the reason for each.

| Habit | What was measured | Why there is no rule |
| --- | --- | --- |
| `SPA`, `StepM` and `RealityCheck` studentize the losses since arch 9.0 when `studentize=True`, the default, which did nothing before | On the same seeded losses the p-values of `SPA` were 0.6733, 0.91 and 0.96 on 8.0.0 and 0.67, 0.8967 and 0.9533 on 9.0.0, and the lowest critical value moved from 0.1353 to 1.5984. `studentize=False` on 9.0.0 gave the numbers of 8.0.0. | A rule would report nearly every call, because few calls say `studentize` |
| `nested=True` on the same tests runs a bootstrap inside every replication since 9.0 | With 300 replications `SPA` took 1.14 seconds, against 0.0105 without it and 0.0109 on 8.0.0 | The option asks for the extra work |
| `forecast(start=...)` on a fit without `last_obs` forecasts rows that the fit has seen | The numbers are under "The proof each rule needs" | The rule reported more right code than wrong and was dropped |
| `starting_values` from the last window of a rolling fit | 40 windows took 0.209 seconds without them and 0.200 with them | The fit was not faster |
| an `IIDBootstrap` on a series that is correlated in time, where arch's documentation gives the block bootstraps and `optimal_block_length` | `CircularBlockBootstrap` and `MovingBlockBootstrap` raise TypeError since 9.0 on a block size that is not a whole number, such as the 55.04 that `optimal_block_length` returned for one series. `StationaryBootstrap` takes it. | A pattern cannot see whether a series is correlated, and the block size is a variable in such code |
| a unit root test on the wrong series, or a p-value read against the wrong null hypothesis | On 1,000 log prices of the S&P 500, `ADF` gave a p-value of 0.648 and `KPSS` gave 0.0001, because the null of `ADF` is a unit root and the null of `KPSS` is no unit root | A pattern sees neither the series nor the reading |
| a `block_size` given as a float | It was a variable in every file that was read | `arch-rejected-number` reads only a number written in the call |

## pymoo

The pymoo rules find code from older pymoo releases that raises or does nothing on pymoo 0.6.2,
problem classes that hand pymoo wrong numbers, runs whose result is not the one the code asks
for, and reference points and weights that give another number than the code means. They run on
files that import [pymoo](https://pymoo.org/), a library for single-objective and
multi-objective optimization with evolutionary algorithms.

| Group | What its rules report |
| --- | --- |
| Older releases | imports, keywords and options of pymoo 0.4 and 0.5 that raise, warn or do nothing on 0.6.2 |
| The problem class | an `_evaluate` that hands pymoo wrong numbers or raises |
| Running an algorithm | `minimize` keywords that pymoo never reads, and an algorithm read or reused after a run |
| Random numbers | runs without a seed, and draws from `np.random` that the seed does not reach |
| Indicators and decisions | reference points and weights that give another number than the code means |

Every number in the pymoo section was measured on pymoo 0.6.2 with NumPy 2.4.6, unless another
release is named. The examples assume that the pymoo names they use are imported, and names such
as `problem`, `F` and `pf` stand for values from your program.

xpd picks the rule documents that match the pymoo release in the project's venv. pymoo renamed
its modules in 0.5.0, and it removed `pymoo.factory` and most keywords of the 0.4 API in 0.6.0
and 0.6.1. It moved its random numbers to a generator of its own in 0.6.1.6. A rule document about
such a change carries `since`. A document carries `until` as well when its advice differs by
release, and a second document then gives the advice of the newer releases. xpd reads the
installed version from the venv as it does for pandas:

```text
on(check a file that imports pymoo)
  if the venv has no pymoo, or there is no venv
    run every document that has no until
  else
    run each document whose since is at most the installed version
      and whose until, if it has one, is above it
```

A project on pymoo 0.5.0 therefore gets no report about 0.6. The table gives the three moves that
need `until`, and a project gets the module of the release in its venv. From 0.6.0 to 0.6.1.5
the import from `pymoo.parallelization` raised ModuleNotFoundError.

| Names | 0.5.0 | 0.6.0 to 0.6.1.5 | 0.6.1.6 and later |
| --- | --- | --- | --- |
| `StarmapParallelization`, `DaskParallelization` | not in pymoo | `pymoo.core.problem` | `pymoo.parallelization` |
| Nelder and Mead, pattern search | `pymoo.algorithms.soo.nonconvex.nelder_mead`, `pattern_search` | `pymoo.algorithms.soo.nonconvex.nelder`, `pattern` | `pymoo.algorithms.soo.nonconvex.nelder`, `pattern` |
| `DES` | `pymoo.algorithms.soo.nonconvex.de` | not in pymoo | `pymoo.operators.selection.des` from 0.6.2 |

Every rule ties what it reports to pymoo by a name, because the rules match the shape of the code
and do not know the type of a variable. A class or
a function must be one that the file imports from pymoo, such as `Problem` from
`pymoo.core.problem` or `minimize` from `pymoo.optimize`. An object must be a name that the file
binds to such a class. One document of `deprecated-in-pymoo`, about the `"feasible"` key, reads
the name of a variable instead, and its section says so.

### The proof each rule needs

Every rule document has a proof that was run, and the table gives the four kinds of proof.

| Proof | How it was run | What it shows |
| --- | --- | --- |
| interpreter | The old form and the new form ran. For a removed import, the old form also ran on the last release that has it and on the next one, from the eleven releases from 0.4.2.2 to 0.6.2 on PyPI. | The old form raises or warns and the new form runs clean. `since` names the first release where the old form raises. |
| value | The old and the new form ran on the same input. | Both results. |
| rerun | The old form ran twice. | The two results differ, and the new form gives the same result twice. |
| speed | Both forms were timed. | The two times, and whether the two forms return the same values. |

A document is `error` only on an interpreter proof or a value proof, and its message quotes what
was measured. A document that can also match code that is meant, such as a mutation that few
individuals should get, is a `warning`.

The lists inside the rules were read from the installed wheels, the package files that PyPI
serves for each release, and not written by hand.

| List | Entries | Check |
| --- | --- | --- |
| removed modules | 130 | each import ran on the release before and on the release named |
| names that a kept module lost | 49 | the same |
| termination names that 0.6.0 dropped | 10 | read from the wheels |
| algorithm classes | 45 | read from the wheels |
| replacement modules that a fix writes | 89 | each was imported on 0.6.2 with every name the fix lets through |

The first two lists leave out `pymoo.experimental`, `pymoo.usage`, `pymoo.docs` and
`pymoo.vendor`.

The rules were then run over two sets of code, and the reports were read. A rule that reported
correct code was narrowed, and the table after the list of sets gives what is left.

| Set | Files | What the files are |
| --- | --- | --- |
| pymoo's own code at the 0.6.2 tag | 541 | the library, its examples, its tests and the Python code of its documentation pages |
| public code | 2,046 from 409 repositories | files that import pymoo, found by a code search on GitHub. Another 2,464 files in the repositories are copies of pymoo's own modules and are not counted. |

| Rule | Severity | Proof | Reports on pymoo's own code | Reports on public code | Wrong advice | Fix |
| --- | --- | --- | --- | --- | --- | --- |
| [`removed-in-pymoo`](#removed-in-pymoo) | error | interpreter: ModuleNotFoundError, ImportError, Exception, AttributeError | 0 | 1,694 | 0 of 40 read | unsafe |
| [`deprecated-in-pymoo`](#deprecated-in-pymoo) | warning | interpreter: DeprecationWarning, and the same class or array | 2 | 97 | 0 of 36 read | safe |
| [`n-constr-keyword`](#n-constr-keyword) | info | value: the same X, F and G | 2 | 232 | 0 of 28 read | safe |
| [`positional-problem-bounds`](#positional-problem-bounds) | error | value: `xl` [1, 1] and `xu` None, and interpreter: TypeError | 0 | 0 | no reports | none |
| [`ignored-problem-keyword`](#ignored-problem-keyword) | error, warning | value: [0.66, 0.93, 1.26] against [0.14, 0.77, 1.94], and 0 calls of the starmap | 1 | 67 | 0 of 9 read | none |
| [`setup-positional-termination`](#setup-positional-termination) | error | value: past 30 generations where 7 were asked | 0 | 1 | 0 | unsafe |
| [`has-terminated-assignment`](#has-terminated-assignment) | error | value: 0 more generations against 15 | 0 | 4 | 0 | none |
| [`mutation-prob-per-individual`](#mutation-prob-per-individual) | warning | value: 2 percent of the individuals against 57 percent | 0 | 48 | 0 of 14 read | unsafe |
| [`indicator-normalize-ignored`](#indicator-normalize-ignored) | error | value: 25.25 against 0.082 | 0 | 0 | no reports | unsafe |
| [`objectives-stacked-by-row`](#objectives-stacked-by-row) | error | value: a least f2 of 0.003 against 10 | 0 | 0 | no reports | unsafe |
| [`evaluate-returns-value`](#evaluate-returns-value) | error | value: F is inf | 0 | 0 | no reports | none |
| [`constraint-not-declared`](#constraint-not-declared) | error | interpreter: Exception | 0 | 0 | no reports | none |
| [`evaluate-indexing`](#evaluate-indexing) | error | interpreter: IndexError, Exception | 0 | 24 | 0 | none |
| [`elementwise-runner`](#elementwise-runner) | error | interpreter: TypeError, and value: 0 calls of the runner | 0 | 0 | no reports | none |
| [`minimize-ignored-keyword`](#minimize-ignored-keyword) | error | value: 1,000 generations where 7 were asked, and rerun | 2 | 1 | 0 | unsafe |
| [`algorithm-read-after-minimize`](#algorithm-read-after-minimize) | error | value: None, and interpreter: TypeError | 0 | 0 | no reports | none |
| [`reused-algorithm-without-copy`](#reused-algorithm-without-copy) | error | value: the F of the first run | 0 | 0 | no reports | none |
| [`unread-history`](#unread-history) | info | speed: 2.6 times, and 20.3 MB against 0.13 MB | 6 | 110 | 1 of 6 read | none |
| [`moead-arguments`](#moead-arguments) | error | interpreter: TypeError, AttributeError | 0 | 3 | 0 | none |
| [`unseeded-run`](#unseeded-run) | info | rerun: the result differs | 21 | 286 | 0 of 24 read | none |
| [`numpy-seed-ignored`](#numpy-seed-ignored) | warning | rerun: the result differs | 23 | 148 | 0 of 32 read | none |
| [`hv-reference-point`](#hv-reference-point) | warning, error | value: 0.09 against 0.37, and interpreter: AssertionError | 0 | 2 | 0 | none |
| [`decision-weights`](#decision-weights) | warning | value: another solution is picked | 0 | 0 | no reports | none |

A value proof gives the old form's result against the new form's. Wrong advice counts the reports
where following the message would not fix a real problem in that code. Where a rule has more
reports than were read, the cell says how many were read, and every report of the other rules
was read.

The one report with wrong advice is on a documentation page of pymoo that saves the history for
a later page to plot, and xpd reads the later page as another file. The public reports of
`unread-history` were not judged, because 94 of the 110 are in a repository where another file
names `history`, and the rule is `info` for the same reason. Apart from the
one report with wrong advice, the 57 reports on pymoo's own code are real.
For example, its examples pass `n_gen=100` to `minimize`, where the keyword does nothing, and its
documentation reads the deprecated `"feasible"` key. The custom operators in its guide draw from
`np.random`.

The rules were narrowed in two rounds, the first after the first run and the second after a
review. The reports on pymoo's own code and on the public files were the same before and after
the second round, line for line.

| Rule | Right code it matched | Change |
| --- | --- | --- |
| `algorithm-read-after-minimize` | 29 callbacks and comprehensions with a variable named like the algorithm | needs the read in the scope of the run, and reads a walrus as a new binding |
| `evaluate-returns-value` | one base class that returns `NotImplemented` | leaves the class alone |
| `unread-history` | runs inside functions, whose caller may read the history | reads runs outside every function |
| `evaluate-indexing`, `objectives-stacked-by-row` | a class that can be built with `elementwise=True` | leave the class alone |
| `reused-algorithm-without-copy` | a loop over the algorithms | leaves the loop alone |
| `has-terminated-assignment`, the `calc` document of `removed-in-pymoo` | a name with no tie to pymoo | need a name that the file ties to pymoo |

The first three rows are the first round. The review also added `positional-problem-bounds`,
which made no report on either set.

Recall was not measured, because no set of real pymoo code was held back. The rule table says
how often a rule is wrong when it reports, and not how often it misses.

The pack has no rule for the habits in the table, because a rule cannot tell from the code of one
file whether the habit is a mistake. The last column gives the reason for each habit.

| Habit | What was measured | Why there is no rule |
| --- | --- | --- |
| A constraint written with the wrong sign | `out["G"]` must be at most 0 where the constraint holds | The direction a constraint is meant in is not in the code |
| A maximized objective that is not negated back | pymoo only minimizes, so `res.F` holds the negated value | The problem class and the code that reads `res.F` are mostly in different files |
| `pop_size` below the number of reference directions in `NSGA3` | pymoo prints a warning, and 54 solutions came back for 91 directions | The number of directions is a binomial of two arguments |
| A problem without `xl` and `xu` | `GA` raised TypeError when it sampled | A binary or a permutation problem needs no bounds |
| Variables given with `vars` and run with `NSGA2` | TypeError, `'>=' not supported between instances of 'dict' and 'dict'` | The problem and the algorithm are tied only by the call that runs them |
| A hypervolume of objectives on another scale than the reference point | `HV(ref_point=[1.1, 1.1])` gave 0.0 for a first objective between 100 and 900 | The scale of the objectives is in the data |
| `ASF` on objectives that are not normalized | It picked another solution than on normalized objectives | Whether the objectives share a scale is in the data |
| Silencing the warning about functions that are not compiled | With the compiled functions switched off, sorting 2,000 points took 27 times as long on pymoo 0.6.1.6 and no longer than before on 0.6.2 | Nothing is slower on the current release |

### Older releases

The rules in the group report forms that worked on an older pymoo and that raise, warn, or are
accepted and do nothing on pymoo 0.6.2.

#### removed-in-pymoo

**Severity:** error. **Fix:** unsafe: the new module for 89 removed modules when every imported name is still in it, such as `pymoo.core.problem` for `pymoo.model.problem` and `pymoo.algorithms.moo.nsga2` for `pymoo.algorithms.nsga2`, the new import for `get_problem`, `get_termination` or `get_reference_directions` imported alone from `pymoo.factory`, `do` for `calc` on an indicator, and the new module for a name that moved, such as `pymoo.parallelization.dask` for `DaskParallelization` from `pymoo.core.problem`, when the statement is on one line and imports only names that moved there. `DES` gets the new module from pymoo 0.6.2 on, the first release that has it. No release from 0.4.2.2 to 0.6.2 had an old and a new form as one object. The new modules of 0.5.0 and the new homes of the moved names came after the old ones went, the `pymoo.factory` functions of 0.6.0 are wrappers that warn and call the new ones, the sorting functions of 0.6.1.6 are other objects than their new copies, and no release has both `calc` and `do` on an indicator. The other forms have no fix.

The rule reports a module, a name or an option that pymoo removed between 0.5.0 and 0.6.2, and the
message names what took its place where there is one.

| Form | Examples |
| --- | --- |
| an import of one of the 130 removed modules | `pymoo.model`, the algorithm modules of 0.4 such as `pymoo.algorithms.nsga2` and `pymoo.algorithms.so_genetic_algorithm`, the long operator modules such as `pymoo.operators.crossover.simulated_binary_crossover`, `pymoo.performance_indicator`, `pymoo.decision_making`, `pymoo.util.termination`, `pymoo.operators.mixed_variable_operator`, `pymoo.interface`, `pymoo.util.function_loader` and `pymoo.factory` |
| a function of `pymoo.factory` that built an object from a name | `get_sampling("real_random")`, and the message names the class to use |
| one of 49 names that a module of 0.6.2 no longer has | `Display` in `pymoo.util.display`, `starmap_parallelized_eval` in `pymoo.core.problem` |
| one of the ten termination names that 0.6.0 dropped, given to `get_termination` or in a tuple given to `minimize` | `"f_tol"`, `"x_tol"` |
| `calc` on an indicator | `Hypervolume(...).calc(F)` |

An import inside a `try` block that catches the error, or behind a version check, is left alone.

Example:

```python
from pymoo.factory import get_problem, get_sampling, get_crossover, get_mutation
from pymoo.algorithms.nsga2 import NSGA2
from pymoo.performance_indicator.hv import Hypervolume

algorithm = NSGA2(
    pop_size=100,
    sampling=get_sampling("real_random"),
    crossover=get_crossover("real_sbx", prob=0.9, eta=15),
    mutation=get_mutation("real_pm", eta=20),
)
res = minimize(get_problem("zdt1"), algorithm, ("f_tol", 0.0025), seed=1)
hv = Hypervolume(ref_point=np.array([1.1, 1.1])).calc(res.F)
```

Use instead:

```python
from pymoo.problems import get_problem
from pymoo.algorithms.moo.nsga2 import NSGA2
from pymoo.indicators.hv import HV
from pymoo.operators.crossover.sbx import SBX
from pymoo.operators.mutation.pm import PM
from pymoo.operators.sampling.rnd import FloatRandomSampling
from pymoo.termination.default import DefaultMultiObjectiveTermination

algorithm = NSGA2(
    pop_size=100,
    sampling=FloatRandomSampling(),
    crossover=SBX(prob=0.9, eta=15),
    mutation=PM(eta=20),
)
res = minimize(get_problem("zdt1"), algorithm, DefaultMultiObjectiveTermination(ftol=0.0025), seed=1)
hv = HV(ref_point=np.array([1.1, 1.1]))(res.F)
```

#### deprecated-in-pymoo

**Severity:** warning. **Fix:** safe: `from pymoo.parallelization import StarmapParallelization` for the import from `pymoo.core.problem` when the statement imports that one name, and `"FEAS"` for `"feasible"` on the population of a result of `minimize`. The key on another object has no fix.

The rule reports two forms that pymoo deprecated.

| Form | On pymoo | Replacement |
| --- | --- | --- |
| `StarmapParallelization` imported from `pymoo.core.problem` | 0.6.1.6: ImportError. 0.6.2: DeprecationWarning. | the import from `pymoo.parallelization`, which gives the same class on 0.6.2 |
| the key `"feasible"` given to `get` | since 0.6.1.2: a DeprecationWarning for every individual, 40 for a population of 20 | `"FEAS"`, which returned an equal array with no warning |

Two documents read the key. One needs `res.pop`, `res.opt` or `res.off`, where `res` is the
result of `minimize`, and it has the fix. The other reads the name of the object, a variable or
an attribute named `pop`, `opt`, `off` or `population`, and it has no fix. It gives a wrong
report on `get` of a dictionary with such a name.

Example:

```python
from pymoo.core.problem import StarmapParallelization

feasible = res.pop.get("feasible")
```

Use instead:

```python
from pymoo.parallelization import StarmapParallelization

feasible = res.pop.get("FEAS")
```

#### n-constr-keyword

**Severity:** info. **Fix:** safe: `n_ieq_constr=`.

The rule reports `n_constr=` given to `super().__init__` in a problem class, or to `Problem` and `ElementwiseProblem` called by name. `Problem` has no such parameter since pymoo 0.6.0 and reads it from `**kwargs` as `n_ieq_constr`. A problem built with either keyword had the same `n_ieq_constr`, and `GA` with `seed=1` returned the same `X`, `F` and `G` for both, bit for bit. A call that passes both keywords is left alone.

Example:

```python
class Welded(Problem):
    def __init__(self):
        super().__init__(n_var=4, n_obj=2, n_constr=4, xl=xl, xu=xu)
```

Use instead:

```python
class Welded(Problem):
    def __init__(self):
        super().__init__(n_var=4, n_obj=2, n_ieq_constr=4, xl=xl, xu=xu)
```

#### positional-problem-bounds

**Severity:** error. **Fix:** unsafe: `xl=` and `xu=` on the bounds given by position, as in `Problem(2, 1, 0, xl=np.zeros(2), xu=np.ones(2))`. The problem gets other bounds than before, and the fix drops comments inside the parentheses.

The rule reports `xl` and `xu` given by position to `super().__init__` in a problem class, or to `Problem` and `ElementwiseProblem` called by name. pymoo 0.5 had `Problem(n_var, n_obj, n_constr, xl, xu)`. Since pymoo 0.6.0 the signature is `(n_var, n_obj, n_ieq_constr, n_eq_constr, xl, xu)`, so the fourth argument is read as the number of equality constraints and the fifth as `xl`.

Example:

```python
class Sphere(Problem):
    def __init__(self):
        super().__init__(2, 1, 0, np.zeros(2), np.ones(2))
```

Use instead:

```python
class Sphere(Problem):
    def __init__(self):
        super().__init__(2, 1, 0, xl=np.zeros(2), xu=np.ones(2))
```

The first class was built with no error on 0.6.0 and later, and `minimize` failed afterwards.

| Arguments of `super().__init__` | pymoo | What happened |
| --- | --- | --- |
| `(2, 1, 0, np.zeros(2), np.ones(2))` | 0.6.0, 0.6.2 | The problem had `n_eq_constr` [0, 0], `xl` [1, 1] and `xu` None, and `minimize` with `GA` raised TypeError, "only integer scalar arrays can be converted to a scalar index". |
| `(2, 1, 0, np.zeros(2), xu=np.ones(2))` | 0.6.0, 0.6.2 | The problem had `xl` None. |
| either of the two above | 0.5.0 | The run had `xl` [0, 0] and `xu` [1, 1]. |
| `(2, 1, 0, xl=np.zeros(2), xu=np.ones(2))` | 0.5.0, 0.6.0, 0.6.2 | The run worked. |

The rule reads a call with exactly five positional arguments and no `xl` or `xu` keyword, and a
call with exactly four and `xu` as a keyword. A whole number written as the fourth argument is a
count, as in `Problem(2, 1, 0, 0, xl)`, and is left alone.

#### ignored-problem-keyword

**Severity:** error for `elementwise_evaluation` and `parallelization`, warning for `type_var`. **Fix:** unsafe for `type_var`: `vtype=` with the same value, which operators and helpers that read `vtype` can act on. None for `elementwise_evaluation` and `parallelization`, and none for `type_var` in a call that passes `vtype` too.

The rule reports a keyword of the pymoo 0.4 `Problem` that pymoo 0.6 stores in `problem.data` and never
reads.

| Keyword | What happened on pymoo 0.6 |
| --- | --- |
| `elementwise_evaluation=True` | `_evaluate` gets the whole population where it expects one solution. A method of `x[0]`, `x[1]` and `x[2]` returned [0.66, 0.93, 1.26] for three solutions whose values are [0.14, 0.77, 1.94]. |
| `parallelization=("starmap", pool.starmap)` | The starmap was called 0 times in 3 generations, where pymoo 0.4.2.2 called it 3 times. |
| `type_var=int` | The problem had `vtype` None. |

Example:

```python
class Sphere(Problem):
    def __init__(self, pool):
        super().__init__(n_var=3, n_obj=1, xl=0.0, xu=1.0, elementwise_evaluation=True, parallelization=("starmap", pool.starmap))

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = x[0] ** 2 + x[1] ** 2 + x[2] ** 2
```

Use instead:

```python
class Sphere(ElementwiseProblem):
    def __init__(self, pool):
        super().__init__(n_var=3, n_obj=1, xl=0.0, xu=1.0, elementwise_runner=StarmapParallelization(pool.starmap))

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = x[0] ** 2 + x[1] ** 2 + x[2] ** 2
```

#### setup-positional-termination

**Severity:** error. **Fix:** unsafe: `termination=` before the second argument.

The rule reports a termination given to `setup` as the second positional argument, on a name the file binds to a pymoo algorithm.

| pymoo | `setup(problem, ("n_gen", 7))` |
| --- | --- |
| 0.5.0 and older | the second parameter is the termination |
| 0.6.0 to 0.6.1.5 | TypeError |
| 0.6.1.6 and later | the second parameter is `verbose`, and `NSGA2` printed a line for every generation and was still running after 30 |

Example:

```python
algorithm = NSGA2(pop_size=100)
algorithm.setup(problem, ("n_gen", 200), seed=1)
```

Use instead:

```python
algorithm = NSGA2(pop_size=100)
algorithm.setup(problem, termination=("n_gen", 200), seed=1)
```

#### has-terminated-assignment

**Severity:** error. **Fix:** none.

The rule reports `has_terminated = False` on a name that the file assigns from `np.load`, `pickle.load` or `dill.load`, also as one name of a tuple. Another object with an attribute of the same name is left alone. The checkpoint recipe of pymoo 0.4 and 0.5 set the attribute to let a loaded algorithm run on, and since 0.6.0 pymoo no longer reads it. An `NSGA2` that had finished its 5 generations ran 0 more generations after `has_terminated = False`, and 15 more after it got a new termination.

Example:

```python
(checkpoint,) = np.load("checkpoint.npy", allow_pickle=True).flatten()
checkpoint.has_terminated = False
res = minimize(problem, checkpoint, ("n_gen", 20), seed=1, copy_algorithm=False)
```

Use instead:

```python
(checkpoint,) = np.load("checkpoint.npy", allow_pickle=True).flatten()
checkpoint.termination = MaximumGenerationTermination(20)
res = minimize(problem, checkpoint, copy_algorithm=False)
```

#### mutation-prob-per-individual

**Severity:** warning. **Fix:** unsafe: `prob_var=`.

The rule reports `prob=` on `PM`, `PolynomialMutation`, `BitflipMutation`, `BFM`, `GaussianMutation` and `GM` when the value is 1 divided by something, or a number of at most 0.2, and the call has no `prob_var`. Up to pymoo 0.5.0 `prob` was the chance that one variable changes. Since 0.6.0 it is the share of individuals that mutate, and `prob_var` is the chance per variable.

| 20,000 individuals of 30 variables | pymoo 0.5.0 | pymoo 0.6.2 |
| --- | --- | --- |
| individuals changed by `prob=1/30` | 63 percent | 2 percent |
| individuals changed by `prob_var=1/30` | not run | 57 percent |

Example:

```python
mutation = PM(prob=1 / problem.n_var, eta=20)
```

Use instead:

```python
mutation = PM(prob_var=1 / problem.n_var, eta=20)
```

#### indicator-normalize-ignored

**Severity:** error. **Fix:** unsafe: `zero_to_one=`.

The rule reports `normalize=True` on `GD`, `GDPlus`, `IGD` and `IGDPlus`. The keyword is from pymoo 0.4, and the constructors still accept it and do nothing with it.

| A front whose first objective runs to 1000 | pymoo 0.4.2.2 | pymoo 0.6.2 |
| --- | --- | --- |
| `IGD(pf, normalize=True)` | 0.082 | 25.25 |
| `IGD(pf)` | not run | 25.25 |
| `IGD(pf, zero_to_one=True)` | not run | 0.082 |

Example:

```python
igd = IGD(pf, normalize=True)(res.F)
```

Use instead:

```python
igd = IGD(pf, zero_to_one=True)(res.F)
```

### The problem class

The rules in the group report an `_evaluate` that hands pymoo wrong numbers or raises. The two
problem classes get `x` in different shapes, and both write their results to the dictionary
`out`.

| Class | What `_evaluate` gets as `x` | The first variable |
| --- | --- | --- |
| `Problem` | the whole population, as an array with one row per solution | `x[:, 0]` |
| `ElementwiseProblem` | one solution, as an array with one value per variable | `x[0]` |

#### objectives-stacked-by-row

**Severity:** error. **Fix:** unsafe: `np.column_stack` on the same list.

The rule reports `out["F"]`, `out["G"]` or `out["H"]` set to `np.array`, `np.asarray`, `np.vstack`, `np.stack` or `np.row_stack` of a list, in a class that inherits from `Problem`. Such an array has one row per objective, and pymoo reshapes it to one row per solution without swapping its rows and columns. Two objectives over four solutions, `f1 = x0` and `f2 = 10 + x1`, came back as [[0.1, 0.3], [0.5, 0.7], [10.2, 10.4], [10.6, 10.8]], and `NSGA2` then reported a least `f2` of 0.003 where the true least is 10.

The rule leaves alone an `ElementwiseProblem`, where a list of numbers is right, and a stack followed by `.T`. It also leaves alone a class that passes `elementwise=` with any value but `False`, such as `elementwise=elementwise`, or passes `**kwargs` on to `__init__`, because the caller can then build it with `elementwise=True`.

Example:

```python
class TwoObjectives(Problem):
    def _evaluate(self, x, out, *args, **kwargs):
        f1 = x[:, 0]
        f2 = 10 + x[:, 1]
        out["F"] = np.array([f1, f2])
```

Use instead:

```python
class TwoObjectives(Problem):
    def _evaluate(self, x, out, *args, **kwargs):
        f1 = x[:, 0]
        f2 = 10 + x[:, 1]
        out["F"] = np.column_stack([f1, f2])
```

#### evaluate-returns-value

**Severity:** error. **Fix:** unsafe: `out["F"] = <value>` for the `return`. pymoo then reads the value where it read inf, the method goes on past that line, and a value of another shape raises.

The rule reports a `return` with a value in `_evaluate` when the method never names its `out` parameter. pymoo ignores the returned value, and every objective then comes back as inf. `GA` returned `res.F = [inf]` without an error. A method that hands `out` to a helper is left alone.

Example:

```python
class Sphere(Problem):
    def _evaluate(self, x, out, *args, **kwargs):
        return (x**2).sum(axis=1)
```

Use instead:

```python
class Sphere(Problem):
    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = (x**2).sum(axis=1)
```

#### constraint-not-declared

**Severity:** error. **Fix:** none.

The rule reports `out["G"]` written in a class that never passes `n_ieq_constr`, and `out["H"]` in a class that never passes `n_eq_constr`. Both counts default to 0, and the first evaluation raises Exception, "Problem Error: G can not be set, expected shape (2, 0) but provided (2,)". pymoo 0.5.0 ignored such a constraint without an error. The class must call `super().__init__` with keywords only, so a class that passes `**kwargs` on is left alone.

Example:

```python
class Bounded(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = (x**2).sum(axis=1)
        out["G"] = 0.5 - x[:, 0]
```

Use instead:

```python
class Bounded(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, n_ieq_constr=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = (x**2).sum(axis=1)
        out["G"] = 0.5 - x[:, 0]
```

#### evaluate-indexing

**Severity:** error. **Fix:** none.

The rule reports an index that takes the wrong axis of `x` for the class.

| Class | Code | What happened |
| --- | --- | --- |
| `ElementwiseProblem` | `x[:, 0]` on the parameter of `_evaluate` | IndexError, "too many indices for array" |
| `Problem` | `x[0]`, when `_evaluate` uses the parameter only with whole numbers as subscripts and has no loop | `x[0]` takes the first solution and not the first variable. With 100 solutions of 2 variables it raised Exception, "Problem Error: F can not be set, expected shape (100, 1) but provided (2,)". |

The rule leaves alone a `Problem` class that passes `elementwise=` with any value but `False`, or passes `**kwargs` on to `__init__`, because it can be built with `elementwise=True` and then gets one solution.

Example:

```python
class Sphere(Problem):
    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = x[0] ** 2 + x[1] ** 2
```

Use instead:

```python
class Sphere(ElementwiseProblem):
    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = x[0] ** 2 + x[1] ** 2
```

#### elementwise-runner

**Severity:** error. **Fix:** none.

The rule reports an `elementwise_runner=` that pymoo cannot call or never calls.

| Code | What happened |
| --- | --- |
| `elementwise_runner=pool.starmap` | TypeError, "ElementwiseEvaluationFunction.__call__() takes 2 positional arguments but 3 were given", because a runner is called as `runner(f, X)` |
| `elementwise_runner=` given to a class that the file defines with `Problem` as its base | A `Problem` evaluates the population in one call, so the runner was called 0 times in 3 generations, where an `ElementwiseProblem` called it 3 times. |

A call that also passes `elementwise=` or `**kwargs` is left alone. `StarmapParallelization` is in `pymoo.core.problem` on 0.6.0 to 0.6.1.5 and in `pymoo.parallelization` from 0.6.1.6 on, and the message names the module of the release in the venv.

Example:

```python
problem = MyElementwiseProblem(elementwise_runner=pool.starmap)
```

Use instead:

```python
problem = MyElementwiseProblem(elementwise_runner=StarmapParallelization(pool.starmap))
```

### Running an algorithm

The rules in the group report `minimize` keywords that pymoo never reads, and an algorithm that
is read or run again after `minimize` copied it.

#### minimize-ignored-keyword

**Severity:** error. **Fix:** unsafe: `termination=("n_gen", 7)` for `n_gen=7`, with `n_gen` for `n_max_gen`, `max_gen` and `generations`, so the run stops where the keyword said. `seed=` for `random_state=` and `random_seed=`. A call that passes a termination, `**` or two stopping keywords has no fix.

The rule reports a keyword that `minimize` hands to the algorithm, which stores it and reads it nowhere.

| Keyword | What happened |
| --- | --- |
| `n_gen`, `n_eval`, `n_evals`, `n_iter`, `n_max_gen`, `max_gen`, `generations` | With each of the seven, `NSGA2` ran 1,000 generations and 20,000 evaluations, where the default termination stops. |
| `random_state`, `random_seed` | Two runs with either set to 1 returned different fronts, and two runs with `seed=1` returned the same front. |

Example:

```python
res = minimize(problem, NSGA2(pop_size=100), n_gen=200, random_state=1)
```

Use instead:

```python
res = minimize(problem, NSGA2(pop_size=100), termination=("n_gen", 200), seed=1)
```

#### algorithm-read-after-minimize

**Severity:** error. **Fix:** unsafe: `res.algorithm.pop` for `algorithm.pop`, with the name the code gave the result. The read then gets the algorithm that ran, and a name bound again between the run and the read gives another object.

The rule reports `pop`, `opt`, `off`, `n_gen`, `n_iter`, `history`, `callback` and `result` read from an algorithm after it was given to `minimize`. `minimize` runs a deep copy and returns it as `res.algorithm`.

| After a run of 10 generations | Value |
| --- | --- |
| `algorithm.pop`, `algorithm.opt`, `algorithm.n_gen` | None |
| `algorithm.result()` | TypeError |
| records of a callback given to the constructor | 0 |
| records of the callback in `res.algorithm` | 10 |

The rule leaves alone a call with `copy_algorithm`, and a name that is assigned again after the run or bound by a walrus, as in `(algorithm := res.algorithm)`. It also leaves alone a file that drives the same name with `setup` or `next`.

Example:

```python
algorithm = NSGA2(pop_size=100, callback=Best())
res = minimize(problem, algorithm, ("n_gen", 200), seed=1)
best = algorithm.callback.data["best"]
```

Use instead:

```python
algorithm = NSGA2(pop_size=100, callback=Best())
res = minimize(problem, algorithm, ("n_gen", 200), seed=1)
best = res.algorithm.callback.data["best"]
```

#### reused-algorithm-without-copy

**Severity:** error. **Fix:** none.

The rule reports a second `minimize` on the same algorithm when both calls pass `copy_algorithm=False`, in one block or in a loop that never binds the name. With `copy_algorithm=False`, `minimize` sets the algorithm up only when it has no problem yet. A second call with another problem, `("n_gen", 30)` and another seed returned the `F` of the first call, still at generation 11 and still on the first problem. A loop over the algorithms, as in `for algorithm in [NSGA2(), SMSEMOA()]`, gives each pass another object and is left alone.

Example:

```python
algorithm = NSGA2(pop_size=100)
for problem in problems:
    results.append(minimize(problem, algorithm, ("n_gen", 200), seed=1, copy_algorithm=False))
```

Use instead:

```python
for problem in problems:
    algorithm = NSGA2(pop_size=100)
    results.append(minimize(problem, algorithm, ("n_gen", 200), seed=1, copy_algorithm=False))
```

#### unread-history

**Severity:** info. **Fix:** unsafe: drops `save_history=True`, so `res.history` is empty.

The rule reports `save_history=True` given to `minimize` outside every function, in a file that never names `history` and does not use pickle, dill or joblib. The option keeps a deep copy of the algorithm for every generation. When another file reads the history of the run, the report is wrong.

| `NSGA2`, 100 solutions, 200 generations | Time | Pickled result | `F` |
| --- | --- | --- | --- |
| `save_history=True` | 1.17 seconds | 20.3 MB | the same |
| without it | 0.45 seconds | 0.13 MB | the same |

Example:

```python
res = minimize(problem, algorithm, ("n_gen", 200), seed=1, save_history=True)
print(res.F)
```

Use instead:

```python
res = minimize(problem, algorithm, ("n_gen", 200), seed=1)
print(res.F)
```

#### moead-arguments

**Severity:** error. **Fix:** unsafe: drops `pop_size`, and writes `PBI()`, `Tchebicheff()`, `WeightedSum()`, `ASF()` or `PerpendicularDistance()` for the texts `"pbi"`, `"tchebi"`, `"weighted-sum"`, `"asf"` and `"perp_dist"`, in a file that imports that class from its module in `pymoo.decomposition`. The old call raised. `"auto"`, `"aasf"` and other texts have no fix.

The rule reports two `MOEAD` arguments that raise.

| Argument | What happens |
| --- | --- |
| `pop_size=` | TypeError, "got multiple values for keyword argument 'pop_size'", because `MOEAD` sets the population to the number of reference directions itself since pymoo 0.5.0 |
| `decomposition=` given as a string such as `"pbi"`, the form of pymoo 0.4 | AttributeError in the first generation, "'str' object has no attribute 'do'" |

Example:

```python
algorithm = MOEAD(ref_dirs, pop_size=100, decomposition="pbi")
```

Use instead:

```python
algorithm = MOEAD(ref_dirs, decomposition=PBI())
```

### Random numbers

The rules in the group report runs that give another result each time. Since pymoo 0.6.1.6 the
library draws from a generator that `minimize` builds from `seed`, and `np.random.seed` does not
reach it.

#### unseeded-run

**Severity:** info. **Fix:** none.

The rule reports `minimize` without `seed`. Two runs of `NSGA2` on ZDT1 then returned different fronts, and two runs with `seed=1` returned the same front. The rule leaves alone a call that takes `**kwargs`, a call whose algorithm was built with `seed`, and a file that calls `np.random.seed`, which `numpy-seed-ignored` reports.

Example:

```python
res = minimize(problem, NSGA2(pop_size=100), ("n_gen", 200))
```

Use instead:

```python
res = minimize(problem, NSGA2(pop_size=100), ("n_gen", 200), seed=1)
```

#### numpy-seed-ignored

**Severity:** warning. **Fix:** none.

The rule reports `np.random.seed` in a file whose `minimize` gets no `seed`, and a draw from `np.random` inside `_do` of a class that inherits from `Sampling`, `Crossover` or `Mutation`. Since pymoo 0.6.1.6 the library draws from a generator that `minimize` builds from `seed` and passes to each operator as `random_state`.

On pymoo 0.6.1.5 two runs after `np.random.seed(1)` returned the same front, and on 0.6.1.6 and 0.6.2 they returned different fronts. A custom operator gave the results in the table in two runs with `seed=1`.

| Operator | Draws from `np.random` | Draws from `random_state` |
| --- | --- | --- |
| a sampling | different results | the same result twice |
| a crossover | different results | the same result twice |
| a mutation | different results | the same result twice |

Example:

```python
class MySampling(Sampling):
    def _do(self, problem, n_samples, **kwargs):
        return np.random.random((n_samples, problem.n_var))

np.random.seed(1)
res = minimize(problem, NSGA2(pop_size=100, sampling=MySampling()), ("n_gen", 200))
```

Use instead:

```python
class MySampling(Sampling):
    def _do(self, problem, n_samples, random_state=None, **kwargs):
        return random_state.random((n_samples, problem.n_var))

res = minimize(problem, NSGA2(pop_size=100, sampling=MySampling()), ("n_gen", 200), seed=1)
```

### Indicators and decisions

The rules in the group report a reference point or weights that give another number than the
code means.

#### hv-reference-point

**Severity:** warning for a reference point taken from the solutions, error for none. **Fix:** none.

The rule reports `ref_point=F.max(axis=0)` and `ref_point=np.max(F, axis=0)` on `HV` and `Hypervolume`, and `HV()` with no argument, which raises AssertionError, "For Hypervolume a reference point needs to be provided!". A reference point at the largest value of each objective lies on the solutions that hold the largest values, so they add no volume.

| Reference point | Hypervolume of three solutions | Of the two extreme solutions alone |
| --- | --- | --- |
| `F.max(axis=0)` | 0.09 | 0.0 |
| the fixed point (1, 1) | 0.37 | not run |

Example:

```python
hv = HV(ref_point=res.F.max(axis=0))(res.F)
```

Use instead:

```python
hv = HV(ref_point=np.array([1.1, 1.1]))(res.F)
```

#### decision-weights

**Severity:** warning. **Fix:** unsafe for ASF: `1 / weights`, which picks another solution, the one the code means when a larger weight stands for a more important objective. None for PseudoWeights.

The rule reports weights that the decision method reads in another way than the code means.

| Code | How pymoo reads the weights | What it picked |
| --- | --- | --- |
| `ASF().do(F, np.array([0.2, 0.8]))`, with weights written out and not all the same | `ASF` divides each objective by its weight, so the larger weight counts less. The pymoo guide passes `1 / weights`. | the solution (0.1, 0.9), where `1 / weights` picked (0.9, 0.1) |
| a weight above 1 written out in the argument of `PseudoWeights`, such as (2, 8) | `PseudoWeights` compares the weights with shares that sum to 1. | row 0 of five solutions, where (0.2, 0.8) picked row 3 |

Example:

```python
i = ASF().do(F, np.array([0.2, 0.8])).argmin()
j = PseudoWeights(np.array([2, 8])).do(F)
```

Use instead:

```python
i = ASF().do(F, 1 / np.array([0.2, 0.8])).argmin()
j = PseudoWeights(np.array([0.2, 0.8])).do(F)
```

## plotly

The plotly rules report figures whose drawn sizes do not follow the numbers, ink that shows no
data, colors without an order, and chart forms and words that make a figure harder to read. They
apply Edward Tufte's book "The Visual Display of Quantitative Information", second edition, to
plotly figures. Each rule names the principle of the book that the figure breaks and what the
reader then sees, and it also names the plotly code to write instead. A page number in the plotly section is a
printed page of the book.

| Group | What its rules report |
| --- | --- |
| Sizes that do not follow the numbers | marker areas, bar lengths, wedges, axes and scales that draw a number larger or smaller than it is |
| Ink that shows no data | tinted templates, grids, frames, boxes, repeated colors, hatching and gauges |
| Color without an order | rainbow color scales |
| Chart forms the book argues against | pies, sunbursts, splines, animations, smoothed heatmaps and charts of a few numbers |
| Words on the figure | titles in capitals, legends in place of labels, rotated tick labels and column names as axis titles |

The rules run on files that import `plotly`, and on files that make plotly the plotting backend
of pandas. Every figure measurement in the plotly section used plotly 7.1.0 with pandas 3.1.1 on
Python 3.14. plotly.js resolved each figure through `full_figure_for_development`, which fills
in every default. Where a rule is about a drawn size, the figure was rendered to SVG and the
size was read from the drawn path. The examples assume `import plotly.express as px`,
`import plotly.graph_objects as go` and `import plotly.io as pio`, and names such as `df` and
`fig` stand for values from your program.

A rule reports a call only when the file binds the name in front of the call to a plotly module.

| Code | Proof that the name is plotly |
| --- | --- |
| `import plotly.express as px` or `from plotly import graph_objects as go`, under any alias | yes |
| the full dotted name, as in `plotly.express.pie(...)` | yes |
| a name `px` or `go` that the file binds to something else | no |

A file that makes plotly the plotting backend of pandas need not import plotly. xpd then reads
`df.plot.bar(...)` as `px.bar(df, ...)`, because on pandas 3.1.1 with plotly 7.1.0,
`df.plot.bar(x="team", y="score")` was equal, as JSON, to `px.bar(df, x="team", y="score")`.

```text
on(a file that holds pd.options.plotting.backend = "plotly" or pd.set_option("plotting.backend", "plotly"))
  if the file names no other backend, in an assignment, in set_option or as backend= of a call
    read df.plot.line, bar, area, scatter and box as the px function of the same name
      only when every argument of the call is named   # px.bar takes the frame first, df.plot.bar does not
not read
  df.plot(kind="bar")                     # plot-kind-argument of the pandas pack rewrites it to df.plot.bar
  a file where another module sets the backend
```

[`plot-kind-argument`](#plot-kind-argument) is the pandas rule that rewrites `df.plot(kind=...)`.

A rule about a method, such as `fig.update_xaxes(...)`, needs proof that the object is a plotly
figure. The proof holds when the method is called on the call that makes the figure, as in
`px.bar(...).update_xaxes(...)`, or on a variable that holds one. xpd follows the variable to
its value in three ways:

- The nearest earlier statement that binds the variable is an assignment from such a call,
  however many statements lie between the two. The statement is in the same block as the
  method call or in a block around it, up to the start of the function. A variable assigned
  from another variable, as in `chart = fig`, holds what the other variable held.
- The variable is a parameter with the annotation `go.Figure` or `go.FigureWidget`, or an
  assignment gives it that annotation, as in `fig: go.Figure = load()`.
- The variable is a parameter without that annotation, and every call of its function in the
  file passes a plotly figure. The function must be called at least once, by its name alone.
  A method, a function with a decorator, a function that is passed on as a value and a call
  with `*args` or `**kwargs` give no proof, because the file does not show every argument.

Any other binding of the name on the way ends the proof. Such a binding is a second assignment
that is not a figure, a loop variable, a `with ... as`, an import, a `def` or a `del` of the same
name, or an assignment inside an `if` or a loop that stands before the method call or around it.
A function that is defined in one module and called from another is not followed, so its
parameter needs the annotation.

A figure made by `go.Figure()` and filled later with `fig.add_trace(...)` proves a plotly figure,
and the statements after the assignment prove what it holds. For example, an `add_trace` of a
`go.Bar` makes it a bar chart for [`bar-axis-without-zero`](#bar-axis-without-zero), when no
second assignment of the name comes between the two. The annotation `go.Figure` alone does not
prove a bar chart.

A rule reads a value only where it is written in the call, as a number, a string, `True` or
`None`. A name in the same place may hold any value, so the rule is silent.

No document of the pack carries `since`, because the names that the messages tell you to write
exist on plotly 5.0.0, 5.24.1, 6.0.0 and 7.1.0, where each was run.

### The proof each rule needs

Every rule has a measurement that was run, and its message quotes what was measured. A
measurement is one of two kinds:

- A resolved value is what plotly.js made of the figure, read from
  `full_figure_for_development`, such as `sizemode="diameter"` for a `go.Scatter` that sets no
  size mode.
- A drawn size is read from the SVG that plotly renders, such as the radius of a marker, the
  visible length of a bar, or the highest point of a curve.

| Severity | When |
| --- | --- |
| `error` | the figure is wrong for every reader |
| `warning` | the drawn sizes do not follow the numbers, or the book rejects the form outright |
| `info` | the form is a choice that the book argues against |

The rules were run over a corpus of 1,087 Python files, read as text and never imported or run.
The rule table after the next two paragraphs gives the reports that the current build makes on
the corpus.

| Part of the corpus | Files | What the files are |
| --- | --- | --- |
| docs | 151 | the `python` code blocks of each page of plotly's documentation |
| Dash | 117 | Python files of Dash that name plotly |
| public | 819 | public files that GitHub code search found for the import lines of plotly express, graph objects and `make_subplots` |

Recall, the share of planted mistakes that a rule finds, was measured on forms planted in copies
of 47 of the corpus files. The 1,212 planted forms are 48 in six groups for each rule and 60 in
eight groups for `free-facet-scales`. The groups hold settings written in the call, later updates, dictionaries, objects, and
values held in a name where the form allows one. A planted form counts as found when its rule
reports the lines of the form, and a report that was already in the file does not count. Every
copy passed `ast.parse`, and none was run.

The 47 files are not an unseen set, because they were seen while the rules were written.
Some of their reports had been read before. A planted form counts only
when the data has the values that the form assumes, such as a numeric column. Forms that no rule
reads stay in the count, such as a box given as a `**` dictionary, a bad axis title written out,
and a flat `go.Surface`. An `info` report is a design choice and does not prove that the figure
is wrong.

| Rule | Severity | Page | Measurement | Reports: docs, Dash, public | Total reports | Files with report | Recall | Fix |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [`marker-size-diameter`](#marker-size-diameter) | error | 57 | drawn: areas 1, 4, 16 for sizes 10, 20, 40 | 1, 0, 6 | 7 | 6 | 39 of 48 | none |
| [`bar-axis-without-zero`](#bar-axis-without-zero) | warning | 57 | drawn: bars of 95 and 100 at 163.6 and 327.3 pixels, 145.4 and 290.9 in a `go.Figure` | 0, 0, 0 | 0 | 0 | 37 of 48 | none |
| [`area-axis-without-zero`](#area-axis-without-zero) | warning | 57 | drawn: fills of 95 and 100 at 163.6 and 327.3 pixels | 0, 0, 0 | 0 | 0 | 37 of 48 | none |
| [`polar-bar-area`](#polar-bar-area) | warning | 57 | drawn: radii 1, 2, 3, 4 for values 1, 2, 3, 4 | 10, 4, 1 | 15 | 8 | 48 of 48 | none |
| [`three-d-for-two-d`](#three-d-for-two-d) | error, warning | 71 | resolved: `visible=False`. Resolved: z axis from -1 to 1 | 0, 0, 0 | 0 | 0 | 32 of 48 | none |
| [`dual-y-axis`](#dual-y-axis) | info | 61 | resolved: lines cross at 1.67 or at 1.81 | 15, 5, 41 | 61 | 36 | 32 of 48 | none |
| [`free-facet-scales`](#free-facet-scales) | info | 170 | resolved: one value at 0.70 and at 0.45 of the panel. Drawn: bars of 5 and 10 equally tall | 2, 0, 1 | 3 | 2 | 40 of 60 | none |
| [`default-template`](#default-template) | info | 96 | resolved: `#E5ECF6`. Drawn: 10 grid lines | 102, 18, 231 | 351 | 351 | 32 of 48 | unsafe |
| [`explicit-grid`](#explicit-grid) | info | 112 | drawn: 0 grid lines became 10 | 9, 4, 28 | 41 | 19 | 40 of 48 | unsafe |
| [`boxed-axis-frame`](#boxed-axis-frame) | info | 103 | drawn: 1 axis line segment became 2 | 7, 1, 18 | 26 | 7 | 40 of 48 | none |
| [`box-plot-ink`](#box-plot-ink) | info | 124 | drawn: 36 box segments and 16 offset quartile segments for four groups | 5, 0, 14 | 19 | 9 | 40 of 48 | none |
| [`redundant-color`](#redundant-color) | info | 100 | drawn: 4 legend entries equal to the 4 tick labels. Drawn: 3 shades for 3 bar lengths | 16, 0, 70 | 86 | 55 | 40 of 48 | none |
| [`pattern-fill`](#pattern-fill) | warning | 108 | drawn: 2 pattern images where there were 0 | 19, 0, 1 | 20 | 11 | 40 of 48 | none |
| [`gauge-indicator`](#gauge-indicator) | info | 168 | drawn: 8 shapes and 7 labels for one number | 4, 2, 30 | 36 | 25 | 40 of 48 | none |
| [`rainbow-colorscale`](#rainbow-colorscale) | warning | 154 | resolved: Jet lightness 13.5, 97.1, 25.5 along the scale | 1, 0, 7 | 8 | 6 | 40 of 48 | none |
| [`pie-chart`](#pie-chart) | warning | 178 | resolved: trace type `pie`, 2 traces with a facet or in 2 cells | 28, 7, 120 | 155 | 84 | 48 of 48 | none |
| [`sunburst-as-pie`](#sunburst-as-pie) | warning | 178 | drawn: a wedge of 45 with 2.25 times the area of one of 60 | 14, 2, 11 | 27 | 15 | 48 of 48 | none |
| [`spline-line`](#spline-line) | info | 92 | drawn: 10.55 where the data ends at 10 | 4, 0, 7 | 11 | 8 | 40 of 48 | unsafe |
| [`animation-frame`](#animation-frame) | info | 170 | resolved: 142 of 1,704 rows at once. Drawn: 1 of 3 frames | 11, 1, 27 | 39 | 22 | 40 of 48 | none |
| [`heatmap-smoothing`](#heatmap-smoothing) | info | 92 | drawn: 252 colors for a grid of two values | 0, 0, 0 | 0 | 0 | 40 of 48 | unsafe |
| [`table-for-small-data`](#table-for-small-data) | info | 56 | drawn: 0 of 4 values written, 4 of 4 in `go.Table` | 31, 0, 1 | 32 | 12 | 29 of 48 | none |
| [`legend-instead-of-labels`](#legend-instead-of-labels) | info | 180 | drawn: 2 legend entries became 0 with labels | 7, 3, 71 | 81 | 59 | 21 of 48 | none |
| [`rotated-tick-labels`](#rotated-tick-labels) | info | 183 | drawn: 2 of 2 category labels rotated | 4, 4, 71 | 79 | 51 | 40 of 48 | none |
| [`all-caps-title`](#all-caps-title) | info | 183 | drawn: a title 1.31 times as wide in capitals | 3, 0, 1 | 4 | 2 | 40 of 48 | none |
| [`axis-title-column-name`](#axis-title-column-name) | info | 183 | resolved: axis title `gdpPercap` | 68, 1, 104 | 173 | 71 | 15 of 48 | none |

The corpus counts say how often a rule reports, and not its recall or whether the advice suits
a given chart. `area-axis-without-zero`, `bar-axis-without-zero`, `heatmap-smoothing` and
`three-d-for-two-d` made no report on the corpus.

Every rule but `box-plot-ink`, `pie-chart`, `polar-bar-area` and `sunburst-as-pie` missed the 8
planted forms that hold the value in a name. The table gives every other planted form that a
rule missed.

| Rule | Other missed forms |
| --- | --- |
| `marker-size-diameter` | a size in a `go.Scatter` marker dictionary (1) |
| `bar-axis-without-zero` | `update_layout(yaxis_range=...)` (2), an axis dictionary on an added `go.Bar` (1) |
| `area-axis-without-zero` | `update_layout(yaxis_range=...)` (2), a filled `go.Scatter` (1) |
| `three-d-for-two-d` | a flat `go.Surface` (8) |
| `dual-y-axis` | `update_yaxes(overlaying=...)` (8) |
| `free-facet-scales` | `update_layout(yaxis_matches=...)` (8), a loop that adds a `go.Scatter` to subplots (3), a loop that adds a `go.Bar` to subplots (1) |
| `default-template` | a template set with `Figure.update_layout` (8) |
| `box-plot-ink` | a box given as a `**` dictionary and a trace name (8). It found the 8 forms that hold the values in a name. |
| `table-for-small-data` | written values in `px.pie` (3), a bare `go.Bar` with `y` (3), written values in `px.bar` (2), a `go.Figure` with a `go.Pie` list (2), a bare horizontal `go.Bar` with `x` (1) |
| `legend-instead-of-labels` | `px.line` with `color` (5), `px.line` with a list of columns (5), `px.line` with `color` and markers (5), named `go.Scatter` traces added one by one (2), a `go.Figure` with a list of named `go.Scatter` traces (2) |
| `axis-title-column-name` | an axis title written out (8), a column in `px.scatter` (5), `px.bar` (5), `px.area` (4) and `px.line` (3) |

No fix of the pack is safe, because every fix changes what the reader sees. `--fix` alone writes
nothing for the pack, and `--fix --unsafe-fixes` writes four rewrites:

```diff
-template="ggplot2"          # or "plotly", "seaborn"
+template="simple_white"
-showgrid=True
+showgrid=False
-line_shape="spline"
+line_shape="linear"
-zsmooth="best"              # or "fast"
+zsmooth=False
```

### What the pack does not check

The book states principles that the code of a figure does not show, so no rule covers them.

| Principle | Page | What the code does not show |
| --- | --- | --- |
| "In time-series displays of money, deflated and standardized units of monetary measurement are nearly always better than nominal units" | 68 | whether a column is money |
| "Graphics must not quote data out of context" | 74 | the rows that were left out |
| The range-frame and the dot-dash-plot | 130, 133 | plotly has no axis line that ends at the lowest and highest value, and xpd cannot ask for `marginal_x="rug"` from a missing argument |
| The Shrink Principle | 169 | how many values the figure shows, which sets its right size |

### Sizes that do not follow the numbers

The rules in the group report a figure that draws a number larger or smaller than it is. The
first principle of graphical integrity in the book is "The representation of numbers, as
physically measured on the surface of the graphic itself, should be directly proportional to
the numerical quantities represented" (page 56). The book measures a break of the principle
with the Lie Factor, and calls a factor above 1.05 or below 0.95 a substantial distortion
(page 57).

```text
Lie Factor = size of the effect shown in the graphic / size of the effect in the data
```

#### marker-size-diameter

**Severity:** error. **Fix:** unsafe for `sizemode="diameter"` on a px figure: drops the argument, so the markers get the area that px set. None for a go trace, whose `sizeref` depends on the data.

The rule reports a go trace whose marker size is read from the data and whose size mode is left at the default or written as `"diameter"`. It also reports `sizemode="diameter"` given to `update_traces` in a file that calls a px function with `size=`. The first form covers `marker=dict(size=...)`, `marker={"size": ...}`, `marker=go.scatter.Marker(size=...)` and `marker_size=...` on the 11 trace types that have `marker.sizemode`: `go.Scatter`, `go.Scatter3d`, `go.Scattercarpet`, `go.Scattergeo`, `go.Scattergl`, `go.Scattermap`, `go.Scatterpolar`, `go.Scatterpolargl`, `go.Scattersmith`, `go.Scatterternary` and `go.Splom`.

Example:

```python
fig = go.Figure(go.Scatter(x=df["area"], y=df["rain"], mode="markers", marker=dict(size=df["pop"])))
```

Use instead:

```python
sizeref = 2 * df["pop"].max() / 40**2
fig = go.Figure(go.Scatter(
    x=df["area"], y=df["rain"], mode="markers",
    marker=dict(size=df["pop"], sizemode="area", sizeref=sizeref),
))
```

xpd gives the replacement as advice and doesn't apply it.

All 11 trace types resolved `marker.sizemode` to `"diameter"`, so a marker's area grows with the
square of its size. From 10 to 20 the data grows by 100 percent and the area by 300 percent, a
Lie Factor of 3.

| Size | Radius drawn with `"diameter"` | Area in square pixels with `"diameter"` | Area ratio with `"diameter"` | Area ratio with `sizemode="area"` and the `sizeref` above |
| --- | --- | --- | --- | --- |
| 10 | 5 | 78.5 | 1 | 1 |
| 20 | 10 | 314.2 | 4 | 2 |
| 40 | 20 | 1,256.6 | 16 | 4 |

In the `sizeref` formula, 40 is the diameter in pixels of the largest marker, and the formula is
the one on the plotly page about bubble charts. `px.scatter(size=...)` and the six other px
functions that take `size` set `sizemode="area"` themselves, so they are not reported.

The rule reads a size only when it takes a column by name, as `df["pop"]`, `df["pop"] / 10` and
`df.pop_total` do, from an object that the same trace reads `x`, `y` or another coordinate from.
`go.scatter.Marker(size=df.pop_total)` resolved to `sizemode="diameter"` and drew the same radii
as `dict(size=...)`. The rule leaves alone the sizes in the table.

| Size | Why the rule leaves it alone |
| --- | --- |
| a pandas property such as `df.size` or `df.index`, or a method that is called, such as `np.full(...)` | it is not a column |
| a number, a bare name, a written list, a column of another object, or a size chosen with `np.where` or a condition | it may be a design choice |
| a root, a power or a logarithm of a column | with `np.sqrt(df["pop"])` the areas for 100, 400 and 1,600 were 78.5, 314.2 and 1,256.6 square pixels, a Lie Factor of 1, and `sizemode="area"` would then understate |
| a column named as a size, such as `df["size"]` | it was made for the markers |

The rule is silent in a file that names `sizemode` in any other way than `sizemode="diameter"`,
because the mode can be set on the figure afterwards. After
`update_traces(marker_sizemode="diameter")`, a px bubble chart resolved to `sizemode="diameter"`
and drew the size column as the diameter, so the rule reports the `update_traces` line. The rule
has no fix, because the `sizeref` that keeps the markers visible depends on the data.
`sizemode="area"` alone drew size 10 with a radius of 2.24 pixels.

#### bar-axis-without-zero

**Severity:** warning. **Fix:** none.

The rule reports a bar chart whose value axis is given a range that starts above zero.

| Figure | Range settings read |
| --- | --- |
| `px.bar`, `px.histogram` | `range_y` |
| `px.bar(orientation="h")` | `range_x` |
| a figure of upright bars, afterwards | `update_yaxes(range=...)`, `update_layout(yaxis_range=...)`, `update_layout(yaxis=dict(range=...))` |
| `go.Figure(data=[go.Bar(...)], layout=...)` | `yaxis=dict(range=...)` in the layout, or `layout_yaxis_range=...` |
| a `go.Figure()` that gets its bars from `fig.add_trace(go.Bar(...))` or `fig.add_bar(...)` | the same three settings afterwards |

Example:

```python
fig = px.bar(df, x="team", y="score", range_y=[90, 101])
```

Use instead:

```python
fig = px.bar(df, x="team", y="score")
```

xpd gives the replacement as advice and doesn't apply it.

A bar shows its value by its length. plotly starts a bar axis at zero by itself and keeps an
explicit range as written. With `[90, 101]` the data grows by 5.3 percent and the bar by 100
percent, a Lie Factor of 19.

| Bars for 95 and 100 | Range | Lengths in pixels |
| --- | --- | --- |
| `px.bar` | 0 to 105.26, chosen by plotly | 324.9 and 342.0, the ratio of the values |
| `px.bar(range_y=[90, 101])` | 90 to 101 | 163.6 and 327.3 |
| `px.histogram` of counts 95 and 100, `range_y=[90, 101]` | 90 to 101 | 163.6 and 327.3 |
| lying bars, `range_x=[90, 101]` | 90 to 101 | 245.4 and 490.9 |

When the differences are the message, `px.scatter` shows each value by its position, which
needs no zero.

The rule reads a range only when both ends are written numbers above zero and the lower end
has as many digits before the point as the upper end, or one fewer. For example, `[90, 101]`
and `[2, 5]` are read, and `[1, 100]`, `[0, 101]`, `[-10, 101]` and `[low, high]` are not. A call with
`log_y` is left alone, and a range set on the figure afterwards is left alone in a file that
names a log axis. A figure of upright bars is `px.bar` or `px.histogram` without `orientation`
and `base`, or a `go.Figure(...)` whose own arguments hold a `go.Bar` without `orientation` and
`base`. With `base` each bar starts at its base and not at zero, so bars from 91 to 95 and from
93 to 100 lay whole inside `range_y=[90, 101]`, with lengths in the ratio of their values. A
`go.Figure` with a `go.Bar` and `layout=go.Layout(yaxis=dict(range=[90, 101]))` resolved to the
y range 90 to 101, as `update_yaxes` does.

The rule also reads bars added to an empty `go.Figure()`, and plotly kept their range as
written.

| Bars of 95 and 100 in a `go.Figure` | Lengths in pixels |
| --- | --- |
| added with `add_trace`, without a range | 288.8 and 304.0 |
| added with `add_trace`, then `update_yaxes(range=[90, 101])` | 145.4 and 290.9 |
| added with `add_bar`, then `update_layout(yaxis_range=[90, 101])` | 145.4 and 290.9 |
| given to `go.Figure` itself, with the range | 145.4 and 290.9 |

The bar must be added to the same variable after its assignment and before the range, in the
statement itself or in a loop or a condition, with no second assignment of the name in between.
A bar with `orientation`, `base` or `yaxis` is left alone.

#### area-axis-without-zero

**Severity:** warning. **Fix:** none.

The rule reports an area chart whose value axis is given a range that starts above zero: `range_y` on `px.area`, and on a figure of filled areas `update_yaxes(range=...)`, `update_layout(yaxis_range=...)` and `update_layout(yaxis=dict(range=...))`.

Example:

```python
fig = px.area(df, x="month", y="sales", range_y=[90, 101])
```

Use instead:

```python
fig = px.area(df, x="month", y="sales")
line = px.line(df, x="month", y="sales", range_y=[90, 101])
```

xpd gives the replacement as advice and doesn't apply it.

An area chart shows its value by the height of the fill below the line. plotly starts the axis
of `px.area` and of `go.Scatter(fill="tozeroy")` at zero by itself. It keeps an explicit range
as written and cuts the fill at the lower edge of the plot, a Lie Factor of 19 as for a bar.

| Values 95 and 100 | Range | Fill heights in pixels |
| --- | --- | --- |
| `px.area` | 0 to 105.26, chosen by plotly | 324.9 and 342.0, the ratio of the values |
| `[90, 101]` set in `px.area`, in `update_yaxes` or in `update_layout` | 90 to 101 | 163.6 and 327.3 |
| `px.line` | 94.72 to 100.28, chosen by plotly | no fill |

A line chart needs no zero, so the replacement draws the narrow range with `px.line`.

The rule reads a range as [`bar-axis-without-zero`](#bar-axis-without-zero) does. Both ends must
be written numbers above zero, and the lower end must have as many digits before the point as
the upper end, or one fewer. A call with `orientation` or `log_y` is left alone, and a range set on the
figure afterwards is left alone in a file that names a log axis. A figure of filled areas is
`px.area` without `orientation` and `log_y`, or a `go.Figure(...)` whose own arguments hold a
`go.Scatter` with `fill="tozeroy"`.

#### polar-bar-area

**Severity:** warning. **Fix:** none.

The rule reports `px.bar_polar(...)` and `go.Barpolar(...)`.

Example:

```python
fig = px.bar_polar(wind, r="hours", theta="direction")
```

Use instead:

```python
fig = px.bar(wind, x="direction", y="hours")
```

xpd gives the replacement as advice and doesn't apply it.

A polar bar is a wedge whose radius plotly scales with the value, so the area grows with the
square of the value. From 1 to 2 the data grows by 100 percent and the area by 300 percent, a
Lie Factor of 3.

| Value | Outer radius in pixels | Area ratio |
| --- | --- | --- |
| 1 | 42.75 | 1 |
| 2 | 85.5 | 4 |
| 3 | 128.25 | 9 |
| 4 | 171 | 16 |

The book adds that "the number of information-carrying (variable) dimensions depicted should not
exceed the number of dimensions in the data" (page 71), and a wedge varies in two dimensions to
show one number. The radial
axis takes the types linear, log, date and category, so no axis type draws the area in
proportion.

#### three-d-for-two-d

**Severity:** error for a 3D trace without `z`, and warning for a `z` that is one constant. **Fix:** unsafe: `px.scatter`, `px.line` or `go.Scatter` for the 3D function without `z`, and the same with `z` dropped when `z` is the constant and `x` and `y` vary. The 2D figure draws the points, and a `go.Scatter3d` line with a colorscale raises as a `go.Scatter`. A call with an argument that only a 3D trace takes, such as `range_z`, a constant `x` or `y`, or arguments by position has no fix.

The rule reports `px.scatter_3d`, `px.line_3d` and `go.Scatter3d` with `x` and `y` and no `z`, and the same three with an `x`, `y` or `z` written as one constant: `[0] * n`, `np.zeros(n)`, `np.ones(n)`, `np.zeros_like(a)`, `np.ones_like(a)` or `np.full(n, v)`.

Example:

```python
fig = px.scatter_3d(df, x="area", y="rain")
flat = go.Figure(go.Scatter3d(x=df["area"], y=df["rain"], z=np.zeros(len(df)), mode="markers"))
```

Use instead:

```python
fig = px.scatter(df, x="area", y="rain")
```

xpd gives the replacement as advice and doesn't apply it.

"The number of information-carrying (variable) dimensions depicted should not exceed the
number of dimensions in the data" (page 71).

| Form | What plotly drew | Severity |
| --- | --- | --- |
| no `z` | nothing, because all three resolved the trace to `visible=False` in an empty scene | error |
| `z=[0, 0, 0]` | the points on one plane of a scene whose z axis ran from -1 to 1, so the third axis and the perspective show no data | warning |

The rule leaves alone a call with a `**` argument, and a px call with `z` as its fourth
positional argument. It also leaves alone a file that sets `z` afterwards, as in
`trace.z = values` or `update_traces(z=values)`, because `go.Scatter3d` with `x` and `y` followed
by `trace.z = [3, 1, 2]` resolved to `visible=True`. The second form reads only a constant that
it can see in the call, with `np.` or `numpy.` in front of the function name.

#### dual-y-axis

**Severity:** info. **Fix:** none.

The rule reports a cell with `"secondary_y": True` in the `specs` of `make_subplots`, and an axis with `overlaying="y"` or `overlaying="x"` in `update_layout`, `go.Layout` or `go.Figure`.

The rule reports how the axes are set up, and it does not prove that traces use both axes. An
unused second axis draws no second scale, so the advice applies when traces use both overlaid
axes with independent ranges.

Example:

```python
fig = make_subplots(specs=[[{"secondary_y": True}]])
fig.add_trace(go.Scatter(x=months, y=sales, name="sales"), secondary_y=False)
fig.add_trace(go.Scatter(x=months, y=visits, name="visits"), secondary_y=True)
```

Use instead:

```python
fig = make_subplots(rows=2, cols=1, shared_xaxes=True)
fig.add_trace(go.Scatter(x=months, y=sales, name="sales"), row=1, col=1)
fig.add_trace(go.Scatter(x=months, y=visits, name="visits"), row=2, col=1)
```

xpd gives the replacement as advice and doesn't apply it.

"Show data variation, not design variation" (page 61). Two y axes in one plot have two free
scales, so the ranges, and not the data, decide where the lines cross and which
line looks steeper.
The figure above resolved the second axis to `overlaying="y"` on the right with no `matches`,
and the first trace got the range 8.6 to 31.4.

| Range of the second axis | Lines crossed at |
| --- | --- |
| 393 to 507, chosen by plotly | x=1.67 |
| 0 to 1000, set with `update_yaxes(range=[0, 1000], secondary_y=True)` | x=1.81 |

The rule reports the cell of `specs` and not each `add_trace`, so a figure gets one report for
each cell with two axes. It leaves alone an axis that sets `overlaying` together with `matches`,
because `matches` shares the range with the first axis. Both axes then resolved to -6 to 107. The underscore form
`yaxis2_overlaying="y"` raised `ValueError` in `update_layout` on a `go.Figure`, so the rule
does not read it.

#### free-facet-scales

**Severity:** info. **Fix:** unsafe for `matches=None`: drops the argument, so the panels share one scale again, and drops comments inside the parentheses. None for `make_subplots`, where `shared_yaxes` goes to another statement.

The rule reports `update_yaxes(matches=None)` and `update_xaxes(matches=None)` on a px figure that was made with `facet_col`, `facet_row` or `facet_col_wrap`. It also reports a loop that adds a `go.Scatter`, `go.Scattergl`, `go.Bar`, `go.Box` or `go.Violin` with the same written column as `y` to the cells of a `make_subplots` figure without `shared_yaxes`, where the row or the col changes with the loop.

Example:

```python
fig = px.scatter(tips, x="total_bill", y="tip", facet_col="sex")
fig.update_yaxes(matches=None)
```

Use instead:

```python
fig = px.scatter(tips, x="total_bill", y="tip", facet_col="sex")
```

xpd gives the replacement as advice and doesn't apply it.

The book says of small multiples, a grid of small charts drawn the same way, "The design remains
constant through all the frames, so that attention is devoted entirely to shifts in the data" (page 170). px draws facets on one scale,
and `matches=None` frees the scales. A reader who compares heights across the panels then reads
a difference that the data does not have. Free scales are right when the panels hold different
units, which is why the rule is info.

| Facets of `px.scatter` | Second y axis | Ranges of the two panels | A tip of 5 drawn at |
| --- | --- | --- | --- |
| as px draws them | `matches="y"` | 0.38 to 10.62 in both | the same height in both |
| after `update_yaxes(matches=None)` | no `matches` | 0.62 to 6.88, and 0.38 to 10.62 | 0.70 of the height of one panel, and 0.45 of the other |

`make_subplots` gives every cell its own y axis. The table gives a bar of 5 in one cell and a bar
of 10 in the other.

| `make_subplots` | Ranges | Bar heights in pixels |
| --- | --- | --- |
| `rows=1, cols=2` | 0 to 5.26, and 0 to 10.53, with no `matches` | 304.0 and 304.0 |
| `rows=1, cols=2, shared_yaxes="all"` | 0 to 10.53 in both | 152.0 and 304.0 |
| `rows=2, cols=1, shared_yaxes=True` | 0 to 5.26, and 0 to 10.53 | not measured |

`shared_yaxes=True` shares the axes of one row only, so the message names `"all"`.

Example:

```python
fig = make_subplots(rows=1, cols=2)
for i, (shop, part) in enumerate(df.groupby("shop")):
    fig.add_trace(go.Bar(x=part["month"], y=part["sales"]), row=1, col=i + 1)
```

Use instead:

```python
fig = make_subplots(rows=1, cols=2, shared_yaxes="all")
```

The rule reads a loop because a column written as `y=part["sales"]` in each pass proves that
every panel holds the same measure. It leaves alone a column named `value` or `values`, because
`pd.melt` gives that name to the column of a long table, which holds a different measure in
each panel. It also leaves alone a trace added with `secondary_y`, and a file that names
`matches`, `shared_yaxes` or a y axis range anywhere, because the file may set the scales
afterwards.

### Ink that shows no data

The rules in the group report ink that a figure can lose without losing data. The book defines
the data-ink ratio as the share of a graphic's ink that shows data, and asks to "erase
non-data-ink, within reason" (page 96) and to "erase redundant data-ink, within reason"
(page 100).

#### default-template

**Severity:** info. **Fix:** unsafe for a named template: `"simple_white"`. None for a file that sets no template.

The rule reports a figure drawn with a template that tints the plot area and draws a grid.

| Form | Where it is reported |
| --- | --- |
| A file draws px or go figures with a plot area and never names a template or a `plot_bgcolor`. | once, on the first import of `plotly.express`, or on the first import of `plotly.graph_objects` when the file does not import `plotly.express` |
| A file draws through pandas, imports no plotly module, and calls `line`, `bar`, `barh`, `area`, `scatter`, `box` or `hist` on `df.plot`. | on the statement that makes plotly the plotting backend |
| The template `"plotly"`, `"ggplot2"` or `"seaborn"` is written as a string in a px call, in `go.Layout`, in `go.Figure(layout_template=...)`, in `update_layout` on a plotly figure, or assigned to `pio.templates.default`. | on the string |

Example:

```python
import plotly.express as px

fig = px.scatter(df, x="day", y="tip")
gray = px.scatter(df, x="day", y="tip", template="ggplot2")
```

Use instead:

```python
import plotly.express as px
import plotly.io as pio

pio.templates.default = "simple_white"

fig = px.scatter(df, x="day", y="tip")
gray = px.scatter(df, x="day", y="tip", template="simple_white")
```

plotly draws with the template `"plotly"` unless it is told otherwise. The book says that "the
grid should usually be muted or completely suppressed" (page 112).

| Template of a scatter plot | Plot background | Grid lines drawn | Axis lines |
| --- | --- | --- | --- |
| `"plotly"`, the default | `#E5ECF6` | 10, white | none |
| `"ggplot2"` | `rgb(237,237,237)` | 10, white | not recorded |
| `"seaborn"` | `rgb(234,234,242)` | 10, white | not recorded |
| `"simple_white"` | white, with `showgrid=False` | 0 | yes, with outside ticks |

The rule does not report seven templates. `"plotly_white"`, `"none"`, `"presentation"`,
`"gridon"`, `"xgridoff"` and `"ygridoff"` resolved to a white plot area, and `"plotly_dark"` is a
dark theme. Through the plotting backend of pandas, `df.plot.scatter(x="score", y="cost")` was equal,
as JSON, to `px.scatter(df, x="score", y="cost")`, also with `template="simple_white"`.

The first form is silent in the cases in the table.

| Case | Why |
| --- | --- |
| a file that names `streamlit` | `st.plotly_chart` draws with the theme of Streamlit unless `theme=None` is passed |
| a file whose only figures have no plot area: `px.pie`, `px.treemap`, `px.sunburst`, `px.icicle`, `px.funnel_area`, `px.parallel_coordinates`, `px.parallel_categories`, `go.Pie`, `go.Treemap`, `go.Sunburst`, `go.Icicle`, `go.Funnelarea`, `go.Parcoords`, `go.Parcats`, `go.Sankey`, `go.Table`, `go.Indicator` | they drew no tint and no grid line with the default template |
| a module that the file imports sets `pio.templates.default = ...` | a scatter plot in a module of a package whose `__init__.py` assigns `pio.templates.default = "simple_white"` resolved to a white plot area with no grid, and so did one in a script that imports the module with the assignment |
| a module that the file imports sets `px.defaults.template = ...`, and the file draws only px figures | the setting changed the px figure and left a `go.Figure` at `#E5ECF6` |

`hovertemplate` and `texttemplate` are other attributes and do not count as a template, and a
scatter plot with a `hovertemplate` still resolved to `#E5ECF6`. To find a template set in
another module, xpd reads the `__init__.py` of each package the file is in, each module of the
project that the file imports, and each module that an imported module imports in turn. It
does not read a module that imports the file, such as the script that starts the program. xpd
therefore still reports a file beside the one with the assignment when the file does not import
it. The fix is unsafe, because
`"simple_white"` also changes the colors of the marks.

#### explicit-grid

**Severity:** info. **Fix:** unsafe for `showgrid=True`: `showgrid=False`. None for a black grid.

The rule reports `showgrid=True` without `gridcolor` and `gridwidth` beside it, and `gridcolor` set to black, on an x or y axis of a plotly figure. It reads the keyword in `update_xaxes`, `update_yaxes`, `go.layout.XAxis` and `go.layout.YAxis`, the underscore form such as `xaxis_showgrid` in `update_layout` and `go.Layout`, and `xaxis=dict(showgrid=True)` or `xaxis={"showgrid": True}` in `update_layout`, `go.Layout` and `go.Figure`.

Example:

```python
fig = px.scatter(df, x="day", y="tip")
fig.update_xaxes(showgrid=True)
fig.update_yaxes(gridcolor="black")
```

Use instead:

```python
fig = px.scatter(df, x="day", y="tip")
fig.update_yaxes(showgrid=True, gridcolor="#eee")
```

"The grid should usually be muted or completely suppressed so that its presence is only
implicit" (page 112), and "dark grid lines are chartjunk" (page 113). `showgrid=True` either
does nothing or brings back the ink that the template left out.

| Template of a scatter plot | Grid lines without `showgrid=True` | Grid lines with it |
| --- | --- | --- |
| `"plotly"` | 10 | 10 |
| `"none"` | 10 | 10 |
| `"simple_white"` | 0 | 10, in `rgb(232,232,232)` |

The rule leaves alone a call that sets `gridcolor` or `gridwidth` beside `showgrid=True`,
because the call has chosen its grid. It reads black as `"black"`, `"#000"`, `"#000000"` and
`"rgb(0,0,0)"`, in upper or lower case, because how dark another color is against the
background cannot be read from the code. A black `gridcolor` beside `showgrid=False` draws
nothing and is left alone, and so are the lines of latitude and longitude on a map. The fix is unsafe, because it removes grid lines from a figure
drawn with `"simple_white"`.

#### boxed-axis-frame

**Severity:** info. **Fix:** unsafe: drops `mirror`, so the box around the plot is open.

The rule reports `mirror=True`, `mirror="all"`, `mirror="ticks"` or `mirror="allticks"` beside `showline=True` on an x or y axis of a plotly figure, in `update_xaxes`, `update_yaxes`, `go.layout.XAxis`, `go.layout.YAxis`, and in `xaxis=dict(...)` or `xaxis={...}` in `update_layout`, `go.Layout` and `go.Figure`.

Example:

```python
fig = px.scatter(df, x="day", y="tip")
fig.update_xaxes(showline=True, mirror=True)
fig.update_yaxes(showline=True, mirror=True)
```

Use instead:

```python
fig = px.scatter(df, x="day", y="tip")
fig.update_xaxes(showline=True)
fig.update_yaxes(showline=True)
```

xpd gives the replacement as advice and doesn't apply it.

The book redraws a chart until it has "only the frame lines remaining as pure non-information"
(page 103), and says that a frame which shows the range of the data "should replace the
non-data-bearing frame" (page 131). `mirror=True` draws each axis line a second time on the
far side of the plot, which closes a box around the data. The x axis line had one segment with
`showline=True` alone and two with `mirror=True`. Under the default template `mirror=True`
without `showline` drew no line, so the rule reads the two together.

#### box-plot-ink

**Severity:** info. **Fix:** none.

The rule reports `px.box` and `go.Box` with exactly one of `x` or `y`, when the call writes no point
settings, notches, means, quartile method, precomputed statistics, box style or orientation, and
has no `**` argument. A px call with `color`, a facet or an animation is left alone. The rule
does not prove that a box plot is wrong, and the advice holds only when the five numbers are
all the reader needs.

Example:

```python
fig = go.Figure(go.Box(y=values))
```

Use instead, if only the minimum, quartiles, median and maximum are needed:

```python
low, q1, median, q3, high = 1, 2.5, 4, 5.5, 7
fig = go.Figure(go.Scatter(
    x=[0, 0, None, 0.03, 0.03, None, 0.03, 0.03, None, 0, 0, None],
    y=[low, q1, None, q1, median - 0.05, None,
       median + 0.05, q3, None, q3, high, None],
    mode="lines",
))
```

The five numbers above are an example, computed from the values 1 to 7 with linear quartiles.
Compute the five numbers for each distribution, and choose the offset and the gap at the median
for the scale of the data. Keep the box when its outliers, notches, hover information or
statistical convention are needed. The whiskers of a default box end at the last values within
1.5 times the interquartile range of the box, and the two ends need not be the minimum and the
maximum. A drawing from the minimum to the maximum therefore shows another statistic.

The book prefers the quartile plot with the middle half offset (page 124), and says that the
choice also rests on statistical and aesthetic criteria. The table gives what plotly 7.1.0 drew
for four groups. The numbers measure the drawing and do not show that the two forms are equal
as statistics, so the rule has no fix.

| Drawing of four groups | Straight segments | Filled box area in square pixels |
| --- | --- | --- |
| four default `go.Box` traces | 36 | 15,241.7 |
| a thinner box | 28, and the median was hidden | not recorded |
| the offset quartile plot in `go.Scatter` | 16 | 0 |

#### redundant-color

**Severity:** info. **Fix:** unsafe for px: drops `color`, so the chart draws one color and no legend, and drops comments inside the parentheses. None for `go.Bar`, where the one color to use is not in the code.

The rule reports a px call whose `color` is the same column name as its `x` or its `y`, both written as strings. It covers the 13 px functions that take `color`, `x` and `y`: `px.area`, `px.bar`, `px.box`, `px.density_contour`, `px.ecdf`, `px.funnel`, `px.histogram`, `px.line`, `px.line_3d`, `px.scatter`, `px.scatter_3d`, `px.strip` and `px.violin`.

Example:

```python
fig = px.bar(df, x="day", y="tip", color="day")
```

Use instead:

```python
fig = px.bar(df, x="day", y="tip")
```

xpd gives the replacement as advice and doesn't apply it.

"Erase redundant data-ink, within reason" (page 100). `px.bar(x="day", y="tip")` drew one trace
and no legend. With `color="day"` it drew four traces and a legend with the entries Fri, Sat,
Sun and Thur, the same four words as the tick labels under the bars. With `color="tip"` it
drew a colorbar for the numbers that the heights of the bars already show.

One color for each category is sometimes meant, to match the colors of another chart of the
same categories, which is why the rule is info. A column given by a name, as in
`color=column`, is left alone.

The rule also reads `go.Bar` whose `marker_color`, or `color` inside `marker`, is the same
expression as its `x` or `y`, as in `go.Bar(x=names, y=df["sales"], marker_color=df["sales"])`.
The length of a bar already shows its value, so the colors repeat it.

| `go.Bar(y=[1, 2, 4], ...)` | Colors of the three bars |
| --- | --- |
| without `marker_color` | `rgb(99, 110, 250)` for all three |
| `marker_color=[1, 2, 4]` | `rgb(13, 8, 135)`, `rgb(156, 23, 158)` and `rgb(240, 249, 33)` |

A written string or number as the color is left alone. When the repeated coordinate holds
categories, the colors repeat the categories and not the lengths of the bars. For example,
`go.Bar(x=["red", "blue"], y=[1, 2], marker_color=["red", "blue"])` gives each category a color.
Keep such colors when they match another chart or serve another purpose.

#### pattern-fill

**Severity:** warning. **Fix:** none.

The rule reports hatching.

| Setting | Calls |
| --- | --- |
| `pattern_shape="column"` | `px.area`, `px.bar`, `px.bar_polar`, `px.histogram`, `px.timeline` |
| a pattern shape on `marker.pattern` | `go.Bar`, `go.Barpolar`, `go.Funnelarea`, `go.Histogram`, `go.Icicle`, `go.Pie`, `go.Sunburst`, `go.Treemap` |
| `fillpattern` | `go.Scatter` |

Example:

```python
fig = px.bar(df, x="day", y="tip", pattern_shape="shop")
```

Use instead:

```python
fig = px.bar(df, x="day", y="tip", color="shop", color_discrete_sequence=["#555", "#aaa"])
```

xpd gives the replacement as advice and doesn't apply it.

"This moiré vibration, probably the most common form of graphical clutter, is inevitably bad
art and bad data graphics" (page 108), and "cross-hatching should be replaced with tint screens
of shades of gray" (page 111). `px.bar(pattern_shape="sex")` resolved the pattern shapes of its
two traces to `""` and `"/"`, and the drawn figure held two pattern images, where the same
chart without the argument held none. `go.Bar(marker_pattern_shape="/")` and
`go.Scatter(fillpattern=dict(shape="x"))` each drew one.

On a go trace the rule reads `marker_pattern_shape=`, `marker=dict(pattern=dict(shape=...))`,
`fillpattern_shape=` and `fillpattern=dict(shape=...)`, with `dict(...)` or `{...}`. A shape of
`""` draws no pattern and is left alone, and so is a shape given by a name.

#### gauge-indicator

**Severity:** info. **Fix:** none.

The rule reports `go.Indicator` whose `mode` is a string that names `gauge`, such as `mode="gauge+number"`.

Example:

```python
fig = go.Figure(go.Indicator(mode="gauge+number", value=270, gauge=dict(axis=dict(range=[0, 500]))))
```

Use instead:

```python
fig = go.Figure(go.Indicator(mode="number+delta", value=270, delta=dict(reference=250)))
```

xpd gives the replacement as advice and doesn't apply it.

"Maximize data density and the size of the data matrix, within reason" (page 168). The book
calls a graphic that "purveys Graphical Style rather than quantitative information" a duck
(page 116), and a gauge draws a dial around one number.

| `go.Indicator` | Shapes drawn | Labels drawn |
| --- | --- | --- |
| `mode="gauge+number"` | 8 | 7 |
| `mode="number"`, the default | 0 | 2 |

plotly draws the dial only when `mode` names it. The rule does not report a `gauge` argument
with the default mode, because it drew no shape. A gauge with `shape="bullet"` is a bar and not a
dial, and is left alone. The rule has no fix, because a gauge can carry a threshold that the
number alone does not show.

### Color without an order

The rule in the group reports a color scale whose colors do not get lighter or darker in one
direction. The book says that "the mind's eye does not readily give a visual ordering to colors,
except possibly for red to reflect higher levels than other colors", and that "varying shades
of gray show varying quantities better than color" (page 154).

#### rainbow-colorscale

**Severity:** warning. **Fix:** none.

The rule reports the color scales Jet, Rainbow, Turbo and HSV, written as a string in upper or lower case with or without `_r`, or as an attribute of `px.colors` such as `px.colors.sequential.Jet`, when it is given to `colorscale`, `color_continuous_scale` or a setting that ends in `_colorscale` inside a plotly call, or assigned to `px.defaults.color_continuous_scale`.

The rule reports a scale where the code sets it, also on a color axis that no trace uses. The
advice applies only when a trace colors numbers with the scale. A scatter plot without numeric
colors, or without a reference to the color axis, does not show the scale.

Example:

```python
fig = px.imshow(grid, color_continuous_scale="jet")
fig = go.Figure(go.Heatmap(z=grid, colorscale=px.colors.sequential.Turbo))
```

Use instead:

```python
fig = px.imshow(grid, color_continuous_scale="Greys")
fig = go.Figure(go.Heatmap(z=grid, colorscale="Viridis"))
```

xpd gives the replacement as advice and doesn't apply it.

The test of order is lightness. Each of the 94 named scales was resolved on a `go.Heatmap` and
sampled at 21 even steps, and the CIELAB lightness of each step was computed, from 0 for black
to 100 for white. A scale that changes direction draws a low and a high value equally dark.

| Scale | Changes of direction in lightness | Reported |
| --- | --- | --- |
| 61 scales, such as `"Greys"`, `"Blues"` and `"Viridis"` | 0 | no |
| Jet, which rose from 13.5 to 97.1 and fell back to 25.5 | 3 | yes |
| HSV | 3 | yes |
| Rainbow | 2 | yes |
| Turbo, with its lightest color in the middle | 1 | yes |
| Blackbody, Bluered, Portland, Picnic | 1, near one end or at a midpoint as the two-sided RdBu does | no |
| Earth, mrybm, mygbm | more than 0 | no, because they appear in no file the rules were run on |

`px.imshow` with `"jet"`, a `go.Scatter` marker with `"Jet"` and
`update_layout(coloraxis_colorscale="Jet")` all resolved to the same six stops.
`color_discrete_sequence` is left alone, because it colors groups and not quantities. Numbers
that name groups, such as cluster numbers, are better colored as categories, and the message
says so. The rule has no fix, because the scale to use depends on whether the data has a
midpoint.

### Chart forms the book argues against

The rules in the group report pies, sunbursts, smoothed lines and cells, animations, and charts
of a few numbers that a table shows better.

#### pie-chart

**Severity:** warning. **Fix:** none.

The rule reports `px.pie(...)` and `go.Pie(...)`. A second message, for several pies, goes to the forms in the table.

| Form | Example |
| --- | --- |
| `px.pie` with `facet_col` or `facet_row` | `px.pie(df, names="day", values="tip", facet_col="shop")` |
| two `go.Pie` in the data list of `go.Figure` or of `add_traces` | `go.Figure([go.Pie(...), go.Pie(...)])` |
| an `add_trace` of a pie next to another `add_trace` of a pie on the same figure | `fig.add_trace(go.Pie(...), row=1, col=1)` twice |
| a pie added in a loop to a cell that changes with the loop | `fig.add_trace(go.Pie(...), row=1, col=i)` in `for i in ...` |

Example:

```python
fig = px.pie(df, names="day", values="tip")
several = px.pie(df, names="day", values="tip", facet_col="shop")
```

Use instead:

```python
fig = px.bar(df.sort_values("tip"), x="tip", y="day", orientation="h")
several = px.bar(df, x="tip", y="day", orientation="h", facet_col="shop")
```

xpd gives the replacement as advice and doesn't apply it.

"A table is nearly always better than a dumb pie chart; the only worse design than a pie chart
is several of them, for then the viewer is asked to compare quantities located in spatial
disarray both within and between pies. Given their low data-density and failure to order
numbers along a visual dimension, pie charts should never be used" (page 178). For a few
shares, `go.Table` shows the exact numbers.

| Code | What plotly made of it |
| --- | --- |
| `px.pie`, `go.Pie`, with and without `hole` | a trace of type `pie` |
| `px.pie(facet_col="sex")` | two pies side by side |
| `px.pie(facet_col="sex", facet_row=...)` | four pies |
| `make_subplots(rows=1, cols=2)` with two cells of type `"domain"` and a `go.Pie` added to each | two traces of type `pie`, with the domains 0 to 0.45 and 0.55 to 1 across |

Each `go.Pie` call is reported once. It gets the message for several pies when it is in a written
`go.Figure` data list, or when the figure it is added to is proved to be a plotly figure. When the
object that several pies are added to is not proved to be a figure, each pie keeps the first
message. Two `add_trace` calls count as one figure only when no second assignment of the name
comes between them.

A facet given as `None` draws one pie and gets the first message. `px.sunburst` has its own
rule, [`sunburst-as-pie`](#sunburst-as-pie). `px.funnel_area` is not reported, because its
sections were drawn with areas in the ratio of the values, stacked from top to bottom.

#### sunburst-as-pie

**Severity:** warning. **Fix:** none.

The rule reports `px.sunburst(...)` with `values` or `path`, and `go.Sunburst(...)` with `values`.

Example:

```python
fig = px.sunburst(df, path=["region", "shop"], values="sales")
```

Use instead:

```python
fig = px.bar(df.sort_values("sales"), x="sales", y="shop", color="region", orientation="h")
```

xpd gives the replacement as advice and doesn't apply it.

The book rejects pie charts for "their low data-density and failure to order numbers along a
visual dimension" (page 178). A sunburst draws each value as a wedge, as a pie does. Its rings
make the order worse, because the outer ring has three times the area of the inner ring for each
degree. The angles keep the order only within one ring. `px.sunburst` and
`go.Sunburst` resolved to a trace of type `sunburst`.

The test sunburst had the regions west 60 and east 40 in the inner ring, and the shops c 15 and
d 45 of west and a 30 and b 10 of east in the outer ring.

| Wedge | Value | Area in square pixels |
| --- | --- | --- |
| region west, inner ring | 60 | 7,963.9 |
| shop d, outer ring | 45 | 17,918.9, which is 2.25 times the area of west |

Shop a with 30 against region east with 40 was reversed in the same way.

The rule leaves alone a sunburst without numbers. A `go.Sunburst` with `labels` and `parents` and
no `values` gave each leaf one equal share, so it draws a hierarchy, such as a flavor wheel, and
not quantities. `px.sunburst` with `path` and no `values` counts the rows of each group, and the
rule reports it because the counts are numbers.

#### spline-line

**Severity:** info. **Fix:** unsafe: `"linear"`.

The rule reports the line shape `"spline"`.

| Setting | Calls |
| --- | --- |
| `line_shape="spline"` | `px.area`, `px.line`, `px.line_polar`, `px.line_ternary`, `update_traces` on a plotly figure, and the five go traces whose line takes the shape: `go.Scatter`, `go.Scattercarpet`, `go.Scatterpolar`, `go.Scattersmith`, `go.Scatterternary` |
| `line=dict(shape="spline")`, `line={"shape": "spline"}` | the same five go traces |

The rule reports the setting and does not prove that the trace draws a line. A trace with only
markers draws no curve, so the advice applies only when the trace draws lines. The unsafe fix
changes the shape also when no line uses it.

Example:

```python
fig = px.line(df, x="hour", y="reading", line_shape="spline")
```

Use instead:

```python
fig = px.line(df, x="hour", y="reading", line_shape="linear")
```

"Above all else show the data" (page 92). A spline draws a smooth curve through the points,
and the curve passes through values that the data does not hold. plotly computes the spline in
pixels, so how far the curve leaves the data depends on the data, on the axis range and on the
size of the figure. Because the distance varies, the rule is info.

| Readings 0, 0, 10, 10, 0, 0, drawn at 700 by 500 pixels | Highest point drawn | Lowest point drawn |
| --- | --- | --- |
| a straight line | 10 | 0 |
| `go.Scatter(line_shape="spline")`, y range chosen by plotly | 10.55 | -0.37 |
| `px.line(line_shape="spline")`, y range chosen by plotly | 10.51 | -0.34 |
| `go.Scatter(line_shape="spline")`, y axis set to -5 to 15 | 10.8 | -0.53 |
| `px.line(line_shape="spline")`, y axis set to -5 to 15 | 10.75 | -0.5 |

The fix is unsafe, because straight segments make the figure look different. It leaves a
`smoothing` argument in place, which plotly ignores for a straight line. A shape given by a
name is left alone.

#### animation-frame

**Severity:** info. **Fix:** none.

The rule reports `animation_frame="column"` on the 28 px functions that take it. The message names `facet_col` for the 17 that have it, and `make_subplots` for the other 11. It also reports frames built with `go.Frame`, as `frames=` in `go.Figure` or in `update` on a plotly figure, and as `fig.frames = ...`.

The rule reports how the frames are set up. An empty figure or a single frame has no different
data to compare, so the advice to draw small multiples applies when several frames show
different data and their panels fit on the page.

Example:

```python
fig = px.scatter(gap, x="gdpPercap", y="lifeExp", animation_frame="year")
```

Use instead:

```python
fig = px.scatter(gap, x="gdpPercap", y="lifeExp", facet_col="year", facet_col_wrap=4)
```

xpd gives the replacement as advice and doesn't apply it.

"Small multiples resemble the frames of a movie: a series of graphics, showing the same
combination of variables, indexed by changes in another variable" (page 170). The book lists
them as "inevitably comparative" (page 175). An animation shows one frame at a time, so the reader
compares the frames from memory.

| Figure | Frames or panels | Points drawn at once |
| --- | --- | --- |
| gapminder, `animation_frame="year"` | 12 frames | 142 of the 1,704 rows |
| gapminder, `facet_col="year", facet_col_wrap=4` | 12 panels | all 1,704 rows |
| `go.Figure` with three frames of one point each | 3 frames | 1 |
| the same three points in `make_subplots(rows=1, cols=3)` | 3 panels | 3 |

`full_figure_for_development` keeps no frames, so the frames were counted in the figure and in
the page that `to_html` wrote, which passed the three frames to `Plotly.addFrames`.

The rule is info and has no fix, because panels need room and an animation can be the better
choice when there are many frames. 11 of the 28 functions have no `facet_col`, and
`px.scatter_3d(facet_col=...)` raised TypeError, "got an unexpected keyword argument
'facet_col'". For the 11, which are `bar_polar`, `choropleth_map`, `density_map`, `line_3d`,
`line_map`, `line_polar`, `line_ternary`, `scatter_3d`, `scatter_map`, `scatter_polar` and
`scatter_ternary`, the message says to draw one panel for each frame with `make_subplots`.

The rule leaves alone frames copied from another figure, as `frames=fig.frames`, because the
figure they come from is reported where it is made. It also leaves alone `frames=None` and
`frames=[]`.

#### heatmap-smoothing

**Severity:** info. **Fix:** unsafe: `False`.

The rule reports a `zsmooth` value that blends the colors of the cells.

| Call | Values reported |
| --- | --- |
| `go.Heatmap` | `"best"`, `"fast"` |
| `go.Histogram2d`, `update_traces` on a plotly figure | `"best"` |
| `go.Image` that sets `source` | `"fast"` |

Example:

```python
fig = go.Figure(go.Heatmap(z=grid, zsmooth="best"))
```

Use instead:

```python
fig = go.Figure(go.Heatmap(z=grid, zsmooth=False))
```

"Above all else show the data" (page 92). Smoothing fills the space between the cells with
colors that blend their values.

| A 2 by 2 grid of 0 and 10 in `"Greys"`, 400 by 400 pixels | Colors drawn |
| --- | --- |
| `go.Heatmap(zsmooth=False)` | 2 |
| `go.Heatmap(zsmooth="fast")` | 61 |
| `go.Heatmap(zsmooth="best")` | 252 |
| `go.Histogram2d(zsmooth="best")` | 252 |
| `go.Histogram2d(zsmooth="fast")`, which resolved to `False` | 2 |
| `go.Image(zsmooth="fast")`, a checkerboard given as `z` | 2 |
| `go.Image(zsmooth="fast")`, the same pixels given as a PNG in `source` | 61 |

The two colors of the data covered all 160,000 pixels without smoothing, and 45,156 and 40,401
pixels with it.

The rule reads `"fast"` on `go.Heatmap` only, and on `go.Image` only with `source`, because the
docstring of `go.Image` says that `zsmooth` "only applies for image traces that use the source
attribute". `go.Image` takes `"fast"` or `False`, and `"best"` raised ValueError.
`update_traces(zsmooth="fast")` is left alone, because the rule cannot see which traces the
figure holds.

The fix is unsafe, because the figure looks different when each cell is drawn as a block of one
color.

#### table-for-small-data

**Severity:** info. **Fix:** none.

The rule reports `px.bar` and `go.Bar` whose values, `y`, or `x` with `orientation="h"`, are a written list or tuple of 1 to 20 numbers, and `px.pie` and `go.Pie` whose `values` is one, when the call is a px call or the only trace of a `go.Figure`.

Example:

```python
fig = px.bar(x=["north", "south", "east", "west"], y=[41.3, 37.8, 12.6, 8.3])
```

Use instead:

```python
fig = go.Figure(go.Table(
    header=dict(values=["region", "share"]),
    cells=dict(values=[["north", "south", "east", "west"], [41.3, 37.8, 12.6, 8.3]]),
))
```

xpd gives the replacement as advice and doesn't apply it.

"Tables usually outperform graphics in reporting on small data sets of 20 numbers or less"
(page 56), and "tables are preferable to graphics for many small data sets" (page 178). A
chart of a few numbers draws them as lengths or angles and does not write them.

| Chart | What it wrote |
| --- | --- |
| `px.bar` of 41.3, 37.8, 12.6 and 8.3 | the four names and the ticks 0 to 40, and none of the four values |
| `go.Pie` of 413, 378, 126 and 83 | the four shares as percentages, and none of the values |
| `go.Table` of the same values | all four values |

xpd cannot count the rows of a data frame or the traces of a figure. The rule therefore reads
only a px call or a `go.Figure` whose data is one trace, where the code shows the whole data. A pie is also reported by [`pie-chart`](#pie-chart), for its form. The rule leaves
alone the forms in the table.

| Form | Why |
| --- | --- |
| a trace added with `add_trace` | the code does not show the other traces |
| an `x` of numbers on a bar without `orientation="h"` | the numbers are positions |
| a chart that writes its values with `text`, `texttemplate`, `textinfo` or `text_auto`, or a file that names `textposition` | the values are already written |
| a list with a name, a `None` or a 21st number | the list is not 1 to 20 written numbers |

### Words on the figure

The rules in the group report words that are harder to read than they need to be. The book lists
what makes the words of a graphic friendly or unfriendly (page 183), and each rule quotes one
line of the list.

#### all-caps-title

**Severity:** info. **Fix:** none.

The rule reports a title written in capitals only, with at least two words of four or more capitals, inside a plotly call. A title is a `title`, a `title_text`, a setting that ends in `_title` or `_title_text`, the `text` of a title dict, a value of the `labels` of a px call, or an item of `subplot_titles`.

Example:

```python
fig = px.bar(df, x="region", y="sales", title="SALES BY REGION")
```

Use instead:

```python
fig = px.bar(df, x="region", y="sales", title="Sales by region")
```

xpd gives the replacement as advice and doesn't apply it.

The book lists as friendly "type is upper-and-lower case, with serifs", and as unfriendly "type
is all capitals, sans serif" (page 183). plotly keeps a title as written, and
`title="SALES BY REGION"` resolved to the same text.

| Title drawn at 20 pixels in the default font | Width in pixels |
| --- | --- |
| `"SALES BY REGION"` | 178, which is 1.31 times as wide |
| `"Sales by region"` | 136 |

Two more titles were 1.26 and 1.35 times as wide in capitals. The rule leaves alone short names
in capitals such as `"RSI"`, `"GDP (USD)"` and `"AUC"`, which are abbreviations and right as
written, and a single word such as `"NVIDIA"`. It skips HTML tags such as `<b>`, and leaves alone
an f-string with a field. The rule has no fix, because the case of each word, such as an
abbreviation, is the author's to choose.

#### legend-instead-of-labels

**Severity:** info. **Fix:** none.

The rule reports a figure of two to four named lines with a legend, in a file that names no `showlegend`, no annotation and no text mode. The figure is one of three forms:

- a `go.Figure` whose data is a written list of two to four `go.Scatter(...)` calls, each with a `name`
- two to four such calls added one by one with `fig.add_trace(...)` to an empty `go.Figure()`
- `px.line` with `color="column"`, or with `y` as a written list of two to four column names

Example:

```python
fig = go.Figure(data=[
    go.Scatter(x=years, y=north, name="north"),
    go.Scatter(x=years, y=south, name="south"),
])
```

Use instead:

```python
fig = go.Figure(data=[
    go.Scatter(x=years, y=north, mode="lines+text", text=["", "", "north"], textposition="middle right"),
    go.Scatter(x=years, y=south, mode="lines+text", text=["", "", "south"], textposition="middle right"),
])
fig.update_layout(showlegend=False)
```

xpd gives the replacement as advice and doesn't apply it.

The book asks "to integrate the caption and legend into the design so that the eye is not
required to dart back and forth between textual material and the graphic" (page 180), and lists
as friendly "labels are placed on the graphic itself; no legend is required" (page 183).

| Figure | `showlegend` | What it drew |
| --- | --- | --- |
| `go.Figure` with two named traces | `True` | a legend of two entries |
| two named lines added one by one with `add_trace` | `True` | a legend of two entries, the same figure |
| `px.line(color="sex")` | not recorded | two lines and a legend of two entries |
| `px.line(y=["a", "b"])` | not recorded | a legend of two entries titled "variable" |
| `mode="lines+text"`, a label at the last point of each line, and `showlegend=False` | `False` | no legend, and the names beside the lines |

`add_trace` statements count when they follow each other with no second assignment of the name
in between. The number of values of a column is known only at run time, so a `px.line` with
`color` may draw many lines. The rule leaves alone the forms in the table.

| Form | Why |
| --- | --- |
| a figure that also gets a trace in another way, such as `add_bar`, `add_traces`, an `add_trace` of another trace, or an `add_trace` in a loop or a condition | the code does not show the size of the legend |
| an `add_trace` with `row`, `col` or `secondary_y` | the traces are in different cells or on different axes |
| a call with `line_group` | many lines share one entry |
| a trace with `mode="markers"`, or a file that calls `update_traces(mode="markers")` | there is no line to label |
| a written list with a fifth trace, or a run of five `add_trace` calls | many lines have no room for a label each |

#### rotated-tick-labels

**Severity:** info. **Fix:** none.

The rule reports a `tickangle` written as a number other than 0, 180 and 360, on an x or y axis of a plotly figure. It reads the keyword in `update_xaxes`, `update_yaxes`, `go.layout.XAxis` and `go.layout.YAxis`, the underscore form such as `xaxis_tickangle` in `update_layout` and `go.Layout`, and `xaxis=dict(tickangle=...)` or `xaxis={"tickangle": ...}` in `update_layout`, `go.Layout` and `go.Figure`.

Example:

```python
fig = px.bar(df, x="answer", y="share")
fig.update_xaxes(tickangle=-90)
```

Use instead:

```python
fig = px.bar(df, x="share", y="answer", orientation="h")
```

xpd gives the replacement as advice and doesn't apply it.

The book lists as friendly "words run from left to right, the usual direction for reading
occidental languages", and as unfriendly "words run vertically, particularly along the Y-axis;
words run in several different directions" (page 183). plotly keeps a `tickangle` as written.
`update_xaxes(tickangle=-90)` resolved to -90, and both category labels of a bar chart were drawn
rotated. With the categories on y, as in `px.bar(orientation="h")`, every tick label was
drawn level.

The rule has no fix, because `tickangle=0` can make long labels run into each other, and the
level layout swaps x and y. An angle given by a name is left alone, and so is the `tickangle`
of a colorbar.

#### axis-title-column-name

**Severity:** info. **Fix:** none.

The rule reports a px call whose `x` or `y` is a written column name that is code and not words, such as `"gdpPercap"` or `"total_bill"`, in a call without `labels`.

Example:

```python
fig = px.scatter(df, x="gdpPercap", y="lifeExp")
```

Use instead:

```python
fig = px.scatter(df, x="gdpPercap", y="lifeExp", labels={"gdpPercap": "GDP per person", "lifeExp": "Life expectancy"})
```

xpd gives the replacement as advice and doesn't apply it.

The book lists as friendly "words are spelled out, mysterious and elaborate encoding avoided",
and as unfriendly "abbreviations abound, requiring the viewer to sort through text to decode
abbreviations" (page 183). px titles an axis with the column that `x` or `y` names.

| Call | Axis titles |
| --- | --- |
| `px.scatter(x="gdpPercap", y="lifeExp")` | `gdpPercap`, `lifeExp` |
| `px.histogram(x="total_bill")` | `total_bill` |
| with `labels=`, a later `update_xaxes(title_text=...)` or a later `update_layout(xaxis_title=...)` | the words given there |

The rule reads a name with an underscore, or with a capital inside a word, as in `lifeExp`. It
leaves alone a plain word such as `"day"`, a name given by a variable, and a list of columns,
which px titles `value`. The rule is silent in a file that sets an axis title or an axis
dictionary anywhere, such as `update_xaxes(...)`, `update_layout(xaxis_title=...)` or
`xaxis=dict(...)`, because the title may be replaced on the figure afterwards. It has no fix,
because only the author knows the words.

## Hamilton

The Hamilton rule finds a node that changes an input that other nodes share. It runs on files
that import `hamilton`, the module of [Apache Hamilton](https://hamilton.apache.org/), which PyPI
ships as `apache-hamilton` and `sf-hamilton`. Hamilton builds a graph of steps from plain
functions, so the name of a function is its output and the names of its parameters are its inputs. The results
below are from sf-hamilton 1.90.0 with pandas 3.1.1.

```python
def raw() -> pd.DataFrame: ...                      # node raw
def clipped(raw: pd.DataFrame) -> pd.DataFrame: ... # node clipped, with the input raw
def total_uk(raw: pd.DataFrame) -> float: ...       # node total_uk, with the same raw
```

xpd does not check a node module that uses no Hamilton decorator, because it has no `hamilton`
import.

### Shared inputs

Copy an input before a node changes it, because Hamilton hands the same object to every node that takes the input.

#### node-mutates-input

**Severity:** error. **Fix:** none. The copy goes into another statement, at the start of the node or where the second name is bound.

The rule reports a node that writes to one of its parameters, such as `raw["uk"] = ...`, `raw.loc[mask, "uk"] = 0`, `raw.columns = ...`, `del raw["dk"]`, `raw.pop("uk")` and `rows.append(...)`.

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

xpd gives the replacement as advice and doesn't apply it.

Hamilton calls each function once and hands its result, the same object, to every node that
takes it. The result of `total_uk` therefore depends on whether `clipped` ran first. The Hamilton
documentation asks for the same under
[output immutability](https://hamilton.apache.org/concepts/best-practices/output-immutability/).

| Request | Example | Replacement |
| --- | --- | --- |
| `execute(["total_uk"])` | 2.0 | 2.0 |
| `execute(["clipped", "total_uk"])` | 4.0 | 2.0 |

A node is a function at module level with a return hint and a name that does not start with an
underscore. The rule also reports a write through a second name for the input, as in
`gdf = buildings` followed by `gdf["area"] = ...`. It leaves alone the cases in the list:

- A parameter the function rebinds first, as in `raw = raw.copy()`.
- A function under `@load_from`, `@dataloader`, `@mutate` or `@hamilton_exclude`. No other node
  can take the frame that `@load_from` or `@dataloader` passes in.
- `update`, `append` and the like when the result is used, as in `return state.update(row)`,
  because an immutable object returns a new one.
- `inplace=True`, which [no-inplace](#no-inplace) reports.

A write that reaches the input through a helper function is not seen.

## Python

The Python rules report style choices that no formatter or type checker makes, and they run on
every file, whatever it imports. The `python-best-practices` skill in
[`skills/`](https://github.com/bjoernaagaard/xpd/tree/main/skills) holds the rest of the style,
and the `python-review` skill reviews code against it.

| Group | Rules |
| --- | --- |
| Names | `module-private-name`, `vague-module-name` |
| Modules | `import-side-effect` |
| Types | `typing-cast`, `dict-str-any` |
| Tests | `mock-import`, `test-skip`, `test-rerun`, `test-sleep` |
| Comprehensions | `comprehension-merges-dicts`, `unpacked-comprehension`, `comprehension-two-fors` |

### Names

Name each module after what it holds, and leave the underscore off module level names.

#### module-private-name

**Severity:** warning. **Fix:** none.

The rule reports a function, class or variable at module level whose name starts with an underscore.

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

xpd gives the replacement as advice and doesn't apply it.

`__all__` says in one place which names callers may import, and `from module import *`, type
checkers and documentation tools all read it. An underscore says the same thing once per name
and makes every use of the name harder to read.

The rule leaves alone the cases in the list:

- A method or attribute of a class, such as `def _load(self)`, and a name inside a function.
- `_` alone, and names with two leading underscores such as `__all__` and `__version__`.
- `_T = TypeVar("_T")`, and the same for `ParamSpec` and `TypeVarTuple`. A type variable is
  never part of a module's API, and typeshed, the type stubs of the standard library, and the
  typing documentation write it with an underscore. On Python 3.12, `def first[T](xs: list[T]) -> T` needs no module level name.
- A file that imports `hamilton`, where an underscore tells Hamilton that a function is not a
  node.

#### vague-module-name

**Severity:** warning. **Fix:** none.

The rule reports a module named `util`, `utils`, `helper`, `helpers`, `common` or `misc`, and a package
with one of the same names.

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

xpd gives the replacement as advice and doesn't apply it.

The name says nothing about what the module holds, so unrelated code collects in it. Every
other module then imports it. xpd reports the file on line 1.

### Modules

Keep calls and environment reads out of the top level of a module, so an import runs no work.

#### import-side-effect

**Severity:** warning. **Fix:** none.

The rule reports a call that stands alone at module level, and a read or write of `os.environ` or
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

xpd gives the replacement as advice and doesn't apply it.

A module level call runs on the first import, in an order nobody chose, and again in every
process that imports the module. A program that imports the module to use one function gets
its logging and its library settings changed, and the environment is read before the program
has parsed its own arguments.

The rule leaves alone the cases in the list:

- A file that is run and never imported
  - A file with no function and no class. It is a script, which runs from top to bottom.
  - A `__main__.py`.
- A call that sets up the module itself
  - A method that edits a collection the module defines, such as `NAMES.append("id")`.
  - A call whose name starts with `register`, and `include_router`, `add_typer`,
    `add_middleware` and `mount` on a web or command line app.
  - `pytest.importorskip`, which is how a test module skips itself.
- A call inside `if`, `try`, `with` or a class body, and a call whose result is assigned.

### Types

State the real type in a signature, so the type checker can check the keys and the values the function uses.

#### typing-cast

**Severity:** warning. **Fix:** none.

The rule reports a call to `typing.cast`.

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

xpd gives the replacement as advice and doesn't apply it.

`cast` tells the type checker to believe a type that nothing checks, so a wrong value fails at
run time, far from the cast. `isinstance`, `match` and a `TypeIs` function check the value in the
program, and the type checker then knows its type.

xpd reports `typing.cast(...)`, and `cast(...)` in a file that imports `cast` from `typing` or
`typing_extensions`. ruff has no rule that reports a cast, and its `TC006` only asks for quotes
around the type.

#### dict-str-any

**Severity:** warning. **Fix:** none.

The rule reports `dict[str, Any]` as the type of a parameter or a return value. `Dict`, `Mapping` and
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

xpd gives the replacement as advice and doesn't apply it.

The type says nothing about the keys or the values, so the type checker cannot find a
misspelled key. Every function that takes the dict has to know its keys and the types of its
values. Decode the data
once, where it enters the program, into a dataclass or a `TypedDict`.

ruff's `ANN401` reports a parameter or a return value annotated with `Any` alone, and leaves
`dict[str, Any]` alone.

### Tests

Test with real objects and let every test run once, because a mock, a skip, a rerun or a sleep lets a test pass while the code is wrong.

#### mock-import

**Severity:** warning. **Fix:** none.

The rule reports an import of `unittest.mock`, `mock` or `pytest_mock`.

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

xpd gives the replacement as advice and doesn't apply it.

A test that replaces a function with a mock passes when the mock is called the way the test
expects, whatever the real function does. Run the real code on real input.

#### test-skip

**Severity:** warning. **Fix:** none.

The rule reports `pytest.mark.skip` and `unittest.skip` in a test file.

Example:

```python
@pytest.mark.skip(reason="fails on CI")
def test_total():
    assert total(ORDERS) == 3.0
```

Use instead:

```python
def test_total():
    assert total(ORDERS) == pytest.approx(3.0)
```

xpd gives the replacement as advice and doesn't apply it.

A skipped test stays in the suite and reports nothing, so the failure it was written to catch
comes back unseen. Fix what makes the test fail. When the cause can't be found, delete the test
and write down why. The rule leaves `skipif` alone, because it names the platform or version
where the test can't run, and it leaves `xfail` alone, because `xfail_strict` in the project's
settings decides whether it can hide a failure.

#### test-rerun

**Severity:** warning. **Fix:** none.

The rule reports `pytest.mark.flaky` and an import of `flaky` in a test file.

Example:

```python
@pytest.mark.flaky(reruns=3)
def test_worker():
    done = threading.Event()
    start(done)
    assert done.is_set()
```

Use instead:

```python
def test_worker():
    done = threading.Event()
    start(done)
    assert done.wait(timeout=5)
```

xpd gives the replacement as advice and doesn't apply it.

A test that is run again until it passes hides the run where the code was wrong, and it teaches
whoever reads the result to run the suite once more instead of fixing the code. Find why the
test fails some of the time, which is often an order or a wait that the test doesn't control.
When the cause can't be found, delete the test and write down why.

#### test-sleep

**Severity:** warning. **Fix:** none.

The rule reports `time.sleep` and `asyncio.sleep` in a test file, but not `asyncio.sleep(0)`.

Example:

```python
def test_worker():
    done = threading.Event()
    start(done)
    time.sleep(0.5)
    assert done.is_set()
```

Use instead:

```python
def test_worker():
    done = threading.Event()
    start(done)
    assert done.wait(timeout=5)
```

xpd gives the replacement as advice and doesn't apply it.

A sleep waits a fixed time and hopes the work is done by then, so the test fails on a slow
machine and wastes the time on a fast one. Wait for the result itself, with a join, an event or
a poll that has a deadline. `asyncio.sleep(0)` hands control to the event loop and waits for
nothing, so the rule leaves it alone.

### Comprehensions

Write a loop when a comprehension merges dicts, unpacks or holds two `for` clauses, because the loop is easier to read.

#### comprehension-merges-dicts

**Severity:** warning. **Fix:** none.

The rule reports a comprehension or generator expression that merges dicts with `**` for every item.

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

xpd gives the replacement as advice and doesn't apply it.

The comprehension builds a whole row in one expression, and every part of the row has to be a
helper that returns a dict. A reader follows the loop from top to bottom, and a step in it can
grow without the row being rewritten. A comprehension that maps or filters one value per item is
still fine.

`{**d for d in dicts}`, which Python 3.15 added, is not reported.

#### unpacked-comprehension

**Severity:** warning. **Fix:** none.

The rule reports a dict literal that unpacks a dict comprehension with `**`.

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

xpd gives the replacement as advice and doesn't apply it.

The literal builds a dict, unpacks it and merges it in one expression. A reader has to work
through the comprehension to know which keys the result has.

#### comprehension-two-fors

**Severity:** warning. **Fix:** none.

The rule reports a comprehension or generator expression with more than one `for`.

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

xpd gives the replacement as advice and doesn't apply it.

The clauses run in the order of a nested loop, but the result stands before both of them. A
reader has to reorder the expression to follow it. On Python 3.15,
`[*case.checks for case in cases]` flattens the lists with one `for`.

## Write a rule

Add a rule to xpd by writing an ast-grep rule file in the folder of its pack and a section on
the rules page, then building xpd and running it on a sample. Rules are
[ast-grep rules](https://ast-grep.github.io/guide/rule-config.html) in `.yml` files, compiled
into the binary. A pack lives at `rules/<module>/`, where `<module>` is the name you import.

```text
rules/pandas/
├── pack.yml        # the pack's manifest: import aliases and package checks. A pack may leave it out
├── row-loop.yml    # one file per rule, named after the rule id
└── ...
docs/rules.md       # one #### <rule id> section per rule, under the heading of its pack
```

The build fails when a rule has no `#### <rule id>` section on the rules page, because `xpd rules`,
`xpd find` and the editor hover print the section. A rule file holds the documents of one rule,
one for each shape, and each document has the name of the file as its `id`. The build fails on
any other id. Rules for your own project, outside xpd, are made with `xpd new rule` and checked
with `xpd test`, which [the commands page](commands.md#new) describes.

| Setting | What it does |
| --- | --- |
| `fix:` | One ast-grep fix for the rule. The rule must then say whether the fix keeps what the code does, with `metadata: { fix: safe }` or `metadata: { fix: unsafe }`. |
| `aliases` in `pack.yml` | A fix may write an alias that the pack lists, such as `pd.`, and xpd applies the fix only in a file that imports the module under that alias. An import inside a function or a class does not count. |
| `members` in `pack.yml` | A fix may write a bare name that the pack lists with its module, such as `assert_allclose: numpy.testing`, and xpd applies the fix only in a file that imports the name from that module. An import inside a function or a class does not count. |
| `metadata: { since: "3.1" }` | xpd skips the rule when the project's venv holds an older version of the pack's library. |
| `metadata: { since: "0.6", until: "0.6.1.6" }` | The rule's advice holds only up to a release, and a second document with `since: "0.6.1.6"` gives the newest advice. |
| [`files` and `ignores`](https://ast-grep.github.io/reference/yaml.html#files) | Globs that limit the files a rule runs on. xpd matches a glob against the path as it was given, so start it with `**/` to match at any depth, as in `ignores: ["**/__main__.py"]`. |
| `kind: module` | A rule about the file itself, such as `vague-module-name`, matches the module and is reported on line 1. |
| `everyFile: true` in `pack.yml` | The pack's rules run on every file, as for `rules/python/`. Without it, the rules of a pack run only on files that import the module. |

xpd finds the version of a pack's library the same way for every pack, from the dist-info folder
whose name is the module, or whose `top_level.txt` lists the module. It then picks the documents
to run:

```text
on(run a rule document)
  if the venv's version of the pack's library is known
    run the document when since <= version < until    # a missing since or until sets no bound
  else
    run the document unless it has until
```

### When a rule has a fix

Give a rule document a fix when its message names one rewrite of the code it matches, and mark
the fix safe only when the result cannot differ. Every pack follows the same three steps.

```text
on(a rule document)
  if the message names one rewrite, and the matched code holds everything the rewrite needs
    if the fixed code gives the same object, or a value of the same type that is equal bit for bit
      fix, with metadata: { fix: safe }
    else
      fix, with metadata: { fix: unsafe }     # the comment above the document says what can differ
  else
    no fix                                    # the comment above the document names what is missing
```

| Case | What the document gets |
| --- | --- |
| The old form raises on the library version that the pack is written for | A safe fix when the rewrite is exact: on an older version of the library where both forms run, the new form is the same object as the old one, or gives a value of the same type that is equal bit for bit. Otherwise an unsafe fix. The comment above the document names the version that the proof ran on |
| The rewrite returns another type, such as an array where the old form returned a list | An unsafe fix |
| The rewrite changes what the code does on purpose, because the old form gave a wrong result | An unsafe fix |
| The rewrite drops, adds or renames one argument of a call | A fix that builds the argument list again with `rewriters`, as `deprecated-in-sklearn` does |
| The rewrite adds a statement next to the matched statement | A fix whose template has the new line and the matched statement |
| The rewrite writes a name that the file may not import | A fix, with the name under `aliases` or `members` in `pack.yml`, so that xpd applies it only where the file imports the name |
| The rewrite needs a value that the file does not hold, such as a seed, a unit, a separator or a column name | No fix |
| The rewrite needs a new name, such as a variable for a value that is used twice | No fix |
| The rewrite changes a second place that the match does not reach, such as the body of a loop, another statement or another file | No fix |
| The message names two rewrites and the code does not show which one is meant | No fix |
| The advice is to use another design, such as another chart, another model or a function in place of a loop | No fix |

Split a document when only some of its forms have one rewrite, so that those forms get the fix
and the other forms keep the report. A rule document has at most one fix.

xpd checks each fix again where it applies it. It parses the fixed Python file with
[LibCST](https://github.com/Instagram/LibCST), which reads the grammar of Python itself. It drops
the fix at a match, and keeps the report, when the fixed file no longer parses. A file that does
not parse before the fix gets no fix.

A new folder under `rules/` is a new pack, with no Rust change. Build xpd and run it on a sample:

```sh
cargo build
target/debug/xpd check samples/bad.py --format text
cp samples/bad.py /tmp/bad.py && target/debug/xpd check /tmp/bad.py --fix --unsafe-fixes
```

The first check printed five issues and exited with 1, because one issue is an error:

```text
samples/bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df["a"] * df["w"]. When each row has to become a Python object, loop over itertuples() or to_dict("records").
samples/bad.py:6:5: error [pandas/chained-assignment] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, "a"] instead.
samples/bad.py:7:1: info [pandas/no-inplace] inplace=True gives no speedup and blocks chaining. Reassign instead.
samples/bad.py:8:5: warning [pandas/merge-without-validate] merge without validate= can silently duplicate rows.
samples/bad.py:11:5: warning [pandas/concat-in-loop] out grows by one concat per pass, and each concat copies every earlier row. Collect the frames in a list and concat once after the loop.
xpd: 5 issues, 2 more with --fix --unsafe-fixes
```

The second check fixed 2 issues in the copy and left the 3 warnings. Each pack has a folder of
samples under `samples/<module>/`, and a sample marks each line that a rule reports with a
comment that names the rule:

```python
old.setup(problem, ("n_gen", 7), seed=1)  # xpd: setup-positional-termination
```

`pack.yml` may list package checks under `packages`. xpd warns when the venv has the pack's
library and lacks the package.

| Field | Meaning |
| --- | --- |
| `id` | the name of the check |
| `package` | the package to look for |
| `message` | what the warning says |
| `onlyWith` | optional. A list of tools, and xpd warns only when the project also uses one of them. |

### Index a new pack

Give a pack backed by a library a source index, so that `xpd find` and `xpd api` know the
library's names. Install the library without its dependencies in an empty folder outside the
repository, such as one under `/var/tmp`, and index it with Griffe 2.4.1:

```sh
uv venv /var/tmp/xpd-source/venv
uv pip install --python /var/tmp/xpd-source/venv/bin/python --no-deps scikit-lego
uv run --with griffe==2.4.1 xpd index /var/tmp/xpd-source/venv/lib/python3.15/site-packages --package sklego
```

Use the Python version folder that uv created. Without Griffe 2.4.1, `xpd index` stops with exit
code 2 and prints the command to run. The command reads the version from the distribution
metadata and writes `indexes/sklego.json`. A run on the pymoo 0.6.2 source with
`--output /tmp/pymoo.json` printed `pymoo 0.6.2: 971 entries written to /tmp/pymoo.json`. For a new version, update the package in the same venv and
run the last command again. Commit the file that the command writes, and never edit it by hand.

`build.rs` builds the records that `xpd find` searches, and their word weights, from
`indexes/` and from the rule sections of the rules page. The Python pack has no library, so it needs
only its rule sections. Add real questions for the new pack to `indexes/questions.json`, and
check that each question finds its expected entry in the top five:

```sh
cargo run --quiet -- find --check indexes/questions.json
```

When a question fails, keep it and improve the ranking for the general pattern of words. Read
downloaded source as data, and never import it.

Run the three commands below after you add or change a record, because every record needs a
vector of its current text in `indexes/vectors.bin`. A vector is the list of numbers that the
embedding model computes from the text, and the build compiles the file into xpd. The file stores
a hash of each record's name and summary beside its vector. When a record's text changes, the
build drops its vector, and the check fails and names the record.

```text
on(add or change records)
  cargo build                                                 # new and changed records get no vector
  target/debug/xpd find --write-vectors indexes/vectors.bin   # embeds only those records
  cargo run --quiet -- find --check indexes/questions.json    # builds again and checks
```

`--write-vectors` is not in `xpd find --help`, and it needs the embedding model. Commit
`indexes/vectors.bin` with the change.

A source index can also hold recipes, which are worked examples taken from the library's
documentation. Refresh them in the same command as the names, with `--examples`, `--source-url`
and `--license`, and [the index command](commands.md#index) shows the full command.

| Option | Value |
| --- | --- |
| `--examples` | the documentation and gallery folder of a pinned checkout. For plotly, `doc/python`. For scikit-learn, the repository root, which holds `examples/` and the docstrings. |
| `--source-url` | a repository URL that ends in `/blob/<tag>` and the same folder, such as `/blob/<tag>/doc/python` for plotly |
| `--license` | the license file of the source |

`xpd index` parses the examples without running them, and it applies the safe and unsafe fixes
of the rules. It keeps only examples with no issue left in the library's pack and the Python
pack. Each recipe names its source license, and the full license texts are stored once in
`licenses/`. For a license that xpd does not recognize, the recipe holds a link to the source
and no copied code. A pack keeps at most 24 recipes and 96,000 bytes of code, with at most 8,000 bytes for one
example, and the command reports the rejected and capped examples. Do not write recipes by hand
or edit generated entries to raise a question's rank. To use a recipe, run
`xpd find "<task>" --kind recipe` and pass a returned name to `xpd api` for the full example.

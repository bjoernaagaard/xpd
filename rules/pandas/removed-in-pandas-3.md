These names and arguments were removed in pandas 3, so the code raises. Use the replacement that the example and the table show.

```diff lang="python"
-filled = df["load"].fillna(method="ffill")
-months = df.resample("M", on="time")["load"].sum()
+filled = df["load"].ffill()
+months = df.resample("ME", on="time")["load"].sum()
```

## What the rule reports

The rule reports `applymap`, `fillna(method=)`, frequencies such as `"M"`, `"H"` and `"T"`, `delim_whitespace=True`, `dt.week`, and the removed arguments and names listed below.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `map`, `"ME"`, `"h"`, `"min"`, a Timedelta unit such as `"min"` for `"T"`, `sep=r"\s+"`, `lineterminator=`, `sep=` for a separator passed by position, `is_monotonic_increasing`, `np.` for `pd.np.` and `pd.json_normalize`. The unsafe fix writes `ffill()` and `bfill()`, which label and order the rows of a group-by differently, `"M"` in `resample`, `dt.isocalendar().week`, `writer.close()`, `pd.Series(x).value_counts()`, the alias in a comparison with `infer_freq` or `freqstr`, and these forms, which raise on pandas 3: dropping `delim_whitespace=False`, `verbose`, `infer_datetime_format`, `mangle_dupe_cols`, `squeeze=False`, `encoding` of `to_excel` and `freq` of `pd.Timestamp`, `.squeeze("columns")` for `squeeze=True`, `(s - s.mean()).abs().mean()` for `mad()`, `to_numpy()` for `as_matrix()`, `.loc` for `.ix`, `pd.Index(data, dtype="int64")` for `pd.Int64Index(data)`, `from pandas import json_normalize`, `items()`, `ffill()` for `pad()`, `is_monotonic_increasing` for `is_lexsorted()`, `inclusive=` for `closed="left"` or `"right"`, `ValueError` added to a handler of the pytz errors, `dt.days` and `dt.normalize()` for a cast to a day unit. The other forms have no fix, and the comment above each of them in the rule file says why.

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

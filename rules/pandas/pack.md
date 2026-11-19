# pandas

The pandas rules find code that is wrong on pandas 3, where copy-on-write is always on, and code
that has a shorter or faster form. Read the notes of a rule before you apply a fix that can change
the result, as the table shows.

| Fix | Result | How to apply it |
| --- | --- | --- |
| safe | the same result | `xpd check --fix` |
| unsafe | can change the result | `xpd check --fix --unsafe-fixes` |
| none, with the replacement shown as advice | can change the result | edit the code by hand |

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
| [`lambda-in-assign`](lambda-in-assign.md) | pandas 3.0 and later |
| [`concat-then-category`](concat-then-category.md) | pandas 3.1 and later |
| every other pandas rule | every pandas version |

## Code that is wrong or broken on pandas 3

Each rule in this group finds code that gives a wrong result, raises, or warns on pandas 3.0 and
3.1.

- chained-assignment
- inplace-on-column
- inplace-result-assigned
- no-inplace
- epoch-from-int-cast
- compare-to-missing
- in-column
- mask-precedence
- write-through-values
- object-dtype-check
- datetime-unit-check
- groupby-apply-reads-key
- removed-in-pandas-3
- reduction-level-argument
- stack-dropna
- frame-append
- str-replace-regex-default
- positional-key-on-labels
- day-frequency-with-time-zone
- mixed-offsets-without-utc
- deprecated-in-pandas-3

## Shorter forms, including the pandas 3.0 and 3.1 API

Each rule in this group names a shorter way to write the same code, often with an API that pandas 3.0 or 3.1 added. Most of the rules are warnings, and the rest are info.

- sort-then-reset-index
- set-index-then-resample
- subtract-shift
- range-mask
- groupby-then-reset-index
- rename-after-agg
- merge-indicator-anti-join
- value-counts-share
- nested-np-where
- query-string
- filter-items
- inline-column-list
- concat-then-category

## Python work per row or per value

Each rule in this group finds code that calls a Python function once per row or once per value,
and names the pandas method that works on the whole column at once. Several rules don't run on
test files, because a test builds its expected value row by row on purpose.

- apply-known-function
- literal-contains
- copy-after-indexing
- lookup-per-row
- apply-recomputes-aggregate
- enumerate-map-encoding
- copy-in-chain
- row-loop
- apply-axis-1
- apply-text-method
- apply-date-part
- apply-condition
- apply-arithmetic
- builtin-on-column
- list-over-column
- frame-growth-in-loop
- len-of-filter
- sort-then-head
- replace-with-mapping
- concat-in-loop
- apply-math-function
- apply-cast
- apply-comparison
- apply-pd-series
- index-loop
- zip-over-columns
- column-loop
- filter-before-frame

## groupby, agg and transform

Replace a lambda or `apply` in a `groupby` with a built-in aggregation, which pandas runs in compiled code for all groups at once.

- groupby-lambda
- groupby-lambda-window
- groupby-lambda-arithmetic
- groupby-filter-lambda
- groupby-apply
- aggregate-then-merge-back
- named-agg-lambda
- groupby-loop-collects
- groupby-first-skips-missing

## Tidy data

Each rule in this group finds a longer way to reshape a frame, or a default of a reshaping method
that changes values without a warning. A tidy frame has one row per observation and one column per
variable. The measurements used pandas 3.1.1.

- column-loop-to-long
- stack-then-reset-index
- melt-default-names
- pivot-table-default-mean
- unstack-without-fill

## Index alignment and joins

Each rule in this group finds code that turns off the pairing of rows by index label without a
warning, or code that pairs by index through a longer form. pandas pairs rows by index label in an
assignment, in `concat(axis=1)` and in a join. The measurements used pandas 3.1.1.

- assign-drops-alignment
- concat-columns-unchecked
- merge-on-both-indexes
- merge-asof-unsorted

## MultiIndex

Each rule in this group finds a MultiIndex that the code builds or reads one row at a time, and
names the method that works on whole levels.

- multiindex-from-tuples
- level-values-unique
- tuple-slice-on-levels

## Time series

Each rule in this group finds date and time code that works one value at a time, goes through
text, or breaks across a daylight saving change. The time zone rules were run on pandas 2.3.3 and
3.1.1 with quarter-hourly data in `Europe/Copenhagen`.

- rolling-apply
- text-time-key
- object-dates
- apply-on-dates
- strftime-date-part
- apply-date-expression
- strftime-round-trip
- index-year-mask
- apply-date-method
- apply-parse-date
- timedelta-per-value
- tz-localize-without-dst
- date-range-then-localize
- to-datetime-then-localize
- convert-then-drop-zone
- index-plus-offset
- date-range-slice
- index-time-mask
- reindex-to-date-range
- divide-by-shift
- date-part-key
- row-window-on-uneven-times

## Stated intent

Each rule in this group asks for an argument that states what the code expects, so that pandas
raises when the data doesn't match instead of giving a wrong result.

- repeated-reset-index
- repeat-rows-by-index
- numpy-round-trip
- frame-rebuilt-from-columns
- empty-guard
- astype-then-to-numpy
- merge-without-validate
- merge-without-on
- to-csv-index

## Method chains

Each rule in this group asks for one chain of methods where the code builds a frame in separate
steps. A chain names the frame once and lists the steps in the order they run. No variable
holds a frame that is half done. The rules of this group are info, and xpd reports them as
warnings in a project with `style = "strict"` in its settings, which
[`xpd check`](https://bjoernaagaard.github.io/xpd/commands/check/#house-style) describes.

- lambda-in-assign
- reassign-ladder
- filter-ladder
- frame-column-write
- nested-step-calls
- script-steps

## Charts

Each rule in this group asks a file to draw with plotly as the plotting backend of pandas, so that
xpd can check the figure with the rules of its [plotly](../plotly/pack.md) pack. pandas draws `df.plot` with
matplotlib unless the plotting backend is set. With `pd.options.plotting.backend = "plotly"`,
`df.plot.bar(x="team", y="score")` returned a figure equal, as JSON, to
`px.bar(df, x="team", y="score")`. The measurements used pandas 3.1.1 with plotly 7.1.0 and
matplotlib installed.

- plot-without-plotly-backend
- plot-kind-argument
- matplotlib-on-frame

## Reading and writing files

Each rule in this group finds a call that reads or writes a file in a slower way than it needs to,
or that writes a CSV file that reads back wrong.

- read-csv-slow-option
- read-then-head
- read-csv-converters
- read-json-lines-engine
- to-csv-append-header

## Parquet storage

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

- csv-round-trip
- pickle-or-hdf-storage
- read-parquet-all-columns
- read-parquet-then-filter
- chunked-csv-concat

## Packages

`xpd check` warns once per run about each package in the table that the project's venv lacks,
when the venv has pandas. The warning points at the nearest `pyproject.toml`, or at the venv
when there is none. A project without a venv gets no package warning, and `xpd hook` never gives
one.

| Check | Package | Extra condition |
| --- | --- | --- |
| [`missing-pyarrow`](missing-pyarrow.md) | pyarrow | none |
| [`missing-bottleneck`](missing-bottleneck.md) | bottleneck | the checked code calls `std`, `var`, `min`, `max` or `median` |
| [`missing-plotly`](missing-plotly.md) | plotly | the project uses matplotlib or seaborn |
| [`missing-pandas-stubs`](missing-pandas-stubs.md) | pandas-stubs | the project uses mypy, pyright, basedpyright, ty or pyrefly |

A project uses a tool when the venv has it or `pyproject.toml` has a `[tool.<name>]` table for it.
The output shows the four warnings for a venv that holds pandas and matplotlib, next to a
`pyproject.toml` with a `[tool.mypy]` table, and a `report.py` that calls `std` on line 4.

```text
$ xpd check --format text report.py
pyproject.toml: warning [pandas/missing-pyarrow] pyarrow is not installed, so text columns are stored as Python strings. With pyarrow the .str methods ran 3 to 11 times faster on 1,000,000 rows. Add pyarrow to the project.
pyproject.toml: warning [pandas/missing-bottleneck] bottleneck is not installed. pandas uses it for std, var, min, max and median. This code calls them first at report.py:4, and the number of such calls is 1. On 10,000,000 rows std ran 5.7 times faster and min 4.7 times. Add bottleneck to the project.
pyproject.toml: warning [pandas/missing-plotly] The project has matplotlib and no plotly, so df.plot returns a static matplotlib Axes. With plotly installed and pd.options.plotting.backend = "plotly", df.plot.bar(x="team", y="score") returned the same figure as px.bar(df, x="team", y="score"), which xpd checks with its plotly rules. Add plotly to the project.
pyproject.toml: warning [pandas/missing-pandas-stubs] The project has a type checker but not pandas-stubs, so calls to pandas are not type checked. Add pandas-stubs as a development dependency.
xpd: 4 issues
```

numexpr has no check, because it changed little in the measurements. numba has no check, because
it helps only a function passed to `rolling(...).apply`. The `rolling-apply` message names numba.

- missing-pyarrow
- missing-bottleneck
- missing-plotly
- missing-pandas-stubs

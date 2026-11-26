# pandas

The pandas pack catches broken pandas 3 copy-on-write code, slow Python row loops, index alignment traps, and verbose chains on files that import `pandas`. House style rules under `Method chains` report as `info` by default and as `warning` when `style = "strict"` is configured in [`xpd check`](https://bjoernaagaard.github.io/xpd/commands/check/#house-style).

## Code that is wrong or broken on pandas 3

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

- column-loop-to-long
- stack-then-reset-index
- melt-default-names
- pivot-table-default-mean
- unstack-without-fill

## Index alignment and joins

- assign-drops-alignment
- concat-columns-unchecked
- merge-on-both-indexes
- merge-asof-unsorted

## MultiIndex

- multiindex-from-tuples
- level-values-unique
- tuple-slice-on-levels

## Time series

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

- lambda-in-assign
- reassign-ladder
- filter-ladder
- frame-column-write
- nested-step-calls
- script-steps

## Charts

- plot-without-plotly-backend
- plot-kind-argument
- matplotlib-on-frame

## Reading and writing files

- read-csv-slow-option
- read-then-head
- read-csv-converters
- read-json-lines-engine
- to-csv-append-header

## Parquet storage

- csv-round-trip
- pickle-or-hdf-storage
- read-parquet-all-columns
- read-parquet-then-filter
- chunked-csv-concat

## Packages

- missing-pyarrow
- missing-bottleneck
- missing-plotly
- missing-pandas-stubs

# Polars

The Polars pack catches removed Polars 2.0 APIs, expression and null bugs, eager reads that defeat lazy pushdown, and Python loops over rows on files that import `polars`. Version-gated rules check the installed Polars release in the project's virtual environment.

## Removed and deprecated APIs

- removed-in-polars
- polars-removed-parameter
- deprecated-in-polars

## Types and query results

- polars-silent-dtype-coercion
- polars-streaming-order
- polars-removed-cast
- polars-selector-coercion
- polars-mixed-dtype-arithmetic
- polars-null-comparison
- polars-expr-boolean-op
- polars-expr-mask-precedence
- polars-when-without-otherwise
- polars-dataframe-orient
- polars-list-get-oob

## Lazy execution and file I/O

- polars-eager-read-before-query
- polars-collect-then-lazy
- polars-collect-then-write
- polars-collect-in-loop
- polars-lazy-schema-property

## Expressions and Python loops

- polars-map-elements-native
- polars-map-without-return-dtype
- polars-row-iteration
- polars-sequential-with-columns
- polars-concat-in-loop
- polars-groupby-join-over
- polars-sort-then-head
- polars-pandas-api-name
- polars-pandas-round-trip
- polars-builtin-on-expr-or-series

## Joins, categories, SQL and testing

- polars-join-key-unvalidated
- polars-join-without-validate
- polars-join-asof-unsorted
- polars-categorical-vs-enum
- polars-sql-frame-globals
- polars-assert-frame-equals

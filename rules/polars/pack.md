# Polars

The Polars pack checks code that imports `polars`. It reports APIs removed in
Polars 2.0, changes to expressions, and places where a lazy query or a native
expression can replace Python work. Version gates read the project's installed
version when xpd can find it. The rules follow local names to Polars constructors,
annotations and supported method chains. [Python API reference](https://docs.pola.rs/api/python/stable/reference/index.html)
and [Polars 2.0 upgrade guide](https://docs.pola.rs/releases/upgrade/2/).

The probes used Polars 2.1.0 on CPython 3.14.4 with `-W error`. Timings are the
median of seven runs after a warm-up with `POLARS_MAX_THREADS=1`. The two data sizes
were 50,000 and 500,000 rows. Where a rewrite preserves the result, timings compare
equal output values and dtypes using `polars.testing.assert_frame_equal` for frames.

The compiled checker reported 0 Polars diagnostics across the 219 Python files in
the installed Polars 2.1.0 package. A separate scan of [Pandera at 29486d1](https://github.com/unionai-oss/pandera/tree/29486d19a612e6bc018c32ff3e44d76b571a9895)
covered 66 Python files that import Polars. It reported 61 notes about the
`Utf8` alias and one note about collecting in a loop, with zero Polars errors
or warnings.

## Measured behavior

| Rule | Severity | Proof | Fix |
| --- | --- | --- | --- |
| [`removed-in-polars`](removed-in-polars.md) | error, info | AttributeRemovedError. AttributeError. TypeError with 0 arguments. TypeError. ModuleNotFoundError. All 11 removed protocol imports raised ImportError. Config.state omitted the variable when it was set to 1. | unsafe, none |
| [`polars-removed-parameter`](polars-removed-parameter.md) | error | ArgumentRemovedError. TypeError. ComputeError. | none |
| [`deprecated-in-polars`](deprecated-in-polars.md) | warning, info | DeprecationWarning for both methods. DeprecationWarning on concat and union. Identical dtype object. UnstableWarning with warn_unstable=True. | none, safe |
| [`polars-streaming-order`](polars-streaming-order.md) | info | The first two unpivoted rows had labels b and c with streaming and b and b with the in-memory engine. | none |
| [`polars-removed-cast`](polars-removed-cast.md) | error | InvalidOperationError. ComputeError. | none |
| [`polars-selector-coercion`](polars-selector-coercion.md) | info | Taking the union of numeric columns and a string column as a value expression raised SchemaError. Two selectors selected 3 columns. | none |
| [`polars-mixed-dtype-arithmetic`](polars-mixed-dtype-arithmetic.md) | error | InvalidOperationError for two Series. A string Series plus integer scalar raised TypeError. | none |
| [`polars-null-comparison`](polars-null-comparison.md) | warning | UserWarning under -W error | unsafe |
| [`polars-expr-boolean-op`](polars-expr-boolean-op.md) | error | TypeError from ambiguous truth value | none |
| [`polars-expr-mask-precedence`](polars-expr-mask-precedence.md) | error | TypeError from chained comparison truth testing | none |
| [`polars-when-without-otherwise`](polars-when-without-otherwise.md) | info | The unhandled row held None. The explicit branch returned 0. | none |
| [`polars-dataframe-orient`](polars-dataframe-orient.md) | info | Inferred first row (1, 3). Explicit row orientation gave (1, 2). | none |
| [`polars-list-get-oob`](polars-list-get-oob.md) | info | Index 1 raised ComputeError for a one-item list. null_on_oob=True returned [None, 3]. | none |
| [`polars-eager-read-before-query`](polars-eager-read-before-query.md) | info | 4.64x and 32.60x with one thread and row group pruning. | none |
| [`polars-collect-then-lazy`](polars-collect-then-lazy.md) | info | The collect call produced a DataFrame and lazy returned a new LazyFrame. | none |
| [`polars-collect-then-write`](polars-collect-then-write.md) | info | Eager write and streaming sink produced equal data after reading their files. | none |
| [`polars-collect-in-loop`](polars-collect-in-loop.md) | info | Three collect calls produced the same three frames as collect_all in the probe. | none |
| [`polars-lazy-schema-property`](polars-lazy-schema-property.md) | warning | PerformanceWarning. The explicit replacement returned the same value and type. | safe |
| [`polars-map-elements-native`](polars-map-elements-native.md) | warning | PolarsInefficientMapWarning under -W error | none |
| [`polars-map-without-return-dtype`](polars-map-without-return-dtype.md) | info | Inference produced List(Int64) on integer data and raised InvalidOperationError on two Null rows. | none |
| [`polars-row-iteration`](polars-row-iteration.md) | info | 1279.48x and 623.31x with equal sums on integer data. | none |
| [`polars-sequential-with-columns`](polars-sequential-with-columns.md) | info | Independent expressions gave equal frames. | none |
| [`polars-concat-in-loop`](polars-concat-in-loop.md) | info | 16.23x and 269.11x on 50,000 and 500,000 total rows across 25 batches with rechunk=True. | none |
| [`polars-groupby-join-over`](polars-groupby-join-over.md) | info | Three rows with non-null keys gave equal frames. | none |
| [`polars-sort-then-head`](polars-sort-then-head.md) | info | Top-k was 2.51x and 3.96x faster on shuffled keys, and slower on already sorted keys. | none |
| [`polars-pandas-api-name`](polars-pandas-api-name.md) | error | Thirteen AttributeError results on DataFrame, LazyFrame, and Series. | none |
| [`polars-pandas-round-trip`](polars-pandas-round-trip.md) | info | The integer-only round trip matched the original frame on 50,000 and 500,000 rows. | none |
| [`polars-builtin-on-expr-or-series`](polars-builtin-on-expr-or-series.md) | error | Five TypeError results on Expr. All five builtins worked on Series. | none |
| [`polars-join-without-validate`](polars-join-without-validate.md) | info | Two identical keys on each side yielded 4 rows. 1:1 validation raised ComputeError. | none |
| [`polars-join-asof-unsorted`](polars-join-asof-unsorted.md) | info | Descending right keys raised InvalidOperationError with the default sortedness check. | none |
| [`polars-categorical-vs-enum`](polars-categorical-vs-enum.md) | info | Categorical accepted typo. Enum raised InvalidOperationError for 1 unknown value. | none |
| [`polars-sql-frame-globals`](polars-sql-frame-globals.md) | info | Implicit and explicit binding each returned 3 rows. | none |
| [`polars-assert-frame-equals`](polars-assert-frame-equals.md) | warning | equals returned True. assert_frame_equal raised AssertionError for Int32 against Int64. | none |

## Removed and deprecated APIs

- removed-in-polars
- polars-removed-parameter
- deprecated-in-polars

## Types and query results

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

- polars-join-without-validate
- polars-join-asof-unsorted
- polars-categorical-vs-enum
- polars-sql-frame-globals
- polars-assert-frame-equals

## What the pack leaves out

- `pl.count("x")` still counts non-null values. `polars.interchange` still exists
  and exports `CompatLevel`. The rule covers four removed submodules and 11 removed protocol names.
- `pl.Utf8 is pl.String` is true and emits no warning. Avro read and write emit
  `UnstableWarning` only with `warn_unstable=True`. Both are informational.
- The streaming and in-memory engines returned the same order on the single-threaded
  unpivot probe without a limit. A two-row limit exposed different results.
  The rule is informational because a positional limit can select any rows by design.
- A selector combined with a column can be a valid value operation. The mixed
  selector rule is informational. The cast and arithmetic rules require written
  Series values in the types that fail. Inferred column dtypes reach xpd only
  when the source spells out the evidence its rules use.
- A total sort beat top-k on already sorted data by 5.5 times and 3.4 times at the
  two sizes. Its rule is informational. The read, row and concat timings depend
  on file layout, operations and data. Their rules are informational as well.
- Python builtins work on Series, and some return deliberate Python values.
  The builtin rule reports Expr only. The as-of join rule targets the literal
  opt-out from sortedness checking. Sortedness can come from the input data.
- The pack leaves optional packages to the chosen I/O backend. Native readers
  do not require pyarrow, Excel can use another engine, and database reads can
  use ADBC. The pack has no project-wide package check.
- Dynamic imports, unknown receivers, indirect callbacks and most unpacked
  arguments can hide details. The pack relies on direct syntax and local value
  bindings. Other changes outside the listed APIs are covered by the upgrade guide.

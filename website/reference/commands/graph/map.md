Use `xpd graph map` to plan a native Polars reimplementation of a named pandas
pipeline or to plan an eager-to-lazy rewrite of a Polars pipeline. It reads
the Python source without importing or running it.

The plan lists observed column reads, writes and dependencies by stage.
It proposes native expressions, windows and lazy scans where they fit.
It flags unresolved calls, dynamic column names, index behavior and
observable writes that need attention before the rewrite.

Supply `--contract` to declare ordered input and output schemas,
positional row identity and the output materialization boundary. The
[comparison contract](../../equiv/) also tells `xpd equiv` how to compare
the native Polars result with the original pandas result on the same
data. `map` checks observed column names against the declared schemas.
`equiv` verifies the actual returned columns, order, types and values.
A complete plan still needs that comparison.

## Example

```console
$ xpd graph map samples/pipeline/features.py --entry add_features --from pandas --to polars --format text
samples/pipeline/features.py:28: add_features (pandas to polars, partial)
Observed input columns: bill_depth_mm, bill_length_mm, body_mass_g, flipper_length_mm
...
```

## Plan a skfolio migration

Use `--from skfolio --to polars` to find skfolio imports and direct calls in a
named entry function. For `prices_to_returns`, the report proposes Polars
expressions over an explicit date key. It lists the options, downstream types,
adapters and unresolved calls to check. The pandas `--contract` option applies
only to the pandas pair.

```sh
xpd graph map examples/skfolio-migration/replay.py --entry main --from skfolio --to polars --format text
uv run --no-project examples/skfolio-migration/replay.py
```

The example uses skfolio's packaged S&P 500 and factor data. Its walk-forward
caller consumes the returns and compares dates, daily returns, wealth and ending
weights with the original caller. The replay runs the migrated command in a
separate Python environment and requires both skfolio and pandas to be absent.

## Eager to lazy Polars

Use `--from polars --to polars` for a function that reads one Parquet file
and applies native filters or literal column selections in a chain or
through local frame assignments. For a file path, the plan proposes
`scan_parquet` so Polars can push predicates and projections into the scan.
The report marks the return, existing collection or DataFrame consumer
where the pipeline needs materialized data. It flags dynamic expressions,
read options and calls it cannot trace. The `--contract` option belongs to
the pandas-to-Polars planner.

```python title="readings.py"
import polars as pl


def eager(path):
    readings = pl.read_parquet(path)
    return readings.filter(pl.col("keep")).select("row_id", "reading")


def dynamic_projection(path, columns):
    readings = pl.read_parquet(path)
    return readings.filter(pl.col("keep")).select(columns)
```

```console
$ xpd graph map readings.py --entry eager --from polars --to polars --format text
readings.py:4: eager (polars to polars, complete)
...
6: proposed [materialization] Collect once at the return to preserve the DataFrame result.
...
$ xpd graph map readings.py --entry dynamic_projection --from polars --to polars --format json
...
  "status": "partial",
...
      "code": "expression",
...
```

The second command exits with status 1 because it cannot resolve the
columns chosen at runtime.

### Reproduce the comparison

Run from the repository root on Linux with uv and GNU time. The example
generates a Parquet file containing 1,000,000 rows and 15 columns, with
50,000 rows per group. Both implementations materialize a DataFrame with
10,000 rows and the ordered Int64 columns `row_id` and `reading`. The
comparison checks exact values, dtypes, column order and row order.

```sh
(
  set -e
  file=$(mktemp --suffix=.parquet)
  trap 'rm -f "$file"' EXIT
  export POLARS_MAX_THREADS=1
  example=examples/polars-lazy/compare.py
  run() { uv run --no-project --python 3.13 --no-build --with polars==2.1.0 -- "$@"; }
  run python "$example" generate "$file"
  run python "$example" verify "$file"
  run python "$example" eager "$file"
  run python "$example" lazy "$file"
  for mode in eager lazy lazy eager eager lazy lazy eager eager lazy; do
    run /usr/bin/time -f 'peak RSS: %M KiB' python "$example" "$mode" "$file"
  done
)
```

The eager pipeline reads all 15 columns before filtering. The optimized
lazy plan includes the predicate and reads only the three columns needed
for the predicate and output.

```text
simple π 2/2 ["row_id", "reading"]
  Parquet SCAN [<file>]
  PROJECT 3/15 COLUMNS
  SELECTION: col("keep")
  ESTIMATED ROWS: 1000000
```

On Linux x86-64 with an Intel Core i9-13900KF, Python 3.13.18 and Polars
2.1.0, the generated file was 21,079,624 bytes. Polars used one worker
thread and the lazy run collected with the in-memory engine. The table
shows medians from five interleaved runs per implementation in fresh
processes after warming the file cache. Wall time starts after imports
and ends at the materialized result. Peak RSS covers the whole Python
process.

| Pipeline | Wall time | Peak RSS |
| --- | ---: | ---: |
| Eager read | 112.3 ms | 179,032 KiB |
| Lazy scan | 2.8 ms | 64,964 KiB |

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The plan has no unresolved items. |
| 1 | The plan is partial and lists the evidence still needed. |
| 2 | The arguments, source file or contract could not be read or analyzed. |

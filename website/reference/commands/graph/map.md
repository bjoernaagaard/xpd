Run `xpd graph map` to plan a native Polars rewrite from a pandas or skfolio entry function, or an eager to lazy Polars rewrite, directly from Python source without executing it.

| Migration pair | What the plan proposes |
| --- | --- |
| `--from pandas --to polars` | Native expressions, window functions, and schema checks against optional `--contract` (verified afterwards with [`xpd equiv`](../../equiv/)) |
| `--from skfolio --to polars` | Polars expressions for `prices_to_returns` over an explicit date key and downstream adapter list |
| `--from polars --to polars` | `scan_parquet` predicate and projection pushdown with explicit materialization boundaries |

## Examples

```console
$ xpd graph map samples/pipeline/features.py --entry add_features --from pandas --to polars --format text
samples/pipeline/features.py:28: add_features (pandas to polars, partial)
Observed input columns: bill_depth_mm, bill_length_mm, body_mass_g, flipper_length_mm
...
```

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

## Measured lazy scan speedup

On a 1,000,000-row, 15-column Parquet file (`examples/polars-lazy/compare.py`, Polars 2.1.0, 1 thread, median of 5 interleaved runs on Linux x86-64), switching from `read_parquet` to `scan_parquet` read 3 of 15 columns instead of all 15.

| Pipeline | Wall time | Peak RSS |
| --- | ---: | ---: |
| Eager `read_parquet` | 112.3 ms | 179,032 KiB |
| Lazy `scan_parquet` | 2.8 ms | 64,964 KiB |

See `examples/skfolio-migration/replay.py` for an end-to-end skfolio to Polars replay that runs without pandas or skfolio installed.

## Exit status

| Exit code | Meaning |
| --- | --- |
| `0` | Complete plan with no unresolved items |
| `1` | Partial plan listing unresolved calls, dynamic expressions, or missing evidence |
| `2` | Invalid arguments, unreadable source file, or invalid contract |

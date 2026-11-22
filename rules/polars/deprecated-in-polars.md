Polars 2.1.0 warns about older binning methods and redundant horizontal concatenation arguments. The pack also reports the legacy string dtype spelling and unstable Avro APIs.

```diff lang="python"
-series.cut([1.5])
+series.bin_intervals([1.5], labels=["low", "high"], right_closed=True)
```

## What the rule reports

| Form | Replacement example | Proof on 2.1.0 |
| --- | --- | --- |
| `series.cut([1.5])` | `series.bin_intervals([1.5], labels=["low", "high"], right_closed=True)` | DeprecationWarning for both methods |
| `pl.concat([df, df.rename({"x": "z", "y": "w"})], how="horizontal", strict=True)` | `pl.concat([df, df.rename({"x": "z", "y": "w"})], how="horizontal")` | DeprecationWarning on concat and union |
| `pl.Utf8` | `pl.String` | Identical dtype object |
| `pl.read_avro("data.avro")` | `pl.read_parquet("data.parquet")` | UnstableWarning with warn_unstable=True |
| `df.write_avro("data.avro")` | `df.write_parquet("data.parquet")` | UnstableWarning with warn_unstable=True |

## How to fix it

Use `xpd check --fix` to apply the safe rewrite when the report offers one.
Edit forms without a rewrite using their replacement examples.

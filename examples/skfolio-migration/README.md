# Reimplement two skfolio workflows in native Polars

Run the replay from the root of the xpd checkout with uv and Python 3.14.

```sh
uv run --no-project examples/skfolio-migration/replay.py
```

The replay obtains the datasets from the pinned skfolio package, checks the input
contracts, and runs the native command in a separate environment without pandas
or skfolio. It compares the persisted Parquet outputs with the pinned baseline
and prints the results as JSON.

- [Returns manifest](returns.manifest.json) specifies the input, output schema,
  factor joins, and missing-price policies.
- [Walk-forward manifest](walk-forward.manifest.json) specifies fold boundaries,
  trade costs, and weight and return tolerances.
- [Map report](map-evidence.json) records the planner's partial result for
  skfolio's upstream preprocessing function.
- [Replay evidence](evidence.json) records a verified run. Returns matched for
  8,312 rows, and the backtests matched within tolerance for 12 observation-based
  and 58 Friday-calendar portfolios.

The portfolio example covers fully invested, long-only minimum variance. It
uses explicit date columns and CLARABEL. See the manifests for the scope of the
reimplementation and the remaining cases outside that scope.

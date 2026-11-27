---
title: Review time series before forecasting
description: Measure cadence, intermittency, seasonality and group coherence with a recorded xpd ts-review ledger.
---

Use `xpd ts-review` to decide which patterns are supported by the observations and which series belong together. Write a `series()` adapter that returns the observations and the metadata you want to compare.

1. Run `xpd ts-review start forecast.py --check "python -m py_compile forecast.py" --adapter series.py`. Inspect the bare list with `xpd ts-review forecast.py --adapter series.py --format json`.
2. Check what `ds` represents. Read missing and duplicate stamps before interpreting periods. Use ADI and CV squared to describe intermittent demand. Look for seasonal phase, breaks and changes in variance. Treat these measurements as evidence about the series.
3. Compare period and phase within groups. Read metadata eta squared, silhouette and cluster assignments. Record whether each entity should be independent, pooled or reconciled in a hierarchy. Name the calendar drivers and what changed around candidate breaks.
4. Write a plan and run `xpd ts-review plan plan.json`. Record checklist answers with reasons. Commit one fix at a time and run `xpd ts-review gate <fix-id>`. Use `xpd ts-review approve <fix-id>` for an approved interface change and `xpd ts-review drop <fix-id> --reason "<why>"` after restoring an abandoned fix.
5. Check progress with `xpd ts-review status` and close the ledger with `xpd ts-review finish`.

See the [command reference](../../commands/ts-review/) for adapter inputs and plan stages.

The sample forecast file has four review findings.

```console
$ xpd ts-review samples/tsreview/faults.py --format json | jq -c '[.findings[].kind]'
["random-time-split","mape-on-intermittent","ungrouped-lag","missing-future-exog"]
```

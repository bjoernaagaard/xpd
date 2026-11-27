---
name: xpd-ts-review
description: Inspect time series cadence, intermittency, seasonality, structural breaks, and group coherence before forecasting in a recorded xpd ts-review ledger.
---

# Review time series before forecasting with xpd

Use `xpd ts-review` to diagnose time series structure, seasonal drivers, regime breaks, and pooling or hierarchy groups before choosing a forecasting model.

| Group | Example finding kinds |
| --- | --- |
| Evaluation | `random-time-split`, `mape-on-intermittent`, `mase-without-train-df`, `intervals-without-calibration`, `unscaled-returns` |
| Lags and exogenous features | `ungrouped-lag`, `missing-future-exog`, `mintrace-without-insample`, `fitted-values-without-fitted`, `indptr-missing-zero` |
| Grouping and identity | `not-a-time-series`, `incoherent-group`, `time-before-entity-index`, `panel-ols-without-effects`, `clustered-without-clusters` |

Run `xpd ts-review --questions --format json` for the full 14-fact and 12-question catalog. After recording a strategy, read the forecasting playbook with `xpd api xpd/recipe/nixtla-forecasting`.

## Adapter

Pass `--adapter series.py` where `series()` returns a pandas or Polars frame, a data file path, or a dict of columns holding `unique_id`, `ds`, `y`, and candidate group metadata columns. Return `{"data": frame, "periods": {...}, "hierarchy": {...}}` to declare explicit periods or hierarchy parents.

## Recorded run

1. **Start**: run `xpd ts-review start <paths> --check "<project-check>" --adapter series.py` from a clean commit.
2. **List**: run `xpd ts-review <paths> --adapter series.py --format json`.
3. **Judge**: inspect cadence, missing stamps, ADI and CV squared intermittency, seasonal periods, phase alignment, breaks, metadata eta squared, silhouette scores, and cluster assignments. Record one modeling strategy per series or group (independent, pooled, or hierarchical) and name the calendar or physical drivers.
4. **Plan**: write `plan.json` with ordered `fixes` (mechanical `findings` or `"goal": "design"` with `questions`), `accepted` findings, and `answered` verdicts (`pass`, `na`, or `accepted`) with reasons. When history covers fewer than two cycles of a candidate period, drop that period and answer `na` with that reason. Run `xpd ts-review plan plan.json`.
5. **Approve and gate**: run `xpd ts-review approve <fix-id>` for approved interface changes, commit one fix, and run `xpd ts-review gate <fix-id>`.
6. **Drop, status, and finish**: restore an abandoned edit before `xpd ts-review drop <fix-id> --reason "<why>"`, resume with `xpd ts-review status`, and close with `xpd ts-review finish`.

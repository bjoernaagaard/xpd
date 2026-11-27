---
name: xpd-feature-review
description: Research and review how each model feature relates to its target, what is known at prediction time, and whether the feature adds lift in development folds with xpd feature-review. Use for feature engineering, lags, rolling windows, target encoding, Hamilton lineage, and feature ablation.
---

# Review model features with xpd

Use `xpd feature-review` to ground every model feature in a clear domain mechanism, prove its publication cutoff, check fold stability, and measure paired ablation lift on development folds before touching a holdout set.

| Group | Example finding kinds |
| --- | --- |
| Cutoff and folds | `encode-before-split`, `fit-before-split`, `scaler-fit-before-split`, `target-encoder-before-outer-cv`, `rolling-includes-current`, `rolling-crosses-groups`, `transform-with-test-target` |
| Computation | `duplicate-feature-computation`, `node-mutates-input`, `frame-column-write`, `nan-into-pipeline`, `zero-regularization` |
| Pipeline | `cols-left-to-dtype`, `named-cols-after-array-step`, `stateful-function-transformer`, `unseeded-random-estimator`, `score-on-training-rows` |

Run `xpd feature-review --questions --format json` for the full fact and question catalog, and `xpd api xpd/recipe/feature-module` for the one-function-per-feature house style.

## Manifest and adapter

- **Manifest (`feature-review.json` at Git root)**: declares `target`, `prediction_cutoff`, `development` snapshot and holdout boundary, `baseline` columns, raw `inputs`, and every implemented `feature` or deferred `hypothesis` with a stable ID and owning file.
- **Development-only adapter (`--adapter adapter.py`)**: defines `review()` returning the development DataFrame, chronological or group `splits`, `mode`, `build_features`, `make_estimator`, and `score`. Never include the final holdout set.

## Recorded run

1. **Inspect and research**: profile raw inputs with `xpd profile`, search APIs with `xpd find`, and research the domain mechanism from primary sources (using `library_search` and `library_read` when a local paper library is connected). Record expected direction, units, publication lag, and failure regimes.
2. **Start and list**: commit the manifest, sources, and adapter, then run `xpd feature-review start <paths> --adapter adapter.py --check "<project-check>"` and `xpd feature-review <paths> --adapter adapter.py --format json`.
3. **Trace and verify**: inspect lineage with `xpd graph`, verify rewiring on nulls and multiple entities with `xpd equiv old.py new.py`, and compare paired fold lift, stability, correlation clusters, and unseen categories. Keep low-lift features only with a source-backed domain reason tied to measured lift.
4. **Plan**: run `xpd feature-review plan plan.json` with ordered `fixes`, `accepted` findings, and `answered` verdicts (`pass`, `na`, or `accepted`) with evidence.
5. **Approve and gate**: run `xpd feature-review approve <fix-id>` for approved interface changes, commit one fix, and run `xpd feature-review gate <fix-id>`. A kept gate reopens checklist answers whose measured evidence changed.
6. **Drop, status, and finish**: restore an abandoned edit before `xpd feature-review drop <fix-id> --reason "<why>"`, check `xpd feature-review status`, record any remaining answers, and run `xpd feature-review finish`.

## Example mechanism

Day-ahead residual load equals forecasted demand minus forecasted wind and solar generation, approximating the load thermal suppliers must cover. Compare load and wind forecasts in the same units and verify that both vintages publish before the auction cutoff. Available generation capacity and fuel prices can shift the slope, so check local operating regimes rather than assuming a constant effect.

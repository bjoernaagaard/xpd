# Review model validation and calibration

Use `xpd model-review` to examine development folds, baseline comparisons, test-set tuning, classifier probability calibration and conformal interval coverage.

## Asks

- review model validation and calibration
- audit test-set tuning and early stopping
- compare validation scores with a naive baseline
- check model interval coverage on held-out data

## Code

```python
from sklearn.model_selection import TimeSeriesSplit


def development_folds(features):
    splitter = TimeSeriesSplit(n_splits=3)
    folds = []
    for number, (fit, validation) in enumerate(splitter.split(features), start=1):
        folds.append({"id": str(number), "fit": fit, "validation": validation})
    return folds
```

## Commands

1. Run `xpd model-review <paths> --limit 0 --format json`. Add `--adapter adapter.py` to measure declared development folds. Keep the final test set out of those folds.
2. Commit the code and adapter inputs. Run `xpd model-review start <paths> --check "git diff --check" --adapter adapter.py`. Repeat the original list with the adapter and read each model's findings, measurements and questions.
3. Run `xpd model-review plan plan.json` to record fixes, accepted findings and reasoned answers. Get interface approval through `xpd model-review approve <fix>` when needed.
4. Commit one fix and run `xpd model-review gate <fix>`. Restore abandoned work and use `xpd model-review drop <fix> --reason "<reason>"`. Use `xpd model-review status` for the next stage.
5. Use `xpd model-review finish` when the planned fixes and current checklist are settled.

For compatible contracts, cite the actual `discovery_gate` and `gates` report from `xpd mcp`. Use `xpd refit model/contract.json` for saved-model agreement when that is the only pipeline proof needed. A high validation score cannot supply these verdicts.

## Rules

- sklearn/tune-on-test
- xgboost/early-stop-on-test
- lightgbm/early-stop-on-test
- mapie/calibrate-on-training-rows

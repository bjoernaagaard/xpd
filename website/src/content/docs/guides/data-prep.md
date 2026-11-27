---
title: Prepare features and targets
description: Record a data preparation review from the raw input through feature and target scaling.
sidebar:
  order: 7
---

Start with a committed preparation script, a reproducible project check, and an adapter for the data the script reads. The adapter below loads a CSV next to the adapter file.

```python
from pathlib import Path

model_family = "linear"
target = "sale_price"

def columns():
    return Path(__file__).with_name("sales.csv")
```

1. Start with `xpd data-prep start prep.py --check "python prep.py" --adapter columns.py`. List with `xpd data-prep prep.py --adapter columns.py --format json`.
2. Review the graph and the measured columns. Confirm what sentinels, nulls and outliers mean before changing values. Check join cardinality and duplicate rows, and choose a time or group split when rows are dependent.
3. Record findings and checklist decisions with `xpd data-prep plan plan.json`. Each answer has an `item` such as `amount/outlier-meaning`, a `verdict` of `pass`, `na` or `accepted`, and a `reason`. Design fixes cite item IDs in `questions` and use `"goal": "design"`.
4. For an approved interface change, run `xpd data-prep approve <fix-id>`. Commit the fix and run `xpd data-prep gate <fix-id>`. Restore an abandoned change before running `xpd data-prep drop <fix-id> --reason "<why>"`.
5. Run `xpd data-prep status` to see what needs attention and `xpd data-prep finish` to close the run once fixes and checklist items are resolved.

Fit scalers and imputers inside a pipeline or on training rows only. For example, the adapter suggests `RobustScaler` when a feature has heavy tails, and can suggest a `TransformedTargetRegressor` for a right-skewed nonnegative target. Check the transformed target against the loss in the original units. A pure tree adapter gets no scaling recommendation.

See the [command reference](../../commands/data-prep/) for adapter inputs and the stage commands.

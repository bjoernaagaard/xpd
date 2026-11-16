"""Contrast block resampling with forward validation on made up ordered rows."""

import numpy as np
from arch.bootstrap import CircularBlockBootstrap, MovingBlockBootstrap, StationaryBootstrap
from mapie.subsample import BlockBootstrap
from sklearn.model_selection import TimeSeriesSplit

values = np.arange(48, dtype=float)
stationary = StationaryBootstrap(4, values, seed=0)
circular = CircularBlockBootstrap(4, values, seed=0)
moving = MovingBlockBootstrap(4, values, seed=0)
bootstrap = BlockBootstrap(n_resamplings=3, length=4, overlapping=True, random_state=0)
forward = TimeSeriesSplit(n_splits=3, test_size=8, gap=2)
result = {
    "arch": [next(item.bootstrap(1))[0][0] for item in (stationary, circular, moving)],
    "bootstrap": list(bootstrap.split(values)),
    "forward": list(forward.split(values)),
}
print(f"arch_lengths={[len(rows) for rows in result['arch']]}")
print(f"bootstrap_folds={len(result['bootstrap'])}, forward_folds={len(result['forward'])}")
print(f"forward_bounds={[(int(a[-1]), int(b[0])) for a, b in result['forward']]}")

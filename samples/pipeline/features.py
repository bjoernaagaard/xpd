"""Feature steps of the penguin model, for `xpd graph features.py`.

Each step reads only the row it is given, so it fits nothing and can run on any rows. The
training script puts `add_features` first in its Pipeline, so the saved model takes the raw
columns.

    add_features(penguins)
    ├── mass()      body mass in kilograms
    └── ratios()    bill shape, and flipper length per kilogram
"""

import pandas as pd

GRAMS_PER_KILOGRAM = 1000


def mass(penguins: pd.DataFrame) -> pd.DataFrame:
    return penguins.assign(body_mass_kg=penguins["body_mass_g"] / GRAMS_PER_KILOGRAM)


def ratios(penguins: pd.DataFrame) -> pd.DataFrame:
    return penguins.assign(
        bill_ratio=penguins["bill_length_mm"] / penguins["bill_depth_mm"],
        flipper_per_kg=penguins["flipper_length_mm"] / penguins["body_mass_kg"],
    )


def add_features(penguins: pd.DataFrame) -> pd.DataFrame:
    return penguins.pipe(mass).pipe(ratios)

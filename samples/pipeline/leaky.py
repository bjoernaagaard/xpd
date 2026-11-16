"""The training script of train.py with one leak, to show a gate of docs/code-mode.md failing.

It picks the regularization strength by the score on the held out rows, so the score it reports
for those rows is too high. `xpd check leaky.py` reports the loop as sklearn/tune-on-test. It
saves its model and contract where train.py does, so run train.py again afterwards.
"""

import json
import platform
from pathlib import Path

import joblib
import numpy as np
import pandas as pd
import sklearn
from features import add_features
from sklearn.compose import ColumnTransformer
from sklearn.impute import SimpleImputer
from sklearn.linear_model import LogisticRegression
from sklearn.model_selection import StratifiedKFold, cross_val_score, train_test_split
from sklearn.pipeline import Pipeline, make_pipeline
from sklearn.preprocessing import FunctionTransformer, OneHotEncoder, StandardScaler

DATA = Path("penguins.csv")
MODEL = Path("model/model.joblib")
CONTRACT = Path("model/contract.json")
TARGET = "species"
CATEGORICAL = ["island", "sex"]
MEASURED = ["bill_length_mm", "bill_depth_mm", "flipper_length_mm", "body_mass_g"]
DERIVED = ["body_mass_kg", "bill_ratio", "flipper_per_kg"]
SEED = 0
TEST_SIZE = 0.25
FOLDS = 5
SCORING = "accuracy"

penguins = pd.read_csv(DATA)
X = penguins[CATEGORICAL + MEASURED]
y = penguins[TARGET]
X_train, X_test, y_train, y_test = train_test_split(
    X, y, test_size=TEST_SIZE, random_state=SEED, stratify=y
)


def pipeline(strength: float) -> Pipeline:
    columns = ColumnTransformer(
        [
            (
                "numeric",
                make_pipeline(SimpleImputer(strategy="median"), StandardScaler()),
                MEASURED + DERIVED,
            ),
            (
                "categorical",
                make_pipeline(
                    SimpleImputer(strategy="most_frequent"), OneHotEncoder(handle_unknown="ignore")
                ),
                CATEGORICAL,
            ),
        ]
    )
    return Pipeline(
        [
            ("features", FunctionTransformer(add_features)),
            ("columns", columns),
            ("classifier", LogisticRegression(C=strength, max_iter=1000)),
        ]
    )


best_score, best_strength = 0.0, 1.0
for strength in (0.01, 0.1, 1.0, 10.0):  # xpd: tune-on-test
    candidate = pipeline(strength=strength)
    candidate.fit(X_train, y_train)
    score = candidate.score(X_test, y_test)
    if score > best_score:
        best_score, best_strength = score, strength
model = pipeline(strength=best_strength)

folds = StratifiedKFold(FOLDS, shuffle=True, random_state=SEED)
cv_scores = cross_val_score(model, X_train, y_train, cv=folds, scoring=SCORING)
model.fit(X_train, y_train)
test_score = model.score(X_test, y_test)

MODEL.parent.mkdir(exist_ok=True)
joblib.dump(model, MODEL)
contract = {
    "data": {"file": DATA.as_posix(), "target": TARGET, "rows": len(penguins)},
    "split": {
        "seed": SEED,
        "test_size": TEST_SIZE,
        "stratified": True,
        "train_rows": len(X_train),
        "test_rows": len(X_test),
    },
    "inputs": [{"column": name, "dtype": str(dtype)} for name, dtype in X.dtypes.items()],
    "steps": [{"name": name, "estimator": type(step).__name__} for name, step in model.steps],
    "validation": {
        "scoring": SCORING,
        "folds": FOLDS,
        "cv_scores": cv_scores.round(4).tolist(),
        "test_score": round(test_score, 4),
    },
    "model": {
        "file": MODEL.as_posix(),
        "type": f"{type(model).__module__}.{type(model).__name__}",
        "versions": {
            "python": platform.python_version(),
            "scikit-learn": sklearn.__version__,
            "pandas": pd.__version__,
            "numpy": np.__version__,
            "joblib": joblib.__version__,
        },
    },
}
CONTRACT.write_text(json.dumps(contract, indent=2) + "\n")

result = pd.Series(model.predict(X_test), index=X_test.index, name=TARGET)

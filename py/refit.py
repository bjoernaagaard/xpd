"""Refit a saved Pipeline on raw training rows with the project's Python."""

import contextlib
import json
import sys
from pathlib import Path

try:
    import joblib
    import numpy as np
    import pandas as pd
    from numpy import testing as npt
    from pandas import testing as pdt
    from sklearn import model_selection
    from sklearn.base import clone
    from sklearn.metrics import get_scorer
    from sklearn.model_selection import cross_val_score, train_test_split
    from sklearn.pipeline import Pipeline
except ImportError as error:
    missing = {"error": f"Refit needs pandas and scikit-learn in the project's Python. {error}"}
    print(json.dumps(missing))
    sys.exit(2)


def matches(old: object, new: object) -> bool:
    try:
        if isinstance(old, pd.DataFrame):
            pdt.assert_frame_equal(old, new)
        elif isinstance(old, pd.Series):
            pdt.assert_series_equal(old, new)
        else:
            npt.assert_equal(new, old)
    except AssertionError:
        return False
    return True


def inside(raw: str) -> Path:
    path = Path(raw).resolve()
    if not path.is_relative_to(Path.cwd().resolve()):
        msg = f"{raw} is outside the project root."
        raise ValueError(msg)
    return path


def refit(file: str) -> dict:
    contract = json.loads(inside(file).read_text(encoding="utf-8"))
    for code in contract["model"].get("code", []):
        if not inside(code).is_file():
            msg = f"The model imports {code}, which is missing."
            raise FileNotFoundError(msg)
    data = inside(contract["data"]["file"])
    readers = {
        ".csv": pd.read_csv,
        ".parquet": pd.read_parquet,
        ".feather": pd.read_feather,
        ".json": pd.read_json,
        ".xlsx": pd.read_excel,
    }
    reader = readers.get(data.suffix.lower())
    if reader is None:
        msg = f"unsupported file type {data.suffix!r}"
        raise ValueError(msg)
    rows = reader(data)
    inputs = rows[[item["column"] for item in contract["inputs"]]]
    target = rows[contract["data"]["target"]]
    split = contract["split"]
    train, held, train_y, held_y = train_test_split(
        inputs,
        target,
        test_size=split["test_size"],
        random_state=split["seed"],
        stratify=target if split["stratified"] else None,
    )
    saved = joblib.load(inside(contract["model"]["file"]))
    if not isinstance(saved, Pipeline):
        msg = "The saved object is not a sklearn.pipeline.Pipeline."
        raise TypeError(msg)
    fitted = clone(saved).fit(train, train_y)
    methods = ("predict_proba", "decision_function", "predict")
    output = next(name for name in methods if hasattr(saved, name))
    old, new = getattr(saved, output)(held), getattr(fitted, output)(held)
    left, right = np.asarray(old), np.asarray(new)
    gap = None
    if left.shape == right.shape and left.dtype.kind in "biufc" and right.dtype.kind in "biufc":
        dtype = np.result_type(left.dtype, right.dtype, np.float64)
        gap = float(np.max(np.abs(np.subtract(left, right, dtype=dtype)), initial=0))
    validation = contract["validation"]
    spec = validation["splitter"]
    splitter_class = getattr(model_selection, spec["class"], None)
    if not isinstance(splitter_class, type) or not hasattr(splitter_class, "split"):
        msg = f"{spec['class']} is not a splitter class of sklearn.model_selection."
        raise ValueError(msg)
    folds = splitter_class(**spec["args"])
    scores = cross_val_score(
        clone(saved), train, train_y, cv=folds, scoring=validation["scoring"], error_score="raise"
    ).tolist()
    score = float(get_scorer(validation["scoring"])(fitted, held, held_y))
    model_matches = matches(old, new)
    test_matches = matches(validation["test_score"], score)
    cv_matches = matches(validation["cv_scores"], scores)
    counts_match = (
        len(rows) == contract["data"]["rows"]
        and len(train) == split["train_rows"]
        and len(held) == split["test_rows"]
        and len(scores) == validation["folds"]
    )
    return {
        "rows": len(rows),
        "train_rows": len(train),
        "test_rows": len(held),
        "output": output,
        "largest_gap": gap,
        "model_matches": model_matches,
        "contract_test_score": validation["test_score"],
        "refit_test_score": score,
        "contract_cv_scores": validation["cv_scores"],
        "refit_cv_scores": scores,
        "test_score_matches": test_matches,
        "cv_scores_match": cv_matches,
        "counts_match": counts_match,
        "passed": model_matches and test_matches and cv_matches and counts_match,
    }


def main() -> None:
    try:
        with contextlib.redirect_stdout(sys.stderr):
            result = refit(sys.argv[1])
        print(json.dumps(result, allow_nan=False))
    except Exception as error:  # noqa: BLE001  the command reports project code failures
        print(json.dumps({"error": f"{type(error).__name__}. {error}"}))
        sys.exit(2)
    sys.exit(0 if result["passed"] else 1)


if __name__ == "__main__":
    main()

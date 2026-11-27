"""Measure candidate features in development folds."""

from __future__ import annotations

import hashlib
import json
import random
import runpy
import sys
from contextlib import redirect_stdout
from itertools import combinations
from pathlib import Path
from typing import Any

import numpy as np
import pandas as pd

MINIMUM_SPLITS = 2
MINIMUM_PAIRS = 3
CORRELATION_THRESHOLD = 0.95


def fingerprint(value: object) -> str:
    encoded = json.dumps(value, default=str, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def require(condition: object, reason: str) -> None:
    if not condition:
        raise ValueError(reason)


def split_evidence(
    frame: pd.DataFrame, train: pd.Index, validation: pd.Index, config: dict[str, Any]
) -> dict[str, Any]:
    require(len(train) > 0 and len(validation) > 0, "inner fold has an empty side")
    require(train.is_unique and validation.is_unique, "inner fold contains duplicate row IDs")
    require(
        train.isin(frame.index).all() and validation.isin(frame.index).all(),
        "inner fold contains unknown row IDs",
    )
    require(train.intersection(validation).empty, "inner fold train and validation rows overlap")
    evidence: dict[str, Any] = {"overlap": 0}
    mode = config["mode"]
    if mode in ("time", "both"):
        prediction = pd.to_datetime(frame[config["prediction_time"]], utc=True)
        published = pd.to_datetime(frame[config["label_available_time"]], utc=True)
        cutoff = prediction.loc[validation].min()
        require(
            prediction.notna().all()
            and prediction.is_monotonic_increasing
            and prediction.loc[train].is_monotonic_increasing
            and prediction.loc[validation].is_monotonic_increasing,
            "prediction timestamps must be present and sorted",
        )
        latest_train = prediction.loc[train].max()
        require(
            latest_train < cutoff, "inner training prediction timestamps reach validation cutoff"
        )
        train_labels = published.loc[train]
        latest_label = train_labels.max()
        require(
            train_labels.notna().all() and latest_label <= cutoff,
            "inner training label was unavailable at validation cutoff",
        )
        evidence.update(
            train_latest=latest_train.isoformat(),
            validation_cutoff=cutoff.isoformat(),
            train_label_latest=latest_label.isoformat(),
        )
    if mode in ("group", "both"):
        groups = frame[config["group"]]
        tr, va = groups.loc[train], groups.loc[validation]
        require(tr.notna().all() and va.notna().all(), "inner fold has a missing group")
        require(set(tr).isdisjoint(va), "inner training and validation groups overlap")
        evidence["group_overlap"] = 0
    return evidence


def unseen_levels(train: pd.Series, validation: pd.Series) -> dict[str, Any]:
    known = train.dropna()
    observed = validation.dropna()
    count = int((~observed.isin(known)).sum())
    return {
        "numerator": count,
        "denominator": len(observed),
        "rate": count / len(observed) if len(observed) else None,
        "missing": len(validation) - len(observed),
        "reason": None if len(observed) else "No nonmissing validation levels",
    }


def training_stability(values: pd.Series, *, categorical: bool) -> tuple[dict[str, Any], pd.Series]:
    usable = values.dropna() if categorical else values.replace([np.inf, -np.inf], np.nan).dropna()
    result = {"usable_count": len(usable), "missing_share": 1 - len(usable) / len(values)}
    if categorical:
        return {**result, "levels": usable.nunique()}, usable
    return {
        **result,
        "median": float(usable.median()) if len(usable) else None,
        "iqr": float(usable.quantile(0.75) - usable.quantile(0.25)) if len(usable) else None,
        "reason": None if len(usable) else "No finite training values",
    }, usable


def collinearity(numeric: dict[str, pd.Series]) -> dict[str, Any]:
    edges = []
    components: list[set[str]] = []
    for left, right in combinations(sorted(numeric), 2):
        pairs = pd.concat([numeric[left], numeric[right]], axis=1, join="inner")
        if len(pairs) < MINIMUM_PAIRS or min(pairs.nunique()) < MINIMUM_SPLITS:
            continue
        ranks = pairs.rank()
        coefficient = float(np.clip(ranks.iloc[:, 0].corr(ranks.iloc[:, 1]), -1, 1))
        if abs(coefficient) < CORRELATION_THRESHOLD:
            continue
        edges.append(
            {"left": left, "right": right, "coefficient": coefficient, "pairs": len(pairs)}
        )
        component = {left, right}
        disjoint = []
        for previous in components:
            if previous.isdisjoint(component):
                disjoint.append(previous)
            else:
                component.update(previous)
        components = [*disjoint, component]
    return {"edges": edges, "components": sorted(sorted(component) for component in components)}


def score_variant(
    config: dict[str, Any],
    columns: list[str],
    train: tuple[pd.DataFrame, pd.Series],
    validation: tuple[pd.DataFrame, pd.Series],
) -> float:
    random.seed(0)
    estimator = config["make_estimator"](columns)
    train_features, train_y = train
    validation_features, validation_y = validation
    estimator.fit(train_features.loc[:, columns], train_y)
    return float(config["score"](estimator, validation_features.loc[:, columns], validation_y))


def measure_fold(
    config: dict[str, Any],
    manifest: dict[str, Any],
    frame: pd.DataFrame,
    train: pd.Index,
    validation: pd.Index,
) -> dict[str, Any]:
    target = manifest["target"]
    train_y, validation_y = frame.loc[train, target], frame.loc[validation, target]
    predictors = frame.drop(columns=target)
    random.seed(0)
    tr, va, categories = config["build_features"](
        predictors.loc[train], train_y.copy(), predictors.loc[validation]
    )
    require(
        tr.index.equals(train) and va.index.equals(validation),
        "feature matrices changed fold row IDs",
    )
    require(
        tr.columns.is_unique and va.columns.is_unique, "feature matrices have duplicate columns"
    )
    baseline = manifest["baseline"]
    baseline_score = score_variant(config, baseline, (tr, train_y), (va, validation_y))
    stability, unseen, numeric, ablation = {}, {}, {}, {}
    for feature in manifest["features"]:
        if feature["kind"] != "feature":
            continue
        feature_id, column = feature["id"], feature["column"]
        categorical = feature_id in categories
        if categorical:
            raw_train, raw_validation = categories[feature_id]
            require(
                raw_train.index.equals(train) and raw_validation.index.equals(validation),
                "categorical levels changed fold row IDs",
            )
            unseen[feature_id] = unseen_levels(raw_train, raw_validation)
        else:
            raw_train = tr[column]
            require(
                pd.api.types.is_numeric_dtype(raw_train), "categorical feature needs raw levels"
            )
        stability[feature_id], usable = training_stability(raw_train, categorical=categorical)
        if not categorical:
            numeric[feature_id] = usable
        score = score_variant(config, [*baseline, column], (tr, train_y), (va, validation_y))
        improvement = (
            baseline_score - score if config["direction"] == "loss" else score - baseline_score
        )
        ablation[feature_id] = {"score": score, "improvement": improvement}
    return {
        "baseline_score": baseline_score,
        "stability": stability,
        "unseen": unseen,
        "clusters": collinearity(numeric),
        "ablation": ablation,
    }


def measure_features(config: dict[str, Any], manifest: dict[str, Any]) -> dict[str, Any]:
    frame = config["development"]
    require(frame.index.is_unique and not frame.index.hasnans, "development needs unique row IDs")
    require(frame.columns.is_unique, "development has duplicate columns")
    mode, direction = config["mode"], config["direction"]
    require(mode in ("time", "group", "both", "iid"), "unsupported inner split mode")
    require(
        mode != "iid" or bool(config.get("justification", "").strip()), "IID needs justification"
    )
    require(direction in ("loss", "gain"), "metric direction must be loss or gain")
    splits = [(pd.Index(train), pd.Index(validation)) for train, validation in config["splits"]]
    memberships = [(train.tolist(), validation.tolist()) for train, validation in splits]
    require(
        len({fingerprint(pair) for pair in memberships}) == len(splits) >= MINIMUM_SPLITS,
        "development needs at least two distinct inner splits",
    )
    boundaries = [split_evidence(frame, train, validation, config) for train, validation in splits]
    development_hash = pd.util.hash_pandas_object(frame, index=True).tolist()
    return {
        "schema_version": 1,
        "protocol": {
            "development_fingerprint": fingerprint([frame.columns.tolist(), development_hash]),
            "split_fingerprint": fingerprint(memberships),
            "development_rows": len(frame),
            "mode": mode,
            "baseline": manifest["baseline"],
            "metric": config["metric"],
            "direction": direction,
            "justification": config.get("justification"),
        },
        "folds": [
            {
                "id": index,
                "train_count": len(train),
                "validation_count": len(validation),
                "boundary": boundary,
                **measure_fold(config, manifest, frame, train, validation),
            }
            for index, ((train, validation), boundary) in enumerate(
                zip(splits, boundaries, strict=True), start=1
            )
        ],
    }


def main() -> None:
    adapter = Path(sys.argv[1])
    sys.path.insert(0, str(adapter.parent))
    with redirect_stdout(sys.stderr):
        manifest = json.loads(Path("feature-review.json").read_text(encoding="utf-8"))
        config = runpy.run_path(str(adapter))["review"]()
        facts = measure_features(config, manifest)
    print(json.dumps(facts, allow_nan=False))


if __name__ == "__main__":
    try:
        main()
    except (AttributeError, ImportError, KeyError, OSError, TypeError, ValueError) as error:
        print(json.dumps({"error": str(error)}))
        sys.exit(2)
